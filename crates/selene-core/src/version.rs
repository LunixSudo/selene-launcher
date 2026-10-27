use std::collections::HashMap;
use std::path::Path;

use reqwest::Client;
use serde::Deserialize;

use crate::download::Download;
use crate::manifest::VersionEntry;
use crate::rules::{self, Environment, Rule};
use crate::{Error, Result, download};

const ASSETS_BASE_URL: &str = "https://resources.download.minecraft.net";

/// The full description of one game version: what to download and how to start it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub id: String,
    pub main_class: String,
    pub asset_index: AssetIndexRef,
    pub downloads: VersionDownloads,
    #[serde(default)]
    pub libraries: Vec<Library>,
    pub java_version: Option<JavaVersion>,
    pub arguments: Option<Arguments>,
    /// Space-separated game arguments used by versions before 1.13.
    pub minecraft_arguments: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndexRef {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub total_size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionDownloads {
    pub client: Artifact,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Artifact {
    pub path: Option<String>,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub component: String,
    pub major_version: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: LibraryDownloads,
    #[serde(default)]
    pub rules: Vec<Rule>,
    /// Old-style natives: OS name to classifier, e.g. `windows` to `natives-windows`.
    #[serde(default)]
    pub natives: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<Artifact>,
    #[serde(default)]
    pub classifiers: HashMap<String, Artifact>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Argument>,
    #[serde(default)]
    pub jvm: Vec<Argument>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Argument {
    Plain(String),
    Conditional {
        rules: Vec<Rule>,
        value: ArgumentValue,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

impl VersionInfo {
    /// Fetches the version JSON and checks it against the hash from the manifest.
    pub async fn fetch(client: &Client, entry: &VersionEntry) -> Result<Self> {
        let bytes = client
            .get(&entry.url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        let actual = download::sha1_bytes(&bytes);
        if !actual.eq_ignore_ascii_case(&entry.sha1) {
            return Err(Error::ChecksumMismatch {
                url: entry.url.clone(),
                expected: entry.sha1.clone(),
                actual,
            });
        }

        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn libraries<'a>(&'a self, env: &'a Environment) -> impl Iterator<Item = &'a Library> {
        self.libraries
            .iter()
            .filter(|library| rules::allowed(&library.rules, env))
    }

    /// Client jar and every library this machine needs, laid out the way the game expects.
    pub fn downloads(
        &self,
        env: &Environment,
        versions_dir: &Path,
        libraries_dir: &Path,
    ) -> Vec<Download> {
        let client = &self.downloads.client;
        let mut downloads = vec![Download {
            url: client.url.clone(),
            path: versions_dir.join(&self.id).join(format!("{}.jar", self.id)),
            sha1: client.sha1.clone(),
            size: client.size,
        }];

        for library in self.libraries(env) {
            let artifacts = library
                .downloads
                .artifact
                .iter()
                .chain(library.native_artifact(env));
            for artifact in artifacts {
                let Some(path) = &artifact.path else {
                    continue;
                };
                downloads.push(Download {
                    url: artifact.url.clone(),
                    path: libraries_dir.join(path),
                    sha1: artifact.sha1.clone(),
                    size: artifact.size,
                });
            }
        }

        downloads
    }
}

impl Library {
    pub fn native_artifact(&self, env: &Environment) -> Option<&Artifact> {
        let classifier = self.natives.get(env.os)?;
        // Some old classifiers carry the JVM bitness, e.g. `natives-windows-${arch}`.
        let classifier = classifier.replace("${arch}", if env.arch == "x86" { "32" } else { "64" });
        self.downloads.classifiers.get(&classifier)
    }
}

impl AssetIndexRef {
    pub fn download(&self, assets_dir: &Path) -> Download {
        Download {
            url: self.url.clone(),
            path: assets_dir.join("indexes").join(format!("{}.json", self.id)),
            sha1: self.sha1.clone(),
            size: self.size,
        }
    }
}

impl AssetIndex {
    /// Assets are stored by hash, so the same file is shared by every version that uses it.
    pub fn downloads(&self, assets_dir: &Path) -> Vec<Download> {
        let objects_dir = assets_dir.join("objects");
        let mut seen = std::collections::HashSet::new();

        self.objects
            .values()
            .filter(|object| seen.insert(object.hash.as_str()))
            .map(|object| {
                let prefix = &object.hash[..2];
                Download {
                    url: format!("{ASSETS_BASE_URL}/{prefix}/{}", object.hash),
                    path: objects_dir.join(prefix).join(&object.hash),
                    sha1: object.hash.clone(),
                    size: object.size,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Features;

    const VERSION: &str = r#"{
        "id": "1.21.10",
        "mainClass": "net.minecraft.client.main.Main",
        "assetIndex": {
            "id": "27", "sha1": "a1", "size": 10, "totalSize": 1000,
            "url": "https://piston-meta.mojang.com/v1/packages/a1/27.json"
        },
        "downloads": {
            "client": { "sha1": "c1", "size": 30, "url": "https://piston-data.mojang.com/v1/objects/c1/client.jar" }
        },
        "javaVersion": { "component": "java-runtime-delta", "majorVersion": 21 },
        "arguments": {
            "game": [
                "--username", "${auth_player_name}",
                { "rules": [{ "action": "allow", "features": { "has_custom_resolution": true } }],
                  "value": ["--width", "${resolution_width}"] }
            ],
            "jvm": [
                { "rules": [{ "action": "allow", "os": { "name": "osx" } }], "value": "-XstartOnFirstThread" },
                "-cp", "${classpath}"
            ]
        },
        "libraries": [
            {
                "name": "com.mojang:brigadier:1.3.10",
                "downloads": { "artifact": {
                    "path": "com/mojang/brigadier/1.3.10/brigadier-1.3.10.jar",
                    "sha1": "l1", "size": 1, "url": "https://libraries.minecraft.net/l1.jar" } }
            },
            {
                "name": "org.lwjgl:lwjgl:3.3.3:natives-macos",
                "downloads": { "artifact": {
                    "path": "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-macos.jar",
                    "sha1": "l2", "size": 2, "url": "https://libraries.minecraft.net/l2.jar" } },
                "rules": [{ "action": "allow", "os": { "name": "osx" } }]
            },
            {
                "name": "org.lwjgl.lwjgl:lwjgl-platform:2.9.4",
                "natives": { "windows": "natives-windows-${arch}" },
                "downloads": { "classifiers": { "natives-windows-64": {
                    "path": "org/lwjgl/lwjgl/lwjgl-platform/2.9.4/lwjgl-platform-2.9.4-natives-windows-64.jar",
                    "sha1": "l3", "size": 3, "url": "https://libraries.minecraft.net/l3.jar" } } }
            }
        ]
    }"#;

    fn windows() -> Environment {
        Environment {
            os: "windows",
            arch: "x86_64",
            features: Features::default(),
        }
    }

    #[test]
    fn parses_version() {
        let version: VersionInfo = serde_json::from_str(VERSION).unwrap();

        assert_eq!(version.java_version.unwrap().major_version, 21);
        let arguments = version.arguments.unwrap();
        assert_eq!(arguments.game.len(), 3);
        assert!(matches!(
            &arguments.jvm[0],
            Argument::Conditional { value: ArgumentValue::One(value), .. } if value == "-XstartOnFirstThread"
        ));
    }

    #[test]
    fn collects_downloads_for_platform() {
        let version: VersionInfo = serde_json::from_str(VERSION).unwrap();
        let downloads =
            version.downloads(&windows(), Path::new("versions"), Path::new("libraries"));
        let hashes: Vec<_> = downloads
            .iter()
            .map(|download| download.sha1.as_str())
            .collect();

        assert_eq!(hashes, ["c1", "l1", "l3"]);
        assert_eq!(downloads[0].path, Path::new("versions/1.21.10/1.21.10.jar"));
        assert_eq!(
            downloads[1].path,
            Path::new("libraries/com/mojang/brigadier/1.3.10/brigadier-1.3.10.jar")
        );
    }

    #[test]
    fn asset_objects_are_stored_by_hash() {
        let index: AssetIndex = serde_json::from_str(
            r#"{ "objects": {
                "minecraft/sounds/a.ogg": { "hash": "abcdef0123", "size": 5 },
                "minecraft/sounds/b.ogg": { "hash": "abcdef0123", "size": 5 }
            } }"#,
        )
        .unwrap();
        let downloads = index.downloads(Path::new("assets"));

        assert_eq!(downloads.len(), 1);
        assert_eq!(downloads[0].url, format!("{ASSETS_BASE_URL}/ab/abcdef0123"));
        assert_eq!(downloads[0].path, Path::new("assets/objects/ab/abcdef0123"));
    }
}
