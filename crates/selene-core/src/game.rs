use std::path::{Path, PathBuf};

use reqwest::Client;

use crate::download::{self, Progress};
use crate::java::{self, JavaRuntime};
use crate::loader::{self, LoaderVersion};
use crate::manifest::VersionEntry;
use crate::rules::{Environment, Features};
use crate::version::{AssetIndex, VersionInfo};
use crate::{Error, Result};

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
}

#[derive(Debug, Clone)]
pub struct Installed {
    pub version: VersionInfo,
    pub java: JavaRuntime,
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
    let mut version = VersionInfo::fetch(client, entry).await?;
    if let Some(loader) = loader {
        let patch = loader::fetch_patch(client, loader, &entry.id).await?;
        version = version.with_patch(patch.resolve(client).await?);
    }
    let env = Environment::current(Features::default());

    let index = fetch_asset_index(client, &version, &dirs.assets()).await?;
    let mut downloads = version.downloads(&env, &dirs.versions(), &dirs.libraries());
    downloads.extend(index.downloads(&dirs.assets()));
    download::download_all(client, downloads, DOWNLOAD_CONCURRENCY, |progress| {
        on_progress(Stage::Files, progress);
    })
    .await?;

    let component = version
        .java_version
        .as_ref()
        .map_or(LEGACY_JAVA, |java| java.component.as_str());
    let runtimes = java::fetch_index(client).await?;
    let java = java::install(client, &runtimes, component, &dirs.runtimes(), |progress| {
        on_progress(Stage::Java, progress);
    })
    .await?;

    Ok(Installed { version, java })
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
