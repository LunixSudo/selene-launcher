use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;
use std::sync::{Mutex, PoisonError};

use chrono::{DateTime, Utc};
use futures_util::{StreamExt, TryStreamExt, stream};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::loader::Loader;
use crate::mods::{CatalogSort, ContentKind, RemoteFile};
use crate::{Result, http};

const API_URL: &str = "https://api.modrinth.com/v2";
const RESOLVE_CONCURRENCY: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Mod,
    Resourcepack,
    Shader,
    Datapack,
    Modpack,
}

/// What a file has to run on: the game version, and the loader tags Modrinth puts on
/// compatible files.
#[derive(Debug, Clone, Copy)]
pub struct Target<'a> {
    pub game_version: &'a str,
    pub loaders: &'static [&'static str],
}

impl<'a> Target<'a> {
    pub fn new(kind: ContentKind, game_version: &'a str, loader: Option<Loader>) -> Self {
        let loaders: &'static [&'static str] = match (kind, loader) {
            (ContentKind::Mod, Some(Loader::Fabric)) => &["fabric"],
            // Quilt also loads Fabric mods.
            (ContentKind::Mod, Some(Loader::Quilt)) => &["quilt", "fabric"],
            (ContentKind::Mod, Some(Loader::Forge)) => &["forge"],
            // NeoForge for 1.20.1 still loads Forge mods.
            (ContentKind::Mod, Some(Loader::NeoForge)) if game_version == "1.20.1" => {
                &["neoforge", "forge"]
            }
            (ContentKind::Mod, Some(Loader::NeoForge)) => &["neoforge"],
            (ContentKind::Mod, None) => &[],
            (ContentKind::ResourcePack, _) => &["minecraft"],
            // Iris runs nearly every pack made for OptiFine.
            (ContentKind::ShaderPack, _) => &["iris", "optifine"],
            (ContentKind::DataPack, _) => &["datapack"],
        };
        Self {
            game_version,
            loaders,
        }
    }

    /// Whether `version` was published for this game version and loader.
    pub fn accepts(&self, version: &Version) -> bool {
        version
            .game_versions
            .iter()
            .any(|game_version| game_version == self.game_version)
            && (self.loaders.is_empty()
                || version
                    .loaders
                    .iter()
                    .any(|loader| self.loaders.contains(&loader.as_str())))
    }
}

#[derive(Debug, Clone)]
pub struct SearchQuery<'a> {
    pub text: &'a str,
    pub kind: ContentKind,
    pub target: Target<'a>,
    /// Every one of these must match.
    pub categories: &'a [String],
    pub sort: CatalogSort,
    pub offset: u32,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub name: String,
    pub project_type: String,
    pub header: String,
}

/// The project page: description in Markdown and the screenshot gallery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDetails {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub body: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    #[serde(default)]
    pub gallery: Vec<GalleryImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalleryImage {
    pub url: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub hits: Vec<SearchHit>,
    pub total_hits: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    #[serde(default)]
    pub display_categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub version_type: String,
    #[serde(default)]
    pub changelog: Option<String>,
    #[serde(default)]
    pub date_published: Option<DateTime<Utc>>,
    pub files: Vec<VersionFile>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
    pub hashes: FileHashes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHashes {
    pub sha1: String,
    pub sha512: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub dependency_type: DependencyType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    Required,
    Optional,
    Incompatible,
    Embedded,
}

impl ProjectType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::Resourcepack => "resourcepack",
            Self::Shader => "shader",
            Self::Datapack => "datapack",
            Self::Modpack => "modpack",
        }
    }
}

impl From<ContentKind> for ProjectType {
    fn from(kind: ContentKind) -> Self {
        match kind {
            ContentKind::Mod => Self::Mod,
            ContentKind::ResourcePack => Self::Resourcepack,
            ContentKind::ShaderPack => Self::Shader,
            ContentKind::DataPack => Self::Datapack,
        }
    }
}

impl Version {
    /// The file the author marked as the main download, or the first one.
    pub fn primary_file(&self) -> Option<&VersionFile> {
        self.files
            .iter()
            .find(|file| file.primary)
            .or_else(|| self.files.first())
    }

    pub fn remote_file(&self) -> Option<RemoteFile> {
        self.primary_file().map(|file| RemoteFile {
            url: file.url.clone(),
            file_name: file.filename.clone(),
            sha1: file.hashes.sha1.clone(),
            size: file.size,
        })
    }
}

pub async fn search(client: &Client, query: &SearchQuery<'_>) -> Result<SearchResults> {
    let project_type = ProjectType::from(query.kind);
    let mut facets = vec![
        vec![format!("project_type:{}", project_type.as_str())],
        vec![format!("versions:{}", query.target.game_version)],
    ];
    // Resource packs are all tagged `minecraft`, so filtering by it would change nothing.
    if query.kind != ContentKind::ResourcePack && !query.target.loaders.is_empty() {
        facets.push(
            query
                .target
                .loaders
                .iter()
                .map(|name| format!("categories:{name}"))
                .collect(),
        );
    }
    for category in query.categories {
        facets.push(vec![format!("categories:{category}")]);
    }
    let index = match query.sort {
        CatalogSort::Relevance => "relevance",
        CatalogSort::Downloads => "downloads",
        CatalogSort::Newest => "newest",
        CatalogSort::Updated => "updated",
    };

    let results = http::send(
        client,
        client.get(format!("{API_URL}/search")).query(&[
            ("query", query.text),
            ("facets", &serde_json::to_string(&facets)?),
            ("index", index),
            ("offset", &query.offset.to_string()),
            ("limit", &query.limit.to_string()),
        ]),
    )
    .await?
    .error_for_status()?
    .json()
    .await?;
    Ok(results)
}

/// Versions of a project that run on the target, newest first.
pub async fn compatible_versions(
    client: &Client,
    project_id: &str,
    target: Target<'_>,
) -> Result<Vec<Version>> {
    let mut query = vec![(
        "game_versions",
        serde_json::to_string(&[target.game_version])?,
    )];
    if !target.loaders.is_empty() {
        query.push(("loaders", serde_json::to_string(target.loaders)?));
    }
    let mut versions: Vec<Version> = http::send(
        client,
        client
            .get(format!("{API_URL}/project/{project_id}/version"))
            .query(&query),
    )
    .await?
    .error_for_status()?
    .json()
    .await?;
    newest_first(&mut versions);
    Ok(versions)
}

/// Every version of a project, newest first, for picking an older one.
pub async fn all_versions(client: &Client, project_id: &str) -> Result<Vec<Version>> {
    let mut versions: Vec<Version> = http::send(
        client,
        client.get(format!("{API_URL}/project/{project_id}/version")),
    )
    .await?
    .error_for_status()?
    .json()
    .await?;
    newest_first(&mut versions);
    Ok(versions)
}

pub async fn project(client: &Client, id: &str) -> Result<ProjectDetails> {
    let project = http::send(client, client.get(format!("{API_URL}/project/{id}")))
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(project)
}

/// Category tags for one kind of project, without the ones that describe resolution or
/// performance impact.
pub async fn categories(client: &Client, kind: ContentKind) -> Result<Vec<Category>> {
    // Data packs share their tags with mods.
    let project_type = match kind {
        ContentKind::DataPack => ProjectType::Mod,
        kind => ProjectType::from(kind),
    }
    .as_str();
    let categories: Vec<Category> =
        http::send(client, client.get(format!("{API_URL}/tag/category")))
            .await?
            .error_for_status()?
            .json()
            .await?;
    Ok(categories
        .into_iter()
        .filter(|category| category.project_type == project_type && category.header == "categories")
        .collect())
}

pub async fn version(client: &Client, version_id: &str) -> Result<Version> {
    let version = http::send(
        client,
        client.get(format!("{API_URL}/version/{version_id}")),
    )
    .await?
    .error_for_status()?
    .json()
    .await?;
    Ok(version)
}

pub async fn projects(client: &Client, ids: &[String]) -> Result<Vec<Project>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let projects = http::send(
        client,
        client
            .get(format!("{API_URL}/projects"))
            .query(&[("ids", serde_json::to_string(ids)?)]),
    )
    .await?
    .error_for_status()?
    .json()
    .await?;
    Ok(projects)
}

/// Looks files up by SHA-1, so mods added by hand are recognised too.
pub async fn versions_by_hash(
    client: &Client,
    sha1s: &[String],
) -> Result<HashMap<String, Version>> {
    // What a file is never changes, so each hash is asked about once per session.
    static KNOWN: Mutex<Option<HashMap<String, Option<Version>>>> = Mutex::new(None);
    let lock = || KNOWN.lock().unwrap_or_else(PoisonError::into_inner);

    let unknown: Vec<&String> = {
        let known = lock();
        sha1s
            .iter()
            .filter(|sha1| {
                known
                    .as_ref()
                    .is_none_or(|known| !known.contains_key(*sha1))
            })
            .collect()
    };
    if !unknown.is_empty() {
        let mut found: HashMap<String, Version> = http::send(
            client,
            client
                .post(format!("{API_URL}/version_files"))
                .json(&json!({ "hashes": unknown, "algorithm": "sha1" })),
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
        let mut known = lock();
        let known = known.get_or_insert_default();
        for sha1 in unknown {
            known.insert(sha1.clone(), found.remove(sha1));
        }
    }

    let known = lock();
    Ok(sha1s
        .iter()
        .filter_map(|sha1| {
            let version = known.as_ref()?.get(sha1)?.clone()?;
            Some((sha1.clone(), version))
        })
        .collect())
}

/// The newest compatible version for each file, keyed by the file's SHA-1. `current` holds
/// the installed versions; each file only moves to its own release channel or a more stable
/// one, so a release is never swapped for an alpha.
pub async fn updates_by_hash<S: BuildHasher>(
    client: &Client,
    current: &HashMap<String, Version, S>,
    target: Target<'_>,
) -> Result<HashMap<String, Version>> {
    let mut by_channel: HashMap<&[&str], Vec<&String>> = HashMap::new();
    for (sha1, version) in current {
        by_channel
            .entry(stable_as(&version.version_type))
            .or_default()
            .push(sha1);
    }
    let mut updates = HashMap::new();
    for (channels, sha1s) in by_channel {
        let found: HashMap<String, Version> = http::send(
            client,
            client
                .post(format!("{API_URL}/version_files/update"))
                .json(&json!({
                    "hashes": sha1s,
                    "algorithm": "sha1",
                    "loaders": target.loaders,
                    "game_versions": [target.game_version],
                    "version_types": channels,
                })),
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
        updates.extend(found);
    }
    Ok(updates)
}

/// The API lists newest first, but its mirror keeps no order.
fn newest_first(versions: &mut [Version]) {
    versions.sort_by_key(|version| std::cmp::Reverse(version.date_published));
}

/// Channels at least as stable as `version_type`.
fn stable_as(version_type: &str) -> &'static [&'static str] {
    match version_type {
        "alpha" => &["release", "beta", "alpha"],
        "beta" => &["release", "beta"],
        _ => &["release"],
    }
}

/// The projects to download, each with its kind and the version picked for the instance:
/// the requested ones first, then their required dependencies. A dependency can be of
/// another kind than what it is for, like a mod a resource pack needs; `target_for` says
/// what each kind has to run on, or `None` when the instance cannot take that kind.
/// Dependencies pinned to a version get it when it fits the instance; one with no fitting
/// version is skipped rather than failing the install.
pub async fn resolve_install<'a>(
    client: &Client,
    project_ids: &[String],
    kind: ContentKind,
    target_for: impl Fn(ContentKind) -> Option<Target<'a>>,
    installed_projects: impl IntoIterator<Item = String>,
) -> Result<Vec<(ContentKind, Version)>> {
    let mut seen: HashSet<String> = installed_projects.into_iter().collect();
    let mut level: Vec<(String, Option<String>)> =
        project_ids.iter().map(|id| (id.clone(), None)).collect();
    let mut plan = Vec::new();

    // Each depth of the dependency tree is looked up at once.
    while !level.is_empty() {
        level.retain(|(project, _)| seen.insert(project.clone()));
        let found: Vec<_> = stream::iter(level.drain(..))
            .map(|(project, pinned)| resolve_one(client, project, pinned, kind, &target_for))
            .buffered(RESOLVE_CONCURRENCY)
            .try_collect()
            .await?;
        for (found_kind, version) in found.into_iter().flatten() {
            level.extend(
                version
                    .dependencies
                    .iter()
                    .filter(|dependency| dependency.dependency_type == DependencyType::Required)
                    .filter_map(|dependency| {
                        Some((
                            dependency.project_id.clone()?,
                            dependency.version_id.clone(),
                        ))
                    }),
            );
            plan.push((found_kind, version));
        }
    }
    Ok(plan)
}

async fn resolve_one<'a>(
    client: &Client,
    project: String,
    pinned: Option<String>,
    kind: ContentKind,
    target_for: &impl Fn(ContentKind) -> Option<Target<'a>>,
) -> Result<Option<(ContentKind, Version)>> {
    if let Some(version_id) = pinned {
        let version = version(client, &version_id).await?;
        let pinned_kind = kind_of(&version, kind);
        // A pin made for another game version or loader would not run here.
        if target_for(pinned_kind).is_some_and(|target| target.accepts(&version)) {
            return Ok(Some((pinned_kind, version)));
        }
    }
    newest_for_kind(client, &project, kind, target_for).await
}

/// The newest fitting version, looked up as `kind` first and then as the project's own kind.
async fn newest_for_kind<'a>(
    client: &Client,
    project_id: &str,
    kind: ContentKind,
    target_for: &impl Fn(ContentKind) -> Option<Target<'a>>,
) -> Result<Option<(ContentKind, Version)>> {
    if let Some(target) = target_for(kind)
        && let Some(version) = compatible_versions(client, project_id, target)
            .await?
            .into_iter()
            .next()
    {
        return Ok(Some((kind, version)));
    }
    let Some(own_kind) = project_kind(client, project_id).await? else {
        return Ok(None);
    };
    if own_kind == kind {
        return Ok(None);
    }
    let Some(target) = target_for(own_kind) else {
        return Ok(None);
    };
    Ok(compatible_versions(client, project_id, target)
        .await?
        .into_iter()
        .next()
        .map(|version| (own_kind, version)))
}

async fn project_kind(client: &Client, project_id: &str) -> Result<Option<ContentKind>> {
    #[derive(Deserialize)]
    struct ProjectType {
        project_type: String,
    }
    let project: ProjectType = http::send(
        client,
        client.get(format!("{API_URL}/project/{project_id}")),
    )
    .await?
    .error_for_status()?
    .json()
    .await?;
    Ok(match project.project_type.as_str() {
        "mod" => Some(ContentKind::Mod),
        "resourcepack" => Some(ContentKind::ResourcePack),
        "shader" => Some(ContentKind::ShaderPack),
        "datapack" => Some(ContentKind::DataPack),
        _ => None,
    })
}

/// A version's kind from its loader tags, for versions pinned by a dependency.
fn kind_of(version: &Version, fallback: ContentKind) -> ContentKind {
    let has = |tag: &str| version.loaders.iter().any(|loader| loader == tag);
    if has("minecraft") {
        ContentKind::ResourcePack
    } else if has("iris") || has("optifine") || has("canvas") {
        ContentKind::ShaderPack
    } else if has("datapack") {
        ContentKind::DataPack
    } else if version.loaders.is_empty() {
        fallback
    } else {
        ContentKind::Mod
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updates_never_get_less_stable() {
        assert_eq!(stable_as("release"), ["release"]);
        assert_eq!(stable_as("beta"), ["release", "beta"]);
        assert!(stable_as("alpha").contains(&"alpha"));
    }

    #[test]
    fn parses_versions() {
        let version: Version = serde_json::from_str(
            r#"{
                "id": "AQpu5aS1", "project_id": "AANobbMI", "name": "Sodium 0.7.0",
                "version_number": "mc1.21.9-0.7.0-fabric", "version_type": "release",
                "files": [
                    { "url": "https://cdn.modrinth.com/a.jar", "filename": "a.jar", "primary": false,
                      "size": 1, "hashes": { "sha1": "aa", "sha512": "bb" } },
                    { "url": "https://cdn.modrinth.com/b.jar", "filename": "b.jar", "primary": true,
                      "size": 2, "hashes": { "sha1": "cc", "sha512": "dd" } }
                ],
                "dependencies": [
                    { "project_id": "P7dR8mSH", "version_id": null, "dependency_type": "required" }
                ],
                "game_versions": ["1.21.9", "1.21.10"], "loaders": ["fabric"]
            }"#,
        )
        .unwrap();

        assert_eq!(version.primary_file().unwrap().filename, "b.jar");
        assert_eq!(
            version.dependencies[0].dependency_type,
            DependencyType::Required
        );
        let target = |game_version, loader| Target::new(ContentKind::Mod, game_version, loader);
        assert!(target("1.21.10", Some(Loader::Quilt)).accepts(&version));
        assert!(!target("1.21.11", Some(Loader::Fabric)).accepts(&version));
    }

    #[test]
    fn quilt_accepts_fabric_mods() {
        let target = Target::new(ContentKind::Mod, "1.21.10", Some(Loader::Quilt));
        assert_eq!(target.loaders, ["quilt", "fabric"]);
    }

    #[test]
    fn tells_pinned_dependency_kind_by_loaders() {
        let with = |loaders: &[&str]| Version {
            id: String::new(),
            project_id: String::new(),
            name: String::new(),
            version_number: String::new(),
            version_type: "release".into(),
            changelog: None,
            date_published: None,
            files: Vec::new(),
            dependencies: Vec::new(),
            game_versions: Vec::new(),
            loaders: loaders.iter().map(ToString::to_string).collect(),
        };
        let fallback = ContentKind::ResourcePack;
        assert_eq!(kind_of(&with(&["fabric"]), fallback), ContentKind::Mod);
        assert_eq!(
            kind_of(&with(&["minecraft"]), fallback),
            ContentKind::ResourcePack
        );
        assert_eq!(kind_of(&with(&["iris"]), fallback), ContentKind::ShaderPack);
        assert_eq!(kind_of(&with(&[]), fallback), fallback);
    }
}
