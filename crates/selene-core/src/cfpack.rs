//! CurseForge modpack zips: `manifest.json` lists mods by project and file id, everything else
//! sits in `overrides/`.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::curseforge::{self, CurseForge};
use crate::loader::Loader;
use crate::mrpack::{self, ExportOptions};
use crate::{Error, Result, mods};

const MANIFEST_FILE: &str = "manifest.json";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackManifest {
    minecraft: Minecraft,
    manifest_type: &'static str,
    manifest_version: u32,
    name: String,
    version: &'static str,
    author: &'static str,
    files: Vec<ManifestFile>,
    overrides: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Minecraft {
    version: String,
    mod_loaders: Vec<ModLoader>,
}

#[derive(Debug, Clone, Serialize)]
struct ModLoader {
    id: String,
    primary: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ManifestFile {
    #[serde(rename = "projectID")]
    project_id: u32,
    #[serde(rename = "fileID")]
    file_id: u32,
    required: bool,
}

/// Writes the instance as a CurseForge modpack. Mods CurseForge does not know are copied in.
pub async fn export(
    curseforge: &CurseForge,
    game_dir: &Path,
    options: &ExportOptions,
    out: &Path,
) -> Result<()> {
    let mut fingerprinted = Vec::new();
    for file in mods::list(game_dir)
        .await?
        .into_iter()
        .filter(|file| file.enabled)
    {
        let path = mods::mods_dir(game_dir).join(&file.file_name);
        let bytes = tokio::fs::read(&path)
            .await
            .map_err(|err| Error::io(&path, err))?;
        fingerprinted.push((file.file_name, curseforge::fingerprint(&bytes)));
    }
    let fingerprints: Vec<u32> = fingerprinted.iter().map(|(_, print)| *print).collect();
    let known = curseforge.files_by_fingerprint(&fingerprints).await?;

    let mut files = Vec::new();
    let mut copied = Vec::new();
    for (file_name, print) in fingerprinted {
        match known.get(&print) {
            Some(file) => files.push(ManifestFile {
                project_id: file.mod_id,
                file_id: file.file_id,
                required: true,
            }),
            None => copied.push(PathBuf::from("mods").join(file_name)),
        }
    }

    let mod_loaders = options
        .loader
        .iter()
        .map(|loader| {
            let name = match loader.loader {
                Loader::Fabric => "fabric",
                Loader::Quilt => "quilt",
            };
            ModLoader {
                id: format!("{name}-{}", loader.version),
                primary: true,
            }
        })
        .collect();
    let manifest = PackManifest {
        minecraft: Minecraft {
            version: options.game_version.clone(),
            mod_loaders,
        },
        manifest_type: "minecraftModpack",
        manifest_version: 1,
        name: options.name.clone(),
        version: "1.0.0",
        author: "",
        files,
        overrides: "overrides",
    };

    let manifest = serde_json::to_vec_pretty(&manifest)?;
    mrpack::write_pack(
        out,
        MANIFEST_FILE,
        manifest,
        game_dir,
        copied,
        &options.folders,
    )
    .await
}
