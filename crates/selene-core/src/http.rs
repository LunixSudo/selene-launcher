use std::time::Duration;

use reqwest::Client;
use serde::de::DeserializeOwned;

use crate::Result;

/// Modrinth asks API clients to identify themselves with a contactable user agent.
pub const USER_AGENT: &str = concat!(
    "LunixSudo/selene-launcher/",
    env!("CARGO_PKG_VERSION"),
    " (github.com/LunixSudo/selene-launcher)"
);

pub fn client() -> Result<Client> {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .build()?;
    Ok(client)
}

pub async fn get_json<T: DeserializeOwned>(client: &Client, url: &str) -> Result<T> {
    let value = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(value)
}
