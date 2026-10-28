use std::path::Path;

use reqwest::Client;
use serde::de::DeserializeOwned;

use crate::{Error, Result, fs, http};

/// Fetches JSON from `url` and keeps a copy at `path`. When the network is down, the copy is
/// used instead, so the game still starts offline once it has been installed.
pub async fn get_json<T: DeserializeOwned>(client: &Client, url: &str, path: &Path) -> Result<T> {
    match fetch(client, url).await {
        Ok(bytes) => {
            let value = serde_json::from_slice(&bytes)?;
            let _ = fs::write_atomic(path, &bytes).await;
            Ok(value)
        }
        Err(err @ Error::Http(_)) => match tokio::fs::read(path).await {
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
