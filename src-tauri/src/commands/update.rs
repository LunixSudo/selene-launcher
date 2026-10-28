use selene_core::update::{self, Update};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

const RELEASES_URL: &str = "https://github.com/LunixSudo/selene-launcher/releases/latest";

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
    if crate::portable_data_dir().is_some() {
        // The installer would put a second copy in Program Files; the portable zip is on
        // the release page.
        return app
            .opener()
            .open_url(RELEASES_URL, None::<&str>)
            .map_err(|err| CommandError::Io(std::io::Error::other(err)));
    }
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

/// Whether this copy was installed and can be removed with its uninstaller. The portable
/// version has none: deleting its folder is enough.
#[tauri::command]
pub fn can_uninstall() -> bool {
    uninstaller().is_some()
}

/// Starts the Windows uninstaller and closes the launcher so its files can be removed.
#[tauri::command]
pub fn uninstall_launcher(app: AppHandle) -> CommandResult<()> {
    let uninstaller = uninstaller()
        .ok_or_else(|| CommandError::Io(std::io::Error::from(std::io::ErrorKind::NotFound)))?;
    std::process::Command::new(uninstaller)
        .spawn()
        .map_err(CommandError::Spawn)?;
    app.exit(0);
    Ok(())
}

fn uninstaller() -> Option<std::path::PathBuf> {
    if !cfg!(windows) || crate::portable_data_dir().is_some() {
        return None;
    }
    let path = std::env::current_exe()
        .ok()?
        .parent()?
        .join("uninstall.exe");
    path.is_file().then_some(path)
}
