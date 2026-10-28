use std::collections::HashMap;

use selene_core::curseforge::CurseForge;
use selene_core::instance::Instance;
use selene_core::loader::Loader;
use selene_core::modrinth::{self, Project, ProjectType, SearchQuery};
use selene_core::mods::{self, ModFile, RemoteFile};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

const PAGE_SIZE: u32 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Modrinth,
    Curseforge,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogHit {
    source: Source,
    id: String,
    title: String,
    description: String,
    author: String,
    icon_url: Option<String>,
    downloads: u64,
    page_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPage {
    hits: Vec<CatalogHit>,
    has_more: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    installed: usize,
    /// Mods whose authors only allow downloads from the CurseForge website.
    manual: Vec<ManualDownload>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualDownload {
    title: String,
    url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledMod {
    #[serde(flatten)]
    file: ModFile,
    project: Option<Project>,
    version_number: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModUpdate {
    file_name: String,
    title: String,
    from: Option<String>,
    to: String,
}

/// Searches one source, already filtered to the instance's version and loader.
#[tauri::command]
pub async fn search_mods(
    launcher: State<'_, Launcher>,
    instance_id: String,
    source: Source,
    query: String,
    page: u32,
) -> CommandResult<CatalogPage> {
    let instance = instance(&launcher, &instance_id).await?;
    let loader = loader_of(&instance)?;
    let offset = page * PAGE_SIZE;

    match source {
        Source::Modrinth => {
            let results = modrinth::search(
                &launcher.client,
                &SearchQuery {
                    text: &query,
                    project_type: ProjectType::Mod,
                    game_version: &instance.version,
                    loader: Some(loader),
                    offset,
                    limit: PAGE_SIZE,
                },
            )
            .await?;
            Ok(CatalogPage {
                has_more: offset + PAGE_SIZE < results.total_hits,
                hits: results
                    .hits
                    .into_iter()
                    .map(|hit| CatalogHit {
                        source,
                        page_url: format!("https://modrinth.com/mod/{}", hit.slug),
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
            let results = curseforge(&launcher)?
                .search(&query, &instance.version, loader, offset, PAGE_SIZE)
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

/// Catalog entries already in the instance, as `source:id`. A jar uploaded to both sites
/// counts for both.
#[tauri::command]
pub async fn installed_projects(
    launcher: State<'_, Launcher>,
    instance_id: String,
) -> CommandResult<Vec<String>> {
    let game_dir = launcher.instances.game_dir(&instance_id);
    let files = mods::list(&game_dir).await?;
    let hashes: Vec<String> = files.iter().map(|file| file.sha1.clone()).collect();

    let from_modrinth = async {
        modrinth::versions_by_hash(&launcher.client, &hashes)
            .await
            .map(|versions| {
                versions
                    .into_values()
                    .map(|version| format!("modrinth:{}", version.project_id))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let from_curseforge = async {
        let Some(curseforge) = &launcher.curseforge else {
            return Vec::new();
        };
        installed_curseforge_mods(curseforge, &game_dir, &files)
            .await
            .into_iter()
            .map(|mod_id| format!("curseforge:{mod_id}"))
            .collect()
    };
    let (mut installed, from_curseforge) = tokio::join!(from_modrinth, from_curseforge);
    installed.extend(from_curseforge);
    installed.sort();
    installed.dedup();
    Ok(installed)
}

/// Mods in the instance, with names and icons for the ones Modrinth recognises by hash.
#[tauri::command]
pub async fn installed_mods(
    launcher: State<'_, Launcher>,
    instance_id: String,
) -> CommandResult<Vec<InstalledMod>> {
    let game_dir = launcher.instances.game_dir(&instance_id);
    let files = mods::list(&game_dir).await?;
    let hashes: Vec<String> = files.iter().map(|file| file.sha1.clone()).collect();
    // Without a connection the list still shows, just with file names only.
    let versions = modrinth::versions_by_hash(&launcher.client, &hashes)
        .await
        .unwrap_or_default();

    let mut project_ids: Vec<String> = versions
        .values()
        .map(|version| version.project_id.clone())
        .collect();
    project_ids.sort();
    project_ids.dedup();
    let projects: HashMap<String, Project> = modrinth::projects(&launcher.client, &project_ids)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|project| (project.id.clone(), project))
        .collect();

    let mut installed: Vec<InstalledMod> = files
        .into_iter()
        .map(|file| {
            let version = versions.get(&file.sha1);
            InstalledMod {
                project: version.and_then(|version| projects.get(&version.project_id).cloned()),
                version_number: version.map(|version| version.version_number.clone()),
                file,
            }
        })
        .collect();
    installed.sort_by_cached_key(|installed| display_name(installed).to_lowercase());
    Ok(installed)
}

/// A catalog entry the user picked.
#[derive(Debug, Clone, Deserialize)]
pub struct CatalogItem {
    source: Source,
    id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedMod {
    title: String,
    /// `false` for dependencies that come along with the picked mods.
    requested: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPreview {
    mods: Vec<PlannedMod>,
    manual: Vec<ManualDownload>,
}

struct Plan {
    files: Vec<RemoteFile>,
    preview: InstallPreview,
}

/// What installing the picked mods would download, dependencies included.
#[tauri::command]
pub async fn preview_install(
    launcher: State<'_, Launcher>,
    instance_id: String,
    items: Vec<CatalogItem>,
) -> CommandResult<InstallPreview> {
    Ok(plan(&launcher, &instance_id, &items).await?.preview)
}

/// Installs the picked mods and their required dependencies.
#[tauri::command]
pub async fn install_mods(
    launcher: State<'_, Launcher>,
    instance_id: String,
    items: Vec<CatalogItem>,
) -> CommandResult<InstallResult> {
    let plan = plan(&launcher, &instance_id, &items).await?;
    if plan.files.is_empty() && plan.preview.manual.is_empty() {
        return Err(CommandError::NoCompatibleVersion);
    }
    let game_dir = launcher.instances.game_dir(&instance_id);
    mods::install(&launcher.client, &game_dir, &plan.files).await?;
    Ok(InstallResult {
        installed: plan.files.len(),
        manual: plan.preview.manual,
    })
}

async fn plan(
    launcher: &Launcher,
    instance_id: &str,
    items: &[CatalogItem],
) -> CommandResult<Plan> {
    let instance = instance(launcher, instance_id).await?;
    let loader = loader_of(&instance)?;
    let game_dir = launcher.instances.game_dir(instance_id);
    let present = mods::list(&game_dir).await?;

    let ids_from = |source: Source| -> Vec<String> {
        items
            .iter()
            .filter(|item| item.source == source)
            .map(|item| item.id.clone())
            .collect()
    };
    let mut plan = Plan {
        files: Vec::new(),
        preview: InstallPreview {
            mods: Vec::new(),
            manual: Vec::new(),
        },
    };

    let modrinth_ids = ids_from(Source::Modrinth);
    if !modrinth_ids.is_empty() {
        let hashes: Vec<String> = present.iter().map(|file| file.sha1.clone()).collect();
        let installed = modrinth::versions_by_hash(&launcher.client, &hashes).await?;
        let versions = modrinth::resolve_install(
            &launcher.client,
            &modrinth_ids,
            &instance.version,
            Some(loader),
            installed.into_values().map(|version| version.project_id),
        )
        .await?;
        let project_ids: Vec<String> = versions.iter().map(|v| v.project_id.clone()).collect();
        let titles: HashMap<String, String> = modrinth::projects(&launcher.client, &project_ids)
            .await?
            .into_iter()
            .map(|project| (project.id, project.title))
            .collect();
        for version in &versions {
            let Some(file) = version.remote_file() else {
                continue;
            };
            plan.preview.mods.push(PlannedMod {
                title: titles
                    .get(&version.project_id)
                    .cloned()
                    .unwrap_or_else(|| file.file_name.clone()),
                requested: modrinth_ids.contains(&version.project_id),
            });
            plan.files.push(file);
        }
    }

    let curseforge_ids: Vec<u32> = ids_from(Source::Curseforge)
        .iter()
        .filter_map(|id| id.parse().ok())
        .collect();
    if !curseforge_ids.is_empty() {
        let curseforge = curseforge(launcher)?;
        let installed = installed_curseforge_mods(curseforge, &game_dir, &present).await;
        let resolved = curseforge
            .resolve_install(&curseforge_ids, &instance.version, loader, installed)
            .await?;
        let mod_ids: Vec<u32> = resolved.files.iter().map(|(id, _)| *id).collect();
        let titles: HashMap<u32, String> = curseforge
            .mods(&mod_ids)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|found| (found.id, found.name))
            .collect();
        for (mod_id, file) in resolved.files {
            plan.preview.mods.push(PlannedMod {
                title: titles
                    .get(&mod_id)
                    .cloned()
                    .unwrap_or_else(|| file.file_name.clone()),
                requested: curseforge_ids.contains(&mod_id),
            });
            plan.files.push(file);
        }
        plan.preview.manual = manual_downloads(curseforge, &resolved.manual).await;
    }

    // Requested mods first, then the dependencies they pull in.
    plan.preview.mods.sort_by_key(|planned| !planned.requested);
    Ok(plan)
}

async fn installed_curseforge_mods(
    curseforge: &CurseForge,
    game_dir: &std::path::Path,
    present: &[ModFile],
) -> Vec<u32> {
    let mut fingerprints = Vec::with_capacity(present.len());
    for file in present {
        let path = mods::mods_dir(game_dir).join(&file.file_name);
        if let Ok(bytes) = tokio::fs::read(&path).await {
            fingerprints.push(selene_core::curseforge::fingerprint(&bytes));
        }
    }
    curseforge
        .files_by_fingerprint(&fingerprints)
        .await
        .map(|found| found.into_values().map(|file| file.mod_id).collect())
        .unwrap_or_default()
}

pub(crate) async fn manual_downloads(
    curseforge: &CurseForge,
    mod_ids: &[u32],
) -> Vec<ManualDownload> {
    let mut downloads = Vec::new();
    for &mod_id in mod_ids {
        if let Ok(info) = curseforge.mod_info(mod_id).await {
            downloads.push(ManualDownload {
                title: info.name,
                url: format!("{}/files", info.links.website_url),
            });
        }
    }
    downloads
}

#[tauri::command]
pub async fn set_mod_enabled(
    launcher: State<'_, Launcher>,
    instance_id: String,
    file_name: String,
    enabled: bool,
) -> CommandResult<()> {
    let game_dir = launcher.instances.game_dir(&instance_id);
    Ok(mods::set_enabled(&game_dir, &file_name, enabled).await?)
}

#[tauri::command]
pub async fn remove_mod(
    launcher: State<'_, Launcher>,
    instance_id: String,
    file_name: String,
) -> CommandResult<()> {
    let game_dir = launcher.instances.game_dir(&instance_id);
    Ok(mods::remove(&game_dir, &file_name).await?)
}

/// Newer versions of installed mods for this instance's game version and loader.
#[tauri::command]
pub async fn check_mod_updates(
    launcher: State<'_, Launcher>,
    instance_id: String,
) -> CommandResult<Vec<ModUpdate>> {
    let instance = instance(&launcher, &instance_id).await?;
    let loader = loader_of(&instance)?;
    let installed = installed_mods(launcher.clone(), instance_id).await?;
    let hashes: Vec<String> = installed
        .iter()
        .map(|mod_| mod_.file.sha1.clone())
        .collect();
    let updates =
        modrinth::updates_by_hash(&launcher.client, &hashes, &instance.version, loader).await?;

    Ok(installed
        .iter()
        .filter_map(|mod_| {
            let update = updates.get(&mod_.file.sha1)?;
            let file = update.primary_file()?;
            (file.hashes.sha1 != mod_.file.sha1).then(|| ModUpdate {
                file_name: mod_.file.file_name.clone(),
                title: display_name(mod_).to_owned(),
                from: mod_.version_number.clone(),
                to: update.version_number.clone(),
            })
        })
        .collect())
}

/// Updates every mod that has a newer compatible version. Returns how many were updated.
#[tauri::command]
pub async fn update_mods(launcher: State<'_, Launcher>, instance_id: String) -> CommandResult<u32> {
    let instance = instance(&launcher, &instance_id).await?;
    let loader = loader_of(&instance)?;
    let game_dir = launcher.instances.game_dir(&instance_id);
    let files = mods::list(&game_dir).await?;
    let hashes: Vec<String> = files.iter().map(|file| file.sha1.clone()).collect();
    let available =
        modrinth::updates_by_hash(&launcher.client, &hashes, &instance.version, loader).await?;

    let mut count = 0;
    for file in &files {
        let Some(update) = available.get(&file.sha1) else {
            continue;
        };
        let Some(remote) = update.remote_file() else {
            continue;
        };
        if remote.sha1 != file.sha1 {
            mods::replace(&launcher.client, &game_dir, file, &remote).await?;
            count += 1;
        }
    }
    Ok(count)
}

async fn instance(launcher: &Launcher, id: &str) -> CommandResult<Instance> {
    launcher
        .instances
        .get(id)
        .await
        .ok_or_else(|| CommandError::UnknownInstance(id.to_owned()))
}

fn curseforge(launcher: &Launcher) -> CommandResult<&CurseForge> {
    launcher
        .curseforge
        .as_ref()
        .ok_or(CommandError::NoCompatibleVersion)
}

fn loader_of(instance: &Instance) -> CommandResult<Loader> {
    instance
        .loader
        .as_ref()
        .map(|loader| loader.loader)
        .ok_or(CommandError::NoLoader)
}

fn display_name(installed: &InstalledMod) -> &str {
    installed
        .project
        .as_ref()
        .map_or(installed.file.file_name.as_str(), |project| {
            project.title.as_str()
        })
}
