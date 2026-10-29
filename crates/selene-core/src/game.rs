use std::path::{Component, Path, PathBuf};

use reqwest::Client;

use crate::download::{self, Progress};
use crate::java::{self, JavaRuntime};
use crate::loader::{self, LoaderVersion};
use crate::manifest::VersionEntry;
use crate::rules::{self, Environment, Features};
use crate::version::{self, AssetIndex, VersionInfo};
use crate::{Error, Result, forge};

const DOWNLOAD_CONCURRENCY: usize = 32;
/// Versions older than the `javaVersion` field all ran on Java 8.
const LEGACY_JAVA: &str = "jre-legacy";

/// Shared folders every instance reuses, so a version is only downloaded once.
#[derive(Debug, Clone)]
pub struct GameDirs {
    pub root: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Files,
    Java,
    /// Forge or NeoForge running their installer.
    Loader,
}

#[derive(Debug, Clone)]
pub struct Installed {
    pub version: VersionInfo,
    pub java: JavaRuntime,
    pub asset_index: AssetIndex,
}

impl GameDirs {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn assets(&self) -> PathBuf {
        self.root.join("assets")
    }

    pub fn libraries(&self) -> PathBuf {
        self.root.join("libraries")
    }

    pub fn versions(&self) -> PathBuf {
        self.root.join("versions")
    }

    pub fn runtimes(&self) -> PathBuf {
        self.root.join("runtimes")
    }

    pub fn natives(&self, version_id: &str) -> PathBuf {
        self.versions().join(version_id).join("natives")
    }
}

/// Downloads everything a version needs to start: client, libraries, assets, Java and the
/// mod loader if there is one.
pub async fn install<F>(
    client: &Client,
    dirs: &GameDirs,
    entry: &VersionEntry,
    loader: Option<&LoaderVersion>,
    on_progress: F,
) -> Result<Installed>
where
    F: Fn(Stage, Progress) + Sync,
{
    let mut version = VersionInfo::fetch(client, entry, &dirs.versions()).await?;
    if let Some(loader) = loader.filter(|loader| !loader.loader.uses_installer()) {
        let patch = loader::resolved_patch(client, loader, &entry.id, &dirs.versions()).await?;
        version = version.with_patch(patch);
    }
    let env = Environment::current(Features::default());

    let index = fetch_asset_index(client, &version, &dirs.assets()).await?;
    let mut downloads = version.downloads(&env, &dirs.versions(), &dirs.libraries());
    downloads.extend(index.downloads(&dirs.assets()));
    // Starting the game should not hash a gigabyte of assets each time.
    download::download_missing(client, downloads, DOWNLOAD_CONCURRENCY, |progress| {
        on_progress(Stage::Files, progress);
    })
    .await?;

    let (component, major) = version
        .java_version
        .as_ref()
        .map_or((LEGACY_JAVA, 8), |java| {
            (java.component.as_str(), java.major_version)
        });
    let java = java::install_any(client, component, major, &dirs.runtimes(), |progress| {
        on_progress(Stage::Java, progress);
    })
    .await?;

    if let Some(loader) = loader.filter(|loader| loader.loader.uses_installer()) {
        let patch = forge::install(
            client,
            dirs,
            loader,
            &entry.id,
            &java.console_executable(),
            |progress| on_progress(Stage::Loader, progress),
        )
        .await?;
        let libraries = patch
            .libraries
            .iter()
            .filter(|library| rules::allowed(&library.rules, &env));
        let downloads = version::library_downloads(libraries, &env, &dirs.libraries());
        download::download_missing(client, downloads, DOWNLOAD_CONCURRENCY, |progress| {
            on_progress(Stage::Files, progress);
        })
        .await?;
        version = version.with_patch(patch);
    }

    Ok(Installed {
        version,
        java,
        asset_index: index,
    })
}

impl Installed {
    /// The folder the game reads its assets from. Versions before 1.7.3 want them as plain
    /// files by name, so those get links into the shared store, or copies where links fail.
    pub fn game_assets(&self, dirs: &GameDirs, game_dir: &Path) -> Result<PathBuf> {
        let index = &self.asset_index;
        let target = if index.map_to_resources {
            game_dir.join("resources")
        } else if index.is_virtual {
            dirs.assets()
                .join("virtual")
                .join(&self.version.asset_index.id)
        } else {
            return Ok(dirs.assets());
        };

        let objects = dirs.assets().join("objects");
        for (name, object) in &index.objects {
            let relative = Path::new(name);
            if !relative
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
            {
                continue;
            }
            let path = target.join(relative);
            if std::fs::metadata(&path).is_ok_and(|meta| meta.len() == object.size) {
                continue;
            }
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|err| Error::io(parent, err))?;
            }
            let source = objects.join(&object.hash[..2]).join(&object.hash);
            let _ = std::fs::remove_file(&path);
            if std::fs::hard_link(&source, &path).is_err() {
                std::fs::copy(&source, &path).map_err(|err| Error::io(&path, err))?;
            }
        }
        Ok(target)
    }
}

async fn fetch_asset_index(
    client: &Client,
    version: &VersionInfo,
    assets_dir: &Path,
) -> Result<AssetIndex> {
    let index = version.asset_index.download(assets_dir);
    let path = index.path.clone();
    download::download_all(client, vec![index], 1, |_| {}).await?;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|err| Error::io(&path, err))?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installed(index: &str) -> Installed {
        let version = serde_json::from_str(
            r#"{
                "id": "1.5.2", "type": "release", "mainClass": "net.minecraft.client.Minecraft",
                "assetIndex": { "id": "pre-1.6", "sha1": "a", "size": 1, "totalSize": 1, "url": "https://example.com" },
                "downloads": { "client": { "sha1": "c", "size": 1, "url": "https://example.com" } }
            }"#,
        )
        .unwrap();
        Installed {
            version,
            java: JavaRuntime {
                component: "jre-legacy".into(),
                version: "8".into(),
                home: PathBuf::from("java"),
            },
            asset_index: serde_json::from_str(index).unwrap(),
        }
    }

    #[test]
    fn lays_out_old_assets_by_name() {
        let root = tempfile::tempdir().unwrap();
        let dirs = GameDirs::new(root.path().join("shared"));
        let game_dir = root.path().join("game");
        let object = dirs.assets().join("objects/ab/abcdef");
        std::fs::create_dir_all(object.parent().unwrap()).unwrap();
        std::fs::write(&object, b"ogg").unwrap();

        let old = installed(
            r#"{ "map_to_resources": true, "objects": {
                "sound/step/grass1.ogg": { "hash": "abcdef", "size": 3 },
                "../escape.ogg": { "hash": "abcdef", "size": 3 }
            } }"#,
        );
        let target = old.game_assets(&dirs, &game_dir).unwrap();

        assert_eq!(target, game_dir.join("resources"));
        assert_eq!(
            std::fs::read(target.join("sound/step/grass1.ogg")).unwrap(),
            b"ogg"
        );
        assert!(!game_dir.join("escape.ogg").exists());

        let modern = installed(r#"{ "objects": {} }"#);
        assert_eq!(modern.game_assets(&dirs, &game_dir).unwrap(), dirs.assets());
    }
}
