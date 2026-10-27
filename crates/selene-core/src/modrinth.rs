use std::collections::{HashMap, HashSet};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::Result;
use crate::loader::Loader;
use crate::mods::RemoteFile;

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

#[derive(Debug, Clone)]
pub struct SearchQuery<'a> {
    pub text: &'a str,
    pub project_type: ProjectType,
    pub game_version: &'a str,
    pub loader: Option<Loader>,
    pub offset: u32,
    pub limit: u32,
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
    pub files: Vec<VersionFile>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
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

/// Loaders whose mods run on `loader`: Quilt also loads Fabric mods.
fn compatible_loaders(loader: Loader) -> &'static [&'static str] {
    match loader {
        Loader::Fabric => &["fabric"],
        Loader::Quilt => &["quilt", "fabric"],
    }
}

pub async fn search(client: &Client, query: &SearchQuery<'_>) -> Result<SearchResults> {
    let mut facets = vec![
        vec![format!("project_type:{}", query.project_type.as_str())],
        vec![format!("versions:{}", query.game_version)],
    ];
    if let Some(loader) = query.loader {
        facets.push(
            compatible_loaders(loader)
                .iter()
                .map(|name| format!("categories:{name}"))
                .collect(),
        );
    }

    let results = client
        .get(format!("{API_URL}/search"))
        .query(&[
            ("query", query.text),
            ("facets", &serde_json::to_string(&facets)?),
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

/// Versions of a project that run on this game version and loader, newest first.
pub async fn compatible_versions(
    client: &Client,
    project_id: &str,
    game_version: &str,
    loader: Option<Loader>,
) -> Result<Vec<Version>> {
    let mut query = vec![("game_versions", serde_json::to_string(&[game_version])?)];
    if let Some(loader) = loader {
        query.push((
            "loaders",
            serde_json::to_string(compatible_loaders(loader))?,
        ));
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

/// The newest compatible version for each file, keyed by the file's SHA-1.
pub async fn updates_by_hash(
    client: &Client,
    sha1s: &[String],
    game_version: &str,
    loader: Loader,
) -> Result<HashMap<String, Version>> {
    if sha1s.is_empty() {
        return Ok(HashMap::new());
    }
    let versions = client
        .post(format!("{API_URL}/version_files/update"))
        .json(&json!({
            "hashes": sha1s,
            "algorithm": "sha1",
            "loaders": compatible_loaders(loader),
            "game_versions": [game_version],
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(versions)
}

/// The project plus every required dependency that is not installed yet.
///
/// Dependencies pinned to a version get that version; the rest get their newest compatible
/// one. A dependency with no compatible version is skipped rather than failing the install,
/// the same way the Modrinth app handles it.
pub async fn resolve_install(
    client: &Client,
    project_id: &str,
    game_version: &str,
    loader: Option<Loader>,
    installed_projects: impl IntoIterator<Item = String>,
) -> Result<Vec<Version>> {
    let mut seen: HashSet<String> = installed_projects.into_iter().collect();
    let mut queue = vec![(project_id.to_owned(), None::<String>)];
    let mut plan = Vec::new();

    while let Some((project, pinned)) = queue.pop() {
        if !seen.insert(project.clone()) {
            continue;
        }
        let version = match pinned {
            Some(version_id) => Some(version(client, &version_id).await?),
            None => compatible_versions(client, &project, game_version, loader)
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
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(version.primary_file().unwrap().filename, "b.jar");
        assert_eq!(
            version.dependencies[0].dependency_type,
            DependencyType::Required
        );
    }

    #[test]
    fn quilt_accepts_fabric_mods() {
        assert_eq!(compatible_loaders(Loader::Quilt), ["quilt", "fabric"]);
    }
}
