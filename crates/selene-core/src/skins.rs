//! Skins and capes: changing them on the Minecraft profile, and a local library of skin
//! files to switch between.

use std::path::PathBuf;

use reqwest::Client;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::{self, MINECRAFT_PROFILE_URL, Profile};
use crate::download;
use crate::{Error, Result};

const SERVICE: &str = "Minecraft Services";
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// The arm model a skin is drawn for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Variant {
    #[default]
    Classic,
    Slim,
}

impl Variant {
    fn api_name(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Slim => "slim",
        }
    }

    /// Minecraft Services writes these in upper case, the request takes lower case.
    pub fn from_api(name: &str) -> Self {
        if name.eq_ignore_ascii_case("slim") {
            Self::Slim
        } else {
            Self::Classic
        }
    }
}

/// Makes `png` the profile's skin and returns the updated profile.
pub async fn upload_skin(
    client: &Client,
    token: &str,
    variant: Variant,
    png: Vec<u8>,
) -> Result<Profile> {
    check_skin(&png)?;
    let file = Part::bytes(png)
        .file_name("skin.png")
        .mime_str("image/png")?;
    let form = Form::new()
        .text("variant", variant.api_name())
        .part("file", file);
    let response = client
        .post(format!("{MINECRAFT_PROFILE_URL}/skins"))
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await?;
    auth::parse_response(response, SERVICE).await
}

/// Goes back to the default skin the game picks for the account.
pub async fn reset_skin(client: &Client, token: &str) -> Result<Profile> {
    let response = client
        .delete(format!("{MINECRAFT_PROFILE_URL}/skins/active"))
        .bearer_auth(token)
        .send()
        .await?;
    auth::parse_response(response, SERVICE).await
}

/// Shows one of the capes the account owns, or hides the cape with `None`.
pub async fn set_cape(client: &Client, token: &str, cape_id: Option<&str>) -> Result<Profile> {
    let url = format!("{MINECRAFT_PROFILE_URL}/capes/active");
    let request = match cape_id {
        Some(id) => client.put(url).json(&json!({ "capeId": id })),
        None => client.delete(url),
    };
    let response = request.bearer_auth(token).send().await?;
    auth::parse_response(response, SERVICE).await
}

/// A skin file the user added to the launcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSkin {
    /// SHA-1 of the file, so the same skin is never stored twice.
    pub id: String,
    pub name: String,
    pub variant: Variant,
}

pub struct SkinLibrary {
    dir: PathBuf,
}

impl SkinLibrary {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn list(&self) -> Result<Vec<SavedSkin>> {
        let path = self.index_path();
        match std::fs::read(&path) {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(err) => Err(Error::io(path, err)),
        }
    }

    pub fn file(&self, id: &str) -> Result<PathBuf> {
        if id.len() == 40 && id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            Ok(self.dir.join(format!("{id}.png")))
        } else {
            Err(Error::InvalidSkin)
        }
    }

    pub fn read(&self, id: &str) -> Result<Vec<u8>> {
        let path = self.file(id)?;
        std::fs::read(&path).map_err(|err| Error::io(path, err))
    }

    /// Copies a skin file into the library. Adding the same file again only renames it.
    pub fn add(&self, png: &[u8], name: &str, variant: Variant) -> Result<SavedSkin> {
        check_skin(png)?;
        let skin = SavedSkin {
            id: download::sha1_bytes(png),
            name: name.to_owned(),
            variant,
        };
        std::fs::create_dir_all(&self.dir).map_err(|err| Error::io(&self.dir, err))?;
        let path = self.file(&skin.id)?;
        std::fs::write(&path, png).map_err(|err| Error::io(path, err))?;

        let mut skins = self.list()?;
        skins.retain(|saved| saved.id != skin.id);
        skins.insert(0, skin.clone());
        self.save(&skins)?;
        Ok(skin)
    }

    pub fn set_variant(&self, id: &str, variant: Variant) -> Result<()> {
        let mut skins = self.list()?;
        for skin in skins.iter_mut().filter(|skin| skin.id == id) {
            skin.variant = variant;
        }
        self.save(&skins)
    }

    pub fn remove(&self, id: &str) -> Result<()> {
        let path = self.file(id)?;
        let mut skins = self.list()?;
        skins.retain(|skin| skin.id != id);
        self.save(&skins)?;
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(Error::io(path, err)),
        }
    }

    fn save(&self, skins: &[SavedSkin]) -> Result<()> {
        let path = self.index_path();
        let bytes = serde_json::to_vec_pretty(skins)?;
        std::fs::write(&path, bytes).map_err(|err| Error::io(path, err))
    }

    fn index_path(&self) -> PathBuf {
        self.dir.join("skins.json")
    }
}

/// Minecraft takes 64x64 skins, and 64x32 ones from before 1.8. The size sits in the PNG
/// header, so there is no need to decode the image.
fn check_skin(png: &[u8]) -> Result<()> {
    let size = |at: usize| {
        png.get(at..at + 4)
            .map(|bytes| u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    };
    let valid = png.starts_with(PNG_SIGNATURE)
        && png.get(12..16) == Some(b"IHDR")
        && size(16) == Some(64)
        && matches!(size(20), Some(64 | 32));
    if valid {
        Ok(())
    } else {
        Err(Error::InvalidSkin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = PNG_SIGNATURE.to_vec();
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
        bytes
    }

    #[test]
    fn accepts_only_skin_sized_pngs() {
        assert!(check_skin(&png(64, 64)).is_ok());
        assert!(check_skin(&png(64, 32)).is_ok());
        assert!(check_skin(&png(128, 128)).is_err());
        assert!(check_skin(b"GIF89a").is_err());
    }

    #[test]
    fn keeps_one_copy_of_each_skin() {
        let dir = tempfile::tempdir().unwrap();
        let library = SkinLibrary::new(dir.path().to_path_buf());
        let skin = library
            .add(&png(64, 64), "Steve", Variant::Classic)
            .unwrap();
        library.add(&png(64, 32), "Old", Variant::Classic).unwrap();
        library
            .add(&png(64, 64), "Steve again", Variant::Slim)
            .unwrap();

        let skins = library.list().unwrap();
        assert_eq!(skins.len(), 2);
        assert_eq!(skins[0].name, "Steve again");

        library.set_variant(&skin.id, Variant::Classic).unwrap();
        library.remove(&skin.id).unwrap();
        assert_eq!(library.list().unwrap().len(), 1);
        assert!(library.read(&skin.id).is_err());
        assert!(library.file("../accounts").is_err());
    }
}
