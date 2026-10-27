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

/// Results per source, so a merged page holds up to twice as many.
const PAGE_SIZE: u32 = 10;

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

/// One search over Modrinth and CurseForge, already filtered to the instance's version and
/// loader. Results alternate between the two; a mod listed on both shows up once.
#[tauri::command]
pub async fn search_mods(
    launcher: State<'_, Launcher>,
    instance_id: String,
    query: String,
    page: u32,
) -> CommandResult<CatalogPage> {
    let instance = instance(&launcher, &instance_id).await?;
    let loader = loader_of(&instance)?;
    let offset = page * PAGE_SIZE;

    let modrinth_query = SearchQuery {
        text: &query,
        project_type: ProjectType::Mod,
        game_version: &instance.version,
        loader: Some(loader),
        offset,
        limit: PAGE_SIZE,
    };
    let modrinth_search = modrinth::search(&launcher.client, &modrinth_query);
    let curseforge_search = async {
        match &launcher.curseforge {
            Some(curseforge) => curseforge
                .search(&query, &instance.version, loader, offset, PAGE_SIZE)
                .await
                .ok(),
            None => None,
        }
    };
    let (modrinth_results, curseforge_results) = tokio::join!(modrinth_search, curseforge_search);
    let modrinth_results = modrinth_results?;

    let has_more = offset + PAGE_SIZE < modrinth_results.total_hits
        || curseforge_results
            .as_ref()
            .is_some_and(|results| offset + PAGE_SIZE < results.total);
    let from_modrinth = modrinth_results.hits.into_iter().map(|hit| CatalogHit {
        source: Source::Modrinth,
        page_url: format!("https://modrinth.com/mod/{}", hit.slug),
        id: hit.project_id,
        title: hit.title,
        description: hit.description,
        author: hit.author,
        icon_url: hit.icon_url,
        downloads: hit.downloads,
    });
    let from_curseforge = curseforge_results
        .map(|results| results.mods)
        .unwrap_or_default()
        .into_iter()
        .map(|found| CatalogHit {
            source: Source::Curseforge,
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
        });

    Ok(CatalogPage {
        hits: merge(from_modrinth, from_curseforge),
        has_more,
    })
}

fn merge(
    first: impl Iterator<Item = CatalogHit>,
    second: impl Iterator<Item = CatalogHit>,
) -> Vec<CatalogHit> {
    let mut seen = std::collections::HashSet::new();
    let mut first = first.peekable();
    let mut second = second.peekable();
    let mut merged = Vec::new();
    while first.peek().is_some() || second.peek().is_some() {
        for hit in [first.next(), second.next()].into_iter().flatten() {
            if seen.insert(title_key(&hit.title)) {
                merged.push(hit);
            }
        }
    }
    merged
}

/// "Just Enough Items (JEI)" and "just-enough-items" compare equal.
fn title_key(title: &str) -> String {
    title
        .split('(')
        .next()
        .unwrap_or(title)
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
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

/// Installs a mod and its required dependencies.
#[tauri::command]
pub async fn install_mod(
    launcher: State<'_, Launcher>,
    instance_id: String,
    source: Source,
    id: String,
) -> CommandResult<InstallResult> {
    let instance = instance(&launcher, &instance_id).await?;
    let loader = loader_of(&instance)?;
    let game_dir = launcher.instances.game_dir(&instance_id);

    let (files, manual) = match source {
        Source::Modrinth => (
            modrinth_plan(&launcher, &instance, loader, &id).await?,
            Vec::new(),
        ),
        Source::Curseforge => {
            let curseforge = launcher
                .curseforge
                .as_ref()
                .ok_or(CommandError::NoCompatibleVersion)?;
            let mod_id = id.parse().map_err(|_| CommandError::NoCompatibleVersion)?;
            let plan = curseforge
                .resolve_install(mod_id, &instance.version, loader)
                .await?;
            (plan.files, manual_downloads(curseforge, &plan.manual).await)
        }
    };
    if files.is_empty() && manual.is_empty() {
        return Err(CommandError::NoCompatibleVersion);
    }

    mods::install(&launcher.client, &game_dir, &files).await?;
    Ok(InstallResult {
        installed: files.len(),
        manual,
    })
}

async fn modrinth_plan(
    launcher: &Launcher,
    instance: &Instance,
    loader: Loader,
    project_id: &str,
) -> CommandResult<Vec<RemoteFile>> {
    let game_dir = launcher.instances.game_dir(&instance.id);
    let hashes: Vec<String> = mods::list(&game_dir)
        .await?
        .into_iter()
        .map(|file| file.sha1)
        .collect();
    let installed = modrinth::versions_by_hash(&launcher.client, &hashes).await?;
    let plan = modrinth::resolve_install(
        &launcher.client,
        project_id,
        &instance.version,
        Some(loader),
        installed.into_values().map(|version| version.project_id),
    )
    .await?;
    Ok(plan
        .iter()
        .filter_map(modrinth::Version::remote_file)
        .collect())
}

async fn manual_downloads(curseforge: &CurseForge, mod_ids: &[u32]) -> Vec<ManualDownload> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(source: Source, title: &str) -> CatalogHit {
        CatalogHit {
            source,
            id: title.into(),
            title: title.into(),
            description: String::new(),
            author: String::new(),
            icon_url: None,
            downloads: 0,
            page_url: String::new(),
        }
    }

    #[test]
    fn merges_sources_without_duplicates() {
        let modrinth = vec![
            hit(Source::Modrinth, "Sodium"),
            hit(Source::Modrinth, "Lithium"),
        ];
        let curseforge = vec![
            hit(Source::Curseforge, "Just Enough Items (JEI)"),
            hit(Source::Curseforge, "sodium"),
            hit(Source::Curseforge, "Jade"),
        ];

        let merged = merge(modrinth.into_iter(), curseforge.into_iter());
        let titles: Vec<_> = merged.iter().map(|hit| hit.title.as_str()).collect();

        assert_eq!(
            titles,
            ["Sodium", "Just Enough Items (JEI)", "Lithium", "Jade"]
        );
    }
}
