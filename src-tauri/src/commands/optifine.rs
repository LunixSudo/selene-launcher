//! `OptiFine` may only be downloaded from its own site. The site opens in a window of its own,
//! and the jar the player downloads there goes straight into the instance's mods folder.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use selene_core::download::{self, Download};
use selene_core::loader::Loader;
use selene_core::mods::DISABLED_SUFFIX;
use tauri::webview::{DownloadEvent, PageLoadEvent};
use tauri::{AppHandle, Emitter, Manager, State, Url, WebviewUrl, WebviewWindowBuilder};

use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

const SITE: &str = "optifine.net";
const DOWNLOADS: &str = "https://optifine.net/downloads";
const MIRROR: &str = "https://optifine.net/adloadx";
/// The list changes a few times a year; this spares a second download when the window opens
/// right after the button checked the list.
const PAGE_KEPT: Duration = Duration::from_secs(10 * 60);

static PAGE: tokio::sync::Mutex<Option<(Instant, String)>> = tokio::sync::Mutex::const_new(None);
/// One window at a time: a double click must not try to open a second one with the same name.
static OPENING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Whether to offer `OptiFine` for the instance: a Forge instance whose game version has a
/// build that is not in its mods folder yet.
#[tauri::command]
pub async fn optifine_offered(
    launcher: State<'_, Launcher>,
    instance_id: String,
) -> CommandResult<bool> {
    let instance = launcher
        .instances
        .get(&instance_id)
        .await
        .ok_or_else(|| CommandError::UnknownInstance(instance_id.clone()))?;
    if instance.loader.as_ref().map(|loader| loader.loader) != Some(Loader::Forge) {
        return Ok(false);
    }
    let Some(newest) = newest_build(&downloads_page(&launcher).await?, &instance.version) else {
        return Ok(false);
    };
    let mods = launcher.instances.game_dir(&instance_id).join("mods");
    let mut entries = match tokio::fs::read_dir(&mods).await {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(err) => return Err(err.into()),
    };
    let wanted = newest.trim_start_matches("preview_");
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let name = name.trim_end_matches(DISABLED_SUFFIX);
        if name
            .trim_start_matches("preview_")
            .eq_ignore_ascii_case(wanted)
        {
            return Ok(false);
        }
    }
    Ok(true)
}

async fn downloads_page(launcher: &Launcher) -> CommandResult<String> {
    let mut kept = PAGE.lock().await;
    if let Some((fetched, page)) = kept.as_ref()
        && fetched.elapsed() < PAGE_KEPT
    {
        return Ok(page.clone());
    }
    let page = launcher
        .client
        .get(DOWNLOADS)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(selene_core::Error::from)?
        .text()
        .await
        .map_err(selene_core::Error::from)?;
    *kept = Some((Instant::now(), page.clone()));
    Ok(page)
}

/// The newest build for a game version, preferring releases over previews.
fn newest_build(page: &str, version: &str) -> Option<String> {
    let heading = format!("{version}</h2>");
    let section = page
        .split("<h2>Minecraft ")
        .find(|section| section.starts_with(&heading))?;
    let files: Vec<&str> = section
        .split("optifine.net/adloadx?f=")
        .skip(1)
        .filter_map(|rest| rest.split(['&', '"', '\'']).next())
        .collect();
    files
        .iter()
        .find(|file| !file.starts_with("preview_"))
        .or_else(|| files.first())
        .map(|file| (*file).to_owned())
}

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
    let _opening = OPENING.lock().await;
    if let Some(window) = app.get_webview_window(&label) {
        // A site that never finished loading still gets its window on the second press.
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }
    // The download page of the right file has one button and no ad detour; the whole list is
    // the fallback when the site is laid out differently.
    let url = match downloads_page(&launcher).await {
        Ok(page) => newest_build(&page, &instance.version)
            .and_then(|file| Url::parse_with_params(MIRROR, [("f", file)]).ok()),
        Err(_) => None,
    }
    .unwrap_or_else(|| DOWNLOADS.parse().expect("the downloads address is valid"));
    // The navigation that turns into the download also reports a finished load, which would
    // bring the window back for a moment after it was hidden.
    let downloading = Arc::new(AtomicBool::new(false));
    let loaded_downloading = Arc::clone(&downloading);
    WebviewWindowBuilder::new(&app, label, WebviewUrl::External(url))
        .title("OptiFine")
        .inner_size(1040.0, 760.0)
        .min_inner_size(640.0, 480.0)
        .initialization_script(page_script(&instance.version))
        // Shown once the page is there, instead of opening as a blank white window first.
        .visible(false)
        .on_page_load(move |window, payload| {
            if payload.event() == PageLoadEvent::Finished
                && !loaded_downloading.load(Ordering::Acquire)
            {
                let _ = window.show();
                let _ = window.set_focus();
            }
        })
        .on_navigation(|url| url.host_str().is_some_and(is_site))
        .on_download(move |webview, event| {
            // The webview's own download leaves half-written files behind when the site
            // stumbles, so the launcher fetches the file itself and cancels the webview's.
            if let DownloadEvent::Requested { url, destination } = event
                && let Some(name) = jar_name(&url, destination)
            {
                downloading.store(true, Ordering::Release);
                let app = webview.app_handle().clone();
                let window = webview.window();
                let path = mods.join(name);
                let instance_id = instance_id.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = window.hide();
                    let result = fetch_jar(&app, url.as_str(), &path).await;
                    let _ = window.destroy();
                    match result {
                        Ok(()) => {
                            log::info!("OptiFine saved as {}", path.display());
                            let _ = app.emit("optifine-installed", &instance_id);
                        }
                        Err(err) => {
                            log::warn!("OptiFine download failed: {err}");
                            let _ = app.emit("optifine-failed", &instance_id);
                        }
                    }
                });
            }
            false
        })
        .build()
        .map_err(|err| CommandError::Io(std::io::Error::other(err)))?;
    Ok(())
}

/// Downloads next to the target and moves the file in only once it is whole and really a jar,
/// since an overloaded site answers with an error page instead.
async fn fetch_jar(app: &AppHandle, url: &str, path: &Path) -> selene_core::Result<()> {
    let launcher = app.state::<Launcher>();
    let partial = path.with_extension("jar.part");
    // Without a checksum to go by, a leftover part would be taken as already downloaded.
    let _ = tokio::fs::remove_file(&partial).await;
    download::download_all(
        &launcher.client,
        vec![Download {
            url: url.to_owned(),
            path: partial.clone(),
            sha1: String::new(),
            size: 0,
        }],
        1,
        |_| {},
    )
    .await?;
    let mut magic = [0; 2];
    let is_jar = std::fs::File::open(&partial)
        .and_then(|mut file| std::io::Read::read_exact(&mut file, &mut magic))
        .is_ok()
        && magic == *b"PK";
    if !is_jar {
        let _ = tokio::fs::remove_file(&partial).await;
        return Err(selene_core::Error::NotTheFile {
            url: url.to_owned(),
            reason: "a page",
        });
    }
    selene_core::fs::rename(&partial, path).await
}

fn is_site(host: &str) -> bool {
    host == SITE
        || host
            .strip_suffix(SITE)
            .is_some_and(|sub| sub.ends_with('.'))
}

/// Runs on the downloads page. Older versions sit in a hidden block there, so it is unfolded
/// before scrolling to the instance's version. The main download buttons go through an ad
/// service that opens other downloads in this window; they are pointed at the site's own
/// mirror link instead, which the page offers next to each of them.
fn page_script(version: &str) -> String {
    let heading = serde_json::to_string(&format!("Minecraft {version}")).unwrap_or_default();
    format!(
        r#"addEventListener("DOMContentLoaded", () => {{
  if (location.pathname !== "/downloads") return;
  for (const link of document.querySelectorAll('a[href*="adfoc.us"]')) {{
    const target = new URL(link.href).searchParams.get("url");
    if (target) {{
      link.href = target;
      link.removeAttribute("target");
    }}
  }}
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
        assert!(!is_site("adfoc.us"));
    }

    #[test]
    fn finds_the_newest_release() {
        let page = r#"<h2>Minecraft 1.8.9</h2>
<a href="http://adfoc.us/serve/sitelinks/?id=1&url=http://optifine.net/adloadx?f=preview_OptiFine_1.8.9_HD_U_M6_pre2.jar&x=e">x</a>
<a href="http://optifine.net/adloadx?f=preview_OptiFine_1.8.9_HD_U_M6_pre2.jar">(Mirror)</a>
<a href="http://optifine.net/adloadx?f=OptiFine_1.8.9_HD_U_M5.jar">(Mirror)</a>
<h2>Minecraft 1.8</h2><a href="http://optifine.net/adloadx?f=OptiFine_1.8.0_HD_U_I7.jar">"#;
        assert_eq!(
            newest_build(page, "1.8.9").as_deref(),
            Some("OptiFine_1.8.9_HD_U_M5.jar")
        );
        assert_eq!(
            newest_build(page, "1.8").as_deref(),
            Some("OptiFine_1.8.0_HD_U_I7.jar")
        );
        assert_eq!(newest_build(page, "1.12.2"), None);
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
