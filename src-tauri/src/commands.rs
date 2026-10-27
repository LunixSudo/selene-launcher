#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri hands command arguments over by value"
)]

pub mod mods;
pub mod packs;

use std::process::Stdio;
use std::sync::atomic::{AtomicU32, Ordering};

use chrono::{DateTime, Utc};
use selene_core::download::Progress;
use selene_core::game::{self, Stage};
use selene_core::instance::Instance;
use selene_core::launch::{self, LaunchAccount, LaunchOptions};
use selene_core::loader::{self, Loader, LoaderRelease, LoaderVersion};
use selene_core::manifest::VersionKind;
use selene_core::memory;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::account::{Account, AccountList};
use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionSummary {
    id: String,
    kind: VersionKind,
    release_time: DateTime<Utc>,
}

/// Amounts in megabytes.
#[derive(Debug, Clone, Serialize)]
pub struct MemoryInfo {
    total: u64,
    max: u32,
    recommended: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchProgress<'a> {
    instance_id: &'a str,
    stage: &'static str,
    bytes_done: u64,
    bytes_total: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GameExited {
    instance_id: String,
    code: Option<i32>,
}

#[tauri::command]
pub fn accounts(launcher: State<'_, Launcher>) -> AccountList {
    launcher.accounts.list()
}

/// Opens the Microsoft sign-in page in the browser and waits for the user to finish there.
#[tauri::command]
pub async fn sign_in(app: AppHandle, launcher: State<'_, Launcher>) -> CommandResult<Account> {
    launcher
        .accounts
        .sign_in(&launcher.client, |url| {
            app.opener()
                .open_url(url, None::<&str>)
                .map_err(|err| CommandError::Io(std::io::Error::other(err)))
        })
        .await
}

#[tauri::command]
pub fn cancel_sign_in(launcher: State<'_, Launcher>) {
    launcher.accounts.cancel_sign_in();
}

#[tauri::command]
pub async fn sign_out(launcher: State<'_, Launcher>, id: String) -> CommandResult<()> {
    launcher.accounts.sign_out(&id).await
}

#[tauri::command]
pub fn set_active_account(launcher: State<'_, Launcher>, id: String) -> CommandResult<()> {
    launcher.accounts.set_active(&id)
}

#[tauri::command]
pub async fn versions(launcher: State<'_, Launcher>) -> CommandResult<Vec<VersionSummary>> {
    let manifest = launcher.manifest().await?;
    Ok(manifest
        .versions
        .iter()
        .map(|entry| VersionSummary {
            id: entry.id.clone(),
            kind: entry.kind,
            release_time: entry.release_time,
        })
        .collect())
}

#[tauri::command]
pub async fn loader_versions(
    launcher: State<'_, Launcher>,
    loader: Loader,
    game_version: String,
) -> CommandResult<Vec<LoaderRelease>> {
    Ok(loader::releases(&launcher.client, loader, &game_version).await?)
}

#[tauri::command]
pub async fn instances(launcher: State<'_, Launcher>) -> CommandResult<Vec<Instance>> {
    Ok(launcher.instances.list().await?)
}

#[tauri::command]
pub async fn create_instance(
    launcher: State<'_, Launcher>,
    name: String,
    version: String,
    loader: Option<LoaderVersion>,
) -> CommandResult<Instance> {
    if launcher.manifest().await?.get(&version).is_none() {
        return Err(CommandError::UnknownVersion(version));
    }
    Ok(launcher.instances.create(&name, &version, loader).await?)
}

#[tauri::command]
pub async fn memory_info(
    launcher: State<'_, Launcher>,
    instance_id: String,
) -> CommandResult<MemoryInfo> {
    Ok(MemoryInfo {
        total: launcher.total_memory_mb,
        max: memory::max_mb(launcher.total_memory_mb),
        recommended: recommended_memory(&launcher, &instance_id).await?,
    })
}

/// `None` goes back to the automatic amount.
#[tauri::command]
pub async fn set_instance_memory(
    launcher: State<'_, Launcher>,
    instance_id: String,
    max_memory_mb: Option<u32>,
) -> CommandResult<Instance> {
    let mut instance = launcher
        .instances
        .get(&instance_id)
        .await
        .ok_or_else(|| CommandError::UnknownInstance(instance_id.clone()))?;
    instance.max_memory_mb = max_memory_mb;
    launcher.instances.save(&instance).await?;
    Ok(instance)
}

/// Renames the instance and changes how its card looks.
#[tauri::command]
pub async fn update_instance(
    launcher: State<'_, Launcher>,
    id: String,
    name: String,
    icon: Option<String>,
    cover: Option<String>,
) -> CommandResult<Instance> {
    let mut instance = launcher
        .instances
        .get(&id)
        .await
        .ok_or_else(|| CommandError::UnknownInstance(id.clone()))?;
    let name = name.trim();
    if !name.is_empty() {
        name.clone_into(&mut instance.name);
    }
    instance.icon = icon;
    instance.cover = cover;
    launcher.instances.save(&instance).await?;
    Ok(instance)
}

async fn recommended_memory(launcher: &Launcher, instance_id: &str) -> CommandResult<u32> {
    let mods = selene_core::mods::list(&launcher.instances.game_dir(instance_id)).await?;
    let enabled = mods.iter().filter(|file| file.enabled).count();
    Ok(memory::recommended_mb(launcher.total_memory_mb, enabled))
}

#[tauri::command]
pub async fn open_instance_folder(
    app: AppHandle,
    launcher: State<'_, Launcher>,
    id: String,
) -> CommandResult<()> {
    let dir = launcher.instances.game_dir(&id);
    tokio::fs::create_dir_all(&dir).await?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|err| CommandError::Io(std::io::Error::other(err)))
}

#[tauri::command]
pub async fn delete_instance(launcher: State<'_, Launcher>, id: String) -> CommandResult<()> {
    if launcher.is_running(&id) {
        return Err(CommandError::AlreadyRunning(id));
    }
    Ok(launcher.instances.delete(&id).await?)
}

/// Downloads whatever the instance is missing and starts the game.
///
/// Resolves once the game process has started; `game-exited` is emitted when it closes.
#[tauri::command]
pub async fn launch_instance(
    app: AppHandle,
    launcher: State<'_, Launcher>,
    id: String,
) -> CommandResult<()> {
    let instance = launcher
        .instances
        .get(&id)
        .await
        .ok_or_else(|| CommandError::UnknownInstance(id.clone()))?;
    let account = launcher.accounts.launch_account(&launcher.client).await?;
    if !launcher.start_running(&id) {
        return Err(CommandError::AlreadyRunning(id));
    }

    let result = start_game(&app, &launcher, instance, account).await;
    if result.is_err() {
        launcher.stop_running(&id);
    }
    result
}

async fn start_game(
    app: &AppHandle,
    launcher: &Launcher,
    mut instance: Instance,
    account: LaunchAccount,
) -> CommandResult<()> {
    let entry = launcher
        .manifest()
        .await?
        .get(&instance.version)
        .ok_or_else(|| CommandError::UnknownVersion(instance.version.clone()))?;

    let last_percent = AtomicU32::new(u32::MAX);
    let installed = game::install(
        &launcher.client,
        &launcher.dirs,
        entry,
        instance.loader.as_ref(),
        |stage, progress| {
            // One event per percent is plenty for a progress bar; thousands of small files would
            // otherwise flood the UI.
            let percent = percent(progress);
            if last_percent.swap(percent, Ordering::Relaxed) != percent {
                let _ = app.emit(
                    "launch-progress",
                    LaunchProgress {
                        instance_id: &instance.id,
                        stage: match stage {
                            Stage::Files => "files",
                            Stage::Java => "java",
                        },
                        bytes_done: progress.bytes_done,
                        bytes_total: progress.bytes_total,
                    },
                );
            }
        },
    )
    .await?;

    let dirs = &launcher.dirs;
    let options = LaunchOptions {
        java: installed.java.executable(),
        game_dir: launcher.instances.game_dir(&instance.id),
        assets_dir: dirs.assets(),
        libraries_dir: dirs.libraries(),
        versions_dir: dirs.versions(),
        natives_dir: dirs.natives(&installed.version.id),
        account,
        max_memory_mb: match instance.max_memory_mb {
            Some(amount) => amount,
            None => recommended_memory(launcher, &instance.id).await?,
        },
        resolution: None,
        quick_play: None,
        extra_jvm_args: Vec::new(),
    };
    launch::extract_natives(&installed.version, &options)?;

    let mut child = tokio::process::Command::from(
        launch::build_command(&installed.version, &options).to_command(),
    )
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()
    .map_err(CommandError::Spawn)?;

    instance.last_played = Some(Utc::now());
    launcher.instances.save(&instance).await?;
    let _ = app.emit("game-started", &instance.id);

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let code = child.wait().await.ok().and_then(|status| status.code());
        app.state::<Launcher>().stop_running(&instance.id);
        let _ = app.emit(
            "game-exited",
            GameExited {
                instance_id: instance.id,
                code,
            },
        );
    });
    Ok(())
}

fn percent(progress: Progress) -> u32 {
    let total = progress.bytes_total.max(1);
    u32::try_from(progress.bytes_done.min(total) * 100 / total).unwrap_or(100)
}
