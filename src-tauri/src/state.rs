use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use reqwest::Client;
use selene_core::crash::CrashReport;
use selene_core::curseforge::CurseForge;
use selene_core::game::GameDirs;
use selene_core::instance::Instances;
use selene_core::manifest::VersionManifest;
use selene_core::skins::SkinLibrary;
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
    /// Instances being prepared or played; the sender stops a game that has started.
    running: Mutex<HashMap<String, Option<oneshot::Sender<()>>>>,
    logs: Mutex<HashMap<String, VecDeque<String>>>,
    startup_launch: Mutex<Option<String>>,
    /// Crashes of games that ended while the window was closed, shown once it opens again.
    missed_crashes: Mutex<HashMap<String, CrashReport>>,
}

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

    /// Marks an instance as running; returns `false` if it already was.
    pub fn start_running(&self, id: &str) -> bool {
        let mut running = self.running_map();
        if running.contains_key(id) {
            return false;
        }
        running.insert(id.to_owned(), None);
        true
    }

    /// Remembers how to stop the game once its process exists.
    pub fn set_stopper(&self, id: &str, stop: oneshot::Sender<()>) {
        if let Some(slot) = self.running_map().get_mut(id) {
            *slot = Some(stop);
        }
    }

    /// Asks a running game to stop; returns `false` if it was not running.
    pub fn stop_game(&self, id: &str) -> bool {
        self.running_map()
            .get_mut(id)
            .and_then(Option::take)
            .is_some_and(|stop| stop.send(()).is_ok())
    }

    pub fn stop_running(&self, id: &str) {
        self.running_map().remove(id);
    }

    pub fn is_running(&self, id: &str) -> bool {
        self.running_map().contains_key(id)
    }

    pub fn any_running(&self) -> bool {
        !self.running_map().is_empty()
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
    pub fn clear_log(&self, id: &str) {
        self.log_map().remove(id);
    }

    pub fn push_log(&self, id: &str, line: String) {
        let mut logs = self.log_map();
        let log = logs.entry(id.to_owned()).or_default();
        if log.len() == LOG_LINES {
            log.pop_front();
        }
        log.push_back(line);
    }

    pub fn log(&self, id: &str) -> Vec<String> {
        self.log_map()
            .get(id)
            .map(|log| log.iter().cloned().collect())
            .unwrap_or_default()
    }

    fn log_map(&self) -> std::sync::MutexGuard<'_, HashMap<String, VecDeque<String>>> {
        self.logs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn running_map(
        &self,
    ) -> std::sync::MutexGuard<'_, HashMap<String, Option<oneshot::Sender<()>>>> {
        self.running
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
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
