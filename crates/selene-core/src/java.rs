use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::download::{self, Download, Progress};
use crate::{Error, Result, cache, http};

/// Index of every Java runtime Mojang ships, the same one the official launcher reads.
pub const RUNTIME_INDEX_URL: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

const INSTALL_MARKER: &str = ".selene-runtime.json";
const DOWNLOAD_CONCURRENCY: usize = 16;

/// Runtime releases keyed by platform, then by component (`java-runtime-delta`, `jre-legacy`, ...).
pub type RuntimeIndex = HashMap<String, HashMap<String, Vec<RuntimeRelease>>>;

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeRelease {
    pub manifest: FileRef,
    pub version: RuntimeVersion,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeVersion {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileRef {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeManifest {
    pub files: BTreeMap<String, RuntimeEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum RuntimeEntry {
    File {
        #[serde(default)]
        executable: bool,
        downloads: FileDownloads,
    },
    Directory,
    Link {
        target: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileDownloads {
    pub raw: FileRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct InstallMarker {
    version: String,
    manifest_sha1: String,
}

#[derive(Debug, Clone)]
pub struct JavaRuntime {
    pub component: String,
    pub version: String,
    pub home: PathBuf,
}

impl JavaRuntime {
    /// `javaw` on Windows so the game starts without a console window.
    pub fn executable(&self) -> PathBuf {
        if cfg!(windows) {
            self.home.join("bin").join("javaw.exe")
        } else if cfg!(target_os = "macos") {
            self.home.join("jre.bundle/Contents/Home/bin/java")
        } else {
            self.home.join("bin").join("java")
        }
    }

    /// `java` with console output, for tools whose output the launcher reads.
    pub fn console_executable(&self) -> PathBuf {
        if cfg!(windows) {
            self.home.join("bin").join("java.exe")
        } else {
            self.executable()
        }
    }
}

/// Mojang's name for the platform this launcher was built for.
pub fn platform() -> Option<&'static str> {
    platform_key(std::env::consts::OS, std::env::consts::ARCH)
}

fn platform_key(os: &str, arch: &str) -> Option<&'static str> {
    let key = match (os, arch) {
        ("windows", "x86_64") => "windows-x64",
        ("windows", "x86") => "windows-x86",
        ("windows", "aarch64") => "windows-arm64",
        ("macos", "x86_64") => "mac-os",
        ("macos", "aarch64") => "mac-os-arm64",
        ("linux", "x86_64") => "linux",
        ("linux", "x86") => "linux-i386",
        _ => return None,
    };
    Some(key)
}

/// The list of Java runtimes, from the copy in `runtimes_dir` when offline.
pub async fn fetch_index(client: &Client, runtimes_dir: &Path) -> Result<RuntimeIndex> {
    cache::get_json(client, RUNTIME_INDEX_URL, &runtimes_dir.join("index.json")).await
}

pub fn find_release<'a>(
    index: &'a RuntimeIndex,
    platform: &str,
    component: &str,
) -> Option<&'a RuntimeRelease> {
    index.get(platform)?.get(component)?.first()
}

/// Installs a runtime component into `runtimes_dir/<component>` and returns it.
///
/// A runtime that is already installed from the same manifest is returned without touching the
/// network beyond the index, so this is cheap to call before every launch.
pub async fn install<F>(
    client: &Client,
    index: &RuntimeIndex,
    component: &str,
    runtimes_dir: &Path,
    on_progress: F,
) -> Result<JavaRuntime>
where
    F: Fn(Progress) + Sync,
{
    let platform = platform().ok_or(Error::UnsupportedPlatform)?;
    let release =
        find_release(index, platform, component).ok_or_else(|| Error::JavaUnavailable {
            component: component.to_owned(),
            platform: platform.to_owned(),
        })?;

    let runtime = JavaRuntime {
        component: component.to_owned(),
        version: release.version.name.clone(),
        home: runtimes_dir.join(component),
    };
    let marker = InstallMarker {
        version: release.version.name.clone(),
        manifest_sha1: release.manifest.sha1.clone(),
    };

    if read_marker(&runtime.home).await.as_ref() == Some(&marker)
        && fs::try_exists(runtime.executable()).await.unwrap_or(false)
    {
        return Ok(runtime);
    }

    let manifest = fetch_manifest(client, &release.manifest).await?;
    install_files(client, &manifest, &runtime.home, on_progress).await?;
    write_marker(&runtime.home, &marker).await?;

    Ok(runtime)
}

async fn fetch_manifest(client: &Client, manifest: &FileRef) -> Result<RuntimeManifest> {
    let bytes = http::get(client, &manifest.url)
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let actual = download::sha1_bytes(&bytes);
    if !actual.eq_ignore_ascii_case(&manifest.sha1) {
        return Err(Error::ChecksumMismatch {
            url: manifest.url.clone(),
            expected: manifest.sha1.clone(),
            actual,
        });
    }

    Ok(serde_json::from_slice(&bytes)?)
}

async fn install_files<F>(
    client: &Client,
    manifest: &RuntimeManifest,
    home: &Path,
    on_progress: F,
) -> Result<()>
where
    F: Fn(Progress) + Sync,
{
    let mut downloads = Vec::new();
    let mut executables = Vec::new();
    let mut links = Vec::new();

    for (relative, entry) in &manifest.files {
        let path = home.join(relative);
        match entry {
            RuntimeEntry::Directory => {
                fs::create_dir_all(&path)
                    .await
                    .map_err(|err| Error::io(&path, err))?;
            }
            RuntimeEntry::File {
                executable,
                downloads: files,
            } => {
                if *executable {
                    executables.push(path.clone());
                }
                downloads.push(Download {
                    url: files.raw.url.clone(),
                    path,
                    sha1: files.raw.sha1.clone(),
                    size: files.raw.size,
                });
            }
            RuntimeEntry::Link { target } => links.push((path, target.clone())),
        }
    }

    download::download_all(client, downloads, DOWNLOAD_CONCURRENCY, on_progress).await?;
    #[cfg(unix)]
    {
        set_executable(&executables)?;
        create_links(&links)?;
    }
    Ok(())
}

/// Windows runtimes need neither: files are executable by extension and the manifest has no links.
#[cfg(unix)]
fn set_executable(paths: &[PathBuf]) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    for path in paths {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .map_err(|err| Error::io(path, err))?;
    }
    Ok(())
}

#[cfg(unix)]
fn create_links(links: &[(PathBuf, String)]) -> Result<()> {
    for (path, target) in links {
        if std::fs::symlink_metadata(path).is_ok() {
            std::fs::remove_file(path).map_err(|err| Error::io(path, err))?;
        }
        std::os::unix::fs::symlink(target, path).map_err(|err| Error::io(path, err))?;
    }
    Ok(())
}

async fn read_marker(home: &Path) -> Option<InstallMarker> {
    let bytes = fs::read(home.join(INSTALL_MARKER)).await.ok()?;
    serde_json::from_slice(&bytes).ok()
}

async fn write_marker(home: &Path, marker: &InstallMarker) -> Result<()> {
    let path = home.join(INSTALL_MARKER);
    fs::write(&path, serde_json::to_vec(marker)?)
        .await
        .map_err(|err| Error::io(&path, err))
}

#[cfg(test)]
mod tests {
    use super::*;

    const INDEX: &str = r#"{
        "windows-x64": {
            "java-runtime-delta": [{
                "availability": { "group": 1, "progress": 100 },
                "manifest": { "sha1": "d1", "size": 100, "url": "https://example.com/delta.json" },
                "version": { "name": "21.0.7", "released": "2025-04-15T00:00:00+00:00" }
            }],
            "jre-legacy": []
        }
    }"#;

    const MANIFEST: &str = r#"{
        "files": {
            "bin": { "type": "directory" },
            "bin/javaw.exe": {
                "type": "file",
                "executable": true,
                "downloads": {
                    "lzma": { "sha1": "aa", "size": 10, "url": "https://example.com/javaw.lzma" },
                    "raw": { "sha1": "bb", "size": 20, "url": "https://example.com/javaw.exe" }
                }
            },
            "legal/java.base/LICENSE": { "type": "link", "target": "../java.base/LICENSE" }
        }
    }"#;

    #[test]
    fn maps_platforms() {
        assert_eq!(platform_key("windows", "x86_64"), Some("windows-x64"));
        assert_eq!(platform_key("macos", "aarch64"), Some("mac-os-arm64"));
        assert_eq!(platform_key("freebsd", "x86_64"), None);
    }

    #[test]
    fn finds_latest_release() {
        let index: RuntimeIndex = serde_json::from_str(INDEX).unwrap();

        let release = find_release(&index, "windows-x64", "java-runtime-delta").unwrap();
        assert_eq!(release.version.name, "21.0.7");
        assert!(find_release(&index, "windows-x64", "jre-legacy").is_none());
        assert!(find_release(&index, "linux", "java-runtime-delta").is_none());
    }

    #[test]
    fn parses_runtime_manifest() {
        let manifest: RuntimeManifest = serde_json::from_str(MANIFEST).unwrap();

        assert!(matches!(manifest.files["bin"], RuntimeEntry::Directory));
        let RuntimeEntry::File {
            executable,
            downloads,
        } = &manifest.files["bin/javaw.exe"]
        else {
            panic!("expected a file entry");
        };
        assert!(executable);
        assert_eq!(downloads.raw.size, 20);
        assert!(matches!(
            &manifest.files["legal/java.base/LICENSE"],
            RuntimeEntry::Link { target } if target == "../java.base/LICENSE"
        ));
    }

    #[tokio::test]
    async fn marker_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let marker = InstallMarker {
            version: "21.0.7".into(),
            manifest_sha1: "d1".into(),
        };

        assert!(read_marker(dir.path()).await.is_none());
        write_marker(dir.path(), &marker).await.unwrap();
        assert_eq!(read_marker(dir.path()).await, Some(marker));
    }
}
