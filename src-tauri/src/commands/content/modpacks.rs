//! Ready-made modpacks from Modrinth and CurseForge: found here, downloaded, and then
//! imported like a pack file dropped on the window.

use selene_core::download::{self, Download};
use selene_core::modrinth;
use selene_core::mods::CatalogSort;
use tauri::{AppHandle, State};

use super::{CatalogHit, CatalogPage, PAGE_SIZE, Source, curseforge_api};
use crate::commands::packs::{ImportResult, import_file};
use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

#[tauri::command]
pub async fn search_modpacks(
    launcher: State<'_, Launcher>,
    source: Source,
    query: String,
    sort: CatalogSort,
    page: u32,
) -> CommandResult<CatalogPage> {
    let offset = page * PAGE_SIZE;
    match source {
        Source::Modrinth => {
            let results =
                modrinth::search_modpacks(&launcher.client, &query, sort, offset, PAGE_SIZE)
                    .await?;
            Ok(CatalogPage {
                has_more: offset + PAGE_SIZE < results.total_hits,
                hits: results
                    .hits
                    .into_iter()
                    .map(|hit| CatalogHit {
                        source,
                        page_url: format!("https://modrinth.com/modpack/{}", hit.slug),
                        id: hit.project_id,
                        title: hit.title,
                        description: hit.description,
                        author: hit.author,
                        icon_url: hit.icon_url,
                        downloads: hit.downloads,
                    })
                    .collect(),
            })
        }
        Source::Curseforge => {
            let results = curseforge_api(&launcher)?
                .search_modpacks(&query, sort, offset, PAGE_SIZE)
                .await?;
            Ok(CatalogPage {
                has_more: offset + PAGE_SIZE < results.total,
                hits: results
                    .mods
                    .into_iter()
                    .map(|found| CatalogHit {
                        source,
                        id: found.id.to_string(),
                        author: found
                            .authors
                            .first()
                            .map(|author| author.name.clone())
                            .unwrap_or_default(),
                        icon_url: found.logo.map(|logo| logo.thumbnail_url),
                        downloads: found.download_count,
                        page_url: found.links.website_url,
                        title: found.name,
                        description: found.summary,
                    })
                    .collect(),
            })
        }
    }
}

/// Downloads the newest version of a modpack and makes an instance of it.
#[tauri::command]
pub async fn install_modpack(
    app: AppHandle,
    launcher: State<'_, Launcher>,
    source: Source,
    project_id: String,
) -> CommandResult<ImportResult> {
    let (url, file_name, sha1, size) = match source {
        Source::Modrinth => {
            let versions = modrinth::all_versions(&launcher.client, &project_id).await?;
            let file = versions
                .iter()
                .find_map(|version| {
                    version
                        .primary_file()
                        .filter(|file| file.filename.ends_with(".mrpack"))
                })
                .ok_or(CommandError::NoCompatibleVersion)?;
            (
                file.url.clone(),
                file.filename.clone(),
                file.hashes.sha1.clone(),
                file.size,
            )
        }
        Source::Curseforge => {
            let id = project_id
                .parse()
                .map_err(|_| CommandError::NoCompatibleVersion)?;
            let file = curseforge_api(&launcher)?
                .latest_file(id)
                .await?
                .ok_or(CommandError::NoCompatibleVersion)?;
            let remote = file
                .remote_file()
                .ok_or(CommandError::NoCompatibleVersion)?;
            (remote.url, remote.file_name, remote.sha1, remote.size)
        }
    };
    let dir = launcher.dirs.root.join(".modpacks");
    let path = dir.join(sanitize(&file_name));
    download::download_all(
        &launcher.client,
        vec![Download {
            url,
            path: path.clone(),
            sha1,
            size,
        }],
        1,
        |_| {},
    )
    .await?;
    let result = import_file(&app, &launcher, path.clone()).await;
    let _ = tokio::fs::remove_file(&path).await;
    let _ = tokio::fs::remove_dir(&dir).await;
    result
}

/// A file name from a site, made safe to put on disk.
fn sanitize(name: &str) -> String {
    let name: String = name
        .chars()
        .map(|ch| {
            if r#"<>:"/\|?*"#.contains(ch) || ch.is_control() {
                '_'
            } else {
                ch
            }
        })
        .collect();
    let known = std::path::Path::new(&name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zip") || ext.eq_ignore_ascii_case("mrpack"));
    if known { name } else { format!("{name}.zip") }
}
