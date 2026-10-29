//! The Modrinth modpack format: a zip with `modrinth.index.json` listing files to download,
//! plus `overrides/` with files copied as they are.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::download::{self, Download};
use crate::loader::{Loader, LoaderVersion};
use crate::mods::ContentKind;
use crate::{Error, Result, modrinth, mods};

const INDEX_FILE: &str = "modrinth.index.json";
pub(crate) const OVERRIDES: &str = "overrides";
const CLIENT_OVERRIDES: &str = "client-overrides";
const DOWNLOAD_CONCURRENCY: usize = 8;
/// Hosts the format allows in `downloads`; anything else has to travel in `overrides`.
const ALLOWED_HOSTS: &[&str] = &[
    "cdn.modrinth.com",
    "github.com",
    "raw.githubusercontent.com",
    "gitlab.com",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackIndex {
    pub format_version: u32,
    pub game: String,
    pub version_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub files: Vec<PackFile>,
    pub dependencies: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackFile {
    pub path: String,
    pub hashes: PackHashes,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<PackEnv>,
    pub downloads: Vec<String>,
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackHashes {
    pub sha1: String,
    pub sha512: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackEnv {
    pub client: String,
    pub server: String,
}

#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub name: String,
    pub game_version: String,
    pub loader: Option<LoaderVersion>,
    /// Folders inside the game directory to copy into the pack, such as `config`.
    pub folders: Vec<String>,
}

impl PackIndex {
    pub fn game_version(&self) -> Result<&str> {
        self.dependencies
            .get("minecraft")
            .map(String::as_str)
            .ok_or_else(|| Error::InvalidPack("no minecraft version".into()))
    }

    /// The pack's mod loader. A known loader wins over keys this launcher does not know.
    pub fn loader(&self) -> Result<Option<LoaderVersion>> {
        const LOADERS: [(&str, Loader); 4] = [
            ("fabric-loader", Loader::Fabric),
            ("quilt-loader", Loader::Quilt),
            ("forge", Loader::Forge),
            ("neoforge", Loader::NeoForge),
        ];
        if let Some((loader, version)) = LOADERS
            .iter()
            .find_map(|(name, loader)| Some((*loader, self.dependencies.get(*name)?)))
        {
            return Ok(Some(LoaderVersion {
                loader,
                version: version.clone(),
            }));
        }
        match self.dependencies.keys().find(|name| *name != "minecraft") {
            Some(other) => Err(Error::UnsupportedLoader(other.clone())),
            None => Ok(None),
        }
    }
}

pub fn read_index(pack: &Path) -> Result<PackIndex> {
    let mut archive = open(pack)?;
    let mut entry = archive
        .by_name(INDEX_FILE)
        .map_err(|_| Error::InvalidPack(format!("{INDEX_FILE} is missing")))?;
    let mut json = String::new();
    entry
        .read_to_string(&mut json)
        .map_err(|err| Error::io(pack, err))?;
    let index: PackIndex = serde_json::from_str(&json)?;
    if index.game != "minecraft" {
        return Err(Error::InvalidPack(format!("made for {}", index.game)));
    }
    Ok(index)
}

/// Downloads the pack's files and unpacks its overrides into `game_dir`.
pub async fn install(
    client: &Client,
    pack: &Path,
    index: &PackIndex,
    game_dir: &Path,
) -> Result<()> {
    let downloads = index
        .files
        .iter()
        .filter(|file| {
            file.env
                .as_ref()
                .is_none_or(|env| env.client != "unsupported")
        })
        .map(|file| {
            // The format allows only these hosts, so a pack cannot send the launcher anywhere.
            let url = file
                .downloads
                .iter()
                .find(|url| is_allowed_host(url))
                .ok_or_else(|| {
                    Error::InvalidPack(format!("{} has no allowed download", file.path))
                })?;
            Ok(Download {
                url: url.clone(),
                path: game_dir.join(safe_relative_path(&file.path)?),
                sha1: file.hashes.sha1.clone(),
                size: file.file_size,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    download::download_all(client, downloads, DOWNLOAD_CONCURRENCY, |_| {}).await?;

    let pack = pack.to_owned();
    let game_dir = game_dir.to_owned();
    tokio::task::spawn_blocking(move || {
        extract_overrides(&pack, &game_dir, &[OVERRIDES, CLIENT_OVERRIDES])
    })
    .await
    .map_err(|err| Error::io(OVERRIDES, std::io::Error::other(err)))?
}

/// Writes a pack of the instance. Mods Modrinth knows are linked; everything else, including
/// mods from other sites, is copied into the pack so it always opens complete.
pub async fn export(
    client: &Client,
    game_dir: &Path,
    options: &ExportOptions,
    out: &Path,
) -> Result<()> {
    let enabled: Vec<mods::ModFile> = mods::list(game_dir, ContentKind::Mod)
        .await?
        .into_iter()
        .filter(|file| file.enabled)
        .collect();
    let hashes: Vec<String> = enabled.iter().map(|file| file.sha1.clone()).collect();
    let known = modrinth::versions_by_hash(client, &hashes).await?;

    let mut files = Vec::new();
    let mut copied = Vec::new();
    for file in enabled {
        let linked = known.get(&file.sha1).and_then(|version| {
            let remote = version
                .files
                .iter()
                .find(|remote| remote.hashes.sha1 == file.sha1)?;
            let sha512 = remote.hashes.sha512.clone()?;
            is_allowed_host(&remote.url).then(|| PackFile {
                path: format!("mods/{}", file.file_name),
                hashes: PackHashes {
                    sha1: file.sha1.clone(),
                    sha512,
                },
                env: None,
                downloads: vec![remote.url.clone()],
                file_size: file.size,
            })
        });
        match linked {
            Some(entry) => files.push(entry),
            None => copied.push(PathBuf::from("mods").join(&file.file_name)),
        }
    }

    let mut dependencies = HashMap::from([("minecraft".to_owned(), options.game_version.clone())]);
    if let Some(loader) = &options.loader {
        let key = match loader.loader {
            Loader::Fabric => "fabric-loader",
            Loader::Quilt => "quilt-loader",
            Loader::Forge => "forge",
            Loader::NeoForge => "neoforge",
        };
        dependencies.insert(key.to_owned(), loader.version.clone());
    }
    let index = PackIndex {
        format_version: 1,
        game: "minecraft".into(),
        version_id: "1.0.0".into(),
        name: options.name.clone(),
        summary: None,
        files,
        dependencies,
    };

    let index = serde_json::to_vec_pretty(&index)?;
    write_pack(out, INDEX_FILE, index, game_dir, copied, &options.folders).await
}

/// Writes a pack archive: the index file, then `copied` files and whole `folders` from the
/// game folder under `overrides/`.
pub(crate) async fn write_pack(
    out: &Path,
    index_name: &'static str,
    index: Vec<u8>,
    game_dir: &Path,
    copied: Vec<PathBuf>,
    folders: &[String],
) -> Result<()> {
    let out = out.to_owned();
    let game_dir = game_dir.to_owned();
    let folders = folders.to_vec();
    tokio::task::spawn_blocking(move || {
        write_archive(&out, index_name, &index, &game_dir, copied, &folders)
    })
    .await
    .map_err(|err| Error::io(OVERRIDES, std::io::Error::other(err)))?
}

fn write_archive(
    out: &Path,
    index_name: &str,
    index: &[u8],
    game_dir: &Path,
    copied: Vec<PathBuf>,
    folders: &[String],
) -> Result<()> {
    let file = File::create(out).map_err(|err| Error::io(out, err))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let zip_err = |err: zip::result::ZipError| Error::io(out, err.into());

    zip.start_file(index_name, options).map_err(zip_err)?;
    zip.write_all(index).map_err(|err| Error::io(out, err))?;

    let mut entries = copied;
    for folder in folders {
        collect_files(
            &game_dir.join(safe_relative_path(folder)?),
            game_dir,
            &mut entries,
        )?;
    }
    for relative in entries {
        let source = game_dir.join(&relative);
        let name = format!("{OVERRIDES}/{}", zip_path(&relative));
        zip.start_file(name, options).map_err(zip_err)?;
        let mut input = File::open(&source).map_err(|err| Error::io(&source, err))?;
        std::io::copy(&mut input, &mut zip).map_err(|err| Error::io(&source, err))?;
    }
    zip.finish().map_err(zip_err)?;
    Ok(())
}

fn collect_files(dir: &Path, root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(Error::io(dir, err)),
    };
    for entry in entries {
        let path = entry.map_err(|err| Error::io(dir, err))?.path();
        if path.is_dir() {
            collect_files(&path, root, out)?;
        } else if let Ok(relative) = path.strip_prefix(root) {
            out.push(relative.to_owned());
        }
    }
    Ok(())
}

/// Unpacks every file under the `folders` of the pack into the game folder. A later folder wins
/// over an earlier one, whatever order the archive stores them in.
pub(crate) fn extract_overrides(pack: &Path, game_dir: &Path, folders: &[&str]) -> Result<()> {
    let mut archive = open(pack)?;
    for folder in folders {
        let prefix = format!("{folder}/");
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|err| Error::io(pack, err.into()))?;
            let name = entry
                .name()
                .map_err(|err| Error::io(pack, err.into()))?
                .into_owned();
            let Some(relative) = name.strip_prefix(&prefix) else {
                continue;
            };
            if entry.is_dir() || relative.is_empty() {
                continue;
            }
            let target = game_dir.join(safe_relative_path(relative)?);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|err| Error::io(parent, err))?;
            }
            let mut out = File::create(&target).map_err(|err| Error::io(&target, err))?;
            std::io::copy(&mut entry, &mut out).map_err(|err| Error::io(&target, err))?;
        }
    }
    Ok(())
}

pub(crate) fn open(pack: &Path) -> Result<ZipArchive<File>> {
    let file = File::open(pack).map_err(|err| Error::io(pack, err))?;
    ZipArchive::new(file).map_err(|_| Error::InvalidPack("not a zip archive".into()))
}

/// Paths in a pack come from strangers: only plain relative paths inside the game folder.
fn safe_relative_path(path: &str) -> Result<PathBuf> {
    let path = Path::new(path);
    let plain = path
        .components()
        .all(|component| matches!(component, Component::Normal(_)));
    if plain && !path.as_os_str().is_empty() {
        Ok(path.to_owned())
    } else {
        Err(Error::InvalidPack(format!(
            "unsafe path {}",
            path.display()
        )))
    }
}

fn zip_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn is_allowed_host(url: &str) -> bool {
    url::Url::parse(url).is_ok_and(|url| {
        url.scheme() == "https"
            && url
                .host_str()
                .is_some_and(|host| ALLOWED_HOSTS.contains(&host))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_paths_leaving_the_game_folder() {
        assert!(safe_relative_path("mods/sodium.jar").is_ok());
        assert!(safe_relative_path("../evil.jar").is_err());
        assert!(safe_relative_path("/etc/passwd").is_err());
        assert!(safe_relative_path("mods/../../evil").is_err());
        assert!(safe_relative_path("").is_err());
    }

    #[test]
    fn only_allowed_hosts_are_linked() {
        assert!(is_allowed_host("https://cdn.modrinth.com/data/a/b.jar"));
        assert!(!is_allowed_host(
            "https://edge.forgecdn.net/files/1/2/a.jar"
        ));
        assert!(!is_allowed_host("http://cdn.modrinth.com/data/a/b.jar"));
    }

    #[test]
    fn reads_loader_from_dependencies() {
        let mut index = PackIndex {
            format_version: 1,
            game: "minecraft".into(),
            version_id: "1".into(),
            name: "Pack".into(),
            summary: None,
            files: Vec::new(),
            dependencies: HashMap::from([
                ("minecraft".into(), "1.21.10".into()),
                ("fabric-loader".into(), "0.20.1".into()),
            ]),
        };

        assert_eq!(index.game_version().unwrap(), "1.21.10");
        assert_eq!(
            index.loader().unwrap(),
            Some(LoaderVersion {
                loader: Loader::Fabric,
                version: "0.20.1".into()
            })
        );

        index.dependencies.remove("fabric-loader");
        index
            .dependencies
            .insert("neoforge".into(), "21.10.64".into());
        assert_eq!(index.loader().unwrap().unwrap().loader, Loader::NeoForge);

        index.dependencies.remove("neoforge");
        index
            .dependencies
            .insert("liteloader".into(), "1.12.2".into());
        assert!(matches!(index.loader(), Err(Error::UnsupportedLoader(_))));
    }

    #[tokio::test]
    async fn round_trips_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("game");
        std::fs::create_dir_all(game.join("config/sub")).unwrap();
        std::fs::write(game.join("config/sub/a.toml"), b"x = 1").unwrap();
        let index = PackIndex {
            format_version: 1,
            game: "minecraft".into(),
            version_id: "1".into(),
            name: "Pack".into(),
            summary: None,
            files: Vec::new(),
            dependencies: HashMap::from([("minecraft".into(), "1.21.10".into())]),
        };
        let pack = dir.path().join("pack.mrpack");
        let bytes = serde_json::to_vec(&index).unwrap();
        write_pack(
            &pack,
            INDEX_FILE,
            bytes,
            &game,
            Vec::new(),
            &["config".into()],
        )
        .await
        .unwrap();

        let read = read_index(&pack).unwrap();
        assert_eq!(read.name, "Pack");
        let target = dir.path().join("imported");
        extract_overrides(&pack, &target, &[OVERRIDES, CLIENT_OVERRIDES]).unwrap();
        assert_eq!(
            std::fs::read_to_string(target.join("config/sub/a.toml")).unwrap(),
            "x = 1"
        );
    }
}
