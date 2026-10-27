use std::process::Stdio;
use std::sync::atomic::{AtomicU32, Ordering};

use chrono::{DateTime, Utc};
use selene_core::download::Progress;
use selene_core::game::{self, Stage};
use selene_core::instance::Instance;
use selene_core::launch::{self, LaunchAccount, LaunchOptions};
use selene_core::manifest::VersionKind;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::account;
use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

const DEFAULT_MEMORY_MB: u32 = 4096;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionSummary {
    id: String,
    kind: VersionKind,
    release_time: DateTime<Utc>,
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
pub async fn instances(launcher: State<'_, Launcher>) -> CommandResult<Vec<Instance>> {
    Ok(launcher.instances.list().await?)
}

#[tauri::command]
pub async fn create_instance(
    launcher: State<'_, Launcher>,
    name: String,
    version: String,
) -> CommandResult<Instance> {
    if launcher.manifest().await?.get(&version).is_none() {
        return Err(CommandError::UnknownVersion(version));
    }
    Ok(launcher.instances.create(&name, &version).await?)
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
    let account = account::launch_account()?;
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
        max_memory_mb: instance.max_memory_mb.unwrap_or(DEFAULT_MEMORY_MB),
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
    .spawn()?;

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
