use std::io;
use std::path::Path;
use std::time::Duration;

use crate::{Error, Result};

/// How long to keep retrying a rename that an antivirus or the search indexer blocks while it
/// looks at a freshly written file.
const RENAME_ATTEMPTS: u32 = 10;
const RENAME_RETRY_DELAY: Duration = Duration::from_millis(100);

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
    rename(&temporary, path).await
}

/// The blocking version, for code outside an async runtime.
pub fn write_atomic_blocking(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| Error::io(parent, err))?;
    }
    let temporary = temporary_path(path);
    std::fs::write(&temporary, bytes).map_err(|err| Error::io(&temporary, err))?;
    let mut attempt = 1;
    loop {
        match std::fs::rename(&temporary, path) {
            Err(err) if is_transient(&err) && attempt < RENAME_ATTEMPTS => {
                attempt += 1;
                std::thread::sleep(RENAME_RETRY_DELAY);
            }
            result => return result.map_err(|err| Error::io(path, err)),
        }
    }
}

/// Moves `from` over `to`, waiting out short locks that Windows programs put on new files.
pub async fn rename(from: &Path, to: &Path) -> Result<()> {
    let mut attempt = 1;
    loop {
        match tokio::fs::rename(from, to).await {
            Err(err) if is_transient(&err) && attempt < RENAME_ATTEMPTS => {
                attempt += 1;
                tokio::time::sleep(RENAME_RETRY_DELAY).await;
            }
            result => return result.map_err(|err| Error::io(to, err)),
        }
    }
}

/// Access denied and sharing violations, which on Windows usually mean someone else has the
/// file open for a moment.
fn is_transient(err: &io::Error) -> bool {
    const ERROR_SHARING_VIOLATION: i32 = 32;
    cfg!(windows)
        && (err.kind() == io::ErrorKind::PermissionDenied
            || err.raw_os_error() == Some(ERROR_SHARING_VIOLATION))
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
