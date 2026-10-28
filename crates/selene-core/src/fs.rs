use std::path::Path;

use crate::{Error, Result};

/// Writes `bytes` next to `path` and renames the result over it, so a crash or a full disk
/// never leaves a half-written settings file behind.
pub async fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|err| Error::io(parent, err))?;
    }
    let temporary = temporary_path(path);
    tokio::fs::write(&temporary, bytes)
        .await
        .map_err(|err| Error::io(&temporary, err))?;
    tokio::fs::rename(&temporary, path)
        .await
        .map_err(|err| Error::io(path, err))
}

/// The blocking version, for code outside an async runtime.
pub fn write_atomic_blocking(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| Error::io(parent, err))?;
    }
    let temporary = temporary_path(path);
    std::fs::write(&temporary, bytes).map_err(|err| Error::io(&temporary, err))?;
    std::fs::rename(&temporary, path).map_err(|err| Error::io(path, err))
}

fn temporary_path(path: &Path) -> std::path::PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn replaces_the_file_whole() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/settings.json");

        write_atomic(&path, b"{}").await.unwrap();
        write_atomic(&path, b"{\"a\":1}").await.unwrap();
        write_atomic_blocking(&path, b"{\"a\":2}").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"{\"a\":2}");
        assert!(!dir.path().join("sub/settings.json.tmp").exists());
    }
}
