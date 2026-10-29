use serde::Serialize;
use tauri::State;

use crate::error::CommandResult;
use crate::state::Launcher;

/// The date of the current privacy policy. The installer writes it once the user agrees, so
/// bump it here and in `nsis/hooks.nsh` whenever `PRIVACY.md` changes.
pub const PRIVACY_VERSION: &str = "2026-10-29";
const ACCEPTED_FILE: &str = "privacy-accepted.txt";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyStatus {
    version: &'static str,
    accepted: bool,
    /// An older version was accepted before, so the policy changed since.
    updated: bool,
}

#[tauri::command]
pub async fn privacy_status(launcher: State<'_, Launcher>) -> CommandResult<PrivacyStatus> {
    let previous = tokio::fs::read_to_string(launcher.data_dir.join(ACCEPTED_FILE))
        .await
        .ok();
    let accepted = previous
        .as_deref()
        .is_some_and(|version| version.trim() == PRIVACY_VERSION);
    Ok(PrivacyStatus {
        version: PRIVACY_VERSION,
        accepted,
        updated: previous.is_some() && !accepted,
    })
}

#[tauri::command]
pub async fn accept_privacy(launcher: State<'_, Launcher>) -> CommandResult<()> {
    tokio::fs::create_dir_all(&launcher.data_dir).await?;
    tokio::fs::write(launcher.data_dir.join(ACCEPTED_FILE), PRIVACY_VERSION).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installer_writes_the_same_version() {
        let hooks = include_str!("../../nsis/hooks.nsh");
        assert!(hooks.contains(&format!("!define PRIVACY_VERSION \"{PRIVACY_VERSION}\"")));
    }
}
