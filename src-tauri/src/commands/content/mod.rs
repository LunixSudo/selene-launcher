//! Mods, resource packs and shader packs: the catalog and what is installed.

pub mod catalog;
pub mod install;
pub mod installed;
pub mod version_change;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use selene_core::curseforge::{self, CurseForge};
use selene_core::instance::Instance;
use selene_core::loader::Loader;
use selene_core::mods::{ContentKind, ModFile};
use selene_core::saves;
use serde::{Deserialize, Serialize};

use crate::error::{CommandError, CommandResult};
use crate::state::Launcher;

pub(crate) use install::manual_downloads;
pub(crate) use installed::update_all;

const PAGE_SIZE: u32 = 20;
const FABRIC_API: &str = "P7dR8mSH";
const QUILTED_FABRIC_API: &str = "qvIfYCYJ";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
pub struct CategoryTag {
    id: String,
    name: String,
}

/// Where an installed file came from, when either site recognises it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectRef {
    source: Source,
    id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Issue {
    /// A required mod is not installed.
    Requires { title: String },
    /// An installed mod is marked as incompatible with this one.
    ConflictsWith { title: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledFile {
    #[serde(flatten)]
    file: ModFile,
    title: Option<String>,
    icon_url: Option<String>,
    version_number: Option<String>,
    project: Option<ProjectRef>,
    issues: Vec<Issue>,
    /// `None` when neither site knows the file.
    fits_version: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    installed: usize,
    /// Files whose authors only allow downloads from the CurseForge website.
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
pub struct ContentUpdate {
    file_name: String,
    title: String,
    from: Option<String>,
    to: String,
}

/// A catalog entry the user picked.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CatalogItem {
    source: Source,
    id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedFile {
    title: String,
    /// `false` for dependencies that come along with the picked projects.
    requested: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPreview {
    mods: Vec<PlannedFile>,
    manual: Vec<ManualDownload>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectVersion {
    id: String,
    name: String,
    version_number: String,
    published: Option<String>,
    /// `release`, `beta` or `alpha`.
    channel: &'static str,
    /// Markdown for Modrinth, HTML for CurseForge; CurseForge sends it separately.
    changelog: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPage {
    title: String,
    description: String,
    /// `markdown` or `html`.
    format: &'static str,
    body: String,
    icon_url: Option<String>,
    gallery: Vec<String>,
    page_url: String,
    downloads: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionChange {
    /// Mods that have a build for the new version and will be swapped for it.
    updated: Vec<ContentUpdate>,
    /// Mods with no build for the new version; they get disabled, not deleted.
    incompatible: Vec<String>,
    /// Mods neither site could check; they stay as they are.
    unknown: Vec<String>,
}

/// CurseForge mod ids of the files it recognises, keyed by the file's SHA-1.
async fn curseforge_matches(
    curseforge: &CurseForge,
    game_dir: &Path,
    kind: ContentKind,
    files: &[ModFile],
) -> HashMap<String, u32> {
    // A fingerprint depends only on the file's contents, so it is kept by SHA-1 and a file
    // is read again only when it changes.
    static FINGERPRINTS: Mutex<Option<HashMap<String, u32>>> = Mutex::new(None);
    let mut by_fingerprint = HashMap::with_capacity(files.len());
    for file in files {
        let known = FINGERPRINTS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .and_then(|cache| cache.get(&file.sha1).copied());
        let fingerprint = if let Some(fingerprint) = known {
            fingerprint
        } else {
            let path = kind.dir(game_dir).join(&file.file_name);
            let Ok(bytes) = tokio::fs::read(&path).await else {
                continue;
            };
            let fingerprint = curseforge::fingerprint(&bytes);
            FINGERPRINTS
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get_or_insert_default()
                .insert(file.sha1.clone(), fingerprint);
            fingerprint
        };
        by_fingerprint.insert(fingerprint, file.sha1.clone());
    }
    let fingerprints: Vec<u32> = by_fingerprint.keys().copied().collect();
    curseforge
        .files_by_fingerprint(&fingerprints)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(fingerprint, known)| {
            Some((by_fingerprint.remove(&fingerprint)?, known.mod_id))
        })
        .collect()
}

async fn instance(launcher: &Launcher, id: &str) -> CommandResult<Instance> {
    launcher
        .instances
        .get(id)
        .await
        .ok_or_else(|| CommandError::UnknownInstance(id.to_owned()))
}

/// The folder the content kind lives under: the instance, or one of its worlds.
fn content_dir(
    launcher: &Launcher,
    instance_id: &str,
    kind: ContentKind,
    world: Option<&str>,
) -> CommandResult<PathBuf> {
    let game_dir = launcher.game_dir(instance_id)?;
    match (kind, world) {
        (ContentKind::DataPack, Some(world)) => Ok(saves::world_dir(&game_dir, world)?),
        (ContentKind::DataPack, None) => Err(CommandError::NoWorld),
        _ => Ok(game_dir),
    }
}

fn curseforge_api(launcher: &Launcher) -> CommandResult<&CurseForge> {
    launcher
        .curseforge
        .as_ref()
        .ok_or(CommandError::NoCompatibleVersion)
}

fn curseforge_id(id: &str) -> CommandResult<u32> {
    id.parse().map_err(|_| CommandError::NoCompatibleVersion)
}

/// The instance's loader, which mods and shaders need. Resource packs run anywhere.
fn loader_for(instance: &Instance, kind: ContentKind) -> CommandResult<Option<Loader>> {
    let loader = instance.loader.as_ref().map(|loader| loader.loader);
    match (kind, loader) {
        (ContentKind::Mod | ContentKind::ShaderPack, None) => Err(CommandError::NoLoader),
        (_, loader) => Ok(loader),
    }
}

/// CurseForge only tags mod files with a loader.
fn curseforge_loader(kind: ContentKind, loader: Option<Loader>) -> Option<Loader> {
    loader.filter(|_| kind == ContentKind::Mod)
}

fn modrinth_section(kind: ContentKind) -> &'static str {
    match kind {
        ContentKind::Mod => "mod",
        ContentKind::ResourcePack => "resourcepack",
        ContentKind::ShaderPack => "shader",
        ContentKind::DataPack => "datapack",
    }
}

fn channel(version_type: &str) -> &'static str {
    match version_type {
        "beta" => "beta",
        "alpha" => "alpha",
        _ => "release",
    }
}

fn display_name(installed: &InstalledFile) -> &str {
    installed
        .title
        .as_deref()
        .unwrap_or(installed.file.file_name.as_str())
}
