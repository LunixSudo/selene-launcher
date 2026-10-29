use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// Launcher settings that the backend acts on. Look-and-feel settings stay in the UI.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    /// Update mods from Modrinth right before the game starts.
    pub auto_update_mods: bool,
    /// Files downloaded at once; `None` lets each task pick.
    pub max_downloads: Option<usize>,
    /// Download speed cap in KiB/s.
    pub speed_limit_kib: Option<u64>,
    pub download_source: selene_core::http::Source,
}

pub struct ConfigFile {
    path: PathBuf,
    config: Mutex<Config>,
}

impl ConfigFile {
    pub fn load(path: PathBuf) -> Self {
        let config: Config = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        apply(&config);
        Self {
            path,
            config: Mutex::new(config),
        }
    }

    pub fn get(&self) -> Config {
        self.lock().clone()
    }

    pub fn set(&self, config: Config) -> selene_core::Result<()> {
        selene_core::fs::write_atomic_blocking(&self.path, &serde_json::to_vec_pretty(&config)?)?;
        apply(&config);
        *self.lock() = config;
        Ok(())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Config> {
        self.config
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

fn apply(config: &Config) {
    selene_core::download::set_limits(
        config.max_downloads,
        config.speed_limit_kib.map(|kib| kib * 1024),
    );
    selene_core::http::set_source(config.download_source);
}
