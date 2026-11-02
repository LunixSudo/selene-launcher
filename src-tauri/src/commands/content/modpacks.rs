//! Ready-made modpacks from Modrinth and CurseForge: found here, downloaded, and then
//! installed like a pack file dropped on the window. An instance made this way remembers its
//! pack, so it can move to another version of it later.

use chrono::{DateTime, Utc};
use selene_core::download::{self, Download};
use selene_core::instance::{PackOrigin, PackSource};
use selene_core::modrinth;
use selene_core::mods::CatalogSort;
use selene_core::mrpack;
use serde::Serialize;
use tauri::{AppHandle, State};

use super::{CatalogHit, CatalogPage, ContentTask, PAGE_SIZE, Source, Stage, curseforge_api};
use crate::commands::packs::{ImportResult, Pack, create, discard_on_error, while_importing};
use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackVersion {
    id: String,
    name: String,
    game_versions: Vec<String>,
    loaders: Vec<String>,
    published: Option<DateTime<Utc>>,
    release: bool,
}

/// The pack file of one version, with what it is needed to start an instance before the
/// file is downloaded.
struct PackDownload {
    download: Download,
    version: PackVersion,
}

#[tauri::command]
pub async fn search_modpacks(
    launcher: State<'_, Launcher>,
    source: Source,
    query: String,
    game_version: Option<String>,
    sort: CatalogSort,
    page: u32,
) -> CommandResult<CatalogPage> {
    let offset = page * PAGE_SIZE;
    let game_version = game_version.as_deref();
    match source {
        Source::Modrinth => {
            let results = modrinth::search_modpacks(
                &launcher.client,
                &query,
                game_version,
                sort,
                offset,
                PAGE_SIZE,
            )
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
                .search_modpacks(&query, game_version, sort, offset, PAGE_SIZE)
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

/// Versions of a modpack, newest first.
#[tauri::command]
pub async fn modpack_versions(
    launcher: State<'_, Launcher>,
    source: Source,
    project_id: String,
) -> CommandResult<Vec<PackVersion>> {
    Ok(pack_downloads(&launcher, source, &project_id)
        .await?
        .into_iter()
        .map(|pack| pack.version)
        .collect())
}

async fn pack_downloads(
    launcher: &Launcher,
    source: Source,
    project_id: &str,
) -> CommandResult<Vec<PackDownload>> {
    match source {
        Source::Modrinth => {
            let versions = modrinth::all_versions(&launcher.client, project_id).await?;
            Ok(versions
                .into_iter()
                .filter_map(|version| {
                    let file = version
                        .primary_file()
                        .filter(|file| file.filename.ends_with(".mrpack"))?;
                    Some(PackDownload {
                        download: Download {
                            url: file.url.clone(),
                            path: sanitize(&file.filename).into(),
                            sha1: file.hashes.sha1.clone(),
                            size: file.size,
                        },
                        version: PackVersion {
                            release: version.version_type == "release",
                            id: version.id,
                            name: version.version_number,
                            game_versions: version.game_versions,
                            loaders: version.loaders,
                            published: version.date_published,
                        },
                    })
                })
                .collect())
        }
        Source::Curseforge => {
            let id = project_id
                .parse()
                .map_err(|_| CommandError::NoCompatibleVersion)?;
            let files = curseforge_api(launcher)?.pack_files(id).await?;
            Ok(files
                .into_iter()
                .filter_map(|file| {
                    let remote = file.remote_file()?;
                    let (game_versions, loaders) = file
                        .game_versions
                        .iter()
                        .cloned()
                        .partition(|name| name.starts_with(|ch: char| ch.is_ascii_digit()));
                    Some(PackDownload {
                        download: Download {
                            url: remote.url,
                            path: sanitize(&remote.file_name).into(),
                            sha1: remote.sha1,
                            size: remote.size,
                        },
                        version: PackVersion {
                            id: file.id.to_string(),
                            name: if file.display_name.is_empty() {
                                file.file_name.clone()
                            } else {
                                file.display_name.clone()
                            },
                            game_versions,
                            loaders,
                            published: file.file_date.as_deref().and_then(|date| date.parse().ok()),
                            release: file.release_type == 1,
                        },
                    })
                })
                .collect())
        }
    }
}

/// The chosen version, or the newest release when none is chosen.
async fn pack_download(
    launcher: &Launcher,
    source: Source,
    project_id: &str,
    version_id: Option<&str>,
) -> CommandResult<PackDownload> {
    let mut packs = pack_downloads(launcher, source, project_id).await?;
    let index = match version_id {
        Some(id) => packs.iter().position(|pack| pack.version.id == id),
        None => packs
            .iter()
            .position(|pack| pack.version.release)
            .or((!packs.is_empty()).then_some(0)),
    }
    .ok_or(CommandError::NoCompatibleVersion)?;
    Ok(packs.swap_remove(index))
}

/// Downloads the pack file next to the instance and reads it. The caller removes the file.
async fn fetch(
    launcher: &Launcher,
    instance_id: &str,
    pack: &PackDownload,
) -> CommandResult<(std::path::PathBuf, Pack)> {
    let name = pack.download.path.to_string_lossy();
    let path = launcher
        .dirs
        .root
        .join(".modpacks")
        .join(format!("{instance_id}-{name}"));
    download::download_all(
        &launcher.client,
        vec![Download {
            path: path.clone(),
            ..pack.download.clone()
        }],
        1,
        |_| {},
    )
    .await?;
    let read = Pack::read(&path).await;
    if read.is_err() {
        remove_pack_file(&path).await;
    }
    Ok((path, read?))
}

async fn remove_pack_file(path: &std::path::Path) {
    let _ = tokio::fs::remove_file(path).await;
    if let Some(dir) = path.parent() {
        let _ = tokio::fs::remove_dir(dir).await;
    }
}

/// Makes an instance of a modpack. It shows up in the list before anything is downloaded.
#[tauri::command]
pub async fn install_modpack(
    app: AppHandle,
    launcher: State<'_, Launcher>,
    source: Source,
    project_id: String,
    version_id: Option<String>,
    title: String,
) -> CommandResult<ImportResult> {
    let pack = pack_download(&launcher, source, &project_id, version_id.as_deref()).await?;
    let manifest = launcher.manifest().await?;
    let game_version = pack
        .version
        .game_versions
        .iter()
        .find(|version| manifest.get(version).is_some())
        .ok_or(CommandError::NoCompatibleVersion)?;
    let instance = create(&app, &launcher, &title, game_version, None).await?;
    let id = instance.id.clone();
    let task = ContentTask::start(&app, &id, Stage::Importing);
    let origin = PackOrigin {
        source: pack_source(source),
        project_id,
        version_id: pack.version.id.clone(),
        version_name: pack.version.name.clone(),
        addons: Vec::new(),
        installing: None,
    };
    let result = while_importing(&launcher, &id, async {
        let mut instance = instance;
        instance.pack = Some(PackOrigin {
            installing: Some(pack.version.id.clone()),
            ..origin.clone()
        });
        launcher.instances.save(&instance).await?;
        let (path, read) = fetch(&launcher, &id, &pack).await?;
        read.game_version()?.clone_into(&mut instance.version);
        instance.loader = read.loader()?;
        launcher.instances.save(&instance).await?;
        let game_dir = launcher.instances.game_dir(&id);
        let installed = read
            .install(&launcher, &path, &game_dir, |progress| {
                task.report(progress);
            })
            .await;
        remove_pack_file(&path).await;
        let (addons, manual) = installed?;
        instance.pack = Some(PackOrigin { addons, ..origin });
        launcher.instances.save(&instance).await?;
        Ok(ImportResult { instance, manual })
    })
    .await;
    drop(task);
    discard_on_error(&launcher, &id, result).await
}

/// Moves an instance to another version of its modpack. Add-ons the old version brought and
/// the new one lacks are deleted; the player's own stay, and so do switched-off ones.
#[tauri::command]
pub async fn update_modpack(
    app: AppHandle,
    launcher: State<'_, Launcher>,
    instance_id: String,
    version_id: Option<String>,
) -> CommandResult<ImportResult> {
    let mut instance = launcher
        .instances
        .get(&instance_id)
        .await
        .ok_or_else(|| CommandError::UnknownInstance(instance_id.clone()))?;
    let origin = instance
        .pack
        .clone()
        .ok_or(CommandError::NoCompatibleVersion)?;
    let source = match origin.source {
        PackSource::Modrinth => Source::Modrinth,
        PackSource::Curseforge => Source::Curseforge,
    };
    let pack = pack_download(&launcher, source, &origin.project_id, version_id.as_deref()).await?;
    if launcher.is_preparing_in_background(&instance_id) {
        launcher.cancel_preparing(&instance_id).await?;
    }
    let task = ContentTask::start(&app, &instance_id, Stage::Updating);
    let result = while_importing(&launcher, &instance_id, async {
        instance.pack = Some(PackOrigin {
            installing: Some(pack.version.id.clone()),
            ..origin.clone()
        });
        launcher.instances.save(&instance).await?;
        let (path, read) = fetch(&launcher, &instance_id, &pack).await?;
        let game_dir = launcher.instances.game_dir(&instance_id);
        let installed = read
            .install(&launcher, &path, &game_dir, |progress| {
                task.report(progress);
            })
            .await;
        remove_pack_file(&path).await;
        let (addons, manual) = installed?;
        mrpack::keep_switched_off(&game_dir, &addons).await?;
        mrpack::remove_dropped(&game_dir, &origin.addons, &addons).await?;
        read.game_version()?.clone_into(&mut instance.version);
        instance.loader = read.loader()?;
        instance.pack = Some(PackOrigin {
            version_id: pack.version.id.clone(),
            version_name: pack.version.name.clone(),
            addons,
            installing: None,
            ..origin
        });
        launcher.instances.save(&instance).await?;
        Ok(ImportResult { instance, manual })
    })
    .await;
    drop(task);
    result
}

fn pack_source(source: Source) -> PackSource {
    match source {
        Source::Modrinth => PackSource::Modrinth,
        Source::Curseforge => PackSource::Curseforge,
    }
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
