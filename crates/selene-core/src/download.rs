use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use futures_util::{StreamExt, TryStreamExt, stream};
use reqwest::header::CONTENT_TYPE;
use reqwest::{Client, StatusCode};
use sha1::{Digest, Sha1};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use tokio::sync::Notify;

use crate::{Error, Result, http};

const MAX_ATTEMPTS: u32 = 3;
const RETRY_DELAY: Duration = Duration::from_secs(1);
const HASH_BUFFER_SIZE: usize = 256 * 1024;
/// Checking files only reads the disk, so more of them can go at once than downloads.
const HASH_CONCURRENCY: usize = 16;

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

/// Background downloads take few connections, so a slow line still has room for the rest.
const BACKGROUND_CONCURRENCY: usize = 4;
/// Fewer still while a game is running, so they never cost the player ping or frames.
const IN_GAME_CONCURRENCY: usize = 2;
static IN_GAME: AtomicBool = AtomicBool::new(false);

/// Tells background downloads whether a game is running.
pub fn set_in_game(in_game: bool) {
    IN_GAME.store(in_game, Ordering::Relaxed);
}

/// Downloads that idle work makes way for: any instance's game or add-ons.
static BUSY: Activity = Activity::new();
/// Downloads the player started and is waiting on, like a modpack, an add-on or a start.
/// Instances getting ready in the background make way for them.
static ASKED: Activity = Activity::new();

#[derive(Clone, Copy, PartialEq, Eq)]
enum Priority {
    /// Work nobody asked for, like fetching Java ahead of time. Pauses between files while
    /// anything else downloads.
    Idle,
    /// An instance getting ready in the background. Never waits for another instance doing the
    /// same, pauses between files while the player waits on a download, and takes few
    /// connections while a game runs.
    Alongside,
}

tokio::task_local! {
    static PRIORITY: Priority;
}

/// Runs `future` with its downloads paused between files whenever an instance downloads.
pub async fn in_background<F: Future>(future: F) -> F::Output {
    PRIORITY.scope(Priority::Idle, future).await
}

/// Runs `future` next to other instances' downloads without waiting for them.
pub async fn alongside<F: Future>(future: F) -> F::Output {
    PRIORITY.scope(Priority::Alongside, future).await
}

fn priority() -> Option<Priority> {
    PRIORITY.try_with(|priority| *priority).ok()
}

struct Activity {
    running: AtomicUsize,
    done: Notify,
}

impl Activity {
    const fn new() -> Self {
        Self {
            running: AtomicUsize::new(0),
            done: Notify::const_new(),
        }
    }

    fn start(&'static self) -> ActivityGuard {
        self.running.fetch_add(1, Ordering::AcqRel);
        ActivityGuard(self)
    }

    async fn wait_until_done(&self) {
        loop {
            let done = self.done.notified();
            if self.running.load(Ordering::Acquire) == 0 {
                return;
            }
            done.await;
        }
    }
}

struct ActivityGuard(&'static Activity);

impl Drop for ActivityGuard {
    fn drop(&mut self) {
        if self.0.running.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.0.done.notify_waiters();
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
    let priority = priority();
    let _busy = (priority != Some(Priority::Idle)).then(|| BUSY.start());
    let _asked = priority.is_none().then(|| ASKED.start());

    // Files already in place are counted first, so a resumed download starts from where the
    // last one stopped instead of creeping back up between the files it still has to fetch.
    let checked: Vec<(Download, bool)> = stream::iter(downloads)
        .map(|download| async move {
            let present = match check {
                Check::Size if download.size > 0 => has_size(&download.path, download.size).await,
                _ => is_valid(&download.path, &download.sha1).await,
            };
            (download, present)
        })
        .buffer_unordered(HASH_CONCURRENCY)
        .collect()
        .await;
    let (present, missing): (Vec<_>, Vec<_>) =
        checked.into_iter().partition(|(_, present)| *present);
    let bytes_done = &AtomicU64::new(present.iter().map(|(download, _)| download.size).sum());
    let files_done = &AtomicUsize::new(present.len());
    if !present.is_empty() {
        on_progress(Progress {
            bytes_done: bytes_done.load(Ordering::Relaxed),
            bytes_total,
            files_done: present.len(),
            files_total,
        });
    }
    let on_progress = &on_progress;

    stream::iter(missing)
        .map(|(download, _)| async move {
            match priority {
                Some(Priority::Idle) => BUSY.wait_until_done().await,
                Some(Priority::Alongside) => ASKED.wait_until_done().await,
                None => {}
            }
            fetch_with_retry(client, &download).await?;
            let done = bytes_done.fetch_add(download.size, Ordering::Relaxed) + download.size;
            on_progress(Progress {
                bytes_done: done,
                bytes_total,
                files_done: files_done.fetch_add(1, Ordering::Relaxed) + 1,
                files_total,
            });
            Ok::<_, Error>(())
        })
        .buffer_unordered(parallel(concurrency, priority))
        .try_collect()
        .await
}

fn parallel(concurrency: usize, priority: Option<Priority>) -> usize {
    let in_game = IN_GAME.load(Ordering::Relaxed);
    let concurrency = match priority {
        Some(_) if in_game => concurrency.min(IN_GAME_CONCURRENCY),
        Some(Priority::Idle) => concurrency.min(BACKGROUND_CONCURRENCY),
        _ => concurrency,
    };
    match MAX_PARALLEL.load(Ordering::Relaxed) {
        0 => concurrency.max(1),
        limit => concurrency.clamp(1, limit),
    }
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

/// Hashes on a blocking thread with plain reads: through async file reads every chunk would
/// be its own trip to the blocking pool, which makes checking a big folder much slower.
pub async fn sha1_file(path: &Path) -> Result<String> {
    let owned = path.to_owned();
    tokio::task::spawn_blocking(move || {
        use std::io::Read;

        let mut file = std::fs::File::open(&owned).map_err(|err| Error::io(&owned, err))?;
        let mut hasher = Sha1::new();
        let mut buffer = vec![0; HASH_BUFFER_SIZE];
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|err| Error::io(&owned, err))?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        Ok(hex(&hasher.finalize()))
    })
    .await
    .map_err(|err| Error::io(path, std::io::Error::other(err)))?
}

pub fn sha1_bytes(bytes: &[u8]) -> String {
    hex(&Sha1::digest(bytes))
}

async fn fetch_with_retry(client: &Client, download: &Download) -> Result<()> {
    let mut attempt = 1;
    loop {
        match fetch(client, download).await {
            Err(err) if is_retryable(&err) && attempt < MAX_ATTEMPTS => {
                // Doubles each time, so a server asking to slow down gets the room.
                tokio::time::sleep(RETRY_DELAY * 2u32.pow(attempt - 1)).await;
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
        Error::ChecksumMismatch { .. } | Error::NotTheFile { .. } => true,
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
    let response = http::get(client, &download.url).await?.error_for_status()?;
    let source = response.url().to_string();
    let is_page = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/html"));
    if is_page && download.sha1.is_empty() {
        // Blocked or captive networks answer with a page of their own, and without a
        // checksum nothing else would notice.
        http::mark_down(&source);
        return Err(Error::NotTheFile {
            url: source,
            reason: "a web page",
        });
    }
    let mut written = 0;
    let mut body = response.bytes_stream();

    while let Some(chunk) = body.next().await {
        let chunk = chunk.inspect_err(|_| {
            // The next try goes to another source instead of the one that broke off.
            http::mark_down(&source);
        })?;
        written += chunk.len() as u64;
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
        http::mark_down(&source);
        return Err(Error::ChecksumMismatch {
            url: source,
            expected: download.sha1.clone(),
            actual,
        });
    }
    if download.sha1.is_empty() {
        let reason = if written == 0 {
            Some("nothing")
        } else if download.size > 0 && written != download.size {
            Some("a file of the wrong size")
        } else {
            None
        };
        if let Some(reason) = reason {
            http::mark_down(&source);
            return Err(Error::NotTheFile {
                url: source,
                reason,
            });
        }
        log::info!(
            "{} has no checksum, kept after checking its size",
            download.url
        );
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
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    use super::*;

    /// Serves `response` to every connection, as a host that is blocked or broken would.
    async fn serve(response: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let mut request = [0; 1024];
                let _ = stream.read(&mut request).await;
                let _ = stream.write_all(response.as_bytes()).await;
            }
        });
        format!("http://{address}/file.jar")
    }

    async fn fetch_unchecked(url: String, size: u64) -> (Result<()>, PathBuf, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.jar");
        let client = Client::builder().no_proxy().build().unwrap();
        let download = Download {
            url,
            path: path.clone(),
            sha1: String::new(),
            size,
        };
        (fetch(&client, &download).await, path, dir)
    }

    #[tokio::test]
    async fn refuses_a_web_page_without_a_checksum() {
        let url = serve(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 13\r\n\r\n<html></html>",
        )
        .await;
        let (result, path, _dir) = fetch_unchecked(url, 0).await;
        assert!(matches!(result, Err(Error::NotTheFile { .. })));
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn refuses_an_empty_or_short_file_without_a_checksum() {
        let empty = serve("HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n").await;
        let (result, path, _dir) = fetch_unchecked(empty, 0).await;
        assert!(matches!(result, Err(Error::NotTheFile { .. })));
        assert!(!path.exists());

        let short = serve("HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nabc").await;
        let (result, _, _dir) = fetch_unchecked(short.clone(), 10).await;
        assert!(matches!(result, Err(Error::NotTheFile { .. })));
        let (result, path, _dir) = fetch_unchecked(short, 3).await;
        assert!(result.is_ok());
        assert_eq!(std::fs::read(path).unwrap(), b"abc");
    }

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
