use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;

use crate::download::{self, Download, Progress};
use crate::{Error, Result, cache, http};

/// Index of every Java runtime Mojang ships, the same one the official launcher reads.
pub const RUNTIME_INDEX_URL: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

/// Eclipse Temurin and Azul Zulu builds, used when Mojang's runtimes cannot be downloaded.
const ADOPTIUM_API: &str = "https://api.adoptium.net/v3/assets/latest/";
const ADOPTIUM_MIRROR: &str = "https://mirrors.tuna.tsinghua.edu.cn/Adoptium/";
const ZULU_API: &str = "https://api.azul.com/metadata/v1/zulu/packages/";

const INSTALL_MARKER: &str = ".selene-runtime.json";
const DOWNLOAD_CONCURRENCY: usize = 16;

/// Two launches preparing at once must not unpack into the same folder.
static VENDOR_INSTALL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

const VENDORS: [Vendor; 2] = [Vendor::Temurin, Vendor::Zulu];

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

#[derive(Debug, Clone, Deserialize)]
struct AdoptiumAsset {
    binary: AdoptiumBinary,
    release_name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AdoptiumBinary {
    package: AdoptiumPackage,
}

#[derive(Debug, Clone, Deserialize)]
struct AdoptiumPackage {
    checksum: String,
    link: String,
    name: String,
    size: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct ZuluPackage {
    package_uuid: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ZuluDetails {
    name: String,
    download_url: String,
    java_version: Vec<u32>,
    sha256_hash: String,
    size: u64,
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

/// The list of Java runtimes, from the copy in `runtimes_dir` when offline. It is fetched once
/// per session, so launching an installed game does not wait on the network for it.
pub async fn fetch_index(client: &Client, runtimes_dir: &Path) -> Result<RuntimeIndex> {
    static INDEX: tokio::sync::OnceCell<RuntimeIndex> = tokio::sync::OnceCell::const_new();
    let path = runtimes_dir.join("index.json");
    INDEX
        .get_or_try_init(|| cache::get_json(client, RUNTIME_INDEX_URL, &path))
        .await
        .cloned()
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

/// Mojang's runtime from the official hosts, then Temurin and Zulu, then Mojang's runtime from
/// BMCLAPI.
pub async fn install_any<F>(
    client: &Client,
    component: &str,
    major: u32,
    runtimes_dir: &Path,
    on_progress: F,
) -> Result<JavaRuntime>
where
    F: Fn(Progress) + Sync,
{
    if http::source() == http::Source::Fallbacks {
        let index = fetch_index(client, runtimes_dir).await?;
        return install(client, &index, component, runtimes_dir, on_progress).await;
    }
    let official = http::official_only(async {
        let index = fetch_index(client, runtimes_dir).await?;
        install(client, &index, component, runtimes_dir, &on_progress).await
    })
    .await;
    if official.is_ok() {
        return official;
    }
    for vendor in VENDORS {
        if let Some(runtime) = installed_vendor(vendor, major, runtimes_dir).await {
            return Ok(runtime);
        }
    }
    for vendor in VENDORS {
        if let Ok(runtime) = install_vendor(client, vendor, major, runtimes_dir, &on_progress).await
        {
            return Ok(runtime);
        }
    }
    http::fallbacks_only(async {
        let index = fetch_index(client, runtimes_dir).await?;
        install(client, &index, component, runtimes_dir, on_progress).await
    })
    .await
}

/// Other vendors' Java, used when Mojang's runtimes cannot be downloaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    Temurin,
    Zulu,
}

impl Vendor {
    fn component(self, major: u32) -> String {
        match self {
            Self::Temurin => format!("temurin-{major}"),
            Self::Zulu => format!("zulu-{major}"),
        }
    }

    async fn latest(self, client: &Client, major: u32, arch: &str) -> Result<Option<Archive>> {
        match self {
            Self::Temurin => latest_temurin(client, major, arch).await,
            Self::Zulu => latest_zulu(client, major, arch).await,
        }
    }
}

/// A JRE zip and where to get it.
struct Archive {
    name: String,
    version: String,
    sha256: String,
    size: u64,
    urls: Vec<String>,
}

async fn latest_temurin(client: &Client, major: u32, arch: &str) -> Result<Option<Archive>> {
    let url = format!(
        "{ADOPTIUM_API}{major}/hotspot?architecture={arch}&image_type=jre&os=windows&vendor=eclipse"
    );
    let assets: Vec<AdoptiumAsset> = http::get_json(client, &url).await?;
    Ok(assets.into_iter().next().map(|asset| {
        let package = asset.binary.package;
        let mirror = format!(
            "{ADOPTIUM_MIRROR}{major}/jre/{arch}/windows/{}",
            package.name
        );
        Archive {
            version: asset.release_name,
            sha256: package.checksum,
            size: package.size,
            urls: vec![package.link, mirror],
            name: package.name,
        }
    }))
}

async fn latest_zulu(client: &Client, major: u32, arch: &str) -> Result<Option<Archive>> {
    let url = format!(
        "{ZULU_API}?java_version={major}&os=windows&arch={arch}&archive_type=zip\
         &java_package_type=jre&javafx_bundled=false&crac_supported=false&latest=true\
         &release_status=ga&availability_types=CA&certifications=tck&page_size=1"
    );
    let packages: Vec<ZuluPackage> = http::get_json(client, &url).await?;
    let Some(package) = packages.into_iter().next() else {
        return Ok(None);
    };
    let details: ZuluDetails =
        http::get_json(client, &format!("{ZULU_API}{}", package.package_uuid)).await?;
    let version = details
        .java_version
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(".");
    Ok(Some(Archive {
        name: details.name,
        version,
        sha256: details.sha256_hash,
        size: details.size,
        urls: vec![details.download_url],
    }))
}

/// Installs the latest JRE of `major` from `vendor` into `runtimes_dir/<vendor>-<major>`.
pub async fn install_vendor<F>(
    client: &Client,
    vendor: Vendor,
    major: u32,
    runtimes_dir: &Path,
    on_progress: F,
) -> Result<JavaRuntime>
where
    F: Fn(Progress) + Sync,
{
    let arch = vendor_arch(std::env::consts::OS, std::env::consts::ARCH)
        .ok_or(Error::UnsupportedPlatform)?;
    let _guard = VENDOR_INSTALL.lock().await;
    if let Some(runtime) = installed_vendor(vendor, major, runtimes_dir).await {
        return Ok(runtime);
    }

    let component = vendor.component(major);
    let archive =
        vendor
            .latest(client, major, arch)
            .await?
            .ok_or_else(|| Error::JavaUnavailable {
                component: component.clone(),
                platform: format!("windows-{arch}"),
            })?;
    let runtime = JavaRuntime {
        home: runtimes_dir.join(&component),
        component,
        version: archive.version.clone(),
    };

    fs::create_dir_all(runtimes_dir)
        .await
        .map_err(|err| Error::io(runtimes_dir, err))?;
    let path = runtimes_dir.join(format!("{}.part", archive.name));
    let mut result = Ok(());
    for url in &archive.urls {
        result = fetch_archive(client, url, &archive, &path, &on_progress).await;
        if result.is_ok() {
            break;
        }
    }
    if let Err(err) = result {
        let _ = fs::remove_file(&path).await;
        return Err(err);
    }

    let home = runtime.home.clone();
    let extracted = tokio::task::spawn_blocking({
        let path = path.clone();
        move || extract_archive(&path, &home)
    })
    .await
    .map_err(|err| Error::io(&path, std::io::Error::other(err)));
    let _ = fs::remove_file(&path).await;
    extracted??;
    let marker = InstallMarker {
        version: archive.version,
        manifest_sha1: archive.sha256,
    };
    write_marker(&runtime.home, &marker).await?;

    Ok(runtime)
}

/// A build of `vendor` installed earlier. It is used as is: Mojang's runtime takes over again
/// as soon as it can be downloaded.
async fn installed_vendor(vendor: Vendor, major: u32, runtimes_dir: &Path) -> Option<JavaRuntime> {
    let component = vendor.component(major);
    let home = runtimes_dir.join(&component);
    let marker = read_marker(&home).await?;
    let runtime = JavaRuntime {
        component,
        version: marker.version,
        home,
    };
    fs::try_exists(runtime.executable())
        .await
        .unwrap_or(false)
        .then_some(runtime)
}

/// The vendors' name for the architecture. Only Windows builds are used, since the other
/// platforms ship as tarballs or app bundles.
fn vendor_arch(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("windows", "x86_64") => Some("x64"),
        ("windows", "aarch64") => Some("aarch64"),
        _ => None,
    }
}

async fn fetch_archive<F>(
    client: &Client,
    url: &str,
    archive: &Archive,
    path: &Path,
    on_progress: F,
) -> Result<()>
where
    F: Fn(Progress),
{
    let mut file = File::create(path)
        .await
        .map_err(|err| Error::io(path, err))?;
    let mut hasher = Sha256::new();
    let mut body = http::get(client, url)
        .await?
        .error_for_status()?
        .bytes_stream();
    let mut done = 0;

    while let Some(chunk) = body.next().await {
        let chunk = chunk?;
        if let Some(delay) = download::throttle_delay(chunk.len()).filter(|delay| !delay.is_zero())
        {
            tokio::time::sleep(delay).await;
        }
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|err| Error::io(path, err))?;
        done += chunk.len() as u64;
        on_progress(Progress {
            bytes_done: done,
            bytes_total: archive.size,
            ..Progress::default()
        });
    }
    file.flush().await.map_err(|err| Error::io(path, err))?;

    let actual = download::hex(&hasher.finalize());
    if !actual.eq_ignore_ascii_case(&archive.sha256) {
        let _ = fs::remove_file(path).await;
        return Err(Error::ChecksumMismatch {
            url: url.to_owned(),
            expected: archive.sha256.clone(),
            actual,
        });
    }
    Ok(())
}

/// Unpacks the JRE zip into `home`, dropping the archive's top folder.
fn extract_archive(archive: &Path, home: &Path) -> Result<()> {
    if home.exists() {
        std::fs::remove_dir_all(home).map_err(|err| Error::io(home, err))?;
    }
    let file = std::fs::File::open(archive).map_err(|err| Error::io(archive, err))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|err| Error::io(archive, err.into()))?;

    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|err| Error::io(archive, err.into()))?;
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        let relative: PathBuf = name.components().skip(1).collect();
        if relative.as_os_str().is_empty() {
            continue;
        }
        let target = home.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&target).map_err(|err| Error::io(&target, err))?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|err| Error::io(parent, err))?;
        }
        let mut out = std::fs::File::create(&target).map_err(|err| Error::io(&target, err))?;
        std::io::copy(&mut entry, &mut out).map_err(|err| Error::io(&target, err))?;
    }
    Ok(())
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

    #[test]
    fn maps_vendor_architectures() {
        assert_eq!(vendor_arch("windows", "x86_64"), Some("x64"));
        assert_eq!(vendor_arch("windows", "aarch64"), Some("aarch64"));
        assert_eq!(vendor_arch("linux", "x86_64"), None);
    }

    #[test]
    fn extracts_without_top_folder() {
        use std::io::Write;

        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("jre.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        zip.add_directory("jdk8u504-b01-jre/bin/", options).unwrap();
        zip.start_file("jdk8u504-b01-jre/bin/javaw.exe", options)
            .unwrap();
        zip.write_all(b"java").unwrap();
        zip.finish().unwrap();

        let home = dir.path().join("temurin-8");
        extract_archive(&archive, &home).unwrap();
        assert_eq!(std::fs::read(home.join("bin/javaw.exe")).unwrap(), b"java");
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
