use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use futures_util::{StreamExt, TryStreamExt, stream};
use reqwest::{Client, StatusCode};
use sha1::{Digest, Sha1};
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Notify;

use crate::{Error, Result, http};

const MAX_ATTEMPTS: u32 = 3;
const RETRY_DELAY: Duration = Duration::from_secs(1);
const READ_BUFFER_SIZE: usize = 64 * 1024;

/// User limits shared by every download in the process. Zero means no limit.
static MAX_PARALLEL: AtomicUsize = AtomicUsize::new(0);
static BYTES_PER_SECOND: AtomicU64 = AtomicU64::new(0);
/// When the speed budget is free again; downloads queue behind each other on it.
static BUDGET_FREE_AT: Mutex<Option<Instant>> = Mutex::new(None);

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
    pub files_done: usize,
    pub files_total: usize,
}

/// Downloads the user is waiting for right now. Background ones hold back while any run.
static FOREGROUND: AtomicUsize = AtomicUsize::new(0);
static FOREGROUND_DONE: Notify = Notify::const_new();

tokio::task_local! {
    static BACKGROUND: ();
}

/// Runs `future` with its downloads held back whenever the user waits on another download,
/// like a mod update while a new instance's game is still being fetched.
pub async fn in_background<F: Future>(future: F) -> F::Output {
    BACKGROUND.scope((), future).await
}

fn is_background() -> bool {
    BACKGROUND.try_with(|()| ()).is_ok()
}

async fn wait_for_foreground() {
    loop {
        let done = FOREGROUND_DONE.notified();
        if FOREGROUND.load(Ordering::Acquire) == 0 {
            return;
        }
        done.await;
    }
}

struct Foreground;

impl Foreground {
    fn start() -> Self {
        FOREGROUND.fetch_add(1, Ordering::AcqRel);
        Self
    }
}

impl Drop for Foreground {
    fn drop(&mut self) {
        if FOREGROUND.fetch_sub(1, Ordering::AcqRel) == 1 {
            FOREGROUND_DONE.notify_waiters();
        }
    }
}

/// Caps how many files download at once and how fast, for slow or metered connections.
pub fn set_limits(max_parallel: Option<usize>, bytes_per_second: Option<u64>) {
    MAX_PARALLEL.store(max_parallel.unwrap_or(0), Ordering::Relaxed);
    BYTES_PER_SECOND.store(bytes_per_second.unwrap_or(0), Ordering::Relaxed);
}

/// How long to wait before using `bytes` more of the speed budget.
pub(crate) fn throttle_delay(bytes: usize) -> Option<Duration> {
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
    run(client, downloads, concurrency, Check::Hash, on_progress).await
}

/// Like [`download_all`], but trusts a file whose size matches without hashing it.
///
/// Only for paths that change whenever their content does, like assets named by their hash
/// and versioned libraries. Every file is written under a temporary name and renamed once
/// its checksum passed, so a file of the right size is a whole one.
pub async fn download_missing<F>(
    client: &Client,
    downloads: Vec<Download>,
    concurrency: usize,
    on_progress: F,
) -> Result<()>
where
    F: Fn(Progress) + Sync,
{
    run(client, downloads, concurrency, Check::Size, on_progress).await
}

#[derive(Clone, Copy)]
enum Check {
    Hash,
    Size,
}

async fn run<F>(
    client: &Client,
    downloads: Vec<Download>,
    concurrency: usize,
    check: Check,
    on_progress: F,
) -> Result<()>
where
    F: Fn(Progress) + Sync,
{
    // Two entries for one file would write the same `.part` at once.
    let mut seen = HashSet::new();
    let downloads: Vec<Download> = downloads
        .into_iter()
        .filter(|download| seen.insert(download.path.clone()))
        .collect();
    let bytes_total = downloads.iter().map(|download| download.size).sum();
    let files_total = downloads.len();
    let bytes_done = &AtomicU64::new(0);
    let files_done = &AtomicUsize::new(0);
    let on_progress = &on_progress;
    let background = is_background();
    let _foreground = (!background).then(Foreground::start);

    stream::iter(downloads)
        .map(|download| async move {
            let present = match check {
                Check::Size if download.size > 0 => has_size(&download.path, download.size).await,
                _ => is_valid(&download.path, &download.sha1).await,
            };
            if !present {
                if background {
                    wait_for_foreground().await;
                }
                fetch_with_retry(client, &download).await?;
            }
            let done = bytes_done.fetch_add(download.size, Ordering::Relaxed) + download.size;
            on_progress(Progress {
                bytes_done: done,
                bytes_total,
                files_done: files_done.fetch_add(1, Ordering::Relaxed) + 1,
                files_total,
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

async fn has_size(path: &Path, size: u64) -> bool {
    fs::metadata(path)
        .await
        .is_ok_and(|metadata| metadata.is_file() && metadata.len() == size)
}

/// An empty `expected_sha1` means the checksum could not be had, so any file there will do.
pub async fn is_valid(path: &Path, expected_sha1: &str) -> bool {
    if expected_sha1.is_empty() {
        return fs::metadata(path)
            .await
            .is_ok_and(|metadata| metadata.is_file());
    }
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
            Err(err) if is_retryable(&err) && attempt < MAX_ATTEMPTS => {
                tokio::time::sleep(RETRY_DELAY * attempt).await;
                attempt += 1;
            }
            Err(err) => {
                let _ = fs::remove_file(partial_path(&download.path)).await;
                return Err(err);
            }
            Ok(()) => return Ok(()),
        }
    }
}

/// Dropped connections, timeouts, server errors and damaged files can go away on a second
/// try; a missing file or a refused request will not.
fn is_retryable(err: &Error) -> bool {
    match err {
        Error::Http(err) => err.status().is_none_or(|status| {
            status.is_server_error()
                || status == StatusCode::REQUEST_TIMEOUT
                || status == StatusCode::TOO_MANY_REQUESTS
        }),
        Error::ChecksumMismatch { .. } => true,
        _ => false,
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
    let result = fetch_into(client, download, &partial).await;
    if result.is_err() {
        let _ = fs::remove_file(&partial).await;
    }
    result
}

async fn fetch_into(client: &Client, download: &Download, partial: &Path) -> Result<()> {
    let path = &download.path;
    let mut file = File::create(partial)
        .await
        .map_err(|err| Error::io(partial, err))?;
    let mut hasher = Sha1::new();
    let mut body = http::get(client, &download.url)
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
            .map_err(|err| Error::io(partial, err))?;
    }
    file.flush().await.map_err(|err| Error::io(partial, err))?;
    drop(file);

    let actual = hex(&hasher.finalize());
    if !download.sha1.is_empty() && !actual.eq_ignore_ascii_case(&download.sha1) {
        return Err(Error::ChecksumMismatch {
            url: download.url.clone(),
            expected: download.sha1.clone(),
            actual,
        });
    }

    crate::fs::rename(partial, path).await
}

/// A name of its own for every download, so two installs fetching the same file at once do not
/// write into each other.
fn partial_path(path: &Path) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}.part", NEXT.fetch_add(1, Ordering::Relaxed)));
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

    #[tokio::test]
    async fn skips_duplicate_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("abc.txt");
        fs::write(&path, b"abc").await.unwrap();
        let download = Download {
            url: "https://example.invalid/abc.txt".into(),
            path,
            sha1: ABC_SHA1.into(),
            size: 3,
        };
        let client = Client::new();
        let last = std::sync::Mutex::new(Progress::default());

        download_all(&client, vec![download.clone(), download], 4, |progress| {
            *last.lock().unwrap() = progress;
        })
        .await
        .unwrap();
        assert_eq!(last.into_inner().unwrap().bytes_total, 3);
    }

    #[tokio::test]
    async fn trusts_files_of_the_right_size() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("abc.txt");
        fs::write(&path, b"xyz").await.unwrap();
        let download = Download {
            url: "https://example.invalid/abc.txt".into(),
            path: path.clone(),
            sha1: ABC_SHA1.into(),
            size: 3,
        };
        let client = Client::new();

        download_missing(&client, vec![download.clone()], 1, |_| {})
            .await
            .unwrap();
        assert!(
            download_all(&client, vec![download], 1, |_| {})
                .await
                .is_err()
        );
    }

    #[test]
    fn partial_paths_differ() {
        let first = partial_path(Path::new("libs/lwjgl-3.3.3.jar"));
        let second = partial_path(Path::new("libs/lwjgl-3.3.3.jar"));
        assert_ne!(first, second);
        assert_eq!(first.parent(), Some(Path::new("libs")));
        let name = first.file_name().unwrap().to_string_lossy();
        assert!(name.starts_with("lwjgl-3.3.3.jar.") && name.ends_with(".part"));
    }
}
