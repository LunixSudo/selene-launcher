use selene_core::instance::Instance;
use selene_core::loader::LoaderVersion;
use selene_core::modrinth::{self, Target};
use selene_core::mods::{self, ContentKind, ModFile, RemoteFile};
use tauri::{AppHandle, State};

use super::installed::list_installed;
use super::{
    ContentTask, ContentUpdate, ProjectRef, Source, Stage, VersionChange, display_name, instance,
};
use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

struct ChangePlan {
    updates: Vec<(ModFile, RemoteFile, ContentUpdate)>,
    incompatible: Vec<(ModFile, String)>,
    unknown: Vec<String>,
}

/// What moving the instance to another game version would do to its mods.
#[tauri::command]
pub async fn preview_version_change(
    launcher: State<'_, Launcher>,
    instance_id: String,
    version: String,
    loader: Option<LoaderVersion>,
) -> CommandResult<VersionChange> {
    let plan = change_plan(&launcher, &instance_id, &version, loader.as_ref(), &[]).await?;
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
    app: AppHandle,
    launcher: State<'_, Launcher>,
    instance_id: String,
    version: String,
    loader: Option<LoaderVersion>,
) -> CommandResult<Instance> {
    // The download in progress is for the old version; the new one is fetched afterwards.
    launcher.cancel_preparing(&instance_id).await?;
    if launcher.manifest().await?.get(&version).is_none() {
        return Err(CommandError::UnknownVersion(version));
    }
    move_mods(
        &app,
        &launcher,
        &instance_id,
        &version,
        loader.as_ref(),
        &[],
    )
    .await?;
    let mut instance = instance(&launcher, &instance_id).await?;
    instance.version = version;
    instance.loader = loader;
    launcher.instances.save(&instance).await?;
    Ok(instance)
}

/// Swaps the instance's mods for builds that run on `version` and switches off the ones without
/// any. Mods at the `skip` paths are left alone: a modpack brings its own.
pub(crate) async fn move_mods(
    app: &AppHandle,
    launcher: &Launcher,
    instance_id: &str,
    version: &str,
    loader: Option<&LoaderVersion>,
    skip: &[String],
) -> CommandResult<()> {
    let plan = change_plan(launcher, instance_id, version, loader, skip).await?;
    let game_dir = launcher.game_dir(instance_id)?;
    let swaps: Vec<(ModFile, RemoteFile)> = plan
        .updates
        .iter()
        .map(|(file, remote, _)| (file.clone(), remote.clone()))
        .collect();
    let task = ContentTask::start(app, instance_id, Stage::Updating);
    mods::replace_all(
        &launcher.client,
        &game_dir,
        ContentKind::Mod,
        &swaps,
        |progress| {
            task.report(progress);
        },
    )
    .await?;
    drop(task);
    super::modpacks::follow_renames(launcher, instance_id, ContentKind::Mod, &swaps).await?;
    for (file, _) in &plan.incompatible {
        mods::set_enabled(&game_dir, ContentKind::Mod, &file.file_name, false).await?;
    }
    Ok(())
}

fn addon_path(file: &ModFile) -> String {
    format!(
        "{}/{}",
        ContentKind::Mod.folder(),
        file.file_name.trim_end_matches(mods::DISABLED_SUFFIX)
    )
}

/// Moves the instance to another build of the loader it already has. Mods stay as they are:
/// builds of one loader for one game version run the same mods.
#[tauri::command]
pub async fn change_loader_build(
    launcher: State<'_, Launcher>,
    instance_id: String,
    version: String,
) -> CommandResult<Instance> {
    launcher.cancel_preparing(&instance_id).await?;
    let mut instance = instance(&launcher, &instance_id).await?;
    let Some(loader) = instance.loader.as_mut() else {
        return Err(CommandError::NoCompatibleVersion);
    };
    loader.version = version;
    launcher.instances.save(&instance).await?;
    Ok(instance)
}

async fn change_plan(
    launcher: &Launcher,
    instance_id: &str,
    version: &str,
    loader: Option<&LoaderVersion>,
    skip: &[String],
) -> CommandResult<ChangePlan> {
    let mut plan = ChangePlan {
        updates: Vec::new(),
        incompatible: Vec::new(),
        unknown: Vec::new(),
    };
    let installed = list_installed(launcher, instance_id, ContentKind::Mod, None).await?;
    let Some(loader) = loader else {
        // Without a loader no mod runs at all.
        plan.incompatible = installed
            .into_iter()
            .filter(|installed| {
                installed.file.enabled && !skip.contains(&addon_path(&installed.file))
            })
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
    // Switched-off mods get the new build too, so turning one on later just works; only
    // switched-on ones are worth a warning when there is none.
    for installed in installed
        .into_iter()
        .filter(|installed| !skip.contains(&addon_path(&installed.file)))
    {
        let title = display_name(&installed).to_owned();
        let build = builds.remove(&installed.file.sha1);
        if build.is_none() && !installed.file.enabled {
            continue;
        }
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
