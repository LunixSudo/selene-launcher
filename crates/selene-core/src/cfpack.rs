//! CurseForge modpack zips: `manifest.json` lists mods by project and file id, everything else
//! sits in `overrides/`.

use std::io::Read;
use std::path::{Path, PathBuf};

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::curseforge::{self, CurseForge};
use crate::loader::{Loader, LoaderVersion};
use crate::mods::ContentKind;
use crate::mrpack::{self, ExportOptions};
use crate::{Error, Result, mods};

const MANIFEST_FILE: &str = "manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackManifest {
    minecraft: Minecraft,
    manifest_type: String,
    manifest_version: u32,
    pub name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    files: Vec<ManifestFile>,
    #[serde(default)]
    overrides: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Minecraft {
    version: String,
    #[serde(default)]
    mod_loaders: Vec<ModLoader>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModLoader {
    id: String,
    #[serde(default)]
    primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestFile {
    #[serde(rename = "projectID")]
    project_id: u32,
    #[serde(rename = "fileID")]
    file_id: u32,
    #[serde(default = "required_by_default")]
    required: bool,
}

fn required_by_default() -> bool {
    true
}

impl PackManifest {
    pub fn game_version(&self) -> &str {
        &self.minecraft.version
    }

    /// The primary loader, e.g. `fabric-0.16.5`. Forge and NeoForge packs are not supported yet.
    pub fn loader(&self) -> Result<Option<LoaderVersion>> {
        let loaders = &self.minecraft.mod_loaders;
        let Some(primary) = loaders
            .iter()
            .find(|loader| loader.primary)
            .or_else(|| loaders.first())
        else {
            return Ok(None);
        };
        let (name, version) = primary
            .id
            .split_once('-')
            .ok_or_else(|| Error::InvalidPack(format!("unknown loader {}", primary.id)))?;
        let loader = match name {
            "fabric" => Loader::Fabric,
            "quilt" => Loader::Quilt,
            "forge" => Loader::Forge,
            "neoforge" => Loader::NeoForge,
            other => return Err(Error::UnsupportedLoader(other.to_owned())),
        };
        Ok(Some(LoaderVersion {
            loader,
            version: version.to_owned(),
        }))
    }
}

pub fn read_manifest(pack: &Path) -> Result<PackManifest> {
    let mut archive = mrpack::open(pack)?;
    let mut entry = archive
        .by_name(MANIFEST_FILE)
        .map_err(|_| Error::InvalidPack(format!("{MANIFEST_FILE} is missing")))?;
    let mut json = String::new();
    entry
        .read_to_string(&mut json)
        .map_err(|err| Error::io(pack, err))?;
    let manifest: PackManifest = serde_json::from_str(&json)?;
    if manifest.manifest_type != "minecraftModpack" {
        return Err(Error::InvalidPack(format!(
            "unexpected manifest type {}",
            manifest.manifest_type
        )));
    }
    Ok(manifest)
}

/// Downloads the pack's mods and unpacks its overrides. Returns the ids of mods whose authors
/// only allow downloads from the CurseForge website.
pub async fn install(
    client: &Client,
    curseforge: &CurseForge,
    pack: &Path,
    manifest: &PackManifest,
    game_dir: &Path,
) -> Result<Vec<u32>> {
    let ids: Vec<u32> = manifest
        .files
        .iter()
        .filter(|file| file.required)
        .map(|file| file.file_id)
        .collect();
    let mut remote = Vec::new();
    let mut manual = Vec::new();
    for file in curseforge.files(&ids).await? {
        match file.remote_file() {
            Some(found) => remote.push(found),
            None => manual.push(file.mod_id),
        }
    }
    mods::install(client, game_dir, ContentKind::Mod, &remote).await?;

    let pack = pack.to_owned();
    let target = game_dir.to_owned();
    tokio::task::spawn_blocking(move || mrpack::extract_overrides(&pack, &target))
        .await
        .map_err(|err| Error::io(game_dir, std::io::Error::other(err)))??;
    Ok(manual)
}

/// Writes the instance as a CurseForge modpack. Mods CurseForge does not know are copied in.
pub async fn export(
    curseforge: &CurseForge,
    game_dir: &Path,
    options: &ExportOptions,
    out: &Path,
) -> Result<()> {
    let mut fingerprinted = Vec::new();
    for file in mods::list(game_dir, ContentKind::Mod)
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
                Loader::Forge => "forge",
                Loader::NeoForge => "neoforge",
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
        manifest_type: "minecraftModpack".to_owned(),
        manifest_version: 1,
        name: options.name.clone(),
        version: "1.0.0".to_owned(),
        author: String::new(),
        files,
        overrides: "overrides".to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(loader: &str) -> PackManifest {
        serde_json::from_value(serde_json::json!({
            "minecraft": { "version": "1.21.1", "modLoaders": [{ "id": loader, "primary": true }] },
            "manifestType": "minecraftModpack",
            "manifestVersion": 1,
            "name": "Pack",
            "files": [{ "projectID": 1, "fileID": 2 }],
        }))
        .unwrap()
    }

    #[test]
    fn reads_the_primary_loader() {
        let loader = manifest("fabric-0.16.5").loader().unwrap().unwrap();
        assert_eq!(loader.loader, Loader::Fabric);
        assert_eq!(loader.version, "0.16.5");
        assert!(manifest("quilt-0.27.1").loader().unwrap().is_some());
    }

    #[test]
    fn reads_forge_and_rejects_unknown_loaders() {
        let loader = manifest("forge-47.3.0").loader().unwrap().unwrap();
        assert_eq!(loader.loader, Loader::Forge);
        assert_eq!(loader.version, "47.3.0");
        assert!(matches!(
            manifest("liteloader-1.12.2").loader(),
            Err(Error::UnsupportedLoader(_))
        ));
    }
}
