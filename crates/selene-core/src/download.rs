use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use futures_util::{StreamExt, TryStreamExt, stream};
use reqwest::Client;
use sha1::{Digest, Sha1};
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{Error, Result};

const MAX_ATTEMPTS: u32 = 3;
const READ_BUFFER_SIZE: usize = 64 * 1024;

/// User limits shared by every download in the process. Zero means no limit.
static MAX_PARALLEL: AtomicUsize = AtomicUsize::new(0);
static BYTES_PER_SECOND: AtomicU64 = AtomicU64::new(0);
/// When the speed budget is free again; downloads queue behind each other on it.
static BUDGET_FREE_AT: Mutex<Option<Instant>> = Mutex::new(None);

/// Caps how many files download at once and how fast, for slow or metered connections.
pub fn set_limits(max_parallel: Option<usize>, bytes_per_second: Option<u64>) {
    MAX_PARALLEL.store(max_parallel.unwrap_or(0), Ordering::Relaxed);
    BYTES_PER_SECOND.store(bytes_per_second.unwrap_or(0), Ordering::Relaxed);
}

/// How long to wait before using `bytes` more of the speed budget.
fn throttle_delay(bytes: usize) -> Option<Duration> {
    let rate = BYTES_PER_SECOND.load(Ordering::Relaxed);
    if rate == 0 {
        return None;
    }
    let bytes = u64::try_from(bytes).unwrap_or(u64::MAX);
    let cost = Duration::from_nanos(bytes.saturating_mul(1_000_000_000) / rate);
    let mut free_at = BUDGET_FREE_AT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let now = Instant::now();
    let start = free_at.filter(|&at| at > now).unwrap_or(now);
    *free_at = Some(start + cost);
    Some(start - now)
}

#[derive(Debug, Clone)]
pub struct Download {
    pub url: String,
    pub path: PathBuf,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Progress {
    pub bytes_done: u64,
    pub bytes_total: u64,
}

/// Downloads every file that is missing or fails its checksum.
///
/// Files already on disk with the right SHA-1 are skipped, so calling this again after an
/// interrupted run only fetches what is left.
pub async fn download_all<F>(
    client: &Client,
    downloads: Vec<Download>,
    concurrency: usize,
    on_progress: F,
) -> Result<()>
where
    F: Fn(Progress) + Sync,
{
    let bytes_total = downloads.iter().map(|download| download.size).sum();
    let bytes_done = &AtomicU64::new(0);
    let on_progress = &on_progress;

    stream::iter(downloads)
        .map(|download| async move {
            if !is_valid(&download.path, &download.sha1).await {
                fetch_with_retry(client, &download).await?;
            }
            let done = bytes_done.fetch_add(download.size, Ordering::Relaxed) + download.size;
            on_progress(Progress {
                bytes_done: done,
                bytes_total,
            });
            Ok::<_, Error>(())
        })
        .buffer_unordered(match MAX_PARALLEL.load(Ordering::Relaxed) {
            0 => concurrency.max(1),
            limit => concurrency.clamp(1, limit),
        })
        .try_collect()
        .await
}

pub async fn is_valid(path: &Path, expected_sha1: &str) -> bool {
    match sha1_file(path).await {
        Ok(actual) => actual.eq_ignore_ascii_case(expected_sha1),
        Err(_) => false,
    }
}

pub async fn sha1_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).await.map_err(|err| Error::io(path, err))?;
    let mut hasher = Sha1::new();
    let mut buffer = vec![0; READ_BUFFER_SIZE];

    loop {
        let read = file
            .read(&mut buffer)
            .await
            .map_err(|err| Error::io(path, err))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex(&hasher.finalize()))
}

pub fn sha1_bytes(bytes: &[u8]) -> String {
    hex(&Sha1::digest(bytes))
}

async fn fetch_with_retry(client: &Client, download: &Download) -> Result<()> {
    let mut attempt = 1;
    loop {
        match fetch(client, download).await {
            Err(Error::Http(_) | Error::ChecksumMismatch { .. }) if attempt < MAX_ATTEMPTS => {
                attempt += 1;
            }
            result => return result,
        }
    }
}

async fn fetch(client: &Client, download: &Download) -> Result<()> {
    let path = &download.path;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|err| Error::io(parent, err))?;
    }

    // Write next to the target and rename at the end, so a crash never leaves a truncated
    // file that looks complete.
    let partial = partial_path(path);
    let mut file = File::create(&partial)
        .await
        .map_err(|err| Error::io(&partial, err))?;
    let mut hasher = Sha1::new();
    let mut body = client
        .get(&download.url)
        .send()
        .await?
        .error_for_status()?
        .bytes_stream();

    while let Some(chunk) = body.next().await {
        let chunk = chunk?;
        if let Some(delay) = throttle_delay(chunk.len()).filter(|delay| !delay.is_zero()) {
            tokio::time::sleep(delay).await;
        }
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|err| Error::io(&partial, err))?;
    }
    file.flush().await.map_err(|err| Error::io(&partial, err))?;
    drop(file);

    let actual = hex(&hasher.finalize());
    if !actual.eq_ignore_ascii_case(&download.sha1) {
        let _ = fs::remove_file(&partial).await;
        return Err(Error::ChecksumMismatch {
            url: download.url.clone(),
            expected: download.sha1.clone(),
            actual,
        });
    }

    fs::rename(&partial, path)
        .await
        .map_err(|err| Error::io(path, err))
}

fn partial_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    path.with_file_name(name)
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;

    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throttling_spreads_bytes_over_time() {
        set_limits(None, Some(1000));
        let first = throttle_delay(1000).unwrap();
        let second = throttle_delay(1000).unwrap();
        set_limits(None, None);

        assert!(first < Duration::from_millis(50));
        assert!(second > Duration::from_millis(900));
        assert!(throttle_delay(1000).is_none());
    }

    const ABC_SHA1: &str = "a9993e364706816aba3e25717850c26c9cd0d89d";

    #[test]
    fn hashes_bytes() {
        assert_eq!(sha1_bytes(b"abc"), ABC_SHA1);
    }

    #[tokio::test]
    async fn validates_file_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("abc.txt");
        fs::write(&path, b"abc").await.unwrap();

        assert!(is_valid(&path, ABC_SHA1).await);
        assert!(is_valid(&path, &ABC_SHA1.to_uppercase()).await);
        assert!(!is_valid(&path, "0000000000000000000000000000000000000000").await);
        assert!(!is_valid(&dir.path().join("missing"), ABC_SHA1).await);
    }

    #[test]
    fn partial_path_keeps_extension() {
        assert_eq!(
            partial_path(Path::new("libs/lwjgl-3.3.3.jar")),
            Path::new("libs/lwjgl-3.3.3.jar.part")
        );
    }
}
