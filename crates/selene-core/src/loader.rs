use std::collections::HashSet;

use futures_util::{StreamExt, TryStreamExt, stream};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

use crate::version::{Arguments, Artifact, Library, LibraryDownloads, VersionInfo};
use crate::{Result, forge, http};

const FABRIC_META_URL: &str = "https://meta.fabricmc.net/v2";
const QUILT_META_URL: &str = "https://meta.quiltmc.org/v3";
const HASH_CONCURRENCY: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Loader {
    Fabric,
    Quilt,
    Forge,
    NeoForge,
}

/// A loader pinned to one of its versions, as stored in an instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoaderVersion {
    pub loader: Loader,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LoaderRelease {
    pub version: String,
    pub stable: bool,
    /// The build Forge suggests for this game version.
    pub recommended: bool,
}

/// The part of a version JSON a loader adds on top of the game version it inherits from.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionPatch {
    pub main_class: String,
    pub arguments: Option<Arguments>,
    #[serde(default)]
    pub libraries: Vec<MavenLibrary>,
}

/// A library given by Maven coordinates and repository instead of a ready download.
#[derive(Debug, Clone, Deserialize)]
pub struct MavenLibrary {
    pub name: String,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

#[derive(Deserialize)]
struct LoaderEntry {
    loader: LoaderInfo,
}

#[derive(Deserialize)]
struct LoaderInfo {
    version: String,
    /// Quilt leaves this out and marks pre-releases in the version string instead.
    stable: Option<bool>,
}

impl Loader {
    /// Forge and NeoForge have to run their installer; the others only add a profile.
    pub fn uses_installer(self) -> bool {
        matches!(self, Self::Forge | Self::NeoForge)
    }

    fn meta_url(self) -> &'static str {
        match self {
            Self::Quilt => QUILT_META_URL,
            _ => FABRIC_META_URL,
        }
    }
}

/// Loader versions that support `game_version`, newest first. Empty if the loader does not.
pub async fn releases(
    client: &Client,
    loader: Loader,
    game_version: &str,
) -> Result<Vec<LoaderRelease>> {
    if loader.uses_installer() {
        return forge::releases(client, loader, game_version).await;
    }
    let url = format!("{}/versions/loader/{game_version}", loader.meta_url());
    let response = client.get(&url).send().await?;
    // Both sites answer versions they do not support (anything before 1.14) with an error.
    if matches!(
        response.status(),
        StatusCode::BAD_REQUEST | StatusCode::NOT_FOUND
    ) {
        return Ok(Vec::new());
    }
    let entries: Vec<LoaderEntry> = response.error_for_status()?.json().await?;
    Ok(entries
        .into_iter()
        .map(|entry| LoaderRelease {
            stable: entry
                .loader
                .stable
                .unwrap_or_else(|| !entry.loader.version.contains('-')),
            recommended: false,
            version: entry.loader.version,
        })
        .collect())
}

pub async fn fetch_patch(
    client: &Client,
    loader: &LoaderVersion,
    game_version: &str,
) -> Result<VersionPatch> {
    let url = format!(
        "{}/versions/loader/{game_version}/{}/profile/json",
        loader.loader.meta_url(),
        loader.version
    );
    http::get_json(client, &url).await
}

impl VersionPatch {
    /// Turns Maven libraries into regular ones so the game version can download and launch
    /// them like its own. Libraries without a hash get it from the `.sha1` file next to the jar.
    pub async fn resolve(self, client: &Client) -> Result<ResolvedPatch> {
        let libraries = stream::iter(self.libraries)
            .map(|library| library.resolve(client))
            .buffered(HASH_CONCURRENCY)
            .try_collect()
            .await?;
        Ok(ResolvedPatch {
            main_class: self.main_class,
            arguments: self.arguments,
            minecraft_arguments: None,
            libraries,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedPatch {
    pub main_class: String,
    pub arguments: Option<Arguments>,
    /// Replaces the game's own arguments in versions before 1.13.
    pub minecraft_arguments: Option<String>,
    pub libraries: Vec<Library>,
}

impl MavenLibrary {
    async fn resolve(self, client: &Client) -> Result<Library> {
        let path = maven_path(&self.name).unwrap_or_default();
        let url = format!("{}/{path}", self.url.trim_end_matches('/'));
        let sha1 = match self.sha1 {
            Some(sha1) => sha1,
            None => fetch_sha1(client, &url).await?,
        };
        Ok(Library {
            name: self.name,
            downloads: LibraryDownloads {
                artifact: Some(Artifact {
                    path: Some(path),
                    sha1,
                    size: self.size.unwrap_or(0),
                    url,
                }),
                ..LibraryDownloads::default()
            },
            rules: Vec::new(),
            natives: std::collections::HashMap::new(),
        })
    }
}

pub(crate) async fn fetch_sha1(client: &Client, jar_url: &str) -> Result<String> {
    let text = client
        .get(format!("{jar_url}.sha1"))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    // Some repositories append the file name after the hash.
    Ok(text
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_owned())
}

/// `group:artifact:version[:classifier]` to `group/path/artifact/version/artifact-version[-classifier].jar`.
pub fn maven_path(name: &str) -> Option<String> {
    let mut parts = name.split(':');
    let group = parts.next()?;
    let artifact = parts.next()?;
    let version = parts.next()?;
    let classifier = parts.next().map(|classifier| format!("-{classifier}"));
    Some(format!(
        "{}/{artifact}/{version}/{artifact}-{version}{}.jar",
        group.replace('.', "/"),
        classifier.unwrap_or_default()
    ))
}

impl VersionInfo {
    /// The game version with a loader on top: its main class, its extra arguments, and its
    /// libraries in place of any the game ships in another version.
    #[must_use]
    pub fn with_patch(mut self, patch: ResolvedPatch) -> Self {
        let replaced: HashSet<&str> = patch
            .libraries
            .iter()
            .map(|library| library_key(&library.name))
            .collect();
        self.libraries
            .retain(|library| !replaced.contains(library_key(&library.name)));
        let mut libraries = patch.libraries;
        libraries.append(&mut self.libraries);
        self.libraries = libraries;

        self.main_class = patch.main_class;
        if patch.minecraft_arguments.is_some() {
            self.minecraft_arguments = patch.minecraft_arguments;
        }
        if let Some(extra) = patch.arguments {
            match &mut self.arguments {
                Some(arguments) => {
                    arguments.jvm.extend(extra.jvm);
                    arguments.game.extend(extra.game);
                }
                None => self.arguments = Some(extra),
            }
        }
        self
    }
}

/// `group:artifact` without the version, so two versions of one library compare equal.
fn library_key(name: &str) -> &str {
    let mut colons = name.match_indices(':').map(|(index, _)| index);
    match (colons.next(), colons.next()) {
        (Some(_), Some(second)) => &name[..second],
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_maven_paths() {
        assert_eq!(
            maven_path("net.fabricmc:fabric-loader:0.20.1").unwrap(),
            "net/fabricmc/fabric-loader/0.20.1/fabric-loader-0.20.1.jar"
        );
        assert_eq!(
            maven_path("org.lwjgl:lwjgl:3.3.3:natives-windows").unwrap(),
            "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar"
        );
        assert!(maven_path("broken").is_none());
    }

    #[test]
    fn library_keys_drop_the_version() {
        assert_eq!(library_key("org.ow2.asm:asm:9.9"), "org.ow2.asm:asm");
        assert_eq!(library_key("broken"), "broken");
    }

    #[test]
    fn patch_replaces_libraries_and_extends_arguments() {
        let version: VersionInfo = serde_json::from_str(
            r#"{
                "id": "1.21.10", "type": "release", "mainClass": "net.minecraft.client.main.Main",
                "assetIndex": { "id": "27", "sha1": "a", "size": 1, "totalSize": 1, "url": "https://example.com" },
                "downloads": { "client": { "sha1": "c", "size": 1, "url": "https://example.com" } },
                "arguments": { "game": ["--demo"], "jvm": ["-cp", "${classpath}"] },
                "libraries": [
                    { "name": "org.ow2.asm:asm:9.6" },
                    { "name": "com.mojang:brigadier:1.3.10" }
                ]
            }"#,
        )
        .unwrap();
        let patch = ResolvedPatch {
            main_class: "net.fabricmc.loader.impl.launch.knot.KnotClient".into(),
            arguments: serde_json::from_str(
                r#"{ "jvm": ["-DFabricMcEmu= net.minecraft.client.main.Main "] }"#,
            )
            .unwrap(),
            minecraft_arguments: None,
            libraries: vec![serde_json::from_str(r#"{ "name": "org.ow2.asm:asm:9.9" }"#).unwrap()],
        };

        let patched = version.with_patch(patch);
        let names: Vec<_> = patched
            .libraries
            .iter()
            .map(|library| library.name.as_str())
            .collect();

        assert_eq!(
            names,
            ["org.ow2.asm:asm:9.9", "com.mojang:brigadier:1.3.10"]
        );
        assert_eq!(
            patched.main_class,
            "net.fabricmc.loader.impl.launch.knot.KnotClient"
        );
        assert_eq!(patched.arguments.unwrap().jvm.len(), 3);
    }

    #[test]
    fn quilt_marks_betas_unstable() {
        let entries: Vec<LoaderEntry> = serde_json::from_str(
            r#"[{ "loader": { "version": "0.29.0" } }, { "loader": { "version": "0.30.0-beta.1" } }]"#,
        )
        .unwrap();
        let stable: Vec<_> = entries
            .into_iter()
            .map(|entry| {
                entry
                    .loader
                    .stable
                    .unwrap_or_else(|| !entry.loader.version.contains('-'))
            })
            .collect();

        assert_eq!(stable, [true, false]);
    }
}
