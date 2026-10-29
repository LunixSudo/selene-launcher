#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri hands command arguments over by value"
)]

pub mod content;
pub mod packs;
pub mod privacy;
pub mod saves;
pub mod shortcut;
pub mod skins;
pub mod update;

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use selene_core::crash::{self, CrashReport};
use selene_core::download::Progress;
use selene_core::game::{self, Installed, Stage};
use selene_core::instance::{Instance, WindowSize};
use selene_core::launch::{self, LaunchAccount, LaunchOptions};
use selene_core::loader::{self, Loader, LoaderRelease, LoaderVersion};
use selene_core::manifest::VersionKind;
use selene_core::memory;
use selene_core::mods::{self, ContentKind};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::oneshot;

use crate::account::{Account, AccountList};
use crate::config::Config;
use crate::discord::Presence;
use crate::error::{CommandError, CommandResult};
use crate::state::{Launcher, LogSnapshot};

/// Placeholder tokens like the offline "0" would blank out every matching character.
const MIN_HIDDEN_TOKEN_LEN: usize = 8;
/// A game at start-up prints thousands of lines; the console gets them in batches.
const LOG_FLUSH_INTERVAL: Duration = Duration::from_millis(100);
const LOG_DRAIN_TIMEOUT: Duration = Duration::from_secs(3);

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
    /// Set when the game closed with an error.
    crash: Option<CrashReport>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GameLog<'a> {
    instance_id: &'a str,
    lines: &'a [LogLine],
}

/// A line with its number in the log, so a console that read the saved log can skip the lines
/// it already has.
#[derive(Debug, Clone, Serialize)]
struct LogLine {
    number: u64,
    text: String,
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
    let cache = launcher.dirs.versions().join("loaders");
    Ok(loader::releases(&launcher.client, loader, &game_version, &cache).await?)
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
    let max = memory::max_mb(launcher.total_memory_mb);
    let min = u32::try_from(memory::MIN_MB).unwrap_or(max);
    instance.max_memory_mb = max_memory_mb.map(|amount| amount.clamp(min.min(max), max));
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
    group: Option<String>,
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
    instance.group = group
        .map(|group| group.trim().to_owned())
        .filter(|group| !group.is_empty());
    launcher.instances.save(&instance).await?;
    Ok(instance)
}

#[tauri::command]
pub async fn duplicate_instance(
    launcher: State<'_, Launcher>,
    id: String,
    name: String,
) -> CommandResult<Instance> {
    if launcher.is_running(&id) {
        return Err(CommandError::AlreadyRunning(id));
    }
    Ok(launcher.instances.duplicate(&id, &name).await?)
}

/// Game window and JVM options for one instance.
#[tauri::command]
pub async fn set_launch_options(
    launcher: State<'_, Launcher>,
    instance_id: String,
    window: Option<WindowSize>,
    fullscreen: bool,
    java_args: String,
    java_path: Option<PathBuf>,
) -> CommandResult<Instance> {
    let mut instance = launcher
        .instances
        .get(&instance_id)
        .await
        .ok_or_else(|| CommandError::UnknownInstance(instance_id.clone()))?;
    instance.window = window;
    instance.fullscreen = fullscreen;
    java_args.trim().clone_into(&mut instance.java_args);
    instance.java_path = java_path.filter(|path| !path.as_os_str().is_empty());
    launcher.instances.save(&instance).await?;
    Ok(instance)
}

async fn recommended_memory(launcher: &Launcher, instance_id: &str) -> CommandResult<u32> {
    let mods = mods::list(&launcher.game_dir(instance_id)?, ContentKind::Mod).await?;
    let enabled = mods.iter().filter(|file| file.enabled).count();
    Ok(memory::recommended_mb(launcher.total_memory_mb, enabled))
}

#[tauri::command]
pub async fn open_instance_folder(
    app: AppHandle,
    launcher: State<'_, Launcher>,
    id: String,
) -> CommandResult<()> {
    let dir = launcher.game_dir(&id)?;
    tokio::fs::create_dir_all(&dir).await?;
    saves::open_path(&app, &dir)
}

#[tauri::command]
pub fn set_discord_presence(launcher: State<'_, Launcher>, presence: Option<Presence>) {
    launcher.discord.set(presence);
}

/// Crashes of games that ended while the launcher window was closed, handed out once.
#[tauri::command]
pub fn take_missed_crashes(launcher: State<'_, Launcher>) -> HashMap<String, CrashReport> {
    launcher.take_missed_crashes()
}

/// Games started before the launcher window was opened again.
#[tauri::command]
pub fn running_games(launcher: State<'_, Launcher>) -> Vec<String> {
    launcher.running_games()
}

/// The instance a desktop shortcut asked to start, handed out once.
#[tauri::command]
pub fn take_startup_launch(launcher: State<'_, Launcher>) -> Option<String> {
    launcher.take_startup_launch()
}

#[tauri::command]
pub fn config(launcher: State<'_, Launcher>) -> Config {
    launcher.config.get()
}

#[tauri::command]
pub fn set_config(launcher: State<'_, Launcher>, config: Config) -> CommandResult<()> {
    Ok(launcher.config.set(config)?)
}

#[tauri::command]
pub fn open_data_folder(app: AppHandle, launcher: State<'_, Launcher>) -> CommandResult<()> {
    std::fs::create_dir_all(&launcher.data_dir)?;
    saves::open_path(&app, &launcher.data_dir)
}

#[tauri::command]
pub async fn delete_instance(launcher: State<'_, Launcher>, id: String) -> CommandResult<()> {
    launcher.cancel_preparing(&id).await?;
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
    server: Option<String>,
) -> CommandResult<()> {
    let instance = launcher
        .instances
        .get(&id)
        .await
        .ok_or_else(|| CommandError::UnknownInstance(id.clone()))?;
    let account = launcher.accounts.launch_account(&launcher.client).await?;
    let Some(cancelled) = launcher.start_running(&id, true) else {
        return Err(CommandError::AlreadyRunning(id));
    };

    let result = start_game(&app, &launcher, instance, account, server, cancelled).await;
    if result.is_err() {
        launcher.stop_running(&id);
        // The window may have been closed while the game was being prepared.
        if app.get_webview_window(crate::MAIN_WINDOW).is_none() && !launcher.any_running() {
            app.exit(0);
        }
    }
    result
}

/// Downloads a new instance's game right away, so the first start is quick. Skipped without
/// an account, since the game cannot be played then anyway.
#[tauri::command]
pub async fn prepare_instance(
    app: AppHandle,
    launcher: State<'_, Launcher>,
    id: String,
) -> CommandResult<()> {
    if launcher.accounts.list().active.is_none() {
        return Ok(());
    }
    let instance = launcher
        .instances
        .get(&id)
        .await
        .ok_or_else(|| CommandError::UnknownInstance(id.clone()))?;
    let Some(cancelled) = launcher.start_running(&id, false) else {
        return Ok(());
    };
    let result = tokio::select! {
        result = install_game(&app, &launcher, &instance) => result.map(drop),
        _ = cancelled => Err(CommandError::Cancelled),
    };
    launcher.stop_running(&id);
    result
}

/// Kills a game that hangs or will not close.
#[tauri::command]
pub fn stop_instance(launcher: State<'_, Launcher>, id: String) -> bool {
    launcher.stop_game(&id)
}

async fn start_game(
    app: &AppHandle,
    launcher: &Launcher,
    instance: Instance,
    account: LaunchAccount,
    server: Option<String>,
    mut cancelled: oneshot::Receiver<()>,
) -> CommandResult<()> {
    tokio::select! {
        result = prepare_game(app, launcher, &instance) => {
            let installed = result?;
            run_game(app, launcher, instance, account, server, installed).await
        }
        _ = &mut cancelled => Err(CommandError::Cancelled),
    }
}

/// Brings the mods up to date if the player wants that, then installs whatever is missing.
async fn prepare_game(
    app: &AppHandle,
    launcher: &Launcher,
    instance: &Instance,
) -> CommandResult<Installed> {
    if launcher.config.get().auto_update_mods && instance.loader.is_some() {
        let _ = app.emit(
            "launch-progress",
            LaunchProgress {
                instance_id: &instance.id,
                stage: "mods",
                bytes_done: 0,
                bytes_total: 0,
            },
        );
        // Being offline should not stop the game from starting with the mods it has.
        let _ = content::update_all(launcher, &instance.id, ContentKind::Mod, None).await;
    }
    install_game(app, launcher, instance).await
}

async fn run_game(
    app: &AppHandle,
    launcher: &Launcher,
    instance: Instance,
    account: LaunchAccount,
    server: Option<String>,
    installed: Installed,
) -> CommandResult<()> {
    let dirs = &launcher.dirs;
    let token = account.access_token.clone();
    let java = match &instance.java_path {
        Some(path) if path.is_file() => path.clone(),
        Some(path) => return Err(CommandError::JavaNotFound(path.clone())),
        None => installed.java.executable(),
    };
    let game_dir = launcher.instances.game_dir(&instance.id);
    let game_assets_dir = installed.game_assets(dirs, &game_dir)?;
    let options = LaunchOptions {
        java,
        game_dir,
        assets_dir: dirs.assets(),
        game_assets_dir,
        libraries_dir: dirs.libraries(),
        versions_dir: dirs.versions(),
        natives_dir: dirs.natives(&installed.version.id),
        account,
        max_memory_mb: match instance.max_memory_mb {
            Some(amount) => amount,
            None => recommended_memory(launcher, &instance.id).await?,
        },
        resolution: instance.window.map(|size| (size.width, size.height)),
        fullscreen: instance.fullscreen,
        server,
        extra_jvm_args: instance
            .java_args
            .split_whitespace()
            .map(str::to_owned)
            .collect(),
    };
    launch::extract_natives(&installed.version, &options)?;
    if let Some(language) =
        sys_locale::get_locale().and_then(|locale| launch::game_language(&locale))
    {
        launch::set_first_language(&options.game_dir, &language)?;
    }

    let child = tokio::process::Command::from(
        launch::build_command(&installed.version, &options).to_command(),
    )
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .map_err(CommandError::Spawn)?;

    let mut latest = launcher
        .instances
        .get(&instance.id)
        .await
        .unwrap_or(instance);
    latest.last_played = Some(Utc::now());
    launcher.instances.save(&latest).await?;
    let instance = latest;
    launcher.clear_log(&instance.id);
    let _ = app.emit("game-started", &instance.id);

    let (stop, stopped) = oneshot::channel();
    launcher.set_stopper(&instance.id, stop);
    tauri::async_runtime::spawn(watch_game(app.clone(), instance.id, token, child, stopped));
    Ok(())
}

/// Downloads the game, Java and mod loader for an instance, reporting progress on its card.
async fn install_game(
    app: &AppHandle,
    launcher: &Launcher,
    instance: &Instance,
) -> CommandResult<Installed> {
    let entry = launcher
        .manifest()
        .await?
        .get(&instance.version)
        .ok_or_else(|| CommandError::UnknownVersion(instance.version.clone()))?;

    let last_percent = AtomicU32::new(u32::MAX);
    game::install(
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
                            Stage::Loader => "loader",
                        },
                        bytes_done: progress.bytes_done,
                        bytes_total: progress.bytes_total,
                    },
                );
            }
        },
    )
    .await
    .inspect_err(|err| {
        // The console tab is the only place the installer's own explanation can be read.
        if let selene_core::Error::LoaderInstaller(output) = err {
            launcher.clear_log(&instance.id);
            for line in output.lines() {
                launcher.push_log(&instance.id, line.to_owned());
            }
        }
    })
    .map_err(Into::into)
}

/// Streams the game's output until it exits or the player stops it, then reports how it went.
async fn watch_game(
    app: AppHandle,
    instance_id: String,
    token: String,
    mut child: tokio::process::Child,
    mut stopped: oneshot::Receiver<()>,
) {
    let started = Instant::now();
    let forward = |output| {
        tauri::async_runtime::spawn(forward_log(
            app.clone(),
            instance_id.clone(),
            token.clone(),
            output,
        ))
    };
    let stdout = child.stdout.take().map(|out| forward(Box::new(out)));
    let stderr = child.stderr.take().map(|err| forward(Box::new(err)));

    let mut killed = false;
    let status = tokio::select! {
        status = child.wait() => status,
        Ok(()) = &mut stopped => {
            killed = true;
            let _ = child.kill().await;
            child.wait().await
        }
    };
    let code = status.ok().and_then(|status| status.code());
    // Let the readers drain what the game printed last; that is where the error is. A process
    // the game left behind can hold the pipes open, so the wait is short.
    for reader in [stdout, stderr].into_iter().flatten() {
        let _ = tokio::time::timeout(LOG_DRAIN_TIMEOUT, reader).await;
    }
    let launcher = app.state::<Launcher>();
    launcher.stop_running(&instance_id);
    add_play_time(&launcher, &instance_id, started.elapsed().as_secs()).await;

    let crash =
        (!killed && code != Some(0)).then(|| crash::analyze(&launcher.log(&instance_id).lines));
    if app.get_webview_window(crate::MAIN_WINDOW).is_none() {
        // The player closed the launcher for the game; it opens again now.
        if let Some(crash) = crash {
            launcher.miss_crash(&instance_id, crash);
        }
        crate::show_main_window(&app);
        return;
    }
    let _ = app.emit(
        "game-exited",
        GameExited {
            instance_id,
            code,
            crash,
        },
    );
}

/// Sends each line the game prints to the UI, with the session token blanked out in case
/// an old version prints it.
///
/// Lines are read as bytes: Java on Windows prints in the system code page, and stopping at
/// the first non-UTF-8 line would leave the pipe full and freeze the game.
async fn forward_log(
    app: AppHandle,
    instance_id: String,
    token: String,
    output: Box<dyn AsyncRead + Send + Unpin>,
) {
    let mut reader = BufReader::new(output);
    let mut buffer = Vec::new();
    let mut pending = Vec::new();
    let mut flush = tokio::time::interval(LOG_FLUSH_INTERVAL);
    flush.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            read = reader.read_until(b'\n', &mut buffer) => {
                // After a flush the read starts over, so a last line without a newline can
                // come back as zero bytes with the line still in the buffer.
                let ended = matches!(read, Ok(0) | Err(_));
                if !buffer.is_empty() {
                    let line = String::from_utf8_lossy(&buffer);
                    let line = line.trim_end_matches(['\r', '\n']);
                    let line = if token.len() < MIN_HIDDEN_TOKEN_LEN {
                        line.to_owned()
                    } else {
                        line.replace(&token, "<hidden>")
                    };
                    buffer.clear();
                    let number = app.state::<Launcher>().push_log(&instance_id, line.clone());
                    pending.push(LogLine { number, text: line });
                }
                if ended {
                    break;
                }
            }
            _ = flush.tick(), if !pending.is_empty() => {
                emit_log(&app, &instance_id, &mut pending);
            }
        }
    }
    emit_log(&app, &instance_id, &mut pending);
}

fn emit_log(app: &AppHandle, instance_id: &str, lines: &mut Vec<LogLine>) {
    if lines.is_empty() {
        return;
    }
    let _ = app.emit(
        "game-log",
        GameLog {
            instance_id,
            lines: lines.as_slice(),
        },
    );
    lines.clear();
}

async fn add_play_time(launcher: &Launcher, id: &str, seconds: u64) {
    // Re-read the instance: it may have been renamed while the game was running.
    if let Some(mut instance) = launcher.instances.get(id).await {
        instance.play_seconds += seconds;
        let _ = launcher.instances.save(&instance).await;
    }
}

/// What the game printed during its last run.
#[tauri::command]
pub fn game_log(launcher: State<'_, Launcher>, instance_id: String) -> LogSnapshot {
    launcher.log(&instance_id)
}

fn percent(progress: Progress) -> u32 {
    let total = progress.bytes_total.max(1);
    u32::try_from(progress.bytes_done.min(total) * 100 / total).unwrap_or(100)
}
