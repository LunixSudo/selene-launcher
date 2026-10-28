use std::path::{Path, PathBuf};

use selene_core::download;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

/// Command-line flag that starts an instance as soon as the launcher opens.
pub const LAUNCH_FLAG: &str = "--launch";
const RECORD_FILE: &str = "shortcut.json";
const ICON_PREFIX: &str = "shortcut-";

/// Where the instance's desktop shortcut is, so it can follow a new name or look.
#[derive(Debug, Serialize, Deserialize)]
struct ShortcutRecord {
    path: PathBuf,
}

/// Puts a shortcut on the desktop that opens the launcher and starts the instance. `icon`
/// is the instance's cover and icon drawn as a PNG.
#[tauri::command]
pub async fn create_shortcut(
    app: AppHandle,
    launcher: State<'_, Launcher>,
    id: String,
    icon: Vec<u8>,
) -> CommandResult<()> {
    write_instance_shortcut(&app, &launcher, &id, &icon).await
}

/// Redraws the desktop shortcut after the instance was renamed or restyled. Does nothing if
/// there is none, or the user deleted it.
#[tauri::command]
pub async fn update_shortcut(
    app: AppHandle,
    launcher: State<'_, Launcher>,
    id: String,
    icon: Vec<u8>,
) -> CommandResult<()> {
    let record_path = instance_dir(&launcher, &id).join(RECORD_FILE);
    let Ok(bytes) = tokio::fs::read(&record_path).await else {
        return Ok(());
    };
    let record: ShortcutRecord = serde_json::from_slice(&bytes).map_err(std::io::Error::from)?;
    if tokio::fs::remove_file(&record.path).await.is_err() {
        let _ = tokio::fs::remove_file(&record_path).await;
        return Ok(());
    }
    write_instance_shortcut(&app, &launcher, &id, &icon).await
}

async fn write_instance_shortcut(
    app: &AppHandle,
    launcher: &Launcher,
    id: &str,
    icon: &[u8],
) -> CommandResult<()> {
    let instance = launcher
        .instances
        .get(id)
        .await
        .ok_or_else(|| CommandError::UnknownInstance(id.to_owned()))?;
    let desktop = app
        .path()
        .desktop_dir()
        .map_err(|err| CommandError::Io(std::io::Error::other(err)))?;
    let file_name: String = instance
        .name
        .chars()
        .map(|ch| if r#"<>:"/\|?*"#.contains(ch) { '_' } else { ch })
        .collect();
    let link = desktop.join(format!("{}.lnk", file_name.trim()));
    let dir = instance_dir(launcher, id);
    // A new file name for every look, because Explorer caches icons by path.
    let icon_path = dir.join(format!(
        "{ICON_PREFIX}{}.ico",
        &download::sha1_bytes(icon)[..8]
    ));
    remove_old_icons(&dir, &icon_path).await;
    tokio::fs::write(&icon_path, png_to_ico(icon)).await?;

    let exe = std::env::current_exe()?;
    write_shortcut(&exe, &format!("{LAUNCH_FLAG} {id}"), &icon_path, &link)?;
    let record =
        serde_json::to_vec(&ShortcutRecord { path: link }).map_err(std::io::Error::from)?;
    tokio::fs::write(dir.join(RECORD_FILE), record).await?;
    Ok(())
}

fn instance_dir(launcher: &Launcher, id: &str) -> PathBuf {
    let game_dir = launcher.instances.game_dir(id);
    game_dir
        .parent()
        .map_or(game_dir.clone(), Path::to_path_buf)
}

async fn remove_old_icons(dir: &Path, keep: &Path) {
    let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
        return;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let old = path != keep
            && entry.file_name().to_string_lossy().starts_with(ICON_PREFIX)
            && path.extension().is_some_and(|extension| extension == "ico");
        if old {
            let _ = tokio::fs::remove_file(path).await;
        }
    }
}

/// An `.ico` holding one PNG image, which Windows scales down for smaller sizes.
fn png_to_ico(png: &[u8]) -> Vec<u8> {
    let size = u32::try_from(png.len()).unwrap_or(u32::MAX);
    let mut ico = Vec::with_capacity(png.len() + 22);
    ico.extend_from_slice(&[0, 0, 1, 0, 1, 0]);
    // Width and height 0 mean 256.
    ico.extend_from_slice(&[0, 0, 0, 0, 1, 0, 32, 0]);
    ico.extend_from_slice(&size.to_le_bytes());
    ico.extend_from_slice(&22u32.to_le_bytes());
    ico.extend_from_slice(png);
    ico
}

/// The instance id after `--launch`, if the launcher was started from a shortcut.
pub fn launch_arg(args: &[String]) -> Option<String> {
    let position = args.iter().position(|arg| arg == LAUNCH_FLAG)?;
    args.get(position + 1).cloned()
}

#[cfg(windows)]
fn write_shortcut(target: &Path, arguments: &str, icon: &Path, path: &Path) -> CommandResult<()> {
    let mut link = mslnk::ShellLink::new(target)
        .map_err(|err| CommandError::Io(std::io::Error::other(err)))?;
    link.set_arguments(Some(arguments.to_owned()));
    link.set_icon_location(Some(icon.to_string_lossy().into_owned()));
    if let Some(dir) = target.parent() {
        link.set_working_dir(Some(dir.to_string_lossy().into_owned()));
    }
    link.create_lnk(path)
        .map_err(|err| CommandError::Io(std::io::Error::other(err)))
}

#[cfg(not(windows))]
fn write_shortcut(
    _target: &Path,
    _arguments: &str,
    _icon: &Path,
    _path: &Path,
) -> CommandResult<()> {
    Err(CommandError::Io(std::io::Error::from(
        std::io::ErrorKind::Unsupported,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_instance_to_launch() {
        let args = ["selene-launcher.exe", "--launch", "survival"].map(String::from);
        assert_eq!(launch_arg(&args).as_deref(), Some("survival"));
        assert_eq!(launch_arg(&args[..1]), None);
    }

    #[test]
    fn wraps_a_png_in_an_icon_file() {
        let ico = png_to_ico(b"png");
        assert_eq!(&ico[..6], &[0, 0, 1, 0, 1, 0]);
        assert_eq!(u32::from_le_bytes(ico[14..18].try_into().unwrap()), 3);
        assert_eq!(&ico[22..], b"png");
    }
}
