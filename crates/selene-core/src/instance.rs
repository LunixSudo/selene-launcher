use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;

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
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub last_played: Option<DateTime<Utc>>,
    #[serde(default)]
    pub max_memory_mb: Option<u32>,
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
        read_config(&self.root.join(id)).await
    }

    pub async fn create(&self, name: &str, version: &str) -> Result<Instance> {
        let id = self.free_id(name).await;
        let instance = Instance {
            id,
            name: name.trim().to_owned(),
            version: version.to_owned(),
            created_at: Utc::now(),
            last_played: None,
            max_memory_mb: None,
        };
        let game_dir = self.game_dir(&instance.id);
        fs::create_dir_all(&game_dir)
            .await
            .map_err(|err| Error::io(&game_dir, err))?;
        self.save(&instance).await?;
        Ok(instance)
    }

    pub async fn save(&self, instance: &Instance) -> Result<()> {
        let path = self.root.join(&instance.id).join(CONFIG_FILE);
        fs::write(&path, serde_json::to_vec_pretty(instance)?)
            .await
            .map_err(|err| Error::io(&path, err))
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        let path = self.root.join(id);
        fs::remove_dir_all(&path)
            .await
            .map_err(|err| Error::io(&path, err))
    }

    async fn free_id(&self, name: &str) -> String {
        let base = slug(name);
        let mut id = base.clone();
        let mut suffix = 2;
        while fs::try_exists(self.root.join(&id)).await.unwrap_or(true) {
            id = format!("{base}-{suffix}");
            suffix += 1;
        }
        id
    }
}

async fn read_config(dir: &Path) -> Option<Instance> {
    let bytes = fs::read(dir.join(CONFIG_FILE)).await.ok()?;
    serde_json::from_slice(&bytes).ok()
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

        let first = instances.create("Survival", "1.21.10").await.unwrap();
        let second = instances.create("Survival", "1.8.9").await.unwrap();
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
    async fn missing_root_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let instances = Instances::new(dir.path().join("none"));

        assert_eq!(instances.list().await.unwrap(), Vec::new());
    }
}
