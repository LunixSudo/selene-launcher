use std::collections::HashMap;

use selene_core::instance::Instance;
use selene_core::loader::Loader;
use selene_core::modrinth::{self, Project, ProjectType, SearchQuery, SearchResults};
use selene_core::mods::{self, ModFile};
use serde::Serialize;
use tauri::State;

use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

const PAGE_SIZE: u32 = 20;

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

#[tauri::command]
pub async fn search_mods(
    launcher: State<'_, Launcher>,
    instance_id: String,
    query: String,
    offset: u32,
) -> CommandResult<SearchResults> {
    let instance = instance(&launcher, &instance_id).await?;
    let loader = loader_of(&instance)?;
    Ok(modrinth::search(
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
    .await?)
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

/// Installs a mod and its required dependencies. Returns the project IDs that were added.
#[tauri::command]
pub async fn install_mod(
    launcher: State<'_, Launcher>,
    instance_id: String,
    project_id: String,
) -> CommandResult<Vec<String>> {
    let instance = instance(&launcher, &instance_id).await?;
    let loader = loader_of(&instance)?;
    let game_dir = launcher.instances.game_dir(&instance_id);

    let hashes: Vec<String> = mods::list(&game_dir)
        .await?
        .into_iter()
        .map(|file| file.sha1)
        .collect();
    let installed = modrinth::versions_by_hash(&launcher.client, &hashes).await?;
    let plan = modrinth::resolve_install(
        &launcher.client,
        &project_id,
        &instance.version,
        Some(loader),
        installed.into_values().map(|version| version.project_id),
    )
    .await?;
    if plan.is_empty() {
        return Err(CommandError::NoCompatibleVersion);
    }

    mods::install(&launcher.client, &game_dir, &plan).await?;
    Ok(plan.into_iter().map(|version| version.project_id).collect())
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
        if update
            .primary_file()
            .is_some_and(|new| new.hashes.sha1 != file.sha1)
        {
            mods::replace(&launcher.client, &game_dir, file, update).await?;
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
