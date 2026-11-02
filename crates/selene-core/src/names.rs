//! Player names: picking one for a new account and changing it later, through the same
//! Minecraft Services calls the official launcher and minecraft.net use.

use chrono::{DateTime, Duration, Utc};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::Result;
use crate::auth::{AuthError, MINECRAFT_PROFILE_URL, Profile, parse_response, send_to_services};

const ENTITLEMENTS_URL: &str = "https://api.minecraftservices.com/entitlements/mcstore";
/// Mojang allows one change every 30 days.
const NAME_CHANGE_COOLDOWN_DAYS: i64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NameStatus {
    Available,
    Taken,
    /// Too short or long, with characters other than letters, digits and `_`, or blocked.
    NotAllowed,
}

/// Checks a name the way the game does before asking the server: 3 to 16 of `a-z`, `A-Z`,
/// `0-9` and `_`.
pub fn is_valid_name(name: &str) -> bool {
    (3..=16).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[derive(Deserialize)]
struct Availability {
    status: String,
}

pub async fn name_status(client: &Client, token: &str, name: &str) -> Result<NameStatus> {
    if !is_valid_name(name) {
        return Ok(NameStatus::NotAllowed);
    }
    let url = format!("{MINECRAFT_PROFILE_URL}/name/{name}/available");
    let response = send_to_services(|| Ok(client.get(&url).bearer_auth(token))).await?;
    let availability: Availability = parse_response(response, "Minecraft Services").await?;
    Ok(match availability.status.as_str() {
        "AVAILABLE" => NameStatus::Available,
        "DUPLICATE" => NameStatus::Taken,
        _ => NameStatus::NotAllowed,
    })
}

#[derive(Deserialize)]
struct Entitlements {
    #[serde(default)]
    items: Vec<Entitlement>,
}

#[derive(Deserialize)]
struct Entitlement {
    name: String,
}

/// Whether the account owns the game, which a new one does before it has a player.
pub async fn owns_game(client: &Client, token: &str) -> Result<bool> {
    let response = send_to_services(|| Ok(client.get(ENTITLEMENTS_URL).bearer_auth(token))).await?;
    let entitlements: Entitlements = parse_response(response, "Minecraft Services").await?;
    Ok(entitlements
        .items
        .iter()
        .any(|item| matches!(item.name.as_str(), "game_minecraft" | "product_minecraft")))
}

/// Creates the player of an account that owns the game but has never played.
pub async fn create_profile(client: &Client, token: &str, name: &str) -> Result<Profile> {
    let body = json!({ "profileName": name });
    let response = send_to_services(|| {
        Ok(client
            .post(MINECRAFT_PROFILE_URL)
            .bearer_auth(token)
            .json(&body))
    })
    .await?;
    name_response(response).await
}

pub async fn change_name(client: &Client, token: &str, name: &str) -> Result<Profile> {
    let url = format!("{MINECRAFT_PROFILE_URL}/name/{name}");
    let response = send_to_services(|| Ok(client.put(&url).bearer_auth(token))).await?;
    name_response(response).await
}

async fn name_response(response: reqwest::Response) -> Result<Profile> {
    match response.status() {
        StatusCode::FORBIDDEN | StatusCode::CONFLICT => Err(AuthError::NameTaken.into()),
        StatusCode::BAD_REQUEST => Err(AuthError::NameNotAllowed.into()),
        _ => parse_response(response, "Minecraft Services").await,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NameChange {
    pub allowed: bool,
    /// When the next change becomes possible, if it is not now.
    pub next_change: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NameChangeInfo {
    changed_at: Option<DateTime<Utc>>,
    name_change_allowed: bool,
}

pub async fn name_change(client: &Client, token: &str) -> Result<NameChange> {
    let url = format!("{MINECRAFT_PROFILE_URL}/namechange");
    let response = send_to_services(|| Ok(client.get(&url).bearer_auth(token))).await?;
    let info: NameChangeInfo = parse_response(response, "Minecraft Services").await?;
    Ok(NameChange {
        allowed: info.name_change_allowed,
        next_change: info
            .changed_at
            .filter(|_| !info.name_change_allowed)
            .map(|changed| changed + Duration::days(NAME_CHANGE_COOLDOWN_DAYS)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_names_like_the_game() {
        assert!(is_valid_name("Steve_42"));
        assert!(!is_valid_name("ab"));
        assert!(!is_valid_name("seventeen_letters"));
        assert!(!is_valid_name("Стив"));
        assert!(!is_valid_name("no spaces"));
    }
}
