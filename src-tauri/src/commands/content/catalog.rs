use selene_core::curseforge;
use selene_core::modrinth::{self, SearchQuery, Target};
use selene_core::mods::{self, CatalogSort, ContentKind};
use tauri::State;

use super::{
    CatalogHit, CatalogPage, CategoryTag, PAGE_SIZE, ProjectPage, ProjectRef, ProjectVersion,
    Source, channel, content_dir, curseforge_api, curseforge_id, curseforge_loader, instance,
    loader_for, modrinth_section,
};
use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

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
            let results = curseforge_api(&launcher)?
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
        Source::Curseforge => curseforge_api(&launcher)?
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
        Source::Curseforge => Ok(curseforge_api(&launcher)?
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
    Ok(curseforge_api(&launcher)?
        .changelog(curseforge_id(&project_id)?, curseforge_id(&file_id)?)
        .await?)
}

/// Swaps an installed file for a chosen version of the same project, older or newer.
#[tauri::command]
pub async fn install_version(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
    world: Option<String>,
    project: ProjectRef,
    version_id: String,
    file_name: String,
) -> CommandResult<()> {
    let game_dir = content_dir(&launcher, &instance_id, kind, world.as_deref())?;
    let old = mods::list(&game_dir, kind)
        .await?
        .into_iter()
        .find(|file| file.file_name == file_name)
        .ok_or(CommandError::NoCompatibleVersion)?;
    let remote = match project.source {
        Source::Modrinth => modrinth::version(&launcher.client, &version_id)
            .await?
            .remote_file(),
        Source::Curseforge => curseforge_api(&launcher)?
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
            let curseforge = curseforge_api(&launcher)?;
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
