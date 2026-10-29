use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

use futures_util::{StreamExt, TryStreamExt, stream};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

use crate::version::{Arguments, Artifact, Library, LibraryDownloads, VersionInfo};
use crate::{Error, Result, forge, fs, http};

const FABRIC_META_URL: &str = "https://meta.fabricmc.net/v2";
const QUILT_META_URL: &str = "https://meta.quiltmc.org/v3";
const HASH_CONCURRENCY: usize = 8;
/// NeoForge's version API in particular fails now and then and answers a moment later.
const RELEASES_RETRY_DELAY: Duration = Duration::from_secs(2);

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

impl Loader {
    /// Forge and NeoForge have to run their installer; the others only add a profile.
    pub fn uses_installer(self) -> bool {
        matches!(self, Self::Forge | Self::NeoForge)
    }

    fn id(self) -> &'static str {
        match self {
            Self::Fabric => "fabric",
            Self::Quilt => "quilt",
            Self::Forge => "forge",
            Self::NeoForge => "neoforge",
        }
    }

    fn meta_url(self) -> &'static str {
        match self {
            Self::Quilt => QUILT_META_URL,
            _ => FABRIC_META_URL,
        }
    }
}

/// Loader versions that support `game_version`, newest first. Empty if the loader does not.
///
/// A failed request is tried once more, then the list saved in `cache_dir` last time is used.
pub async fn releases(
    client: &Client,
    loader: Loader,
    game_version: &str,
    cache_dir: &Path,
) -> Result<Vec<LoaderRelease>> {
    let cached = game_version
        .chars()
        .all(|char| char.is_ascii_alphanumeric() || matches!(char, '.' | '-' | '_'))
        .then(|| cache_dir.join(format!("{loader:?}-{game_version}.json").to_lowercase()));
    let mut fetched = fetch_releases(client, loader, game_version).await;
    if matches!(fetched, Err(Error::Http(_))) {
        tokio::time::sleep(RELEASES_RETRY_DELAY).await;
        fetched = fetch_releases(client, loader, game_version).await;
    }
    match (fetched, cached) {
        (Ok(releases), Some(path)) => {
            if let Ok(bytes) = serde_json::to_vec(&releases) {
                let _ = fs::write_atomic(&path, &bytes).await;
            }
            Ok(releases)
        }
        (Err(err @ Error::Http(_)), Some(path)) => match tokio::fs::read(&path).await {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(_) => Err(err),
        },
        (result, _) => result,
    }
}

async fn fetch_releases(
    client: &Client,
    loader: Loader,
    game_version: &str,
) -> Result<Vec<LoaderRelease>> {
    let mut releases = if loader.uses_installer() {
        forge::releases(client, loader, game_version).await?
    } else {
        profile_releases(client, loader, game_version).await?
    };
    // Quilt lists its builds in no particular order.
    releases.sort_by(|a, b| compare_versions(&b.version, &a.version));
    Ok(releases)
}

async fn profile_releases(
    client: &Client,
    loader: Loader,
    game_version: &str,
) -> Result<Vec<LoaderRelease>> {
    let url = format!("{}/versions/loader/{game_version}", loader.meta_url());
    let response = http::get(client, &url).await?;
    // Both sites answer versions they do not support (anything before 1.14) with an error.
    if matches!(
        response.status(),
        StatusCode::BAD_REQUEST | StatusCode::NOT_FOUND
    ) {
        return Ok(Vec::new());
    }
    let entries: Vec<LoaderEntry> = response.error_for_status()?.json().await?;
    // Fabric calls only its newest build stable, so the version string decides instead.
    Ok(entries
        .into_iter()
        .map(|entry| LoaderRelease {
            stable: !entry.loader.version.contains('-'),
            recommended: false,
            version: entry.loader.version,
        })
        .collect())
}

/// Orders versions like `0.30.0-beta.10` by their numbers, with a pre-release before the
/// release it leads to.
fn compare_versions(a: &str, b: &str) -> Ordering {
    let split = |version: &str| {
        let (release, pre) = version.split_once('-').unwrap_or((version, ""));
        (release.to_owned(), pre.to_owned())
    };
    let (a_release, a_pre) = split(a);
    let (b_release, b_pre) = split(b);
    compare_parts(&a_release, &b_release).then_with(|| match (a_pre.is_empty(), b_pre.is_empty()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => compare_parts(&a_pre, &b_pre),
    })
}

/// Compares dot-separated parts, numbers by value and words alphabetically.
fn compare_parts(a: &str, b: &str) -> Ordering {
    let mut a_parts = a.split(['.', '+']);
    let mut b_parts = b.split(['.', '+']);
    loop {
        return match (a_parts.next(), b_parts.next()) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(a), Some(b)) => {
                let order = match (a.parse::<u64>(), b.parse::<u64>()) {
                    (Ok(a), Ok(b)) => a.cmp(&b),
                    _ => a.cmp(b),
                };
                if order == Ordering::Equal {
                    continue;
                }
                order
            }
        };
    }
}

/// The loader's profile with every library hash filled in. A published profile never
/// changes, so it is saved under `versions_dir` and read from there on later launches.
pub async fn resolved_patch(
    client: &Client,
    loader: &LoaderVersion,
    game_version: &str,
    versions_dir: &Path,
) -> Result<ResolvedPatch> {
    let path = versions_dir.join(game_version).join(format!(
        "{}-{}.json",
        loader.loader.id(),
        loader.version
    ));
    let saved = tokio::fs::read(&path)
        .await
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
    let fresh = saved.is_none();
    let mut profile = match saved {
        Some(profile) => profile,
        None => http::get_json(client, &profile_url(loader, game_version)).await?,
    };
    let mut filled = false;
    if let Some(libraries) = profile
        .get_mut("libraries")
        .and_then(serde_json::Value::as_array_mut)
    {
        filled = fill_hashes(client, libraries).await?;
    }
    if fresh || filled {
        let _ = crate::fs::write_atomic(&path, &serde_json::to_vec(&profile)?).await;
    }
    Ok(serde_json::from_value::<VersionPatch>(profile)?.resolve())
}

fn profile_url(loader: &LoaderVersion, game_version: &str) -> String {
    format!(
        "{}/versions/loader/{game_version}/{}/profile/json",
        loader.loader.meta_url(),
        loader.version
    )
}

/// Adds the hashes the profile lacks and tells whether it found any.
async fn fill_hashes(client: &Client, libraries: &mut [serde_json::Value]) -> Result<bool> {
    let missing: Vec<(usize, String)> = libraries
        .iter()
        .enumerate()
        .filter(|(_, library)| library.get("sha1").is_none_or(serde_json::Value::is_null))
        .filter_map(|(index, library)| {
            let name = library.get("name")?.as_str()?;
            let url = library.get("url")?.as_str()?;
            let path = maven_path(name)?;
            Some((index, format!("{}/{path}", url.trim_end_matches('/'))))
        })
        .collect();
    let hashes: Vec<(usize, String)> = stream::iter(missing)
        .map(|(index, url)| async move {
            Ok::<_, crate::Error>((index, fetch_sha1(client, &url).await))
        })
        .buffer_unordered(HASH_CONCURRENCY)
        .try_collect()
        .await?;
    // A missing hash is not saved, so the next start asks for it again.
    let mut filled = false;
    for (index, sha1) in hashes.into_iter().filter(|(_, sha1)| !sha1.is_empty()) {
        libraries[index]["sha1"] = sha1.into();
        filled = true;
    }
    Ok(filled)
}

impl VersionPatch {
    /// Turns Maven libraries into regular ones so the game version can download and launch
    /// them like its own.
    pub fn resolve(self) -> ResolvedPatch {
        ResolvedPatch {
            main_class: self.main_class,
            arguments: self.arguments,
            minecraft_arguments: None,
            libraries: self
                .libraries
                .into_iter()
                .map(MavenLibrary::resolve)
                .collect(),
        }
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
    fn resolve(self) -> Library {
        let path = maven_path(&self.name).unwrap_or_default();
        let url = format!("{}/{path}", self.url.trim_end_matches('/'));
        Library {
            name: self.name,
            downloads: LibraryDownloads {
                artifact: Some(Artifact {
                    path: Some(path),
                    sha1: self.sha1.unwrap_or_default(),
                    size: self.size.unwrap_or(0),
                    url,
                }),
                ..LibraryDownloads::default()
            },
            rules: Vec::new(),
            natives: std::collections::HashMap::new(),
        }
    }
}

/// The checksum published next to a Maven file, or an empty string when neither the
/// repository nor BMCLAPI has it. The file is then taken unchecked rather than not at all,
/// since that is when the official repository cannot be reached.
pub(crate) async fn fetch_sha1(client: &Client, jar_url: &str) -> String {
    let text: Result<String> = async {
        let url = format!("{jar_url}.sha1");
        Ok(http::get(client, &url)
            .await?
            .error_for_status()?
            .text()
            .await?)
    }
    .await;
    // Some repositories append the file name after the hash.
    text.ok()
        .and_then(|text| text.split_whitespace().next().map(str::to_owned))
        .unwrap_or_default()
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
        let replaced: HashSet<String> = patch
            .libraries
            .iter()
            .map(|library| library_key(&library.name))
            .collect();
        self.libraries
            .retain(|library| !replaced.contains(&library_key(&library.name)));
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

/// `group:artifact[:classifier]` without the version, so two versions of one library compare
/// equal while its natives for other platforms stay apart.
fn library_key(name: &str) -> String {
    let name = name.split('@').next().unwrap_or(name);
    let parts: Vec<&str> = name.split(':').collect();
    match parts.as_slice() {
        [group, artifact, _version] => format!("{group}:{artifact}"),
        [group, artifact, _version, classifier, ..] => format!("{group}:{artifact}:{classifier}"),
        _ => name.to_owned(),
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
        assert_eq!(
            library_key("org.lwjgl:lwjgl:3.3.3:natives-windows"),
            "org.lwjgl:lwjgl:natives-windows"
        );
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
    fn sorts_versions_newest_first() {
        let mut versions = vec![
            "0.30.0-beta.4",
            "0.19.5",
            "0.30.0-beta.10",
            "0.30.0",
            "0.19.10",
            "0.30.0-beta.2",
        ];
        versions.sort_by(|a, b| compare_versions(b, a));

        assert_eq!(
            versions,
            [
                "0.30.0",
                "0.30.0-beta.10",
                "0.30.0-beta.4",
                "0.30.0-beta.2",
                "0.19.10",
                "0.19.5"
            ]
        );
        assert_eq!(compare_versions("21.1.265", "21.1.77"), Ordering::Greater);
        assert_eq!(
            compare_versions("14.23.5.2860", "14.23.5.2860"),
            Ordering::Equal
        );
    }
}
