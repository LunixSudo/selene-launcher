use std::sync::Mutex;
use std::time::{Duration, Instant};

use reqwest::{Client, Response, StatusCode};
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

/// BMCLAPI, the copy of the official hosts that HMCL and PCL use. It is often reachable where
/// they are not, so it is asked once every official host has failed. Files listed with a
/// checksum are still checked against the one from the official index.
const BMCLAPI: &str = "https://bmclapi2.bangbang93.com";
const BMCLAPI_PATHS: &[(&str, &str)] = &[
    ("https://piston-meta.mojang.com/", "/"),
    ("https://launchermeta.mojang.com/", "/"),
    ("https://piston-data.mojang.com/", "/"),
    ("https://launcher.mojang.com/", "/"),
    ("https://resources.download.minecraft.net/", "/assets/"),
    ("https://libraries.minecraft.net/", "/maven/"),
    ("https://meta.fabricmc.net/", "/fabric-meta/"),
    ("https://maven.fabricmc.net/", "/maven/"),
    ("https://meta.quiltmc.org/", "/quilt-meta/"),
    ("https://maven.quiltmc.org/repository/release/", "/maven/"),
    ("https://maven.minecraftforge.net/", "/maven/"),
    ("https://files.minecraftforge.net/maven/", "/maven/"),
    ("https://files.minecraftforge.net/", "/maven/"),
    ("https://maven.neoforged.net/releases/", "/maven/"),
];

/// A host that failed is skipped for a while, so a blocked one does not cost a timeout for
/// every file of a download.
const HOST_RETRY_AFTER: Duration = Duration::from_secs(5 * 60);
static DOWN_HOSTS: Mutex<Vec<(String, Instant)>> = Mutex::new(Vec::new());

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

/// The same file as `url` on BMCLAPI, for hosts it copies.
pub fn bmclapi_of(url: &str) -> Option<String> {
    let mirrors = mirrors_of(url);
    std::iter::once(url)
        .chain(mirrors.iter().map(String::as_str))
        .find_map(|url| {
            BMCLAPI_PATHS.iter().find_map(|(host, path)| {
                url.strip_prefix(host)
                    .map(|rest| format!("{BMCLAPI}{path}{rest}"))
            })
        })
}

/// Sends a GET to `url`, moving on to its mirrors and then to BMCLAPI while the host is
/// unreachable or failing. Any answer that says something about the file itself, like 404,
/// is returned as is.
pub async fn get(client: &Client, url: &str) -> Result<Response> {
    let mut sources = vec![url.to_owned()];
    sources.extend(mirrors_of(url));
    sources.extend(bmclapi_of(url));
    // Hosts that failed lately go last, still there in case the others fail too.
    sources.sort_by_key(|source| is_down(source));

    let mut last = None;
    for source in &sources {
        let response = client.get(source).send().await;
        match &response {
            Ok(answer) if !host_failed(answer.status()) => {
                // BMCLAPI does not copy everything; a file it lacks may still be upstream.
                if !(source.starts_with(BMCLAPI) && answer.status() == StatusCode::NOT_FOUND) {
                    return Ok(response?);
                }
            }
            _ => mark_down(source),
        }
        last = Some(response);
    }
    match last {
        Some(response) => Ok(response?),
        None => Ok(client.get(url).send().await?),
    }
}

fn host_of(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    rest.split('/').next().unwrap_or(rest)
}

fn is_down(url: &str) -> bool {
    let host = host_of(url);
    let mut down = DOWN_HOSTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    down.retain(|(_, since)| since.elapsed() < HOST_RETRY_AFTER);
    down.iter().any(|(down, _)| down == host)
}

fn mark_down(url: &str) {
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
    fn finds_files_on_bmclapi() {
        assert_eq!(
            bmclapi_of("https://maven.neoforged.net/releases/net/neoforged/neoforge/a.jar")
                .as_deref(),
            Some("https://bmclapi2.bangbang93.com/maven/net/neoforged/neoforge/a.jar")
        );
        assert_eq!(
            bmclapi_of("https://resources.download.minecraft.net/ab/abcd").as_deref(),
            Some("https://bmclapi2.bangbang93.com/assets/ab/abcd")
        );
        assert_eq!(
            bmclapi_of("https://maven2.fabricmc.net/net/fabricmc/a.jar").as_deref(),
            Some("https://bmclapi2.bangbang93.com/maven/net/fabricmc/a.jar")
        );
        assert_eq!(bmclapi_of("https://cdn.modrinth.com/a.jar"), None);
    }
}
