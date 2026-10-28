use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::SystemTime;

use futures_util::{StreamExt, TryStreamExt, stream};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::download::{self, Download};
use crate::{Error, Result};

const DISABLED_SUFFIX: &str = ".disabled";
const DOWNLOAD_CONCURRENCY: usize = 8;
const HASH_CONCURRENCY: usize = 8;

/// What a catalog file is, which decides the folder it goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentKind {
    Mod,
    ResourcePack,
    ShaderPack,
    /// Lives inside one world's folder, so its `game_dir` is that world.
    DataPack,
}

impl ContentKind {
    pub const ALL: [Self; 4] = [
        Self::Mod,
        Self::ResourcePack,
        Self::ShaderPack,
        Self::DataPack,
    ];

    pub fn folder(self) -> &'static str {
        match self {
            Self::Mod => "mods",
            Self::ResourcePack => "resourcepacks",
            Self::ShaderPack => "shaderpacks",
            Self::DataPack => "datapacks",
        }
    }

    pub fn dir(self, game_dir: &Path) -> PathBuf {
        game_dir.join(self.folder())
    }

    /// `Some(enabled)` for files of this kind, `None` for anything else in the folder.
    fn state(self, path: &Path) -> Option<bool> {
        let has_extension = |path: &Path, wanted: &str| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case(wanted))
        };
        let extension = match self {
            Self::Mod => "jar",
            Self::ResourcePack | Self::ShaderPack | Self::DataPack => "zip",
        };
        if has_extension(path, extension) {
            Some(true)
        } else if has_extension(path, "disabled")
            && has_extension(&path.with_extension(""), extension)
        {
            Some(false)
        } else {
            None
        }
    }
}

/// Order of catalog results. Each site calls these a little differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CatalogSort {
    #[default]
    Relevance,
    Downloads,
    Newest,
    Updated,
}

/// A file in one of the instance's content folders. Disabled files keep a `.disabled`
/// suffix, which is what every other launcher does too.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModFile {
    pub file_name: String,
    pub enabled: bool,
    pub sha1: String,
    pub size: u64,
}

/// A mod file to download from Modrinth or CurseForge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFile {
    pub url: String,
    pub file_name: String,
    pub sha1: String,
    pub size: u64,
}

pub fn mods_dir(game_dir: &Path) -> PathBuf {
    ContentKind::Mod.dir(game_dir)
}

/// Every file of this kind, enabled or not, sorted by file name.
pub async fn list(game_dir: &Path, kind: ContentKind) -> Result<Vec<ModFile>> {
    let dir = kind.dir(game_dir);
    let mut entries = match fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(Error::io(&dir, err)),
    };

    let mut found = Vec::new();
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|err| Error::io(&dir, err))?
    {
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if let Some(enabled) = kind.state(Path::new(&file_name)) {
            found.push((entry.path(), file_name, enabled));
        }
    }

    let mut mods: Vec<ModFile> = stream::iter(found)
        .map(|(path, file_name, enabled)| async move {
            let metadata = fs::metadata(&path)
                .await
                .map_err(|err| Error::io(&path, err))?;
            Ok::<_, Error>(ModFile {
                sha1: cached_sha1(&path, &metadata).await?,
                file_name,
                enabled,
                size: metadata.len(),
            })
        })
        .buffer_unordered(HASH_CONCURRENCY)
        .try_collect()
        .await?;
    mods.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(mods)
}

/// The list is read every time a tab opens, and shader packs run to hundreds of megabytes,
/// so hashes are kept for as long as a file keeps its size and modification time.
async fn cached_sha1(path: &Path, metadata: &std::fs::Metadata) -> Result<String> {
    let stamp = (metadata.len(), metadata.modified().ok());
    let cached = lock_hashes()
        .get(path)
        .filter(|(known, _)| *known == stamp)
        .map(|(_, sha1)| sha1.clone());
    if let Some(sha1) = cached {
        return Ok(sha1);
    }
    let sha1 = download::sha1_file(path).await?;
    lock_hashes().insert(path.to_owned(), (stamp, sha1.clone()));
    Ok(sha1)
}

type HashCache = HashMap<PathBuf, ((u64, Option<SystemTime>), String)>;

fn lock_hashes() -> MutexGuard<'static, HashCache> {
    static HASHES: OnceLock<Mutex<HashCache>> = OnceLock::new();
    HASHES
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

pub async fn install(
    client: &Client,
    game_dir: &Path,
    kind: ContentKind,
    files: &[RemoteFile],
) -> Result<()> {
    let dir = kind.dir(game_dir);
    let downloads = files
        .iter()
        .map(|file| {
            Ok(Download {
                url: file.url.clone(),
                path: dir.join(safe_file_name(&file.file_name)?),
                sha1: file.sha1.clone(),
                size: file.size,
            })
        })
        .collect::<Result<_>>()?;
    download::download_all(client, downloads, DOWNLOAD_CONCURRENCY, |_| {}).await
}

/// Swaps an installed file for a newer one, keeping it disabled if it was.
pub async fn replace(
    client: &Client,
    game_dir: &Path,
    kind: ContentKind,
    old: &ModFile,
    update: &RemoteFile,
) -> Result<()> {
    install(client, game_dir, kind, std::slice::from_ref(update)).await?;
    if update.file_name != old.file_name.trim_end_matches(DISABLED_SUFFIX) {
        remove(game_dir, kind, &old.file_name).await?;
    }
    if !old.enabled {
        set_enabled(game_dir, kind, &update.file_name, false).await?;
    }
    Ok(())
}

/// Renames `name.jar` to `name.jar.disabled` or back. Takes either file name.
pub async fn set_enabled(
    game_dir: &Path,
    kind: ContentKind,
    file_name: &str,
    enabled: bool,
) -> Result<()> {
    let dir = kind.dir(game_dir);
    let base = safe_file_name(file_name)?.trim_end_matches(DISABLED_SUFFIX);
    let disabled = format!("{base}{DISABLED_SUFFIX}");
    let (from, to) = if enabled {
        (dir.join(disabled), dir.join(base))
    } else {
        (dir.join(base), dir.join(disabled))
    };
    if fs::try_exists(&from).await.unwrap_or(false) {
        crate::fs::rename(&from, &to).await?;
    }
    Ok(())
}

pub async fn remove(game_dir: &Path, kind: ContentKind, file_name: &str) -> Result<()> {
    let path = kind.dir(game_dir).join(safe_file_name(file_name)?);
    fs::remove_file(&path)
        .await
        .map_err(|err| Error::io(&path, err))
}

/// File names come from the UI; anything that could leave the content folder is refused.
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

        let listed = list(dir.path(), ContentKind::Mod).await.unwrap();
        let names: Vec<_> = listed
            .iter()
            .map(|m| (m.file_name.as_str(), m.enabled))
            .collect();
        assert_eq!(
            names,
            [("lithium.jar.disabled", false), ("sodium.jar", true)]
        );

        set_enabled(dir.path(), ContentKind::Mod, "sodium.jar", false)
            .await
            .unwrap();
        set_enabled(dir.path(), ContentKind::Mod, "lithium.jar.disabled", true)
            .await
            .unwrap();
        assert!(mods.join("sodium.jar.disabled").exists());
        assert!(mods.join("lithium.jar").exists());

        remove(dir.path(), ContentKind::Mod, "lithium.jar")
            .await
            .unwrap();
        assert!(!mods.join("lithium.jar").exists());
    }

    #[test]
    fn packs_are_zips() {
        assert_eq!(
            ContentKind::ResourcePack.state(Path::new("Faithful.zip")),
            Some(true)
        );
        assert_eq!(ContentKind::ShaderPack.state(Path::new("sodium.jar")), None);
        assert_eq!(
            ContentKind::ShaderPack.state(Path::new("BSL.zip.disabled")),
            Some(false)
        );
    }

    #[tokio::test]
    async fn refuses_to_rename_outside_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            set_enabled(dir.path(), ContentKind::Mod, "../x.jar", false)
                .await
                .is_err()
        );
    }

    #[test]
    fn refuses_paths_outside_the_mods_folder() {
        assert!(safe_file_name("sodium.jar").is_ok());
        assert!(safe_file_name("../instance.json").is_err());
        assert!(safe_file_name("sub/sodium.jar").is_err());
    }
}
