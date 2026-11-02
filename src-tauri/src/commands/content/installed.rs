use std::collections::{HashMap, HashSet};
use std::path::Path;

use selene_core::curseforge;
use selene_core::loader::Loader;
use selene_core::modrinth::{self, DependencyType, Project, Target, Version};
use selene_core::mods::{self, ContentKind, ModFile, RemoteFile};
use tauri::{AppHandle, State};

use super::{
    ContentTask, ContentUpdate, FABRIC_API, InstalledFile, Issue, ProjectRef, QUILTED_FABRIC_API,
    Source, Stage, content_dir, curseforge_matches, display_name, instance, loader_for,
};
use crate::error::CommandResult;
use crate::state::Launcher;
use crate::watch::FolderWatch;

/// Catalog entries already in the instance, as `source:id` mapped to the file's name. A
/// file uploaded to both sites counts for both.
#[tauri::command]
pub async fn installed_projects(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
    world: Option<String>,
) -> CommandResult<HashMap<String, String>> {
    let game_dir = content_dir(&launcher, &instance_id, kind, world.as_deref())?;
    let files = mods::list(&game_dir, kind).await?;
    let hashes: Vec<String> = files.iter().map(|file| file.sha1.clone()).collect();
    let file_name = |sha1: &str| {
        files
            .iter()
            .find(|file| file.sha1 == sha1)
            .map(|file| file.file_name.clone())
    };

    let from_modrinth = async {
        modrinth::versions_by_hash(&launcher.client, &hashes)
            .await
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(sha1, version)| {
                Some((
                    format!("modrinth:{}", version.project_id),
                    file_name(&sha1)?,
                ))
            })
            .collect::<HashMap<_, _>>()
    };
    let from_curseforge = async {
        let Some(curseforge) = &launcher.curseforge else {
            return HashMap::new();
        };
        curseforge_matches(curseforge, &game_dir, kind, &files)
            .await
            .into_iter()
            .filter_map(|(sha1, mod_id)| Some((format!("curseforge:{mod_id}"), file_name(&sha1)?)))
            .collect()
    };
    let (mut installed, from_curseforge) = tokio::join!(from_modrinth, from_curseforge);
    installed.extend(from_curseforge);
    Ok(installed)
}

/// Installed files with names, icons and problems for the ones either site recognises.
#[tauri::command]
pub async fn installed_content(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
    world: Option<String>,
) -> CommandResult<Vec<InstalledFile>> {
    list_installed(&launcher, &instance_id, kind, world.as_deref()).await
}

pub(super) async fn list_installed(
    launcher: &Launcher,
    instance_id: &str,
    kind: ContentKind,
    world: Option<&str>,
) -> CommandResult<Vec<InstalledFile>> {
    let instance = instance(launcher, instance_id).await?;
    let game_dir = content_dir(launcher, instance_id, kind, world)?;
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
    settle_duplicates(&game_dir, kind, &mut installed, &versions).await;
    installed.sort_by_cached_key(|installed| display_name(installed).to_lowercase());
    Ok(installed)
}

/// What makes two files the same mod: the catalog project, or a mod id the loader reads.
fn identity(file: &InstalledFile) -> impl Iterator<Item = String> + '_ {
    let project = file
        .project
        .iter()
        .map(|project| format!("{:?}:{}", project.source, project.id));
    let ids = file.file.mod_ids.iter().map(|id| format!("id:{id}"));
    project.chain(ids)
}

/// Enabled files grouped by what they share with each other.
fn enabled_twins(installed: &[InstalledFile]) -> Vec<Vec<usize>> {
    let mut owners: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, file) in installed.iter().enumerate() {
        if file.file.enabled {
            for key in identity(file) {
                owners.entry(key).or_default().push(index);
            }
        }
    }
    let mut groups: Vec<Vec<usize>> = owners
        .into_values()
        .filter(|indices| indices.len() > 1)
        .collect();
    for group in &mut groups {
        group.sort_unstable();
        group.dedup();
    }
    groups.sort();
    groups.dedup();
    groups
}

/// The game refuses to start with two copies of one mod, so only the newest copy stays on:
/// by release date when the catalog knows every copy, otherwise by when the file appeared.
async fn keep_newest_duplicates(
    game_dir: &Path,
    installed: &mut [InstalledFile],
    versions: &HashMap<String, Version>,
) {
    for group in enabled_twins(installed) {
        let released = |index: usize| {
            versions
                .get(&installed[index].file.sha1)
                .and_then(|version| version.date_published)
        };
        let by_release = group.iter().all(|&index| released(index).is_some());
        let Some(&newest) = group.iter().max_by_key(|&&index| {
            if by_release {
                (released(index), None)
            } else {
                (None, installed[index].file.modified)
            }
        }) else {
            continue;
        };
        for index in group.into_iter().filter(|&index| index != newest) {
            let file = &mut installed[index].file;
            if !file.enabled {
                continue;
            }
            match mods::set_enabled(game_dir, ContentKind::Mod, &file.file_name, false).await {
                Ok(()) => {
                    file.enabled = false;
                    file.file_name = format!("{}{}", file.file_name, mods::DISABLED_SUFFIX);
                    installed[index].issues.clear();
                }
                Err(err) => log::warn!("could not switch off {}: {err}", file.file_name),
            }
        }
    }
}

async fn settle_duplicates(
    game_dir: &Path,
    kind: ContentKind,
    installed: &mut [InstalledFile],
    versions: &HashMap<String, Version>,
) {
    if kind == ContentKind::Mod {
        keep_newest_duplicates(game_dir, installed, versions).await;
    }
    mark_duplicates(installed);
}

/// Flags enabled files that are the same project, or declare the same mod id, as another
/// enabled file.
fn mark_duplicates(installed: &mut [InstalledFile]) {
    let mut twins: Vec<(usize, String)> = Vec::new();
    for group in enabled_twins(installed) {
        for &index in &group {
            for &other in group.iter().filter(|&&other| other != index) {
                twins.push((index, installed[other].file.file_name.clone()));
            }
        }
    }
    twins.sort();
    twins.dedup();
    for (index, title) in twins {
        installed[index].issues.push(Issue::Duplicate { title });
    }
}

/// Enabled mods made for another game version or loader, which the player turned on anyway.
pub(crate) async fn unfit_mods(launcher: &Launcher, instance_id: &str) -> Vec<String> {
    list_installed(launcher, instance_id, ContentKind::Mod, None)
        .await
        .unwrap_or_default()
        .iter()
        .filter(|installed| installed.file.enabled && installed.fits_version == Some(false))
        .map(|installed| display_name(installed).to_owned())
        .collect()
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

/// Starts watching the folder of the tab that is open, replacing the previous one.
#[tauri::command]
pub async fn watch_content(
    app: AppHandle,
    launcher: State<'_, Launcher>,
    watch: State<'_, FolderWatch>,
    instance_id: String,
    kind: ContentKind,
    world: Option<String>,
) -> CommandResult<()> {
    let dir = kind.dir(&content_dir(
        &launcher,
        &instance_id,
        kind,
        world.as_deref(),
    )?);
    tokio::fs::create_dir_all(&dir).await?;
    if let Err(err) = watch.start(&app, &dir) {
        log::warn!("could not watch {}: {err}", dir.display());
    }
    Ok(())
}

#[tauri::command]
pub fn unwatch_content(watch: State<'_, FolderWatch>) {
    watch.stop();
}

#[tauri::command]
pub async fn set_content_enabled(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
    world: Option<String>,
    file_name: String,
    enabled: bool,
) -> CommandResult<()> {
    let game_dir = content_dir(&launcher, &instance_id, kind, world.as_deref())?;
    if enabled && kind == ContentKind::Mod {
        switch_off_twins(&game_dir, &file_name).await?;
    }
    Ok(mods::set_enabled(&game_dir, kind, &file_name, enabled).await?)
}

/// Turning one copy of a mod on turns the other copies off, so the player can go back to
/// an older version without the newest one taking over again.
async fn switch_off_twins(game_dir: &Path, file_name: &str) -> CommandResult<()> {
    let files = mods::list(game_dir, ContentKind::Mod).await?;
    let base = file_name.trim_end_matches(mods::DISABLED_SUFFIX);
    let Some(chosen) = files
        .iter()
        .find(|file| file.file_name.trim_end_matches(mods::DISABLED_SUFFIX) == base)
    else {
        return Ok(());
    };
    for file in files.iter().filter(|file| {
        file.enabled
            && file.file_name != chosen.file_name
            && file.mod_ids.iter().any(|id| chosen.mod_ids.contains(id))
    }) {
        mods::set_enabled(game_dir, ContentKind::Mod, &file.file_name, false).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn remove_content(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
    world: Option<String>,
    file_name: String,
) -> CommandResult<()> {
    let game_dir = content_dir(&launcher, &instance_id, kind, world.as_deref())?;
    Ok(mods::remove(&game_dir, kind, &file_name).await?)
}

/// Newer versions of installed files that run on this instance.
#[tauri::command]
pub async fn check_updates(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
    world: Option<String>,
) -> CommandResult<Vec<ContentUpdate>> {
    let installed = list_installed(&launcher, &instance_id, kind, world.as_deref()).await?;
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
    app: AppHandle,
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
    world: Option<String>,
) -> CommandResult<u32> {
    update_all(&app, &launcher, &instance_id, kind, world.as_deref()).await
}

pub(crate) async fn update_all(
    app: &AppHandle,
    launcher: &Launcher,
    instance_id: &str,
    kind: ContentKind,
    world: Option<&str>,
) -> CommandResult<u32> {
    let game_dir = content_dir(launcher, instance_id, kind, world)?;
    let files = mods::list(&game_dir, kind).await?;
    let swaps: Vec<(ModFile, RemoteFile)> = available_updates(launcher, instance_id, kind, &files)
        .await?
        .into_iter()
        .filter_map(|(file, update)| Some((file, update.remote_file()?)))
        .collect();
    if swaps.is_empty() {
        return Ok(0);
    }
    let task = ContentTask::start(app, instance_id, Stage::Updating);
    mods::replace_all(&launcher.client, &game_dir, kind, &swaps, |progress| {
        task.report(progress);
    })
    .await?;
    super::modpacks::follow_renames(launcher, instance_id, kind, &swaps).await?;
    Ok(u32::try_from(swaps.len()).unwrap_or(u32::MAX))
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
