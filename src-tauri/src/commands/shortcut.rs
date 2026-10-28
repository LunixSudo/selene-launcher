use std::path::Path;

use tauri::{AppHandle, Manager, State};

use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

/// Command-line flag that starts an instance as soon as the launcher opens.
pub const LAUNCH_FLAG: &str = "--launch";

/// Puts a shortcut on the desktop that opens the launcher and starts the instance.
#[tauri::command]
pub async fn create_shortcut(
    app: AppHandle,
    launcher: State<'_, Launcher>,
    id: String,
) -> CommandResult<()> {
    let instance = launcher
        .instances
        .get(&id)
        .await
        .ok_or_else(|| CommandError::UnknownInstance(id.clone()))?;
    let desktop = app
        .path()
        .desktop_dir()
        .map_err(|err| CommandError::Io(std::io::Error::other(err)))?;
    let file_name: String = instance
        .name
        .chars()
        .map(|ch| if r#"<>:"/\|?*"#.contains(ch) { '_' } else { ch })
        .collect();
    let exe = std::env::current_exe()?;
    write_shortcut(
        &exe,
        &format!("{LAUNCH_FLAG} {id}"),
        &desktop.join(format!("{}.lnk", file_name.trim())),
    )
}

/// The instance id after `--launch`, if the launcher was started from a shortcut.
pub fn launch_arg(args: &[String]) -> Option<String> {
    let position = args.iter().position(|arg| arg == LAUNCH_FLAG)?;
    args.get(position + 1).cloned()
}

#[cfg(windows)]
fn write_shortcut(target: &Path, arguments: &str, path: &Path) -> CommandResult<()> {
    let mut link = mslnk::ShellLink::new(target)
        .map_err(|err| CommandError::Io(std::io::Error::other(err)))?;
    link.set_arguments(Some(arguments.to_owned()));
    if let Some(dir) = target.parent() {
        link.set_working_dir(Some(dir.to_string_lossy().into_owned()));
    }
    link.create_lnk(path)
        .map_err(|err| CommandError::Io(std::io::Error::other(err)))
}

#[cfg(not(windows))]
fn write_shortcut(_target: &Path, _arguments: &str, _path: &Path) -> CommandResult<()> {
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
}
