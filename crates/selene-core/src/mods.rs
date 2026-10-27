use std::path::{Path, PathBuf};

use reqwest::Client;
use serde::Serialize;
use tokio::fs;

use crate::download::{self, Download};
use crate::modrinth::Version;
use crate::{Error, Result};

const MODS_DIR: &str = "mods";
const DISABLED_SUFFIX: &str = ".disabled";
const DOWNLOAD_CONCURRENCY: usize = 8;

/// A jar in the instance's `mods` folder. Disabled mods keep their file with a `.disabled`
/// suffix, which is what every other launcher does too.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFile {
    pub file_name: String,
    pub enabled: bool,
    pub sha1: String,
    pub size: u64,
}

pub fn mods_dir(game_dir: &Path) -> PathBuf {
    game_dir.join(MODS_DIR)
}

/// Every mod jar, enabled or not, sorted by file name.
pub async fn list(game_dir: &Path) -> Result<Vec<ModFile>> {
    let dir = mods_dir(game_dir);
    let mut entries = match fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(Error::io(&dir, err)),
    };

    let mut mods = Vec::new();
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|err| Error::io(&dir, err))?
    {
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let Some(enabled) = mod_state(Path::new(&file_name)) else {
            continue;
        };
        let path = entry.path();
        let size = entry
            .metadata()
            .await
            .map_err(|err| Error::io(&path, err))?
            .len();
        mods.push(ModFile {
            sha1: download::sha1_file(&path).await?,
            file_name,
            enabled,
            size,
        });
    }
    mods.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(mods)
}

/// Downloads the primary file of each version into the `mods` folder.
pub async fn install(client: &Client, game_dir: &Path, versions: &[Version]) -> Result<()> {
    let dir = mods_dir(game_dir);
    let downloads = versions
        .iter()
        .filter_map(Version::primary_file)
        .map(|file| Download {
            url: file.url.clone(),
            path: dir.join(&file.filename),
            sha1: file.hashes.sha1.clone(),
            size: file.size,
        })
        .collect();
    download::download_all(client, downloads, DOWNLOAD_CONCURRENCY, |_| {}).await
}

/// Swaps an installed file for a newer one, keeping it disabled if it was.
pub async fn replace(
    client: &Client,
    game_dir: &Path,
    old: &ModFile,
    update: &Version,
) -> Result<()> {
    install(client, game_dir, std::slice::from_ref(update)).await?;
    let Some(file) = update.primary_file() else {
        return Ok(());
    };
    if file.filename != old.file_name.trim_end_matches(DISABLED_SUFFIX) {
        remove(game_dir, &old.file_name).await?;
    }
    if !old.enabled {
        set_enabled(game_dir, &file.filename, false).await?;
    }
    Ok(())
}

/// Renames `name.jar` to `name.jar.disabled` or back. Takes either file name.
pub async fn set_enabled(game_dir: &Path, file_name: &str, enabled: bool) -> Result<()> {
    let dir = mods_dir(game_dir);
    let base = file_name.trim_end_matches(DISABLED_SUFFIX);
    let disabled = format!("{base}{DISABLED_SUFFIX}");
    let (from, to) = if enabled {
        (dir.join(disabled), dir.join(base))
    } else {
        (dir.join(base), dir.join(disabled))
    };
    if fs::try_exists(&from).await.unwrap_or(false) {
        fs::rename(&from, &to)
            .await
            .map_err(|err| Error::io(&from, err))?;
    }
    Ok(())
}

pub async fn remove(game_dir: &Path, file_name: &str) -> Result<()> {
    let path = mods_dir(game_dir).join(safe_file_name(file_name)?);
    fs::remove_file(&path)
        .await
        .map_err(|err| Error::io(&path, err))
}

/// `Some(true)` for `*.jar`, `Some(false)` for `*.jar.disabled`, `None` for anything else.
fn mod_state(path: &Path) -> Option<bool> {
    let has_extension = |path: &Path, wanted: &str| {
        path.extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case(wanted))
    };
    if has_extension(path, "jar") {
        Some(true)
    } else if has_extension(path, "disabled") && has_extension(&path.with_extension(""), "jar") {
        Some(false)
    } else {
        None
    }
}

/// File names come from the UI; anything that could leave the `mods` folder is refused.
fn safe_file_name(file_name: &str) -> Result<&str> {
    let path = Path::new(file_name);
    if path.components().count() == 1 && path.file_name().is_some() {
        Ok(file_name)
    } else {
        Err(Error::io(
            file_name,
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a plain file name"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn lists_and_toggles_mods() {
        let dir = tempfile::tempdir().unwrap();
        let mods = mods_dir(dir.path());
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::write(mods.join("sodium.jar"), b"a").unwrap();
        std::fs::write(mods.join("lithium.jar.disabled"), b"b").unwrap();
        std::fs::write(mods.join("notes.txt"), b"c").unwrap();

        let listed = list(dir.path()).await.unwrap();
        let names: Vec<_> = listed
            .iter()
            .map(|m| (m.file_name.as_str(), m.enabled))
            .collect();
        assert_eq!(
            names,
            [("lithium.jar.disabled", false), ("sodium.jar", true)]
        );

        set_enabled(dir.path(), "sodium.jar", false).await.unwrap();
        set_enabled(dir.path(), "lithium.jar.disabled", true)
            .await
            .unwrap();
        assert!(mods.join("sodium.jar.disabled").exists());
        assert!(mods.join("lithium.jar").exists());

        remove(dir.path(), "lithium.jar").await.unwrap();
        assert!(!mods.join("lithium.jar").exists());
    }

    #[test]
    fn refuses_paths_outside_the_mods_folder() {
        assert!(safe_file_name("sodium.jar").is_ok());
        assert!(safe_file_name("../instance.json").is_err());
        assert!(safe_file_name("sub/sodium.jar").is_err());
    }
}
