use selene_core::update::{self, Update};
use tauri::{AppHandle, Manager, State};

use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

#[tauri::command]
pub async fn check_update(
    app: AppHandle,
    launcher: State<'_, Launcher>,
) -> CommandResult<Option<Update>> {
    let current = app.package_info().version.to_string();
    Ok(update::check(&launcher.client, &current).await?)
}

/// Downloads the new installer, starts it and closes the launcher so it can be replaced.
/// The release is looked up again here rather than taken from the UI.
#[tauri::command]
pub async fn install_update(app: AppHandle, launcher: State<'_, Launcher>) -> CommandResult<()> {
    let Some(update) = check_update(app.clone(), launcher.clone()).await? else {
        return Ok(());
    };
    let dir = app
        .path()
        .temp_dir()
        .map_err(|err| CommandError::Io(std::io::Error::other(err)))?
        .join("selene-launcher-update");
    let installer = update::download(&launcher.client, &update, &dir).await?;
    // Passive mode shows only a progress bar; /R opens the launcher again when it is done.
    std::process::Command::new(&installer)
        .args(["/P", "/R"])
        .spawn()
        .map_err(CommandError::Spawn)?;
    app.exit(0);
    Ok(())
}
