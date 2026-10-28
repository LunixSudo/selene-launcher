use std::path::PathBuf;

use selene_core::cfpack;
use selene_core::instance::Instance;
use selene_core::loader::LoaderVersion;
use selene_core::mrpack::{self, ExportOptions};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use super::content::{ManualDownload, manual_downloads};
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
    instance: Instance,
    /// Mods from a CurseForge pack that have to be downloaded from the website.
    manual: Vec<ManualDownload>,
}

/// Creates an instance from a `.mrpack` or a CurseForge `.zip`. A failed import leaves nothing
/// behind.
#[tauri::command]
pub async fn import_pack(
    app: AppHandle,
    launcher: State<'_, Launcher>,
    path: PathBuf,
) -> CommandResult<ImportResult> {
    let is_zip = path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"));
    if is_zip {
        import_curseforge(&app, &launcher, path).await
    } else {
        import_mrpack(&app, &launcher, path).await
    }
}

async fn import_mrpack(
    app: &AppHandle,
    launcher: &Launcher,
    path: PathBuf,
) -> CommandResult<ImportResult> {
    let pack = path.clone();
    let index = tokio::task::spawn_blocking(move || mrpack::read_index(&pack))
        .await
        .map_err(|err| CommandError::Io(std::io::Error::other(err)))??;
    let instance = create(
        app,
        launcher,
        &index.name,
        index.game_version()?,
        index.loader()?,
    )
    .await?;
    let game_dir = launcher.instances.game_dir(&instance.id);
    if let Err(err) = mrpack::install(&launcher.client, &path, &index, &game_dir).await {
        let _ = launcher.instances.delete(&instance.id).await;
        return Err(err.into());
    }
    Ok(ImportResult {
        instance,
        manual: Vec::new(),
    })
}

async fn import_curseforge(
    app: &AppHandle,
    launcher: &Launcher,
    path: PathBuf,
) -> CommandResult<ImportResult> {
    let curseforge = launcher
        .curseforge
        .as_ref()
        .ok_or(CommandError::NoCompatibleVersion)?;
    let pack = path.clone();
    let manifest = tokio::task::spawn_blocking(move || cfpack::read_manifest(&pack))
        .await
        .map_err(|err| CommandError::Io(std::io::Error::other(err)))??;
    let instance = create(
        app,
        launcher,
        &manifest.name,
        manifest.game_version(),
        manifest.loader()?,
    )
    .await?;
    let game_dir = launcher.instances.game_dir(&instance.id);
    let installed =
        cfpack::install(&launcher.client, curseforge, &path, &manifest, &game_dir).await;
    match installed {
        Ok(manual) => Ok(ImportResult {
            instance,
            manual: manual_downloads(curseforge, &manual).await,
        }),
        Err(err) => {
            let _ = launcher.instances.delete(&instance.id).await;
            Err(err.into())
        }
    }
}

/// The instance shows up in the list right away, while its files are still downloading.
async fn create(
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

/// Writes the instance as a `.mrpack` (Selene, Prism, Modrinth App) or a CurseForge `.zip`
/// (CurseForge App, Prism, `ATLauncher`).
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
