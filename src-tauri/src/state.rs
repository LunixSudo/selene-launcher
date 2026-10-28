use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::Mutex;

use reqwest::Client;
use selene_core::curseforge::CurseForge;
use selene_core::game::GameDirs;
use selene_core::instance::Instances;
use selene_core::manifest::VersionManifest;
use tokio::sync::OnceCell;

use crate::account::Accounts;
use crate::error::CommandResult;

pub struct Launcher {
    pub client: Client,
    pub curseforge: Option<CurseForge>,
    pub accounts: Accounts,
    pub dirs: GameDirs,
    pub instances: Instances,
    pub total_memory_mb: u64,
    manifest: OnceCell<VersionManifest>,
    running: Mutex<HashSet<String>>,
    logs: Mutex<HashMap<String, VecDeque<String>>>,
}

/// Lines kept per instance; older ones are dropped.
const LOG_LINES: usize = 5000;

impl Launcher {
    pub fn new(data_dir: &Path) -> selene_core::Result<Self> {
        let client = selene_core::http::client()?;
        Ok(Self {
            curseforge: curseforge(&client),
            client,
            accounts: Accounts::load(data_dir.join("accounts.json")),
            dirs: GameDirs::new(data_dir.join("game")),
            instances: Instances::new(data_dir.join("instances")),
            total_memory_mb: total_memory_mb(),
            manifest: OnceCell::new(),
            running: Mutex::new(HashSet::new()),
            logs: Mutex::new(HashMap::new()),
        })
    }

    /// The version list is fetched once per session; Mojang updates it a few times a week.
    pub async fn manifest(&self) -> CommandResult<&VersionManifest> {
        let manifest = self
            .manifest
            .get_or_try_init(|| VersionManifest::fetch(&self.client))
            .await?;
        Ok(manifest)
    }

    /// Marks an instance as running; returns `false` if it already was.
    pub fn start_running(&self, id: &str) -> bool {
        self.running_set().insert(id.to_owned())
    }

    pub fn stop_running(&self, id: &str) {
        self.running_set().remove(id);
    }

    pub fn is_running(&self, id: &str) -> bool {
        self.running_set().contains(id)
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

    fn running_set(&self) -> std::sync::MutexGuard<'_, HashSet<String>> {
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
