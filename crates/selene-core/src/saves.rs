//! Worlds, the server list and screenshots inside an instance's game folder.

use std::cmp::Reverse;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use fastnbt::Value;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;

use crate::{Error, Result};

const SAVES_DIR: &str = "saves";
const BACKUPS_DIR: &str = "backups";
const SCREENSHOTS_DIR: &str = "screenshots";
const SERVERS_FILE: &str = "servers.dat";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct World {
    /// Folder name inside `saves`, which is what Quick Play expects.
    pub folder: String,
    pub name: String,
    pub last_played: Option<DateTime<Utc>>,
    pub game_mode: Option<GameMode>,
    pub icon: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GameMode {
    Survival,
    Creative,
    Adventure,
    Spectator,
    Hardcore,
}

#[derive(Debug, Deserialize)]
struct LevelDat {
    #[serde(rename = "Data")]
    data: LevelData,
}

#[derive(Debug, Deserialize)]
struct LevelData {
    #[serde(rename = "LevelName")]
    name: Option<String>,
    #[serde(rename = "LastPlayed")]
    last_played: Option<i64>,
    #[serde(rename = "GameType")]
    game_type: Option<i32>,
    #[serde(rename = "hardcore")]
    hardcore: Option<i8>,
}

pub fn saves_dir(game_dir: &Path) -> PathBuf {
    game_dir.join(SAVES_DIR)
}

/// Worlds with a readable `level.dat`, most recently played first.
pub fn worlds(game_dir: &Path) -> Result<Vec<World>> {
    let saves = saves_dir(game_dir);
    let entries = match fs::read_dir(&saves) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(Error::io(&saves, err)),
    };

    let mut worlds: Vec<World> = entries
        .flatten()
        .filter_map(|entry| read_world(&entry.path()))
        .collect();
    worlds.sort_by_key(|world| Reverse(world.last_played));
    Ok(worlds)
}

fn read_world(dir: &Path) -> Option<World> {
    let folder = dir.file_name()?.to_str()?.to_owned();
    let mut bytes = Vec::new();
    GzDecoder::new(File::open(dir.join("level.dat")).ok()?)
        .read_to_end(&mut bytes)
        .ok()?;
    let level: LevelDat = fastnbt::from_bytes(&bytes).ok()?;
    let data = level.data;

    let game_mode = match (data.hardcore, data.game_type) {
        (Some(1), _) => Some(GameMode::Hardcore),
        (_, Some(0)) => Some(GameMode::Survival),
        (_, Some(1)) => Some(GameMode::Creative),
        (_, Some(2)) => Some(GameMode::Adventure),
        (_, Some(3)) => Some(GameMode::Spectator),
        _ => None,
    };
    let icon = dir.join("icon.png");
    Some(World {
        name: data
            .name
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| folder.clone()),
        folder,
        last_played: data.last_played.and_then(DateTime::from_timestamp_millis),
        game_mode,
        icon: icon.is_file().then_some(icon),
    })
}

/// Packs a world into `backups/<folder>-<time>.zip` and returns the archive's path.
pub fn back_up_world(game_dir: &Path, folder: &str) -> Result<PathBuf> {
    let world = world_dir(game_dir, folder)?;
    let backups = game_dir.join(BACKUPS_DIR);
    fs::create_dir_all(&backups).map_err(|err| Error::io(&backups, err))?;
    let out = backups.join(format!(
        "{folder}-{}.zip",
        Utc::now().format("%Y-%m-%d_%H-%M-%S")
    ));

    let file = File::create(&out).map_err(|err| Error::io(&out, err))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut stack = vec![world.clone()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)
            .map_err(|err| Error::io(&dir, err))?
            .flatten()
        {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            // The game holds this lock while the world is open; it is never needed in a backup.
            if path.file_name().is_some_and(|name| name == "session.lock") {
                continue;
            }
            let Ok(relative) = path.strip_prefix(game_dir.join(SAVES_DIR)) else {
                continue;
            };
            let name = relative.to_string_lossy().replace('\\', "/");
            zip.start_file(name, options)
                .map_err(|err| Error::io(&out, err.into()))?;
            let bytes = fs::read(&path).map_err(|err| Error::io(&path, err))?;
            zip.write_all(&bytes).map_err(|err| Error::io(&out, err))?;
        }
    }
    zip.finish().map_err(|err| Error::io(&out, err.into()))?;
    Ok(out)
}

/// Resolves a world folder, refusing names that would point outside `saves`.
pub fn world_dir(game_dir: &Path, folder: &str) -> Result<PathBuf> {
    let valid =
        !folder.is_empty() && !folder.contains(['/', '\\']) && folder != "." && folder != "..";
    let dir = saves_dir(game_dir).join(folder);
    if valid && dir.is_dir() {
        Ok(dir)
    } else {
        Err(Error::io(
            dir,
            std::io::Error::from(std::io::ErrorKind::NotFound),
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub name: String,
    pub address: String,
    /// Base64 PNG the game saves after pinging the server.
    pub icon: Option<String>,
}

/// The multiplayer list as the game shows it.
pub fn servers(game_dir: &Path) -> Result<Vec<Server>> {
    let Some(root) = read_servers(game_dir)? else {
        return Ok(Vec::new());
    };
    let Some(Value::List(entries)) = compound(&root).and_then(|root| root.get("servers")) else {
        return Ok(Vec::new());
    };
    Ok(entries
        .iter()
        .filter_map(|entry| {
            let entry = compound(entry)?;
            let text = |key: &str| match entry.get(key) {
                Some(Value::String(value)) => Some(value.clone()),
                _ => None,
            };
            Some(Server {
                name: text("name").unwrap_or_default(),
                address: text("ip")?,
                icon: text("icon"),
            })
        })
        .collect())
}

/// Rewrites the server list, keeping fields the game stores that the launcher does not edit.
pub fn save_servers(game_dir: &Path, servers: &[Server]) -> Result<()> {
    let old: HashMap<String, HashMap<String, Value>> = read_servers(game_dir)?
        .as_ref()
        .and_then(compound)
        .and_then(|root| match root.get("servers") {
            Some(Value::List(entries)) => Some(entries.clone()),
            _ => None,
        })
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| match entry {
            Value::Compound(entry) => match entry.get("ip") {
                Some(Value::String(ip)) => Some((ip.clone(), entry)),
                _ => None,
            },
            _ => None,
        })
        .collect();

    let entries = servers
        .iter()
        .map(|server| {
            let mut entry = old.get(&server.address).cloned().unwrap_or_default();
            entry.insert("name".into(), Value::String(server.name.clone()));
            entry.insert("ip".into(), Value::String(server.address.clone()));
            Value::Compound(entry)
        })
        .collect();
    let root = Value::Compound(HashMap::from([("servers".into(), Value::List(entries))]));

    let path = game_dir.join(SERVERS_FILE);
    let bytes = fastnbt::to_bytes(&root)?;
    fs::write(&path, bytes).map_err(|err| Error::io(&path, err))
}

fn read_servers(game_dir: &Path) -> Result<Option<Value>> {
    let path = game_dir.join(SERVERS_FILE);
    match fs::read(&path) {
        Ok(bytes) => Ok(fastnbt::from_bytes(&bytes).ok()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(Error::io(&path, err)),
    }
}

fn compound(value: &Value) -> Option<&HashMap<String, Value>> {
    match value {
        Value::Compound(map) => Some(map),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub path: PathBuf,
    pub name: String,
    pub taken_at: Option<DateTime<Utc>>,
}

/// Screenshots, newest first.
pub fn screenshots(game_dir: &Path) -> Result<Vec<Screenshot>> {
    let dir = game_dir.join(SCREENSHOTS_DIR);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(Error::io(&dir, err)),
    };
    let mut shots: Vec<Screenshot> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
        })
        .map(|path| Screenshot {
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            taken_at: fs::metadata(&path)
                .and_then(|meta| meta.modified())
                .ok()
                .map(DateTime::<Utc>::from),
            path,
        })
        .collect();
    shots.sort_by_key(|shot| Reverse(shot.taken_at));
    Ok(shots)
}

/// Resolves a screenshot by file name, refusing names that would point elsewhere.
pub fn screenshot_path(game_dir: &Path, name: &str) -> Result<PathBuf> {
    let path = game_dir.join(SCREENSHOTS_DIR).join(name);
    if name.contains(['/', '\\']) || name.starts_with('.') || !path.is_file() {
        return Err(Error::io(
            path,
            std::io::Error::from(std::io::ErrorKind::NotFound),
        ));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use flate2::Compression;
    use flate2::write::GzEncoder;

    use super::*;

    fn write_level(dir: &Path, name: &str, last_played: i64, game_type: i32) {
        let level = Value::Compound(HashMap::from([(
            "Data".into(),
            Value::Compound(HashMap::from([
                ("LevelName".into(), Value::String(name.into())),
                ("LastPlayed".into(), Value::Long(last_played)),
                ("GameType".into(), Value::Int(game_type)),
            ])),
        )]));
        fs::create_dir_all(dir).unwrap();
        let mut gz = GzEncoder::new(
            File::create(dir.join("level.dat")).unwrap(),
            Compression::default(),
        );
        gz.write_all(&fastnbt::to_bytes(&level).unwrap()).unwrap();
        gz.finish().unwrap();
    }

    #[test]
    fn reads_worlds_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        write_level(&dir.path().join("saves/old"), "Old world", 1_000, 0);
        write_level(&dir.path().join("saves/new"), "Creative", 2_000, 1);
        fs::create_dir_all(dir.path().join("saves/broken")).unwrap();

        let worlds = worlds(dir.path()).unwrap();
        assert_eq!(worlds.len(), 2);
        assert_eq!(worlds[0].folder, "new");
        assert_eq!(worlds[0].game_mode, Some(GameMode::Creative));
        assert_eq!(worlds[1].name, "Old world");
    }

    #[test]
    fn backs_up_worlds_without_the_lock() {
        let dir = tempfile::tempdir().unwrap();
        let world = dir.path().join("saves/home");
        write_level(&world, "Home", 0, 0);
        fs::write(world.join("session.lock"), b"x").unwrap();

        let backup = back_up_world(dir.path(), "home").unwrap();
        let archive = zip::ZipArchive::new(File::open(backup).unwrap()).unwrap();
        let names: Vec<String> = archive
            .file_names()
            .map(|name| name.unwrap().into_owned())
            .collect();
        assert_eq!(names, ["home/level.dat"]);
    }

    #[test]
    fn refuses_paths_outside_saves() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("saves")).unwrap();
        assert!(world_dir(dir.path(), "..").is_err());
        assert!(world_dir(dir.path(), "../saves").is_err());
    }

    #[test]
    fn keeps_unknown_server_fields() {
        let dir = tempfile::tempdir().unwrap();
        let first = Server {
            name: "Hub".into(),
            address: "mc.example.com".into(),
            icon: None,
        };
        save_servers(dir.path(), std::slice::from_ref(&first)).unwrap();

        let mut root: Value =
            fastnbt::from_bytes(&fs::read(dir.path().join(SERVERS_FILE)).unwrap()).unwrap();
        if let Value::Compound(root) = &mut root
            && let Some(Value::List(entries)) = root.get_mut("servers")
            && let Value::Compound(entry) = &mut entries[0]
        {
            entry.insert("acceptTextures".into(), Value::Byte(1));
        }
        fs::write(
            dir.path().join(SERVERS_FILE),
            fastnbt::to_bytes(&root).unwrap(),
        )
        .unwrap();

        let renamed = Server {
            name: "Lobby".into(),
            ..first
        };
        save_servers(dir.path(), &[renamed]).unwrap();
        let saved: Value =
            fastnbt::from_bytes(&fs::read(dir.path().join(SERVERS_FILE)).unwrap()).unwrap();
        let text = format!("{saved:?}");
        assert!(text.contains("acceptTextures"));
        assert_eq!(servers(dir.path()).unwrap()[0].name, "Lobby");
    }
}
