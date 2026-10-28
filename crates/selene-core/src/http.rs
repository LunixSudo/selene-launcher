use std::time::Duration;

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

/// Sends a GET to `url`, moving on to its mirrors while the host is unreachable or failing.
/// Any answer that says something about the file itself, like 404, is returned as is.
pub async fn get(client: &Client, url: &str) -> Result<Response> {
    let mut result = client.get(url).send().await;
    for mirror in mirrors_of(url) {
        if matches!(&result, Ok(response) if !host_failed(response.status())) {
            break;
        }
        result = client.get(&mirror).send().await;
    }
    Ok(result?)
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
}
