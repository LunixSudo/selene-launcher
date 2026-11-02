//! Helping players tell us what went wrong: the launcher log, and a bug report on GitHub
//! filled in from inside the launcher.

use std::path::Path;

use tauri::{AppHandle, State, Url};
use tauri_plugin_opener::OpenerExt;

use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

const NEW_ISSUE: &str = "https://github.com/LunixSudo/selene-launcher/issues/new";
/// GitHub refuses addresses much longer than this, so only as much of the end of the log as
/// fits goes into the text. The whole log is saved next to it for the player to attach.
const MAX_URL_LEN: usize = 7500;
const MAX_LOG_CHARS: usize = 4000;
const MAX_DESCRIPTION_CHARS: usize = 2000;
const REPORT_FILE: &str = "selene-report.log";
const MAX_UI_ERROR_CHARS: usize = 2000;

/// Errors the interface hit on its own, which would otherwise only reach the dev tools.
#[tauri::command]
pub fn log_ui_error(message: String) {
    let message: String = message.chars().take(MAX_UI_ERROR_CHARS).collect();
    log::error!("interface: {message}");
}

#[tauri::command]
pub fn open_logs_folder(app: AppHandle, launcher: State<'_, Launcher>) -> CommandResult<()> {
    let dir = launcher.data_dir.join("logs");
    std::fs::create_dir_all(&dir)?;
    super::saves::open_path(&app, &dir)
}

/// Opens a new bug report on GitHub with the player's words, the launcher version and system.
/// With the log, the whole of it is saved as a file and shown in its folder, ready to be dropped
/// into the report, and its end is put into the text in case it is not. Sending stays with them.
#[tauri::command]
pub async fn report_problem(
    app: AppHandle,
    launcher: State<'_, Launcher>,
    description: String,
    game: String,
    with_log: bool,
) -> CommandResult<()> {
    let mut params = vec![
        ("template", "bug_report.yml".to_owned()),
        ("labels", "bug".to_owned()),
        (
            "what",
            description
                .trim()
                .chars()
                .take(MAX_DESCRIPTION_CHARS)
                .collect(),
        ),
        ("version", app.package_info().version.to_string()),
        (
            "system",
            format!("{}, {}", system(), crate::hardware::summary()),
        ),
        ("instance", game.trim().to_owned()),
    ];
    let logs = launcher.data_dir.join("logs");
    // Kept out of the logs folder, so nobody has to tell two logs apart there.
    let report_dir = std::env::temp_dir().join("SeleneLauncher");
    let report = report_dir.join(REPORT_FILE);
    if with_log {
        tokio::fs::create_dir_all(&report_dir).await?;
        let log = hide_home(
            &tokio::fs::read_to_string(logs.join("launcher.log"))
                .await
                .unwrap_or_default(),
        );
        tokio::fs::write(&report, &log).await?;
        let note =
            format!("Full log: drop `{REPORT_FILE}` from the folder the launcher opened here.");
        // Encoding makes Cyrillic and punctuation several times longer, so the end of the log
        // shrinks until the whole address fits.
        let mut chars = MAX_LOG_CHARS;
        loop {
            let text = if chars == 0 {
                note.clone()
            } else {
                format!("{note}\n\n```\n{}\n```", tail(&log, chars))
            };
            let url = issue_url(&[params.as_slice(), &[("log", text.clone())]].concat())?;
            if url.as_str().len() <= MAX_URL_LEN || chars == 0 {
                params.push(("log", text));
                break;
            }
            chars /= 2;
            if chars < 200 {
                chars = 0;
            }
        }
    }
    let url = issue_url(&params)?;
    let opener = app.opener();
    if with_log {
        opener.reveal_item_in_dir(&report).map_err(opener_error)?;
    }
    opener
        .open_url(url.as_str(), None::<&str>)
        .map_err(opener_error)
}

fn issue_url(params: &[(&str, String)]) -> CommandResult<Url> {
    Url::parse_with_params(NEW_ISSUE, params)
        .map_err(|err| CommandError::Io(std::io::Error::other(err)))
}

fn opener_error(err: tauri_plugin_opener::Error) -> CommandError {
    CommandError::Io(std::io::Error::other(err))
}

pub(crate) fn system() -> String {
    let os = sysinfo::System::long_os_version().unwrap_or_else(|| std::env::consts::OS.into());
    format!("{os}, {}", std::env::consts::ARCH)
}

/// The home folder carries the player's account name, which a public report does not need.
/// Java and mods write it with either slash and in any letter case.
fn hide_home(log: &str) -> String {
    match std::env::home_dir().as_deref().and_then(Path::to_str) {
        Some(home) if !home.is_empty() => hide(log, home),
        _ => log.to_owned(),
    }
}

fn hide(log: &str, home: &str) -> String {
    let forms = [home.replace('/', "\\"), home.replace('\\', "/")];
    let mut log = log.to_owned();
    for form in forms {
        log = replace_ignoring_case(&log, &form, "~");
    }
    log
}

fn replace_ignoring_case(text: &str, from: &str, to: &str) -> String {
    let lower = text.to_lowercase();
    let needle = from.to_lowercase();
    // Lowercasing may change byte lengths outside ASCII; then only exact matches are safe.
    if needle.is_empty() || lower.len() != text.len() {
        return text.replace(from, to);
    }
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for (start, _) in lower.match_indices(&needle) {
        if start < last {
            continue;
        }
        out.push_str(&text[last..start]);
        out.push_str(to);
        last = start + needle.len();
    }
    out.push_str(&text[last..]);
    out
}

/// The last `max` characters, starting at a whole line.
fn tail(log: &str, max: usize) -> &str {
    let Some((start, _)) = log.char_indices().rev().nth(max) else {
        return log;
    };
    let rest = &log[start..];
    rest.split_once('\n').map_or(rest, |(_, whole)| whole)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hides_the_home_folder_in_any_form() {
        let log = "C:\\Users\\Ivan\\x | file:/C:/Users/Ivan/y | c:\\users\\ivan\\z | Ivan";
        assert_eq!(
            hide(log, "C:\\Users\\Ivan"),
            "~\\x | file:/~/y | ~\\z | Ivan"
        );
    }

    #[test]
    fn keeps_the_end_of_the_log() {
        assert_eq!(tail("short", 100), "short");
        assert_eq!(tail("first line\nsecond\nthird", 10), "third");
        assert_eq!(tail("ab\nвторая\nтретья", 12), "третья");
    }
}
