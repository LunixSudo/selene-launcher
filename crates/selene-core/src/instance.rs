use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::loader::LoaderVersion;
use crate::{Error, Result};

const CONFIG_FILE: &str = "instance.json";
const GAME_DIR: &str = "minecraft";

/// A separate game folder with its own version, worlds, mods and settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub loader: Option<LoaderVersion>,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub last_played: Option<DateTime<Utc>>,
    #[serde(default)]
    pub max_memory_mb: Option<u32>,
    /// Name of one of the launcher's built-in icons; `None` shows the default.
    #[serde(default)]
    pub icon: Option<String>,
    /// Name of one of the launcher's built-in cover gradients; `None` picks one from the id.
    #[serde(default)]
    pub cover: Option<String>,
    /// Total time spent in the game.
    #[serde(default)]
    pub play_seconds: u64,
    /// Game window size; `None` keeps the game's default.
    #[serde(default)]
    pub window: Option<WindowSize>,
    #[serde(default)]
    pub fullscreen: bool,
    /// Extra JVM arguments, separated by spaces.
    #[serde(default)]
    pub java_args: String,
    /// Java to run the game with instead of the one Mojang ships for this version.
    #[serde(default)]
    pub java_path: Option<PathBuf>,
    /// Group the instance is filed under on the home screen.
    #[serde(default)]
    pub group: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

/// Instances live in `root/<id>/`, each with its config and a `minecraft` game folder.
#[derive(Debug, Clone)]
pub struct Instances {
    root: PathBuf,
}

impl Instances {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn game_dir(&self, id: &str) -> PathBuf {
        self.root.join(id).join(GAME_DIR)
    }

    /// Whether `id` names an existing instance. Ids come from the UI, so anything that is not a
    /// plain folder name is refused before it reaches a path.
    pub fn exists(&self, id: &str) -> bool {
        is_valid_id(id) && self.root.join(id).join(CONFIG_FILE).is_file()
    }

    /// Every readable instance, most recently played first. Broken configs are skipped.
    pub async fn list(&self) -> Result<Vec<Instance>> {
        let mut entries = match fs::read_dir(&self.root).await {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(Error::io(&self.root, err)),
        };

        let mut instances = Vec::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|err| Error::io(&self.root, err))?
        {
            if let Some(instance) = read_config(&entry.path()).await {
                instances.push(instance);
            }
        }
        instances.sort_by(|a, b| {
            b.last_played
                .cmp(&a.last_played)
                .then(b.created_at.cmp(&a.created_at))
        });
        Ok(instances)
    }

    pub async fn get(&self, id: &str) -> Option<Instance> {
        if !is_valid_id(id) {
            return None;
        }
        read_config(&self.root.join(id)).await
    }

    pub async fn create(
        &self,
        name: &str,
        version: &str,
        loader: Option<LoaderVersion>,
    ) -> Result<Instance> {
        let id = self.free_id(name).await?;
        let instance = Instance {
            id,
            name: name.trim().to_owned(),
            version: version.to_owned(),
            loader,
            created_at: Utc::now(),
            last_played: None,
            max_memory_mb: None,
            icon: None,
            cover: None,
            play_seconds: 0,
            window: None,
            fullscreen: true,
            java_args: String::new(),
            java_path: None,
            group: None,
        };
        let game_dir = self.game_dir(&instance.id);
        fs::create_dir_all(&game_dir)
            .await
            .map_err(|err| Error::io(&game_dir, err))?;
        self.save(&instance).await?;
        Ok(instance)
    }

    pub async fn save(&self, instance: &Instance) -> Result<()> {
        let path = self.root.join(checked_id(&instance.id)?).join(CONFIG_FILE);
        crate::fs::write_atomic(&path, &serde_json::to_vec_pretty(instance)?).await
    }

    /// Copies an instance with its mods, worlds and settings under a new name.
    pub async fn duplicate(&self, id: &str, name: &str) -> Result<Instance> {
        let source = self.root.join(checked_id(id)?);
        let mut instance = read_config(&source).await.ok_or_else(|| {
            Error::io(&source, std::io::Error::from(std::io::ErrorKind::NotFound))
        })?;
        instance.id = self.free_id(name).await?;
        name.trim().clone_into(&mut instance.name);
        instance.created_at = Utc::now();
        instance.last_played = None;
        instance.play_seconds = 0;

        let target = self.root.join(&instance.id);
        let from = source.join(GAME_DIR);
        let to = target.join(GAME_DIR);
        let copied = tokio::task::spawn_blocking(move || copy_dir(&from, &to))
            .await
            .map_err(|err| Error::io(&target, std::io::Error::other(err)))?;
        if let Err(err) = copied.and(self.save(&instance).await) {
            let _ = fs::remove_dir_all(&target).await;
            return Err(err);
        }
        Ok(instance)
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        let path = self.root.join(checked_id(id)?);
        fs::remove_dir_all(&path)
            .await
            .map_err(|err| Error::io(&path, err))
    }

    /// Creates an empty folder for `name` and returns its id. Creating the folder is what
    /// claims the id, so two instances made at once never share one.
    async fn free_id(&self, name: &str) -> Result<String> {
        fs::create_dir_all(&self.root)
            .await
            .map_err(|err| Error::io(&self.root, err))?;
        let base = slug(name);
        let mut id = base.clone();
        let mut suffix = 2;
        loop {
            let path = self.root.join(&id);
            match fs::create_dir(&path).await {
                Ok(()) => return Ok(id),
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    id = format!("{base}-{suffix}");
                    suffix += 1;
                }
                Err(err) => return Err(Error::io(&path, err)),
            }
        }
    }
}

/// Instance ids are folder names made by [`slug`], so a valid one is a single plain component.
fn is_valid_id(id: &str) -> bool {
    !id.is_empty() && id != "." && id != ".." && !id.contains(['/', '\\', ':'])
}

fn checked_id(id: &str) -> Result<&str> {
    if is_valid_id(id) {
        Ok(id)
    } else {
        Err(Error::io(
            id,
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "not an instance id"),
        ))
    }
}

/// The folder name is the id, so a folder copied by hand is a separate instance.
async fn read_config(dir: &Path) -> Option<Instance> {
    let bytes = fs::read(dir.join(CONFIG_FILE)).await.ok()?;
    let mut instance: Instance = serde_json::from_slice(&bytes).ok()?;
    dir.file_name()?.to_str()?.clone_into(&mut instance.id);
    Some(instance)
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to).map_err(|err| Error::io(to, err))?;
    let entries = match std::fs::read_dir(from) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(Error::io(from, err)),
    };
    for entry in entries {
        let entry = entry.map_err(|err| Error::io(from, err))?;
        let path = entry.path();
        let target = to.join(entry.file_name());
        let kind = entry.file_type().map_err(|err| Error::io(&path, err))?;
        if kind.is_dir() {
            copy_dir(&path, &target)?;
        } else if kind.is_file() && entry.file_name() != "session.lock" {
            std::fs::copy(&path, &target).map_err(|err| Error::io(&path, err))?;
        }
    }
    Ok(())
}

/// A folder name that is safe on every file system.
fn slug(name: &str) -> String {
    let mut slug = String::new();
    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-');
    if slug.is_empty() {
        "instance".to_owned()
    } else {
        slug.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_safe_folder_names() {
        assert_eq!(slug("My Pack 1.21!"), "my-pack-1-21");
        assert_eq!(slug("  --Fabric  "), "fabric");
        assert_eq!(slug("Выживание"), "instance");
    }

    #[tokio::test]
    async fn creates_lists_and_deletes() {
        let dir = tempfile::tempdir().unwrap();
        let instances = Instances::new(dir.path());

        let first = instances.create("Survival", "1.21.10", None).await.unwrap();
        let second = instances.create("Survival", "1.8.9", None).await.unwrap();
        assert_eq!(first.id, "survival");
        assert_eq!(second.id, "survival-2");
        assert!(instances.game_dir(&first.id).is_dir());

        let mut played = first.clone();
        played.last_played = Some(Utc::now());
        instances.save(&played).await.unwrap();
        let listed = instances.list().await.unwrap();
        assert_eq!(listed, [played, second.clone()]);

        instances.delete(&second.id).await.unwrap();
        assert_eq!(instances.list().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn duplicates_with_files() {
        let dir = tempfile::tempdir().unwrap();
        let instances = Instances::new(dir.path());
        let original = instances.create("Survival", "1.21.10", None).await.unwrap();
        let mods = instances.game_dir(&original.id).join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::write(mods.join("sodium.jar"), b"jar").unwrap();

        let copy = instances
            .duplicate(&original.id, "Survival copy")
            .await
            .unwrap();
        assert_eq!(copy.id, "survival-copy");
        assert_eq!(copy.version, original.version);
        assert!(
            instances
                .game_dir(&copy.id)
                .join("mods/sodium.jar")
                .is_file()
        );
    }

    #[tokio::test]
    async fn refuses_ids_outside_the_root() {
        let dir = tempfile::tempdir().unwrap();
        let instances = Instances::new(dir.path().join("instances"));
        instances.create("Survival", "1.21.10", None).await.unwrap();

        assert!(instances.exists("survival"));
        assert!(!instances.exists(".."));
        assert!(instances.get("../instances/survival").await.is_none());
        assert!(instances.delete("..").await.is_err());
        assert!(dir.path().join("instances").is_dir());
    }

    #[tokio::test]
    async fn missing_root_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let instances = Instances::new(dir.path().join("none"));

        assert_eq!(instances.list().await.unwrap(), Vec::new());
    }
}
