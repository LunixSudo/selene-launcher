use std::path::{Path, PathBuf};

use selene_core::cfpack::{self, PackManifest};
use selene_core::download::Progress;
use selene_core::instance::Instance;
use selene_core::loader::LoaderVersion;
use selene_core::mrpack::{self, ExportOptions, PackIndex};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use super::content::{ContentTask, ManualDownload, Stage, manual_downloads};
use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackFormat {
    Mrpack,
    Curseforge,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub(crate) instance: Instance,
    /// Mods from a CurseForge pack that have to be downloaded from the website.
    pub(crate) manual: Vec<ManualDownload>,
    /// The player's own mods could not be moved to the pack's new game version or loader.
    pub(crate) mods_left_behind: bool,
}

/// A pack file read from disk, in either format.
pub(crate) enum Pack {
    Modrinth(PackIndex),
    Curseforge(PackManifest),
}

impl Pack {
    pub(crate) async fn read(path: &Path) -> CommandResult<Self> {
        let is_zip = path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"));
        let path = path.to_owned();
        tokio::task::spawn_blocking(move || {
            if is_zip {
                cfpack::read_manifest(&path).map(Self::Curseforge)
            } else {
                mrpack::read_index(&path).map(Self::Modrinth)
            }
        })
        .await
        .map_err(|err| CommandError::Io(std::io::Error::other(err)))?
        .map_err(Into::into)
    }

    fn name(&self) -> &str {
        match self {
            Self::Modrinth(index) => &index.name,
            Self::Curseforge(manifest) => &manifest.name,
        }
    }

    pub(crate) fn game_version(&self) -> CommandResult<&str> {
        match self {
            Self::Modrinth(index) => Ok(index.game_version()?),
            Self::Curseforge(manifest) => Ok(manifest.game_version()),
        }
    }

    pub(crate) fn loader(&self) -> CommandResult<Option<LoaderVersion>> {
        match self {
            Self::Modrinth(index) => Ok(index.loader()?),
            Self::Curseforge(manifest) => Ok(manifest.loader()?),
        }
    }

    /// Puts the pack's files in the game folder. Returns the add-ons it placed and the mods
    /// that have to be downloaded by hand.
    pub(crate) async fn install(
        &self,
        launcher: &Launcher,
        path: &Path,
        game_dir: &Path,
        on_progress: impl Fn(Progress) + Sync,
    ) -> CommandResult<(Vec<String>, Vec<ManualDownload>)> {
        match self {
            Self::Modrinth(index) => {
                let addons =
                    mrpack::install(&launcher.client, path, index, game_dir, on_progress).await?;
                Ok((addons, Vec::new()))
            }
            Self::Curseforge(manifest) => {
                let curseforge = launcher
                    .curseforge
                    .as_ref()
                    .ok_or(CommandError::NoCompatibleVersion)?;
                let installed = cfpack::install(
                    &launcher.client,
                    curseforge,
                    path,
                    manifest,
                    game_dir,
                    on_progress,
                )
                .await?;
                let manual = manual_downloads(curseforge, &installed.manual).await;
                Ok((installed.addons, manual))
            }
        }
    }
}

/// Creates an instance from a `.mrpack` or a CurseForge `.zip`. A failed import leaves nothing
/// behind.
#[tauri::command]
pub async fn import_pack(
    app: AppHandle,
    launcher: State<'_, Launcher>,
    path: PathBuf,
) -> CommandResult<ImportResult> {
    let pack = Pack::read(&path).await?;
    let instance = create(
        &app,
        &launcher,
        pack.name(),
        pack.game_version()?,
        pack.loader()?,
    )
    .await?;
    let game_dir = launcher.instances.game_dir(&instance.id);
    let task = ContentTask::start(&app, &instance.id, Stage::Importing);
    let installed = while_importing(
        &launcher,
        &instance.id,
        pack.install(&launcher, &path, &game_dir, |progress| {
            task.report(progress);
        }),
    )
    .await;
    drop(task);
    let (_, manual) = discard_on_error(&launcher, &instance.id, installed).await?;
    Ok(ImportResult {
        instance,
        manual,
        mods_left_behind: false,
    })
}

/// A failed import removes the instance it made. A cancelled one was cancelled by deleting
/// the instance, which removes it already.
pub(crate) async fn discard_on_error<T>(
    launcher: &Launcher,
    id: &str,
    result: CommandResult<T>,
) -> CommandResult<T> {
    if let Err(err) = &result
        && !matches!(err, CommandError::Cancelled)
    {
        let _ = launcher.instances.delete(id).await;
    }
    result
}

/// Runs a pack's download as the instance's download in progress, so deleting the instance
/// meanwhile cancels it instead of racing it.
pub(crate) async fn while_importing<T>(
    launcher: &Launcher,
    id: &str,
    install: impl Future<Output = CommandResult<T>>,
) -> CommandResult<T> {
    let Some(cancelled) = launcher.start_running(id, false) else {
        return Err(CommandError::AlreadyRunning(id.to_owned()));
    };
    let result = tokio::select! {
        result = install => result,
        _ = cancelled => Err(CommandError::Cancelled),
    };
    launcher.stop_running(id);
    result
}

/// The instance shows up in the list right away, while its files are still downloading.
pub(crate) async fn create(
    app: &AppHandle,
    launcher: &Launcher,
    name: &str,
    game_version: &str,
    loader: Option<LoaderVersion>,
) -> CommandResult<Instance> {
    if launcher.manifest().await?.get(game_version).is_none() {
        return Err(CommandError::UnknownVersion(game_version.to_owned()));
    }
    let instance = launcher
        .instances
        .create(name, game_version, loader)
        .await?;
    let _ = app.emit("instance-importing", &instance);
    Ok(instance)
}

/// Writes the instance as a `.mrpack` or a CurseForge `.zip`.
#[tauri::command]
pub async fn export_pack(
    launcher: State<'_, Launcher>,
    instance_id: String,
    path: PathBuf,
    folders: Vec<String>,
    format: PackFormat,
) -> CommandResult<()> {
    let instance = launcher
        .instances
        .get(&instance_id)
        .await
        .ok_or_else(|| CommandError::UnknownInstance(instance_id.clone()))?;
    let options = ExportOptions {
        name: instance.name,
        game_version: instance.version,
        loader: instance.loader,
        folders,
    };
    let game_dir = launcher.game_dir(&instance_id)?;
    match format {
        PackFormat::Mrpack => mrpack::export(&launcher.client, &game_dir, &options, &path).await?,
        PackFormat::Curseforge => {
            let curseforge = launcher
                .curseforge
                .as_ref()
                .ok_or(CommandError::NoCompatibleVersion)?;
            cfpack::export(curseforge, &game_dir, &options, &path).await?;
        }
    }
    Ok(())
}
