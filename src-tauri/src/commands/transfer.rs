//! The launcher's settings in one file, to take them to another PC. Accounts stay out: their
//! tokens belong to this PC's credential store.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

use crate::config::Config;
use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

const FORMAT: &str = "selene-launcher-settings";
const VERSION: u32 = 1;
/// Settings take a few kilobytes; a bigger file is something else picked by mistake.
const MAX_FILE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
struct SettingsFile {
    format: String,
    version: u32,
    /// What the interface keeps: the look, presets and how the launcher behaves.
    interface: Value,
    launcher: Config,
}

#[derive(Debug, Serialize)]
pub struct ImportedSettings {
    interface: Value,
    launcher: Config,
}

#[tauri::command]
pub async fn export_settings(
    launcher: State<'_, Launcher>,
    path: PathBuf,
    interface: Value,
) -> CommandResult<()> {
    let file = SettingsFile {
        format: FORMAT.to_owned(),
        version: VERSION,
        interface,
        launcher: launcher.config.get(),
    };
    let bytes = serde_json::to_vec_pretty(&file).map_err(selene_core::Error::from)?;
    selene_core::fs::write_atomic(&path, &bytes).await?;
    Ok(())
}

/// Reads a settings file and puts its launcher part into effect. The interface part goes back
/// to the page, which checks every value of it the way it checks its own saved settings.
#[tauri::command]
pub async fn import_settings(
    launcher: State<'_, Launcher>,
    path: PathBuf,
) -> CommandResult<ImportedSettings> {
    if tokio::fs::metadata(&path).await?.len() > MAX_FILE_BYTES {
        return Err(CommandError::InvalidSettingsFile);
    }
    let bytes = tokio::fs::read(&path).await?;
    let file = parse(&bytes).ok_or(CommandError::InvalidSettingsFile)?;
    let config = file.launcher.cleaned();
    launcher.config.set(config.clone())?;
    Ok(ImportedSettings {
        interface: file.interface,
        launcher: config,
    })
}

fn parse(bytes: &[u8]) -> Option<SettingsFile> {
    let file: SettingsFile = serde_json::from_slice(bytes).ok()?;
    (file.format == FORMAT && file.version <= VERSION && file.interface.is_object()).then_some(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_only_its_own_files() {
        let file = br#"{"format":"selene-launcher-settings","version":1,"interface":{"theme":"dark"},"launcher":{"autoUpdateMods":true}}"#;
        let parsed = parse(file).unwrap();
        assert!(parsed.launcher.auto_update_mods);
        assert!(parsed.launcher.auto_update_java);
        assert!(parse(br#"{"format":"other","version":1,"interface":{},"launcher":{}}"#).is_none());
        assert!(parse(b"[1, 2]").is_none());
    }
}
