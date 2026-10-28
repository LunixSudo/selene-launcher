use std::path::PathBuf;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use selene_core::auth::{self, Profile};
use selene_core::skins::{self, SavedSkin, Variant};
use serde::Serialize;
use tauri::State;

use crate::error::CommandResult;
use crate::state::Launcher;

/// The skin and capes the active account has on its Minecraft profile.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinProfile {
    name: String,
    skin: Option<ActiveSkin>,
    capes: Vec<Cape>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveSkin {
    url: String,
    variant: Variant,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cape {
    id: String,
    name: String,
    url: String,
    active: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySkin {
    #[serde(flatten)]
    skin: SavedSkin,
    /// The PNG itself, small enough to hand to the 3D preview as a data URL.
    image: String,
}

#[tauri::command]
pub async fn skin_profile(launcher: State<'_, Launcher>) -> CommandResult<SkinProfile> {
    let session = launcher.accounts.session(&launcher.client).await?;
    let profile = auth::fetch_profile(&launcher.client, &session.access_token).await?;
    remember(&launcher, profile).await
}

#[tauri::command]
pub async fn apply_skin(launcher: State<'_, Launcher>, id: String) -> CommandResult<SkinProfile> {
    let saved = launcher
        .skins
        .list()?
        .into_iter()
        .find(|skin| skin.id == id)
        .ok_or(selene_core::Error::InvalidSkin)?;
    let png = launcher.skins.read(&id)?;
    let session = launcher.accounts.session(&launcher.client).await?;
    let profile =
        skins::upload_skin(&launcher.client, &session.access_token, saved.variant, png).await?;
    remember(&launcher, profile).await
}

#[tauri::command]
pub async fn reset_skin(launcher: State<'_, Launcher>) -> CommandResult<SkinProfile> {
    let session = launcher.accounts.session(&launcher.client).await?;
    let profile = skins::reset_skin(&launcher.client, &session.access_token).await?;
    remember(&launcher, profile).await
}

#[tauri::command]
pub async fn set_cape(
    launcher: State<'_, Launcher>,
    id: Option<String>,
) -> CommandResult<SkinProfile> {
    let session = launcher.accounts.session(&launcher.client).await?;
    let profile = skins::set_cape(&launcher.client, &session.access_token, id.as_deref()).await?;
    remember(&launcher, profile).await
}

#[tauri::command]
pub fn saved_skins(launcher: State<'_, Launcher>) -> CommandResult<Vec<LibrarySkin>> {
    launcher
        .skins
        .list()?
        .into_iter()
        .map(|skin| library_skin(&launcher, skin))
        .collect()
}

#[tauri::command]
pub async fn add_skin(
    launcher: State<'_, Launcher>,
    path: PathBuf,
    variant: Variant,
) -> CommandResult<LibrarySkin> {
    let name = path.file_stem().map_or_else(
        || "Skin".to_owned(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    let png = tokio::fs::read(&path).await?;
    let skin = launcher.skins.add(&png, &name, variant)?;
    library_skin(&launcher, skin)
}

#[tauri::command]
pub fn set_skin_variant(
    launcher: State<'_, Launcher>,
    id: String,
    variant: Variant,
) -> CommandResult<()> {
    Ok(launcher.skins.set_variant(&id, variant)?)
}

#[tauri::command]
pub fn remove_skin(launcher: State<'_, Launcher>, id: String) -> CommandResult<()> {
    Ok(launcher.skins.remove(&id)?)
}

async fn remember(launcher: &Launcher, profile: Profile) -> CommandResult<SkinProfile> {
    launcher.accounts.set_profile(&profile).await?;
    let skin = profile
        .skins
        .into_iter()
        .find(|skin| skin.state == "ACTIVE")
        .map(|skin| ActiveSkin {
            variant: Variant::from_api(&skin.variant),
            url: skin.url,
        });
    let capes = profile
        .capes
        .into_iter()
        .map(|cape| Cape {
            active: cape.state == "ACTIVE",
            id: cape.id,
            name: cape.alias,
            url: cape.url,
        })
        .collect();
    Ok(SkinProfile {
        name: profile.name,
        skin,
        capes,
    })
}

fn library_skin(launcher: &Launcher, skin: SavedSkin) -> CommandResult<LibrarySkin> {
    let png = launcher.skins.read(&skin.id)?;
    Ok(LibrarySkin {
        image: format!("data:image/png;base64,{}", STANDARD.encode(png)),
        skin,
    })
}
