use std::path::Path;

use reqwest::Client;
use serde::de::DeserializeOwned;

use crate::{Error, Result, fs, http};

/// Fetches JSON from `url` and keeps a copy at `path`. When the network is down, the copy is
/// used instead, so the game still starts offline once it has been installed.
/// A captive portal or a mirror's error page answers with something that is not the JSON, which
/// counts as no network too.
pub async fn get_json<T: DeserializeOwned>(client: &Client, url: &str, path: &Path) -> Result<T> {
    let fetched = match fetch(client, url).await {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(|value| (value, bytes))
            .map_err(Error::from),
        Err(err) => Err(err),
    };
    match fetched {
        Ok((value, bytes)) => {
            let _ = fs::write_atomic(path, &bytes).await;
            Ok(value)
        }
        Err(err @ (Error::Http(_) | Error::Json(_))) => match tokio::fs::read(path).await {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(_) => Err(err),
        },
        Err(err) => Err(err),
    }
}

pub(crate) async fn fetch(client: &Client, url: &str) -> Result<Vec<u8>> {
    let bytes = http::get(client, url)
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    Ok(bytes.to_vec())
}
