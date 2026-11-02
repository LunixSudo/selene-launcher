//! `OptiFine` may only be downloaded from its own site. The site opens in a window of its own,
//! and the jar the player downloads there goes straight into the instance's mods folder.

use std::path::{Path, PathBuf};

use tauri::webview::DownloadEvent;
use tauri::{AppHandle, Emitter, Manager, State, Url, WebviewUrl, WebviewWindowBuilder};

use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

const SITES: [&str; 2] = ["optifine.net", "adfoc.us"];
const DOWNLOADS: &str = "https://optifine.net/downloads";

#[tauri::command]
pub async fn open_optifine(
    app: AppHandle,
    launcher: State<'_, Launcher>,
    instance_id: String,
) -> CommandResult<()> {
    let instance = launcher
        .instances
        .get(&instance_id)
        .await
        .ok_or_else(|| CommandError::UnknownInstance(instance_id.clone()))?;
    let mods = launcher.instances.game_dir(&instance_id).join("mods");
    tokio::fs::create_dir_all(&mods).await?;
    let label = format!("optifine-{instance_id}");
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.set_focus();
        return Ok(());
    }
    let url = DOWNLOADS.parse().expect("the downloads address is valid");
    WebviewWindowBuilder::new(&app, label, WebviewUrl::External(url))
        .title("OptiFine")
        .inner_size(1040.0, 760.0)
        .min_inner_size(640.0, 480.0)
        .initialization_script(scroll_to_version(&instance.version))
        .on_navigation(|url| url.host_str().is_some_and(is_site))
        .on_download(move |webview, event| match event {
            DownloadEvent::Requested { url, destination } => {
                let Some(name) = jar_name(&url, destination) else {
                    return false;
                };
                *destination = mods.join(name);
                true
            }
            DownloadEvent::Finished { success, .. } => {
                if success {
                    let _ = webview
                        .app_handle()
                        .emit("optifine-installed", &instance_id);
                    let _ = webview.window().close();
                }
                true
            }
            _ => true,
        })
        .build()
        .map_err(|err| CommandError::Io(std::io::Error::other(err)))?;
    Ok(())
}

/// The site's own download button goes through its ad partner first.
fn is_site(host: &str) -> bool {
    SITES.iter().any(|site| {
        host == *site
            || host
                .strip_suffix(site)
                .is_some_and(|sub| sub.ends_with('.'))
    })
}

/// Older versions sit in a hidden block on the page, so it is unfolded first.
fn scroll_to_version(version: &str) -> String {
    let heading = serde_json::to_string(&format!("Minecraft {version}")).unwrap_or_default();
    format!(
        r#"addEventListener("DOMContentLoaded", () => {{
  if (location.pathname !== "/downloads") return;
  const heading = [...document.querySelectorAll("h2")].find((h) => h.textContent.trim() === {heading});
  if (!heading) return;
  const folded = heading.closest('[id^="all"]');
  if (folded && getComputedStyle(folded).visibility === "hidden") document.querySelector("a.showAll")?.click();
  setTimeout(() => heading.scrollIntoView({{ block: "start" }}), 600);
}});"#
    )
}

/// Only `OptiFine` jars are let through, under the name the site gives them.
fn jar_name(url: &Url, suggested: &Path) -> Option<PathBuf> {
    let name = suggested
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| is_optifine_jar(name))
        .or_else(|| {
            url.query_pairs()
                .find(|(key, _)| key == "f")
                .map(|(_, value)| value.into_owned())
        })?;
    let name = Path::new(&name).file_name()?.to_string_lossy().into_owned();
    is_optifine_jar(&name).then(|| name.into())
}

fn is_optifine_jar(name: &str) -> bool {
    name.to_ascii_lowercase().contains("optifine")
        && Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("jar"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_to_the_site() {
        assert!(is_site("optifine.net"));
        assert!(is_site("www.optifine.net"));
        assert!(!is_site("notoptifine.net"));
        assert!(!is_site("optifine.net.example.com"));
        assert!(is_site("adfoc.us"));
    }

    #[test]
    fn takes_only_optifine_jars() {
        let url: Url = "https://optifine.net/downloadx?f=OptiFine_1.8.9_HD_U_M5.jar&x=abc"
            .parse()
            .unwrap();
        assert_eq!(
            jar_name(&url, Path::new("/tmp/downloadx")),
            Some("OptiFine_1.8.9_HD_U_M5.jar".into())
        );
        assert_eq!(
            jar_name(&url, Path::new("/tmp/preview_OptiFine_1.21.jar")),
            Some("preview_OptiFine_1.21.jar".into())
        );
        let other: Url = "https://optifine.net/downloadx?f=../setup.exe"
            .parse()
            .unwrap();
        assert_eq!(jar_name(&other, Path::new("/tmp/setup.exe")), None);
        let sneaky: Url = "https://optifine.net/downloadx?f=../../OptiFine.jar"
            .parse()
            .unwrap();
        assert_eq!(
            jar_name(&sneaky, Path::new("/tmp/x")),
            Some("OptiFine.jar".into())
        );
    }
}
