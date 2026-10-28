use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::Result;
use crate::loader::Loader;
use crate::mods::{CatalogSort, ContentKind, RemoteFile};

const API_URL: &str = "https://api.modrinth.com/v2";

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

    let results = client
        .get(format!("{API_URL}/search"))
        .query(&[
            ("query", query.text),
            ("facets", &serde_json::to_string(&facets)?),
            ("index", index),
            ("offset", &query.offset.to_string()),
            ("limit", &query.limit.to_string()),
        ])
        .send()
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
    let versions = client
        .get(format!("{API_URL}/project/{project_id}/version"))
        .query(&query)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(versions)
}

/// Every version of a project, newest first, for picking an older one.
pub async fn all_versions(client: &Client, project_id: &str) -> Result<Vec<Version>> {
    let versions = client
        .get(format!("{API_URL}/project/{project_id}/version"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(versions)
}

pub async fn project(client: &Client, id: &str) -> Result<ProjectDetails> {
    let project = client
        .get(format!("{API_URL}/project/{id}"))
        .send()
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
    let categories: Vec<Category> = client
        .get(format!("{API_URL}/tag/category"))
        .send()
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
    let version = client
        .get(format!("{API_URL}/version/{version_id}"))
        .send()
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
    let projects = client
        .get(format!("{API_URL}/projects"))
        .query(&[("ids", serde_json::to_string(ids)?)])
        .send()
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
    if sha1s.is_empty() {
        return Ok(HashMap::new());
    }
    let versions = client
        .post(format!("{API_URL}/version_files"))
        .json(&json!({ "hashes": sha1s, "algorithm": "sha1" }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(versions)
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
        let found: HashMap<String, Version> = client
            .post(format!("{API_URL}/version_files/update"))
            .json(&json!({
                "hashes": sha1s,
                "algorithm": "sha1",
                "loaders": target.loaders,
                "game_versions": [target.game_version],
                "version_types": channels,
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        updates.extend(found);
    }
    Ok(updates)
}

/// Channels at least as stable as `version_type`.
fn stable_as(version_type: &str) -> &'static [&'static str] {
    match version_type {
        "alpha" => &["release", "beta", "alpha"],
        "beta" => &["release", "beta"],
        _ => &["release"],
    }
}

/// The project plus every required dependency that is not installed yet.
///
/// Dependencies pinned to a version get that version; the rest get their newest compatible
/// one. A dependency with no compatible version is skipped rather than failing the install,
/// the same way the Modrinth app handles it.
pub async fn resolve_install(
    client: &Client,
    project_ids: &[String],
    target: Target<'_>,
    installed_projects: impl IntoIterator<Item = String>,
) -> Result<Vec<Version>> {
    let mut seen: HashSet<String> = installed_projects.into_iter().collect();
    let mut queue: Vec<(String, Option<String>)> = project_ids
        .iter()
        .rev()
        .map(|id| (id.clone(), None))
        .collect();
    let mut plan = Vec::new();

    while let Some((project, pinned)) = queue.pop() {
        if !seen.insert(project.clone()) {
            continue;
        }
        let version = match pinned {
            Some(version_id) => Some(version(client, &version_id).await?),
            None => compatible_versions(client, &project, target)
                .await?
                .into_iter()
                .next(),
        };
        let Some(version) = version else {
            continue;
        };

        for dependency in &version.dependencies {
            if dependency.dependency_type != DependencyType::Required {
                continue;
            }
            if let Some(project_id) = &dependency.project_id {
                queue.push((project_id.clone(), dependency.version_id.clone()));
            }
        }
        plan.push(version);
    }
    Ok(plan)
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
}
