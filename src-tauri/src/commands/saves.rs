use std::path::{Path, PathBuf};

use selene_core::saves::{self, Screenshot, Server, World};
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldList {
    worlds: Vec<World>,
    /// Whether this game version can open a world or server right after it starts.
    quick_play: bool,
}

#[tauri::command]
pub async fn worlds(
    launcher: State<'_, Launcher>,
    instance_id: String,
) -> CommandResult<WorldList> {
    let instance = launcher
        .instances
        .get(&instance_id)
        .await
        .ok_or_else(|| CommandError::UnknownInstance(instance_id.clone()))?;
    let game_dir = launcher.instances.game_dir(&instance_id);
    let worlds = blocking(move || saves::worlds(&game_dir)).await?;
    // An offline launch has no manifest; the server list still works without Quick Play.
    let quick_play = launcher
        .manifest()
        .await
        .is_ok_and(|manifest| manifest.supports_quick_play(&instance.version));
    Ok(WorldList { worlds, quick_play })
}

#[tauri::command]
pub async fn open_world_folder(
    app: AppHandle,
    launcher: State<'_, Launcher>,
    instance_id: String,
    folder: String,
) -> CommandResult<()> {
    let dir = saves::world_dir(&launcher.instances.game_dir(&instance_id), &folder)?;
    open_path(&app, &dir)
}

/// Zips the world next to the instance's other backups and shows the archive in Explorer.
#[tauri::command]
pub async fn back_up_world(
    app: AppHandle,
    launcher: State<'_, Launcher>,
    instance_id: String,
    folder: String,
) -> CommandResult<PathBuf> {
    let game_dir = launcher.instances.game_dir(&instance_id);
    let backup = blocking(move || saves::back_up_world(&game_dir, &folder)).await?;
    app.opener()
        .reveal_item_in_dir(&backup)
        .map_err(|err| CommandError::Io(std::io::Error::other(err)))?;
    Ok(backup)
}

/// Moves the world to the recycle bin, so a misclick can still be undone.
#[tauri::command]
pub async fn delete_world(
    launcher: State<'_, Launcher>,
    instance_id: String,
    folder: String,
) -> CommandResult<()> {
    if launcher.is_running(&instance_id) {
        return Err(CommandError::AlreadyRunning(instance_id));
    }
    let dir = saves::world_dir(&launcher.instances.game_dir(&instance_id), &folder)?;
    move_to_trash(dir).await
}

#[tauri::command]
pub async fn servers(
    launcher: State<'_, Launcher>,
    instance_id: String,
) -> CommandResult<Vec<Server>> {
    let game_dir = launcher.instances.game_dir(&instance_id);
    blocking(move || saves::servers(&game_dir)).await
}

#[tauri::command]
pub async fn save_servers(
    launcher: State<'_, Launcher>,
    instance_id: String,
    servers: Vec<Server>,
) -> CommandResult<()> {
    let game_dir = launcher.instances.game_dir(&instance_id);
    tokio::fs::create_dir_all(&game_dir).await?;
    blocking(move || saves::save_servers(&game_dir, &servers)).await
}

#[tauri::command]
pub async fn screenshots(
    launcher: State<'_, Launcher>,
    instance_id: String,
) -> CommandResult<Vec<Screenshot>> {
    let game_dir = launcher.instances.game_dir(&instance_id);
    blocking(move || saves::screenshots(&game_dir)).await
}

#[tauri::command]
pub async fn open_screenshot(
    app: AppHandle,
    launcher: State<'_, Launcher>,
    instance_id: String,
    name: String,
) -> CommandResult<()> {
    let path = saves::screenshot_path(&launcher.instances.game_dir(&instance_id), &name)?;
    open_path(&app, &path)
}

#[tauri::command]
pub async fn delete_screenshot(
    launcher: State<'_, Launcher>,
    instance_id: String,
    name: String,
) -> CommandResult<()> {
    let path = saves::screenshot_path(&launcher.instances.game_dir(&instance_id), &name)?;
    move_to_trash(path).await
}

pub(crate) fn open_path(app: &AppHandle, path: &Path) -> CommandResult<()> {
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|err| CommandError::Io(std::io::Error::other(err)))
}

async fn move_to_trash(path: PathBuf) -> CommandResult<()> {
    tokio::task::spawn_blocking(move || trash::delete(&path))
        .await
        .map_err(std::io::Error::other)?
        .map_err(|err| CommandError::Io(std::io::Error::other(err)))
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> selene_core::Result<T> + Send + 'static,
) -> CommandResult<T> {
    Ok(tokio::task::spawn_blocking(work)
        .await
        .map_err(std::io::Error::other)??)
}
