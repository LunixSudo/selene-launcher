use std::path::PathBuf;

use selene_core::cfpack;
use selene_core::instance::Instance;
use selene_core::mrpack::{self, ExportOptions};
use serde::Deserialize;
use tauri::State;

use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackFormat {
    Mrpack,
    Curseforge,
}

/// Creates an instance from a `.mrpack` file. A failed import leaves nothing behind.
#[tauri::command]
pub async fn import_pack(launcher: State<'_, Launcher>, path: PathBuf) -> CommandResult<Instance> {
    let pack = path.clone();
    let index = tokio::task::spawn_blocking(move || mrpack::read_index(&pack))
        .await
        .map_err(|err| CommandError::Io(std::io::Error::other(err)))??;
    let game_version = index.game_version()?.to_owned();
    let loader = index.loader()?;
    if launcher.manifest().await?.get(&game_version).is_none() {
        return Err(CommandError::UnknownVersion(game_version));
    }

    let instance = launcher
        .instances
        .create(&index.name, &game_version, loader)
        .await?;
    let game_dir = launcher.instances.game_dir(&instance.id);
    if let Err(err) = mrpack::install(&launcher.client, &path, &index, &game_dir).await {
        let _ = launcher.instances.delete(&instance.id).await;
        return Err(err.into());
    }
    Ok(instance)
}

/// Writes the instance as a `.mrpack` (Selene, Prism, Modrinth App) or a CurseForge `.zip`
/// (CurseForge App, Prism, `ATLauncher`).
#[tauri::command]
pub async fn export_pack(
    launcher: State<'_, Launcher>,
    instance_id: String,
    path: PathBuf,
    folders: Vec<String>,
    format: PackFormat,
) -> CommandResult<()> {
    let instance = launcher
        .instances
        .get(&instance_id)
        .await
        .ok_or_else(|| CommandError::UnknownInstance(instance_id.clone()))?;
    let options = ExportOptions {
        name: instance.name,
        game_version: instance.version,
        loader: instance.loader,
        folders,
    };
    let game_dir = launcher.instances.game_dir(&instance_id);
    match format {
        PackFormat::Mrpack => mrpack::export(&launcher.client, &game_dir, &options, &path).await?,
        PackFormat::Curseforge => {
            let curseforge = launcher
                .curseforge
                .as_ref()
                .ok_or(CommandError::NoCompatibleVersion)?;
            cfpack::export(curseforge, &game_dir, &options, &path).await?;
        }
    }
    Ok(())
}
