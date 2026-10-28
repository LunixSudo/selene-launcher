//! Launcher updates from GitHub Releases.

use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::{Error, Result, download};

const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/LunixSudo/selene-launcher/releases/latest";
const INSTALLER_PREFIX: &str = "SeleneLauncher-Windows-Setup-";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    pub version: String,
    pub notes: String,
    pub url: String,
    #[serde(skip)]
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    body: String,
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    /// `sha256:<hex>`, which GitHub computes on upload.
    digest: Option<String>,
}

/// The newest release if it is newer than `current` and ships a verifiable installer.
pub async fn check(client: &Client, current: &str) -> Result<Option<Update>> {
    let release: Release = client
        .get(LATEST_RELEASE_URL)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let version = release.tag_name.trim_start_matches('v');
    if !is_newer(version, current) {
        return Ok(None);
    }
    let installer = release.assets.into_iter().find_map(|asset| {
        let sha256 = asset.digest?.strip_prefix("sha256:")?.to_owned();
        let is_installer = asset.name.starts_with(INSTALLER_PREFIX)
            && Path::new(&asset.name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"));
        is_installer.then_some((asset.browser_download_url, sha256))
    });
    Ok(installer.map(|(url, sha256)| Update {
        version: version.to_owned(),
        notes: release.body,
        url,
        sha256,
    }))
}

/// Downloads the installer into `dir` and checks it against the digest GitHub published.
pub async fn download(client: &Client, update: &Update, dir: &Path) -> Result<PathBuf> {
    tokio::fs::create_dir_all(dir)
        .await
        .map_err(|err| Error::io(dir, err))?;
    let path = dir.join(format!("{INSTALLER_PREFIX}{}.exe", update.version));
    let mut file = tokio::fs::File::create(&path)
        .await
        .map_err(|err| Error::io(&path, err))?;
    let mut hasher = Sha256::new();
    let mut body = client
        .get(&update.url)
        .send()
        .await?
        .error_for_status()?
        .bytes_stream();
    while let Some(chunk) = body.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|err| Error::io(&path, err))?;
    }
    file.flush().await.map_err(|err| Error::io(&path, err))?;
    drop(file);

    let actual = download::hex(&hasher.finalize());
    if !actual.eq_ignore_ascii_case(&update.sha256) {
        let _ = tokio::fs::remove_file(&path).await;
        return Err(Error::ChecksumMismatch {
            url: update.url.clone(),
            expected: update.sha256.clone(),
            actual,
        });
    }
    Ok(path)
}

/// Compares `major.minor.patch`; anything unparsable counts as older.
fn is_newer(candidate: &str, current: &str) -> bool {
    let parse = |version: &str| -> Option<(u32, u32, u32)> {
        let mut parts = version.split(['.', '-']).map(str::parse::<u32>);
        Some((
            parts.next()?.ok()?,
            parts.next()?.ok()?,
            parts.next()?.ok()?,
        ))
    };
    matches!((parse(candidate), parse(current)), (Some(new), Some(old)) if new > old)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.2.0", "0.1.9"));
        assert!(is_newer("1.0.0", "0.10.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1", "0.0.1"));
    }
}
