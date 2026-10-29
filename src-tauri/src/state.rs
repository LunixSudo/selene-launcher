use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use reqwest::Client;
use selene_core::crash::CrashReport;
use selene_core::curseforge::CurseForge;
use selene_core::game::GameDirs;
use selene_core::instance::Instances;
use selene_core::manifest::VersionManifest;
use selene_core::skins::SkinLibrary;
use serde::Serialize;
use tokio::sync::{OnceCell, oneshot};

use crate::account::Accounts;
use crate::config::ConfigFile;
use crate::discord::Discord;
use crate::error::{CommandError, CommandResult};

pub struct Launcher {
    pub data_dir: PathBuf,
    pub client: Client,
    pub curseforge: Option<CurseForge>,
    pub accounts: Accounts,
    pub config: ConfigFile,
    pub discord: Discord,
    pub skins: SkinLibrary,
    pub dirs: GameDirs,
    pub instances: Instances,
    pub total_memory_mb: u64,
    manifest: OnceCell<VersionManifest>,
    /// Instances being prepared or played.
    running: Mutex<HashMap<String, Run>>,
    logs: Mutex<HashMap<String, Log>>,
    startup_launch: Mutex<Option<String>>,
    /// Crashes of games that ended while the window was closed, shown once it opens again.
    missed_crashes: Mutex<HashMap<String, CrashReport>>,
}

#[derive(Default)]
struct Log {
    lines: VecDeque<String>,
    /// Lines pushed this session, dropped and cleared ones included.
    end: u64,
}

/// The kept lines of a game's log and the number of the line after the last one.
#[derive(Serialize)]
pub struct LogSnapshot {
    pub lines: Vec<String>,
    pub end: u64,
}

struct Run {
    /// Cancels the download, or stops the game once it has started.
    stop: Option<oneshot::Sender<()>>,
    /// The game will start once the download is done; a plain download does not keep the
    /// launcher open.
    launch: bool,
    playing: bool,
}

/// How long a deletion waits for a cancelled download to let go of the files.
const CANCEL_TIMEOUT: Duration = Duration::from_secs(10);

/// Lines kept per instance; older ones are dropped.
const LOG_LINES: usize = 5000;

impl Launcher {
    pub fn new(data_dir: &Path, startup_launch: Option<String>) -> selene_core::Result<Self> {
        let client = selene_core::http::client()?;
        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            curseforge: curseforge(&client),
            client,
            accounts: Accounts::load(data_dir.join("accounts.json")),
            config: ConfigFile::load(data_dir.join("settings.json")),
            discord: Discord::start(),
            skins: SkinLibrary::new(data_dir.join("skins")),
            dirs: GameDirs::new(data_dir.join("game")),
            instances: Instances::new(data_dir.join("instances")),
            total_memory_mb: total_memory_mb(),
            manifest: OnceCell::new(),
            running: Mutex::new(HashMap::new()),
            logs: Mutex::new(HashMap::new()),
            startup_launch: Mutex::new(startup_launch),
            missed_crashes: Mutex::new(HashMap::new()),
        })
    }

    /// The version list is fetched once per session; Mojang updates it a few times a week.
    pub async fn manifest(&self) -> CommandResult<&VersionManifest> {
        let cache = self.dirs.versions().join("manifest.json");
        let manifest = self
            .manifest
            .get_or_try_init(|| VersionManifest::fetch_cached(&self.client, &cache))
            .await?;
        Ok(manifest)
    }

    /// The game folder of an existing instance. Ids come from the UI, so they are checked here.
    pub fn game_dir(&self, id: &str) -> CommandResult<PathBuf> {
        if self.instances.exists(id) {
            Ok(self.instances.game_dir(id))
        } else {
            Err(CommandError::UnknownInstance(id.to_owned()))
        }
    }

    /// Marks an instance as being prepared. Returns `None` if it already was busy, otherwise
    /// a receiver that fires when the download is cancelled.
    pub fn start_running(&self, id: &str, launch: bool) -> Option<oneshot::Receiver<()>> {
        let mut running = self.running_map();
        if running.contains_key(id) {
            return None;
        }
        let (stop, stopped) = oneshot::channel();
        running.insert(
            id.to_owned(),
            Run {
                stop: Some(stop),
                launch,
                playing: false,
            },
        );
        Some(stopped)
    }

    /// Whether the instance is downloading without anyone waiting to play it.
    pub fn is_preparing_in_background(&self, id: &str) -> bool {
        self.running_map()
            .get(id)
            .is_some_and(|run| !run.launch && !run.playing)
    }

    /// Remembers how to stop the game once its process exists.
    pub fn set_stopper(&self, id: &str, stop: oneshot::Sender<()>) {
        if let Some(run) = self.running_map().get_mut(id) {
            run.stop = Some(stop);
            run.playing = true;
        }
    }

    /// Asks a running game to stop; returns `false` if it was not running.
    pub fn stop_game(&self, id: &str) -> bool {
        self.running_map()
            .get_mut(id)
            .filter(|run| run.playing)
            .and_then(|run| run.stop.take())
            .is_some_and(|stop| stop.send(()).is_ok())
    }

    /// Cancels a download in progress and waits until it has stopped writing. Fails if the
    /// game itself is running.
    pub async fn cancel_preparing(&self, id: &str) -> CommandResult<()> {
        {
            let mut running = self.running_map();
            match running.get_mut(id) {
                None => return Ok(()),
                Some(run) if run.playing => {
                    return Err(CommandError::AlreadyRunning(id.to_owned()));
                }
                Some(run) => {
                    if let Some(stop) = run.stop.take() {
                        let _ = stop.send(());
                    }
                }
            }
        }
        let started = Instant::now();
        while self.is_running(id) && started.elapsed() < CANCEL_TIMEOUT {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        if self.is_running(id) {
            return Err(CommandError::AlreadyRunning(id.to_owned()));
        }
        Ok(())
    }

    pub fn stop_running(&self, id: &str) {
        self.running_map().remove(id);
    }

    pub fn is_running(&self, id: &str) -> bool {
        self.running_map().contains_key(id)
    }

    /// Instances whose game process is up, not just being prepared.
    pub fn running_games(&self) -> Vec<String> {
        self.running_map()
            .iter()
            .filter(|(_, run)| run.playing && run.stop.is_some())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Whether a game is running or about to start.
    pub fn any_running(&self) -> bool {
        self.running_map()
            .values()
            .any(|run| run.launch || run.playing)
    }

    pub fn miss_crash(&self, id: &str, crash: CrashReport) {
        lock(&self.missed_crashes).insert(id.to_owned(), crash);
    }

    pub fn take_missed_crashes(&self) -> HashMap<String, CrashReport> {
        std::mem::take(&mut *lock(&self.missed_crashes))
    }

    pub fn take_startup_launch(&self) -> Option<String> {
        self.startup_launch
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }

    /// Forgets the previous run's output.
    /// Empties the log but keeps counting, so line numbers never repeat in a session.
    pub fn clear_log(&self, id: &str) {
        if let Some(log) = self.log_map().get_mut(id) {
            log.lines.clear();
        }
    }

    /// Adds a line and returns its number.
    pub fn push_log(&self, id: &str, line: String) -> u64 {
        let mut logs = self.log_map();
        let log = logs.entry(id.to_owned()).or_default();
        if log.lines.len() == LOG_LINES {
            log.lines.pop_front();
        }
        log.lines.push_back(line);
        log.end += 1;
        log.end - 1
    }

    /// Whether one of the last `count` lines the game printed contains `text`.
    pub fn log_tail_contains(&self, id: &str, text: &str, count: usize) -> bool {
        self.log_map().get(id).is_some_and(|log| {
            log.lines
                .iter()
                .rev()
                .take(count)
                .any(|line| line.contains(text))
        })
    }

    /// How many lines the game has printed so far.
    pub fn log_end(&self, id: &str) -> u64 {
        self.log_map().get(id).map_or(0, |log| log.end)
    }

    pub fn log(&self, id: &str) -> LogSnapshot {
        self.log_map().get(id).map_or(
            LogSnapshot {
                lines: Vec::new(),
                end: 0,
            },
            |log| LogSnapshot {
                lines: log.lines.iter().cloned().collect(),
                end: log.end,
            },
        )
    }

    fn log_map(&self) -> std::sync::MutexGuard<'_, HashMap<String, Log>> {
        self.logs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn running_map(&self) -> std::sync::MutexGuard<'_, HashMap<String, Run>> {
        lock(&self.running)
    }
}

fn total_memory_mb() -> u64 {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    system.total_memory() / 1024 / 1024
}

/// Release builds get the CurseForge key from CI. Debug builds without a key still try,
/// for development behind a proxy that adds the key itself.
fn curseforge(client: &Client) -> Option<CurseForge> {
    let api_key = option_env!("CURSEFORGE_API_KEY").map(str::to_owned);
    (api_key.is_some() || cfg!(debug_assertions)).then(|| CurseForge::new(client.clone(), api_key))
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
