use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::SystemTime;

use futures_util::{StreamExt, TryStreamExt, stream};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::download::{self, Download, Progress};
use crate::{Error, Result};

pub const DISABLED_SUFFIX: &str = ".disabled";
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
    /// The ids the mod declares for its loader, which two copies of one mod share even
    /// when they are different files.
    #[serde(skip)]
    pub mod_ids: Vec<String>,
    #[serde(skip)]
    pub modified: Option<SystemTime>,
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

    let listed: Vec<Option<ModFile>> = stream::iter(found)
        .map(|(path, file_name, enabled)| async move {
            let read = async {
                let metadata = fs::metadata(&path)
                    .await
                    .map_err(|err| Error::io(&path, err))?;
                let sha1 = cached_sha1(&path, &metadata).await?;
                Ok::<_, Error>((metadata, sha1))
            };
            let (metadata, sha1) = match read.await {
                Ok(read) => read,
                // Renamed or removed while the folder was read, as during an update.
                Err(Error::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(None);
                }
                Err(err) => return Err(err),
            };
            let mod_ids = if kind == ContentKind::Mod {
                cached_mod_ids(&path, &sha1).await
            } else {
                Vec::new()
            };
            Ok(Some(ModFile {
                sha1,
                file_name,
                enabled,
                size: metadata.len(),
                mod_ids,
                modified: metadata.modified().ok(),
            }))
        })
        .buffer_unordered(HASH_CONCURRENCY)
        .try_collect()
        .await?;
    let mut mods: Vec<ModFile> = listed.into_iter().flatten().collect();
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

async fn cached_mod_ids(path: &Path, sha1: &str) -> Vec<String> {
    static IDS: OnceLock<Mutex<HashMap<String, Vec<String>>>> = OnceLock::new();
    let ids = IDS.get_or_init(Mutex::default);
    let lock = || ids.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(known) = lock().get(sha1) {
        return known.clone();
    }
    let path = path.to_owned();
    let found = tokio::task::spawn_blocking(move || read_mod_ids(&path))
        .await
        .unwrap_or_default();
    lock().insert(sha1.to_owned(), found.clone());
    found
}

/// The mod ids in a jar's metadata, for Fabric, Quilt, Forge, NeoForge and old Forge alike.
fn read_mod_ids(path: &Path) -> Vec<String> {
    use std::io::Read;

    let Some(mut archive) = std::fs::File::open(path)
        .ok()
        .and_then(|file| zip::ZipArchive::new(file).ok())
    else {
        return Vec::new();
    };
    let mut read = |name: &str| {
        let mut text = String::new();
        archive.by_name(name).ok()?.read_to_string(&mut text).ok()?;
        Some(text)
    };
    let mut ids = Vec::new();
    if let Some(json) = read("fabric.mod.json") {
        ids.extend(json_id(&json, &["id"]));
    }
    if let Some(json) = read("quilt.mod.json") {
        ids.extend(json_id(&json, &["quilt_loader", "id"]));
    }
    for name in ["META-INF/mods.toml", "META-INF/neoforge.mods.toml"] {
        if let Some(toml) = read(name) {
            ids.extend(toml_mod_ids(&toml));
        }
    }
    if let Some(json) = read("mcmod.info") {
        ids.extend(mcmod_ids(&json));
    }
    ids.sort();
    ids.dedup();
    ids
}

fn json_id(json: &str, path: &[&str]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let id = path.iter().try_fold(&value, |value, key| value.get(key))?;
    id.as_str().map(str::to_owned)
}

/// `modId="…"` lines from `mods.toml`. Libraries the loader brings, like `minecraft` and
/// `forge`, only show up as dependencies, which spell it `modId` too, so only the lines in a
/// `[[mods]]` table count.
fn toml_mod_ids(toml: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut in_mods = false;
    for line in toml.lines().map(str::trim) {
        if line.starts_with('[') {
            in_mods = line == "[[mods]]";
            continue;
        }
        let Some(value) = line.strip_prefix("modId").map(str::trim_start) else {
            continue;
        };
        if let (true, Some(value)) = (in_mods, value.strip_prefix('=')) {
            let id = value.trim().trim_matches(|ch| ch == '"' || ch == '\'');
            if !id.is_empty() && !id.contains('$') {
                ids.push(id.to_owned());
            }
        }
    }
    ids
}

/// `mcmod.info` is a list of mods, or an object with one under `modList`.
fn mcmod_ids(json: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let list = value.get("modList").unwrap_or(&value);
    list.as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("modid")?.as_str().map(str::to_owned))
        .collect()
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
    on_progress: impl Fn(Progress) + Sync,
) -> Result<()> {
    let files: Vec<(ContentKind, RemoteFile)> =
        files.iter().map(|file| (kind, file.clone())).collect();
    install_many(client, game_dir, &files, on_progress).await
}

/// Downloads files of any kinds into their folders, all in one batch.
pub async fn install_many(
    client: &Client,
    game_dir: &Path,
    files: &[(ContentKind, RemoteFile)],
    on_progress: impl Fn(Progress) + Sync,
) -> Result<()> {
    let downloads = files
        .iter()
        .map(|(kind, file)| {
            Ok(Download {
                url: file.url.clone(),
                path: kind.dir(game_dir).join(safe_file_name(&file.file_name)?),
                sha1: file.sha1.clone(),
                size: file.size,
            })
        })
        .collect::<Result<_>>()?;
    download::download_all(client, downloads, DOWNLOAD_CONCURRENCY, on_progress).await
}

/// Swaps an installed file for a newer one, keeping it disabled if it was.
pub async fn replace(
    client: &Client,
    game_dir: &Path,
    kind: ContentKind,
    old: &ModFile,
    update: &RemoteFile,
) -> Result<()> {
    replace_all(
        client,
        game_dir,
        kind,
        &[(old.clone(), update.clone())],
        |_| {},
    )
    .await
}

/// Swaps installed files for newer ones. Everything is downloaded first, all at once, and the
/// old files are only removed once every new one is in place, so a failed download leaves
/// the folder as it was.
pub async fn replace_all(
    client: &Client,
    game_dir: &Path,
    kind: ContentKind,
    swaps: &[(ModFile, RemoteFile)],
    on_progress: impl Fn(Progress) + Sync,
) -> Result<()> {
    let updates: Vec<RemoteFile> = swaps.iter().map(|(_, update)| update.clone()).collect();
    let dir = kind.dir(game_dir);
    let mut existing = HashSet::new();
    for update in &updates {
        if fs::try_exists(dir.join(&update.file_name))
            .await
            .unwrap_or(false)
        {
            existing.insert(update.file_name.clone());
        }
    }
    if let Err(err) = install(client, game_dir, kind, &updates, on_progress).await {
        // Only what this download brought goes; a file already there may be another add-on's.
        for (old, update) in swaps {
            if !same_file(old, update) && !existing.contains(&update.file_name) {
                let _ = remove(game_dir, kind, &update.file_name).await;
            }
        }
        return Err(err);
    }
    let new_names: HashSet<&str> = updates
        .iter()
        .map(|update| update.file_name.as_str())
        .collect();
    for (old, update) in swaps {
        // Another swap may have just put a file under this name.
        if !same_file(old, update) && !new_names.contains(old.file_name.as_str()) {
            match remove(game_dir, kind, &old.file_name).await {
                Err(Error::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {}
                result => result?,
            }
        }
        // Two copies of one mod update to the same file, which stays on if either was on.
        let switched_on = swaps
            .iter()
            .any(|(other, next)| other.enabled && next.file_name == update.file_name);
        if !switched_on {
            set_enabled(game_dir, kind, &update.file_name, false).await?;
        }
    }
    Ok(())
}

fn same_file(old: &ModFile, update: &RemoteFile) -> bool {
    update.file_name == old.file_name.trim_end_matches(DISABLED_SUFFIX)
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

    #[test]
    fn reads_mod_ids_from_forge_metadata() {
        let toml = r#"
modLoader="javafml"
[[mods]]
modId="jei"
version="${file.jarVersion}"
[[dependencies.jei]]
    modId="minecraft"
    mandatory=true
[[mods]]
    modId = 'jei_addon'
"#;
        assert_eq!(toml_mod_ids(toml), ["jei", "jei_addon"]);
        assert_eq!(
            mcmod_ids(r#"[{"modid": "optifine"}, {"modid": "extra"}]"#),
            ["optifine", "extra"]
        );
        assert_eq!(
            mcmod_ids(r#"{"modListVersion": 2, "modList": [{"modid": "ic2"}]}"#),
            ["ic2"]
        );
        assert_eq!(
            json_id(
                r#"{"quilt_loader": {"id": "qsl"}}"#,
                &["quilt_loader", "id"]
            )
            .as_deref(),
            Some("qsl")
        );
    }

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
