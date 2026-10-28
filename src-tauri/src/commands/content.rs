//! Mods, resource packs and shader packs: the catalog and what is installed.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use selene_core::curseforge::{self, CurseForge};
use selene_core::instance::Instance;
use selene_core::loader::{Loader, LoaderVersion};
use selene_core::modrinth::{self, DependencyType, Project, SearchQuery, Target, Version};
use selene_core::mods::{self, CatalogSort, ContentKind, ModFile, RemoteFile};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

const PAGE_SIZE: u32 = 20;
const FABRIC_API: &str = "P7dR8mSH";
const QUILTED_FABRIC_API: &str = "qvIfYCYJ";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
pub struct CategoryTag {
    id: String,
    name: String,
}

/// Where an installed file came from, when either site recognises it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectRef {
    source: Source,
    id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Issue {
    /// A required mod is not installed.
    Requires { title: String },
    /// An installed mod is marked as incompatible with this one.
    ConflictsWith { title: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledFile {
    #[serde(flatten)]
    file: ModFile,
    title: Option<String>,
    icon_url: Option<String>,
    version_number: Option<String>,
    project: Option<ProjectRef>,
    issues: Vec<Issue>,
    /// `None` when neither site knows the file.
    fits_version: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    installed: usize,
    /// Files whose authors only allow downloads from the CurseForge website.
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
pub struct ContentUpdate {
    file_name: String,
    title: String,
    from: Option<String>,
    to: String,
}

/// A catalog entry the user picked.
#[derive(Debug, Clone, Deserialize)]
pub struct CatalogItem {
    source: Source,
    id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedFile {
    title: String,
    /// `false` for dependencies that come along with the picked projects.
    requested: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPreview {
    mods: Vec<PlannedFile>,
    manual: Vec<ManualDownload>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectVersion {
    id: String,
    name: String,
    version_number: String,
    published: Option<String>,
    /// `release`, `beta` or `alpha`.
    channel: &'static str,
    /// Markdown for Modrinth, HTML for CurseForge; CurseForge sends it separately.
    changelog: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPage {
    title: String,
    description: String,
    /// `markdown` or `html`.
    format: &'static str,
    body: String,
    icon_url: Option<String>,
    gallery: Vec<String>,
    page_url: String,
    downloads: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionChange {
    /// Mods that have a build for the new version and will be swapped for it.
    updated: Vec<ContentUpdate>,
    /// Mods with no build for the new version; they get disabled, not deleted.
    incompatible: Vec<String>,
    /// Mods neither site could check; they stay as they are.
    unknown: Vec<String>,
}

struct Plan {
    files: Vec<RemoteFile>,
    preview: InstallPreview,
}

struct ChangePlan {
    updates: Vec<(ModFile, RemoteFile, ContentUpdate)>,
    incompatible: Vec<(ModFile, String)>,
    unknown: Vec<String>,
}

/// Searches one source, already filtered to what runs on the instance.
#[tauri::command]
#[allow(clippy::too_many_arguments, reason = "one argument per search filter")]
pub async fn search_catalog(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
    source: Source,
    query: String,
    categories: Vec<String>,
    sort: CatalogSort,
    page: u32,
) -> CommandResult<CatalogPage> {
    let instance = instance(&launcher, &instance_id).await?;
    let loader = loader_for(&instance, kind)?;
    let offset = page * PAGE_SIZE;

    match source {
        Source::Modrinth => {
            let results = modrinth::search(
                &launcher.client,
                &SearchQuery {
                    text: &query,
                    kind,
                    target: Target::new(kind, &instance.version, loader),
                    categories: &categories,
                    sort,
                    offset,
                    limit: PAGE_SIZE,
                },
            )
            .await?;
            let section = modrinth_section(kind);
            Ok(CatalogPage {
                has_more: offset + PAGE_SIZE < results.total_hits,
                hits: results
                    .hits
                    .into_iter()
                    .map(|hit| CatalogHit {
                        source,
                        page_url: format!("https://modrinth.com/{section}/{}", hit.slug),
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
                .search(&curseforge::SearchQuery {
                    text: &query,
                    kind,
                    game_version: &instance.version,
                    loader,
                    categories: &categories
                        .iter()
                        .filter_map(|id| id.parse().ok())
                        .collect::<Vec<_>>(),
                    sort,
                    offset,
                    limit: PAGE_SIZE,
                })
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

#[tauri::command]
pub async fn catalog_categories(
    launcher: State<'_, Launcher>,
    kind: ContentKind,
    source: Source,
) -> CommandResult<Vec<CategoryTag>> {
    let mut tags: Vec<CategoryTag> = match source {
        Source::Modrinth => modrinth::categories(&launcher.client, kind)
            .await?
            .into_iter()
            .map(|category| CategoryTag {
                id: category.name.clone(),
                name: category.name,
            })
            .collect(),
        Source::Curseforge => curseforge(&launcher)?
            .categories(kind)
            .await?
            .into_iter()
            .map(|category| CategoryTag {
                id: category.id.to_string(),
                name: category.name,
            })
            .collect(),
    };
    tags.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(tags)
}

/// Catalog entries already in the instance, as `source:id`. A file uploaded to both sites
/// counts for both.
#[tauri::command]
pub async fn installed_projects(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
) -> CommandResult<Vec<String>> {
    let game_dir = launcher.instances.game_dir(&instance_id);
    let files = mods::list(&game_dir, kind).await?;
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
        curseforge_matches(curseforge, &game_dir, kind, &files)
            .await
            .into_values()
            .map(|mod_id| format!("curseforge:{mod_id}"))
            .collect()
    };
    let (mut installed, from_curseforge) = tokio::join!(from_modrinth, from_curseforge);
    installed.extend(from_curseforge);
    installed.sort();
    installed.dedup();
    Ok(installed)
}

/// Installed files with names, icons and problems for the ones either site recognises.
#[tauri::command]
pub async fn installed_content(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
) -> CommandResult<Vec<InstalledFile>> {
    list_installed(&launcher, &instance_id, kind).await
}

async fn list_installed(
    launcher: &Launcher,
    instance_id: &str,
    kind: ContentKind,
) -> CommandResult<Vec<InstalledFile>> {
    let instance = instance(launcher, instance_id).await?;
    let game_dir = launcher.instances.game_dir(instance_id);
    let files = mods::list(&game_dir, kind).await?;
    let hashes: Vec<String> = files.iter().map(|file| file.sha1.clone()).collect();
    // Without a connection the list still shows, just with file names only.
    let versions = modrinth::versions_by_hash(&launcher.client, &hashes)
        .await
        .unwrap_or_default();

    let installed_ids: HashSet<&str> = files
        .iter()
        .filter(|file| file.enabled)
        .filter_map(|file| versions.get(&file.sha1))
        .map(|version| version.project_id.as_str())
        .collect();
    let mut project_ids: Vec<String> = versions
        .values()
        .flat_map(|version| {
            std::iter::once(version.project_id.clone()).chain(
                version
                    .dependencies
                    .iter()
                    .filter_map(|dependency| dependency.project_id.clone()),
            )
        })
        .collect();
    project_ids.sort();
    project_ids.dedup();
    let projects: HashMap<String, Project> = modrinth::projects(&launcher.client, &project_ids)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|project| (project.id.clone(), project))
        .collect();

    // Files Modrinth does not know may still be on CurseForge.
    let unknown: Vec<ModFile> = files
        .iter()
        .filter(|file| !versions.contains_key(&file.sha1))
        .cloned()
        .collect();
    let curseforge_files = curseforge_projects(launcher, &game_dir, kind, &unknown).await;

    let satisfied = |project_id: &str| {
        installed_ids.contains(project_id)
            || (project_id == FABRIC_API
                && instance.loader.as_ref().map(|loader| loader.loader) == Some(Loader::Quilt)
                && installed_ids.contains(QUILTED_FABRIC_API))
    };
    let loader = instance.loader.as_ref().map(|loader| loader.loader);
    let target = Target::new(kind, &instance.version, loader);
    let fits = |version: &Version| {
        (kind != ContentKind::Mod || loader.is_some()) && target.accepts(version)
    };
    let title_of = |project_id: &str| {
        projects
            .get(project_id)
            .map_or_else(|| project_id.to_owned(), |project| project.title.clone())
    };

    let mut installed: Vec<InstalledFile> = files
        .into_iter()
        .map(|file| {
            if let Some(version) = versions.get(&file.sha1) {
                let project = projects.get(&version.project_id);
                let issues = if file.enabled {
                    issues(version, &satisfied, &installed_ids, &title_of)
                } else {
                    Vec::new()
                };
                return InstalledFile {
                    title: project.map(|project| project.title.clone()),
                    icon_url: project.and_then(|project| project.icon_url.clone()),
                    version_number: Some(version.version_number.clone()),
                    project: Some(ProjectRef {
                        source: Source::Modrinth,
                        id: version.project_id.clone(),
                    }),
                    issues,
                    fits_version: Some(fits(version)),
                    file,
                };
            }
            let found = curseforge_files.get(&file.sha1);
            InstalledFile {
                title: found.map(|found| found.name.clone()),
                icon_url: found
                    .and_then(|found| found.logo.as_ref().map(|logo| logo.thumbnail_url.clone())),
                version_number: None,
                project: found.map(|found| ProjectRef {
                    source: Source::Curseforge,
                    id: found.id.to_string(),
                }),
                issues: Vec::new(),
                fits_version: None,
                file,
            }
        })
        .collect();
    installed.sort_by_cached_key(|installed| display_name(installed).to_lowercase());
    Ok(installed)
}

/// What installing the picked projects would download, dependencies included.
#[tauri::command]
pub async fn preview_install(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
    items: Vec<CatalogItem>,
) -> CommandResult<InstallPreview> {
    Ok(plan(&launcher, &instance_id, kind, &items).await?.preview)
}

/// Installs the picked projects and their required dependencies.
#[tauri::command]
pub async fn install_content(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
    items: Vec<CatalogItem>,
) -> CommandResult<InstallResult> {
    let plan = plan(&launcher, &instance_id, kind, &items).await?;
    if plan.files.is_empty() && plan.preview.manual.is_empty() {
        return Err(CommandError::NoCompatibleVersion);
    }
    let game_dir = launcher.instances.game_dir(&instance_id);
    mods::install(&launcher.client, &game_dir, kind, &plan.files).await?;
    Ok(InstallResult {
        installed: plan.files.len(),
        manual: plan.preview.manual,
    })
}

async fn plan(
    launcher: &Launcher,
    instance_id: &str,
    kind: ContentKind,
    items: &[CatalogItem],
) -> CommandResult<Plan> {
    let instance = instance(launcher, instance_id).await?;
    let loader = loader_for(&instance, kind)?;
    let game_dir = launcher.instances.game_dir(instance_id);
    let present = mods::list(&game_dir, kind).await?;

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
            Target::new(kind, &instance.version, loader),
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
            plan.preview.mods.push(PlannedFile {
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
        let installed = curseforge_matches(curseforge, &game_dir, kind, &present)
            .await
            .into_values();
        let resolved = curseforge
            .resolve_install(
                &curseforge_ids,
                &instance.version,
                curseforge_loader(kind, loader),
                installed,
            )
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
            plan.preview.mods.push(PlannedFile {
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

    // Requested projects first, then the dependencies they pull in.
    plan.preview.mods.sort_by_key(|planned| !planned.requested);
    Ok(plan)
}

/// Problems a Modrinth version declares with the rest of the instance.
fn issues(
    version: &modrinth::Version,
    satisfied: &impl Fn(&str) -> bool,
    installed: &HashSet<&str>,
    title_of: &impl Fn(&str) -> String,
) -> Vec<Issue> {
    version
        .dependencies
        .iter()
        .filter_map(|dependency| {
            let id = dependency.project_id.as_deref()?;
            match dependency.dependency_type {
                DependencyType::Required if !satisfied(id) => Some(Issue::Requires {
                    title: title_of(id),
                }),
                DependencyType::Incompatible if installed.contains(id) => {
                    Some(Issue::ConflictsWith {
                        title: title_of(id),
                    })
                }
                _ => None,
            }
        })
        .collect()
}

/// CurseForge projects of the files it recognises, keyed by the file's SHA-1.
async fn curseforge_projects(
    launcher: &Launcher,
    game_dir: &Path,
    kind: ContentKind,
    files: &[ModFile],
) -> HashMap<String, curseforge::Mod> {
    let Some(curseforge) = &launcher.curseforge else {
        return HashMap::new();
    };
    if files.is_empty() {
        return HashMap::new();
    }
    let matches = curseforge_matches(curseforge, game_dir, kind, files).await;
    let ids: Vec<u32> = matches.values().copied().collect();
    let info: HashMap<u32, curseforge::Mod> = curseforge
        .mods(&ids)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|found| (found.id, found))
        .collect();
    matches
        .into_iter()
        .filter_map(|(sha1, id)| Some((sha1, info.get(&id)?.clone())))
        .collect()
}

/// CurseForge mod ids of the files it recognises, keyed by the file's SHA-1.
async fn curseforge_matches(
    curseforge: &CurseForge,
    game_dir: &Path,
    kind: ContentKind,
    files: &[ModFile],
) -> HashMap<String, u32> {
    let mut by_fingerprint = HashMap::with_capacity(files.len());
    for file in files {
        let path = kind.dir(game_dir).join(&file.file_name);
        if let Ok(bytes) = tokio::fs::read(&path).await {
            by_fingerprint.insert(curseforge::fingerprint(&bytes), file.sha1.clone());
        }
    }
    let fingerprints: Vec<u32> = by_fingerprint.keys().copied().collect();
    curseforge
        .files_by_fingerprint(&fingerprints)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(fingerprint, known)| {
            Some((by_fingerprint.remove(&fingerprint)?, known.mod_id))
        })
        .collect()
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
pub async fn set_content_enabled(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
    file_name: String,
    enabled: bool,
) -> CommandResult<()> {
    let game_dir = launcher.instances.game_dir(&instance_id);
    Ok(mods::set_enabled(&game_dir, kind, &file_name, enabled).await?)
}

#[tauri::command]
pub async fn remove_content(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
    file_name: String,
) -> CommandResult<()> {
    let game_dir = launcher.instances.game_dir(&instance_id);
    Ok(mods::remove(&game_dir, kind, &file_name).await?)
}

/// Newer versions of installed files that run on this instance.
#[tauri::command]
pub async fn check_updates(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
) -> CommandResult<Vec<ContentUpdate>> {
    let installed = list_installed(&launcher, &instance_id, kind).await?;
    let files: Vec<ModFile> = installed
        .iter()
        .map(|installed| installed.file.clone())
        .collect();
    let updates = available_updates(&launcher, &instance_id, kind, &files).await?;
    Ok(installed
        .iter()
        .filter_map(|installed| {
            let (_, update) = updates
                .iter()
                .find(|(file, _)| file.file_name == installed.file.file_name)?;
            Some(ContentUpdate {
                file_name: installed.file.file_name.clone(),
                title: display_name(installed).to_owned(),
                from: installed.version_number.clone(),
                to: update.version_number.clone(),
            })
        })
        .collect())
}

/// Updates every file that has a newer compatible version. Returns how many were updated.
#[tauri::command]
pub async fn update_content(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
) -> CommandResult<u32> {
    update_all(&launcher, &instance_id, kind).await
}

pub(crate) async fn update_all(
    launcher: &Launcher,
    instance_id: &str,
    kind: ContentKind,
) -> CommandResult<u32> {
    let game_dir = launcher.instances.game_dir(instance_id);
    let files = mods::list(&game_dir, kind).await?;
    let mut count = 0;
    for (file, update) in available_updates(launcher, instance_id, kind, &files).await? {
        if let Some(remote) = update.remote_file() {
            mods::replace(&launcher.client, &game_dir, kind, &file, &remote).await?;
            count += 1;
        }
    }
    Ok(count)
}

async fn available_updates(
    launcher: &Launcher,
    instance_id: &str,
    kind: ContentKind,
    files: &[ModFile],
) -> CommandResult<Vec<(ModFile, modrinth::Version)>> {
    let instance = instance(launcher, instance_id).await?;
    let target = Target::new(kind, &instance.version, loader_for(&instance, kind)?);
    let hashes: Vec<String> = files.iter().map(|file| file.sha1.clone()).collect();
    let current = modrinth::versions_by_hash(&launcher.client, &hashes).await?;
    let mut updates = modrinth::updates_by_hash(&launcher.client, &current, target).await?;
    Ok(files
        .iter()
        .filter_map(|file| {
            let update = updates.remove(&file.sha1)?;
            let installed = current.get(&file.sha1)?;
            // A beta newer than the latest release must not be "updated" back to that release.
            let newer =
                update.id != installed.id && update.date_published >= installed.date_published;
            newer.then(|| (file.clone(), update))
        })
        .collect())
}

/// What moving the instance to another game version would do to its mods.
#[tauri::command]
pub async fn preview_version_change(
    launcher: State<'_, Launcher>,
    instance_id: String,
    version: String,
    loader: Option<LoaderVersion>,
) -> CommandResult<VersionChange> {
    let plan = change_plan(&launcher, &instance_id, &version, loader.as_ref()).await?;
    Ok(VersionChange {
        updated: plan
            .updates
            .into_iter()
            .map(|(_, _, update)| update)
            .collect(),
        incompatible: plan
            .incompatible
            .into_iter()
            .map(|(_, title)| title)
            .collect(),
        unknown: plan.unknown,
    })
}

/// Moves the instance to another game version, swapping mods for matching builds and
/// disabling the ones that have none.
#[tauri::command]
pub async fn change_version(
    launcher: State<'_, Launcher>,
    instance_id: String,
    version: String,
    loader: Option<LoaderVersion>,
) -> CommandResult<Instance> {
    if launcher.is_running(&instance_id) {
        return Err(CommandError::AlreadyRunning(instance_id));
    }
    if launcher.manifest().await?.get(&version).is_none() {
        return Err(CommandError::UnknownVersion(version));
    }
    let plan = change_plan(&launcher, &instance_id, &version, loader.as_ref()).await?;
    let game_dir = launcher.instances.game_dir(&instance_id);
    for (file, remote, _) in &plan.updates {
        mods::replace(&launcher.client, &game_dir, ContentKind::Mod, file, remote).await?;
    }
    for (file, _) in &plan.incompatible {
        mods::set_enabled(&game_dir, ContentKind::Mod, &file.file_name, false).await?;
    }
    let mut instance = instance(&launcher, &instance_id).await?;
    instance.version = version;
    instance.loader = loader;
    launcher.instances.save(&instance).await?;
    Ok(instance)
}

async fn change_plan(
    launcher: &Launcher,
    instance_id: &str,
    version: &str,
    loader: Option<&LoaderVersion>,
) -> CommandResult<ChangePlan> {
    let mut plan = ChangePlan {
        updates: Vec::new(),
        incompatible: Vec::new(),
        unknown: Vec::new(),
    };
    let installed = list_installed(launcher, instance_id, ContentKind::Mod).await?;
    let Some(loader) = loader else {
        // Without a loader no mod runs at all.
        plan.incompatible = installed
            .into_iter()
            .filter(|installed| installed.file.enabled)
            .map(|installed| {
                let title = display_name(&installed).to_owned();
                (installed.file, title)
            })
            .collect();
        return Ok(plan);
    };

    let target = Target::new(ContentKind::Mod, version, Some(loader.loader));
    let hashes: Vec<String> = installed
        .iter()
        .map(|file| file.file.sha1.clone())
        .collect();
    let current = modrinth::versions_by_hash(&launcher.client, &hashes).await?;
    let mut builds = modrinth::updates_by_hash(&launcher.client, &current, target).await?;
    for installed in installed
        .into_iter()
        .filter(|installed| installed.file.enabled)
    {
        let title = display_name(&installed).to_owned();
        let build = builds.remove(&installed.file.sha1);
        match (&installed.project, build) {
            (_, Some(build)) => {
                let Some(remote) = build.remote_file() else {
                    continue;
                };
                if remote.sha1 == installed.file.sha1 {
                    continue;
                }
                let update = ContentUpdate {
                    file_name: installed.file.file_name.clone(),
                    title,
                    from: installed.version_number.clone(),
                    to: build.version_number,
                };
                plan.updates.push((installed.file, remote, update));
            }
            (
                Some(ProjectRef {
                    source: Source::Modrinth,
                    ..
                }),
                None,
            ) => {
                plan.incompatible.push((installed.file, title));
            }
            _ => plan.unknown.push(title),
        }
    }
    Ok(plan)
}

/// Versions of a project that run on this instance, newest first.
#[tauri::command]
pub async fn project_versions(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
    project: ProjectRef,
) -> CommandResult<Vec<ProjectVersion>> {
    let instance = instance(&launcher, &instance_id).await?;
    let loader = loader_for(&instance, kind)?;
    match project.source {
        Source::Modrinth => {
            let target = Target::new(kind, &instance.version, loader);
            Ok(
                modrinth::compatible_versions(&launcher.client, &project.id, target)
                    .await?
                    .into_iter()
                    .map(|version| ProjectVersion {
                        channel: channel(&version.version_type),
                        id: version.id,
                        name: version.name,
                        version_number: version.version_number,
                        published: version.date_published.map(|date| date.to_rfc3339()),
                        changelog: version.changelog,
                    })
                    .collect(),
            )
        }
        Source::Curseforge => Ok(curseforge(&launcher)?
            .compatible_files(
                curseforge_id(&project.id)?,
                &instance.version,
                curseforge_loader(kind, loader),
            )
            .await?
            .into_iter()
            .map(|file| ProjectVersion {
                id: file.id.to_string(),
                version_number: file.display_name.clone(),
                name: file.display_name,
                published: file.file_date,
                channel: match file.release_type {
                    2 => "beta",
                    3 => "alpha",
                    _ => "release",
                },
                changelog: None,
            })
            .collect()),
    }
}

#[tauri::command]
pub async fn curseforge_changelog(
    launcher: State<'_, Launcher>,
    project_id: String,
    file_id: String,
) -> CommandResult<String> {
    Ok(curseforge(&launcher)?
        .changelog(curseforge_id(&project_id)?, curseforge_id(&file_id)?)
        .await?)
}

/// Swaps an installed file for a chosen version of the same project, older or newer.
#[tauri::command]
pub async fn install_version(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
    project: ProjectRef,
    version_id: String,
    file_name: String,
) -> CommandResult<()> {
    let game_dir = launcher.instances.game_dir(&instance_id);
    let old = mods::list(&game_dir, kind)
        .await?
        .into_iter()
        .find(|file| file.file_name == file_name)
        .ok_or(CommandError::NoCompatibleVersion)?;
    let remote = match project.source {
        Source::Modrinth => modrinth::version(&launcher.client, &version_id)
            .await?
            .remote_file(),
        Source::Curseforge => curseforge(&launcher)?
            .files(&[curseforge_id(&version_id)?])
            .await?
            .into_iter()
            .next()
            .and_then(|file| file.remote_file()),
    }
    .ok_or(CommandError::NoCompatibleVersion)?;
    Ok(mods::replace(&launcher.client, &game_dir, kind, &old, &remote).await?)
}

/// Everything the project page shows, without leaving the launcher.
#[tauri::command]
pub async fn project_page(
    launcher: State<'_, Launcher>,
    kind: ContentKind,
    project: ProjectRef,
) -> CommandResult<ProjectPage> {
    match project.source {
        Source::Modrinth => {
            let details = modrinth::project(&launcher.client, &project.id).await?;
            Ok(ProjectPage {
                page_url: format!(
                    "https://modrinth.com/{}/{}",
                    modrinth_section(kind),
                    details.slug
                ),
                title: details.title,
                description: details.description,
                format: "markdown",
                body: details.body,
                icon_url: details.icon_url,
                gallery: details.gallery.into_iter().map(|image| image.url).collect(),
                downloads: details.downloads,
            })
        }
        Source::Curseforge => {
            let curseforge = curseforge(&launcher)?;
            let id = curseforge_id(&project.id)?;
            let (info, body) =
                tokio::try_join!(curseforge.mod_info(id), curseforge.description(id))?;
            Ok(ProjectPage {
                title: info.name,
                description: info.summary,
                format: "html",
                body,
                icon_url: info.logo.map(|logo| logo.thumbnail_url),
                gallery: info.screenshots.into_iter().map(|shot| shot.url).collect(),
                page_url: info.links.website_url,
                downloads: info.download_count,
            })
        }
    }
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

fn curseforge_id(id: &str) -> CommandResult<u32> {
    id.parse().map_err(|_| CommandError::NoCompatibleVersion)
}

/// The instance's loader, which mods and shaders need. Resource packs run anywhere.
fn loader_for(instance: &Instance, kind: ContentKind) -> CommandResult<Option<Loader>> {
    let loader = instance.loader.as_ref().map(|loader| loader.loader);
    match (kind, loader) {
        (ContentKind::Mod | ContentKind::ShaderPack, None) => Err(CommandError::NoLoader),
        (_, loader) => Ok(loader),
    }
}

/// CurseForge only tags mod files with a loader.
fn curseforge_loader(kind: ContentKind, loader: Option<Loader>) -> Option<Loader> {
    loader.filter(|_| kind == ContentKind::Mod)
}

fn modrinth_section(kind: ContentKind) -> &'static str {
    match kind {
        ContentKind::Mod => "mod",
        ContentKind::ResourcePack => "resourcepack",
        ContentKind::ShaderPack => "shader",
    }
}

fn channel(version_type: &str) -> &'static str {
    match version_type {
        "beta" => "beta",
        "alpha" => "alpha",
        _ => "release",
    }
}

fn display_name(installed: &InstalledFile) -> &str {
    installed
        .title
        .as_deref()
        .unwrap_or(installed.file.file_name.as_str())
}
