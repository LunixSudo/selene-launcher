use std::future::Future;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;
use reqwest::{Client, Request, RequestBuilder, Response, StatusCode, Url};
use serde::de::DeserializeOwned;

use crate::Result;

/// Modrinth asks API clients to identify themselves with a contactable user agent.
pub const USER_AGENT: &str = concat!(
    "LunixSudo/selene-launcher/",
    env!("CARGO_PKG_VERSION"),
    " (github.com/LunixSudo/selene-launcher)"
);

/// Hosts that serve the same files, first the main one. Only mirrors the projects run or list
/// themselves: Mojang's older domains, Fabric's fallbacks from its installer and the mirrors
/// in Forge's `mirrors-2.0.json`.
const MIRRORS: &[&[&str]] = &[
    &[
        "https://piston-meta.mojang.com/",
        "https://launchermeta.mojang.com/",
    ],
    &[
        "https://piston-data.mojang.com/",
        "https://launcher.mojang.com/",
    ],
    &[
        "https://meta.fabricmc.net/",
        "https://meta2.fabricmc.net/",
        "https://meta3.fabricmc.net/",
    ],
    &[
        "https://maven.fabricmc.net/",
        "https://maven2.fabricmc.net/",
        "https://maven3.fabricmc.net/",
    ],
    &[
        "https://maven.minecraftforge.net/",
        "https://maven.creeperhost.net/",
    ],
];

/// BMCLAPI, a public copy of the official hosts. It is often reachable where
/// they are not, so it is asked once every official host has failed. Files are checked against
/// the checksums of the index they are listed in, which comes from the copy as well when the
/// official one cannot be had.
const BMCLAPI: &str = "https://bmclapi2.bangbang93.com/";
/// MCIM, a public copy of the Modrinth and CurseForge APIs and files. Like
/// BMCLAPI it is only asked when the official site fails. Mod files are checked against the
/// catalog's checksums, which come from the copy too when the catalog itself is down.
const MCIM: &str = "https://mod.mcimirror.top/";
const FALLBACKS: &[(&str, &str)] = &[
    ("https://piston-meta.mojang.com/", BMCLAPI),
    ("https://launchermeta.mojang.com/", BMCLAPI),
    ("https://piston-data.mojang.com/", BMCLAPI),
    ("https://launcher.mojang.com/", BMCLAPI),
    (
        "https://resources.download.minecraft.net/",
        "https://bmclapi2.bangbang93.com/assets/",
    ),
    (
        "https://libraries.minecraft.net/",
        "https://bmclapi2.bangbang93.com/maven/",
    ),
    (
        "https://meta.fabricmc.net/",
        "https://bmclapi2.bangbang93.com/fabric-meta/",
    ),
    (
        "https://maven.fabricmc.net/",
        "https://bmclapi2.bangbang93.com/maven/",
    ),
    (
        "https://maven.minecraftforge.net/",
        "https://bmclapi2.bangbang93.com/maven/",
    ),
    (
        "https://files.minecraftforge.net/maven/",
        "https://bmclapi2.bangbang93.com/maven/",
    ),
    (
        "https://files.minecraftforge.net/",
        "https://bmclapi2.bangbang93.com/maven/",
    ),
    (
        "https://maven.neoforged.net/releases/",
        "https://bmclapi2.bangbang93.com/maven/",
    ),
    (
        "https://api.modrinth.com/v2/",
        "https://mod.mcimirror.top/modrinth/v2/",
    ),
    ("https://cdn.modrinth.com/", MCIM),
    (
        "https://api.curseforge.com/v1/",
        "https://mod.mcimirror.top/curseforge/v1/",
    ),
    ("https://edge.forgecdn.net/", MCIM),
    ("https://mediafilez.forgecdn.net/", MCIM),
];

/// Headers that stay with the official API and are taken off its copy.
const CREDENTIAL_HEADERS: &[&str] = &[
    "x-api-key",
    "authorization",
    "cookie",
    "proxy-authorization",
];

/// A host that failed is skipped for a while, so a blocked one does not cost a timeout for
/// every file of a download.
const HOST_RETRY_AFTER: Duration = Duration::from_secs(5 * 60);
static DOWN_HOSTS: Mutex<Vec<(String, Instant)>> = Mutex::new(Vec::new());

/// Limits the requests made inside a scope to one kind of source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Only {
    Official,
    Fallbacks,
}

tokio::task_local! {
    static ONLY: Only;
}

/// Runs `future` without BMCLAPI and MCIM, for files that have a better second source.
pub async fn official_only<F: Future>(future: F) -> F::Output {
    ONLY.scope(Only::Official, future).await
}

/// Runs `future` against BMCLAPI and MCIM alone, once the official hosts have failed.
pub async fn fallbacks_only<F: Future>(future: F) -> F::Output {
    ONLY.scope(Only::Fallbacks, future).await
}

fn only() -> Option<Only> {
    ONLY.try_with(|only| *only).ok()
}

pub fn client() -> Result<Client> {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .build()?;
    Ok(client)
}

/// The same file as `url` on each official mirror of its host.
pub fn mirrors_of(url: &str) -> Vec<String> {
    let Some((hosts, path)) = MIRRORS.iter().find_map(|hosts| {
        hosts
            .iter()
            .find_map(|host| url.strip_prefix(host))
            .map(|path| (hosts, path))
    }) else {
        return Vec::new();
    };
    hosts
        .iter()
        .filter(|host| !url.starts_with(**host))
        .map(|host| format!("{host}{path}"))
        .collect()
}

/// The same file as `url` on BMCLAPI or MCIM, for hosts they copy.
pub fn fallback_of(url: &str) -> Option<String> {
    let mirrors = mirrors_of(url);
    std::iter::once(url)
        .chain(mirrors.iter().map(String::as_str))
        .find_map(|url| {
            FALLBACKS.iter().find_map(|(official, copy)| {
                url.strip_prefix(official)
                    .map(|rest| format!("{copy}{rest}"))
            })
        })
}

/// `path` on BMCLAPI, for its own APIs that have no official counterpart to map from.
pub fn bmclapi(path: &str) -> Option<String> {
    (only() != Some(Only::Official)).then(|| format!("{BMCLAPI}{path}"))
}

fn is_fallback(url: &str) -> bool {
    url.starts_with(BMCLAPI) || url.starts_with(MCIM)
}

/// Sends a GET to `url`, moving on to its mirrors and then to BMCLAPI while the host is
/// unreachable or failing. Any answer that says something about the file itself, like 404,
/// is returned as is.
pub async fn get(client: &Client, url: &str) -> Result<Response> {
    let mut sources = vec![url.to_owned()];
    sources.extend(mirrors_of(url));
    match (only(), fallback_of(url)) {
        (Some(Only::Official), _) => {}
        (Some(Only::Fallbacks), Some(copy)) => sources = vec![copy],
        (_, copy) => sources.extend(copy),
    }
    // Hosts that failed lately go last, still there in case the others fail too.
    sources.sort_by_key(|source| is_down(source));

    let sources = sources
        .into_iter()
        .map(|source| (source.clone(), source))
        .collect();
    Ok(race(sources, |url| client.get(url).send()).await?)
}

/// Sends an API request, and the same request to the API's copy when the API cannot be
/// reached. The copy never gets the CurseForge key or any other credential.
pub async fn send(client: &Client, request: RequestBuilder) -> Result<Response> {
    let request = request.build()?;
    let copy = if only() == Some(Only::Official) {
        None
    } else {
        copy_of(&request)
    };
    let Some(copy) = copy else {
        return Ok(client.execute(request).await?);
    };
    if only() == Some(Only::Fallbacks) {
        return Ok(client.execute(copy).await?);
    }
    let mut requests = [request, copy];
    // Hosts that failed lately go last, still there in case the others fail too.
    requests.sort_by_key(|request| is_down(request.url().as_str()));

    let requests = requests
        .into_iter()
        .map(|request| (request.url().to_string(), request))
        .collect();
    Ok(race(requests, |request| client.execute(request)).await?)
}

/// The same request to the API's copy, without the credentials meant for the API itself.
fn copy_of(request: &Request) -> Option<Request> {
    let url = Url::parse(&fallback_of(request.url().as_str())?).ok()?;
    let mut copy = request.try_clone()?;
    *copy.url_mut() = url;
    let headers = copy.headers_mut();
    for name in CREDENTIAL_HEADERS {
        headers.remove(*name);
    }
    Some(copy)
}

/// How long a source may keep us waiting for an answer before the next one is asked too.
/// Blocked hosts in some countries hang instead of refusing, and waiting out each of them in
/// turn is what made lists load forever.
const HEDGE_AFTER: Duration = Duration::from_millis(1500);

/// Sends to the sources in order, asking the next one as well whenever the ones asked so far
/// stay silent for [`HEDGE_AFTER`] or fail. A source that failed lately is only asked once
/// the others have failed, not just for being slow. The first good answer wins and the
/// requests still out are dropped. Fails with the last answer if no source gives a good one;
/// `sources` must not be empty.
async fn race<T, F, Fut>(sources: Vec<(String, T)>, send: F) -> reqwest::Result<Response>
where
    F: Fn(T) -> Fut,
    Fut: Future<Output = reqwest::Result<Response>>,
{
    let ask = |(url, item): (String, T)| {
        let request = send(item);
        async move { (url, request.await) }
    };
    let mut waiting = sources.into_iter().peekable();
    let mut asked = FuturesUnordered::new();
    let mut last = None;
    loop {
        if asked.is_empty() {
            match waiting.next() {
                Some(source) => asked.push(ask(source)),
                None => break,
            }
        }
        tokio::select! {
            Some((url, response)) = asked.next() => {
                match &response {
                    Ok(answer) if !host_failed(answer.status()) => {
                        // The copies lack some files that may still be upstream.
                        if !(is_fallback(&url) && answer.status() == StatusCode::NOT_FOUND) {
                            return response;
                        }
                    }
                    Ok(answer) => {
                        log::warn!("{} answered {}", host_of(&url), answer.status());
                        mark_down(&url);
                    }
                    Err(err) => {
                        log::warn!("{} did not answer: {}", host_of(&url), failure(err));
                        mark_down(&url);
                    }
                }
                last = Some(response);
            }
            () = tokio::time::sleep(HEDGE_AFTER), if waiting.peek().is_some_and(|(url, _)| !is_down(url)) => {
                if let Some(source) = waiting.next() {
                    asked.push(ask(source));
                }
            }
        }
    }
    last.expect("at least one source is asked")
}

/// What went wrong, down to the cause the system gave, such as a refused certificate.
fn failure(err: &reqwest::Error) -> String {
    let kind = if err.is_timeout() {
        "timed out"
    } else if err.is_connect() {
        "could not connect"
    } else {
        "the request failed"
    };
    let mut cause = std::error::Error::source(err);
    while let Some(deeper) = cause.and_then(std::error::Error::source) {
        cause = Some(deeper);
    }
    match cause {
        Some(cause) => format!("{kind} ({cause})"),
        None => kind.to_owned(),
    }
}

fn host_of(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    rest.split('/').next().unwrap_or(rest)
}

/// Whether the host of `url` failed in the last few minutes.
pub fn is_down(url: &str) -> bool {
    let host = host_of(url);
    let mut down = DOWN_HOSTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    down.retain(|(_, since)| since.elapsed() < HOST_RETRY_AFTER);
    down.iter().any(|(down, _)| down == host)
}

/// Has the host of `url` skipped for a few minutes, as after a failed request.
pub fn mark_down(url: &str) {
    let host = host_of(url).to_owned();
    let mut down = DOWN_HOSTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    down.retain(|(down, _)| *down != host);
    down.push((host, Instant::now()));
}

pub async fn get_json<T: DeserializeOwned>(client: &Client, url: &str) -> Result<T> {
    let value = get(client, url).await?.error_for_status()?.json().await?;
    Ok(value)
}

fn host_failed(status: StatusCode) -> bool {
    status.is_server_error()
        || matches!(
            status,
            StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_official_mirrors() {
        assert_eq!(
            mirrors_of("https://maven.fabricmc.net/net/fabricmc/a.jar"),
            [
                "https://maven2.fabricmc.net/net/fabricmc/a.jar",
                "https://maven3.fabricmc.net/net/fabricmc/a.jar",
            ]
        );
        assert_eq!(
            mirrors_of("https://launcher.mojang.com/v1/objects/x/client.jar"),
            ["https://piston-data.mojang.com/v1/objects/x/client.jar"]
        );
        assert_eq!(
            mirrors_of("https://cdn.modrinth.com/a.jar"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn api_copies_get_no_credentials() {
        let client = Client::new();
        let request = client
            .get("https://api.curseforge.com/v1/mods/search")
            .header("x-api-key", "secret")
            .bearer_auth("token")
            .header("accept", "application/json")
            .build()
            .unwrap();

        let copy = copy_of(&request).unwrap();

        assert_eq!(
            copy.url().as_str(),
            "https://mod.mcimirror.top/curseforge/v1/mods/search"
        );
        assert!(copy.headers().get("x-api-key").is_none());
        assert!(copy.headers().get("authorization").is_none());
        assert_eq!(copy.headers()["accept"], "application/json");
    }

    #[test]
    fn finds_files_on_fallbacks() {
        assert_eq!(
            fallback_of("https://maven.neoforged.net/releases/net/neoforged/neoforge/a.jar")
                .as_deref(),
            Some("https://bmclapi2.bangbang93.com/maven/net/neoforged/neoforge/a.jar")
        );
        assert_eq!(
            fallback_of("https://resources.download.minecraft.net/ab/abcd").as_deref(),
            Some("https://bmclapi2.bangbang93.com/assets/ab/abcd")
        );
        assert_eq!(
            fallback_of("https://maven2.fabricmc.net/net/fabricmc/a.jar").as_deref(),
            Some("https://bmclapi2.bangbang93.com/maven/net/fabricmc/a.jar")
        );
        assert_eq!(
            fallback_of("https://api.modrinth.com/v2/project/abc").as_deref(),
            Some("https://mod.mcimirror.top/modrinth/v2/project/abc")
        );
        assert_eq!(fallback_of("https://example.com/a.jar"), None);
    }

    fn answer(status: u16) -> Response {
        http::Response::builder()
            .status(status)
            .body(Vec::<u8>::new())
            .unwrap()
            .into()
    }

    fn sources(urls: &[&'static str]) -> Vec<(String, &'static str)> {
        urls.iter().map(|url| ((*url).to_owned(), *url)).collect()
    }

    #[tokio::test(start_paused = true)]
    async fn asks_the_next_source_while_one_hangs() {
        let started = tokio::time::Instant::now();
        let response = race(
            sources(&["https://hangs.test/a", "https://quick.test/a"]),
            |url| async move {
                if url.contains("hangs") {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
                Ok(answer(200))
            },
        )
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[tokio::test(start_paused = true)]
    async fn moves_on_from_failing_hosts() {
        let response = race(
            sources(&["https://broken.test/a", "https://working.test/a"]),
            |url| async move { Ok(answer(if url.contains("broken") { 503 } else { 200 })) },
        )
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(is_down("https://broken.test/b"));
        assert!(!is_down("https://working.test/b"));
    }

    #[tokio::test(start_paused = true)]
    async fn trusts_a_missing_file_only_upstream() {
        let fallback_missing = race(
            vec![
                (format!("{BMCLAPI}a"), 404),
                ("https://upstream.test/a".to_owned(), 200),
            ],
            |status| async move { Ok(answer(status)) },
        )
        .await
        .unwrap();
        assert_eq!(fallback_missing.status(), StatusCode::OK);

        let upstream_missing = race(
            vec![
                ("https://upstream.test/a".to_owned(), 404),
                (format!("{BMCLAPI}a"), 200),
            ],
            |status| async move { Ok(answer(status)) },
        )
        .await
        .unwrap();
        assert_eq!(upstream_missing.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test(start_paused = true)]
    async fn returns_the_last_failure_when_all_fail() {
        let response = race(
            sources(&["https://a.test/", "https://b.test/"]),
            |_| async { Ok(answer(500)) },
        )
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
