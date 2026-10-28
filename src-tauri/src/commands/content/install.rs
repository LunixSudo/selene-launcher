use std::collections::HashMap;

use selene_core::curseforge::CurseForge;
use selene_core::instance::Instance;
use selene_core::loader::Loader;
use selene_core::modrinth::{self, Target};
use selene_core::mods::{self, ContentKind, ModFile, RemoteFile};
use tauri::State;

use super::{
    CatalogItem, InstallPreview, InstallResult, ManualDownload, PlannedFile, Source, content_dir,
    curseforge_api, curseforge_loader, curseforge_matches, instance, loader_for,
};
use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

struct Plan {
    /// Each file with its kind, since a dependency can be of another kind than what needs it.
    files: Vec<(ContentKind, RemoteFile)>,
    preview: InstallPreview,
}

/// What installing the picked projects would download, dependencies included.
#[tauri::command]
pub async fn preview_install(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
    world: Option<String>,
    items: Vec<CatalogItem>,
) -> CommandResult<InstallPreview> {
    Ok(
        plan(&launcher, &instance_id, kind, world.as_deref(), &items)
            .await?
            .preview,
    )
}

/// Installs the picked projects and their required dependencies.
#[tauri::command]
pub async fn install_content(
    launcher: State<'_, Launcher>,
    instance_id: String,
    kind: ContentKind,
    world: Option<String>,
    items: Vec<CatalogItem>,
) -> CommandResult<InstallResult> {
    let plan = plan(&launcher, &instance_id, kind, world.as_deref(), &items).await?;
    if plan.files.is_empty() && plan.preview.manual.is_empty() {
        return Err(CommandError::NoCompatibleVersion);
    }
    for file_kind in ContentKind::ALL {
        let files: Vec<RemoteFile> = plan
            .files
            .iter()
            .filter(|(kind, _)| *kind == file_kind)
            .map(|(_, file)| file.clone())
            .collect();
        if files.is_empty() {
            continue;
        }
        let dir = content_dir(&launcher, &instance_id, file_kind, world.as_deref())?;
        mods::install(&launcher.client, &dir, file_kind, &files).await?;
    }
    Ok(InstallResult {
        installed: plan.files.len(),
        manual: plan.preview.manual,
    })
}

async fn plan(
    launcher: &Launcher,
    instance_id: &str,
    kind: ContentKind,
    world: Option<&str>,
    items: &[CatalogItem],
) -> CommandResult<Plan> {
    let instance = instance(launcher, instance_id).await?;
    let loader = loader_for(&instance, kind)?;
    let game_dir = content_dir(launcher, instance_id, kind, world)?;
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
        plan_modrinth(
            launcher,
            &instance,
            kind,
            loader,
            &present,
            &modrinth_ids,
            &mut plan,
        )
        .await?;
    }

    let curseforge_ids: Vec<u32> = ids_from(Source::Curseforge)
        .iter()
        .filter_map(|id| id.parse().ok())
        .collect();
    if !curseforge_ids.is_empty() {
        let curseforge = curseforge_api(launcher)?;
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
            plan.files.push((kind, file));
        }
        plan.preview.manual = manual_downloads(curseforge, &resolved.manual).await;
    }

    // Requested projects first, then the dependencies they pull in.
    plan.preview.mods.sort_by_key(|planned| !planned.requested);
    Ok(plan)
}

/// Adds the picked Modrinth projects and their dependencies to `plan`.
async fn plan_modrinth(
    launcher: &Launcher,
    instance: &Instance,
    kind: ContentKind,
    loader: Option<Loader>,
    present: &[ModFile],
    modrinth_ids: &[String],
    plan: &mut Plan,
) -> CommandResult<()> {
    let mut hashes: Vec<String> = present.iter().map(|file| file.sha1.clone()).collect();
    // Dependencies of another kind are nearly always mods, so installed mods count too.
    if kind != ContentKind::Mod && instance.loader.is_some() {
        let game_dir = launcher.instances.game_dir(&instance.id);
        hashes.extend(
            mods::list(&game_dir, ContentKind::Mod)
                .await?
                .into_iter()
                .map(|file| file.sha1),
        );
    }
    let installed = modrinth::versions_by_hash(&launcher.client, &hashes).await?;
    let instance_loader = instance.loader.as_ref().map(|loader| loader.loader);
    let target_for = |dependency_kind: ContentKind| match dependency_kind {
        _ if dependency_kind == kind => Some(Target::new(kind, &instance.version, loader)),
        ContentKind::Mod if instance_loader.is_none() => None,
        // Data packs need a world, which only the requested project names.
        ContentKind::DataPack => None,
        _ => Some(Target::new(
            dependency_kind,
            &instance.version,
            instance_loader,
        )),
    };
    let versions = modrinth::resolve_install(
        &launcher.client,
        modrinth_ids,
        kind,
        target_for,
        installed.into_values().map(|version| version.project_id),
    )
    .await?;
    let project_ids: Vec<String> = versions
        .iter()
        .map(|(_, version)| version.project_id.clone())
        .collect();
    let titles: HashMap<String, String> = modrinth::projects(&launcher.client, &project_ids)
        .await?
        .into_iter()
        .map(|project| (project.id, project.title))
        .collect();
    for (file_kind, version) in &versions {
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
        plan.files.push((*file_kind, file));
    }
    Ok(())
}

pub(crate) async fn manual_downloads(
    curseforge: &CurseForge,
    mod_ids: &[u32],
) -> Vec<ManualDownload> {
    let mut mods = curseforge.mods(mod_ids).await.unwrap_or_default();
    mods.sort_by_key(|info| mod_ids.iter().position(|&id| id == info.id));
    mods.into_iter()
        .map(|info| ManualDownload {
            title: info.name,
            url: format!("{}/files", info.links.website_url),
        })
        .collect()
}
