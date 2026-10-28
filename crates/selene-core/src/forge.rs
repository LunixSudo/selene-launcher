//! Forge and NeoForge. Both patch the game jar while installing, so instead of reading a
//! profile like Fabric the launcher runs their own installer in the background, the same one
//! players run by hand, and then reads the version it leaves in `versions/`.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::process::Stdio;

use reqwest::Client;
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;

use crate::download::{self, Download, Progress};
use crate::game::GameDirs;
use crate::loader::{Loader, LoaderRelease, LoaderVersion, ResolvedPatch, fetch_sha1};
use crate::version::{Arguments, Library};
use crate::{Error, Result, http};

const FORGE_MAVEN_URL: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";
const FORGE_METADATA_URL: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json";
const FORGE_PROMOTIONS_URL: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
const NEOFORGE_MAVEN_URL: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";
const NEOFORGE_VERSIONS_URL: &str =
    "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";
/// NeoForge for 1.20.1 still carried Forge's name and numbering.
const NEOFORGE_1_20_1_MAVEN_URL: &str = "https://maven.neoforged.net/releases/net/neoforged/forge";
const NEOFORGE_1_20_1_VERSIONS_URL: &str =
    "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/forge";
/// Forge 1.12.2 builds before this one ship the old installer, which only works with a window.
const FIRST_HEADLESS_1_12_2_BUILD: u32 = 2851;
/// The installers refuse to install a client without the official launcher's profile list.
const LAUNCHER_PROFILES: &str = "launcher_profiles.json";
const OUTPUT_TAIL_LINES: usize = 20;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Deserialize)]
struct ForgePromotions {
    promos: HashMap<String, String>,
}

#[derive(Deserialize)]
struct NeoForgeVersions {
    versions: Vec<String>,
}

/// The part of `install_profile.json` used to follow the installer's progress.
#[derive(Deserialize)]
struct InstallProfile {
    version: String,
    json: String,
    #[serde(default)]
    libraries: Vec<serde_json::Value>,
    #[serde(default)]
    processors: Vec<Processor>,
}

#[derive(Deserialize)]
struct Processor {
    #[serde(default)]
    sides: Vec<String>,
}

#[derive(Deserialize)]
struct LibraryList {
    #[serde(default)]
    libraries: Vec<serde_json::Value>,
}

/// The version an installer writes, inheriting from the game version.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstalledVersion {
    main_class: String,
    arguments: Option<Arguments>,
    minecraft_arguments: Option<String>,
    #[serde(default)]
    libraries: Vec<Library>,
}

/// Builds that install on `game_version`, newest first.
pub async fn releases(
    client: &Client,
    loader: Loader,
    game_version: &str,
) -> Result<Vec<LoaderRelease>> {
    match loader {
        Loader::NeoForge if game_version == "1.20.1" => neoforge_1_20_1_releases(client).await,
        Loader::NeoForge => {
            let list: NeoForgeVersions = http::get_json(client, NEOFORGE_VERSIONS_URL).await?;
            Ok(list
                .versions
                .into_iter()
                .rev()
                .filter(|version| neoforge_game_version(version).as_deref() == Some(game_version))
                .map(|version| LoaderRelease {
                    stable: !version.contains('-'),
                    recommended: false,
                    version,
                })
                .collect())
        }
        _ => forge_releases(client, game_version).await,
    }
}

/// Builds that share a number with a Forge build are left out: both would install into the
/// same `versions/1.20.1-forge-<build>` folder.
async fn neoforge_1_20_1_releases(client: &Client) -> Result<Vec<LoaderRelease>> {
    let list: NeoForgeVersions = http::get_json(client, NEOFORGE_1_20_1_VERSIONS_URL).await?;
    let mut forge: HashMap<String, Vec<String>> =
        http::get_json(client, FORGE_METADATA_URL).await?;
    let forge = forge.remove("1.20.1").unwrap_or_default();
    Ok(list
        .versions
        .into_iter()
        .rev()
        .filter(|version| version.starts_with("1.20.1-") && !forge.contains(version))
        .filter_map(|version| version.strip_prefix("1.20.1-").map(str::to_owned))
        .map(|version| LoaderRelease {
            stable: !version.contains('-'),
            recommended: false,
            version,
        })
        .collect())
}

async fn forge_releases(client: &Client, game_version: &str) -> Result<Vec<LoaderRelease>> {
    let mut metadata: HashMap<String, Vec<String>> =
        http::get_json(client, FORGE_METADATA_URL).await?;
    let Some(builds) = metadata.remove(game_version) else {
        return Ok(Vec::new());
    };
    let promotions: ForgePromotions = http::get_json(client, FORGE_PROMOTIONS_URL).await?;
    let recommended = promotions
        .promos
        .get(&format!("{game_version}-recommended"));

    let prefix = format!("{game_version}-");
    let builds: Vec<String> = builds
        .iter()
        .rev()
        .filter_map(|build| build.strip_prefix(&prefix))
        .filter(|build| has_headless_installer(game_version, build))
        .map(str::to_owned)
        .collect();
    Ok(builds
        .into_iter()
        .map(|version| LoaderRelease {
            stable: true,
            recommended: Some(&version) == recommended,
            version,
        })
        .collect())
}

/// NeoForge numbers builds after the game: `21.1.77` is for 1.21.1, `21.0.5` for 1.21, and
/// since the yearly versions `26.1.2.10` is for 26.1.2 and `26.1.0.4` for 26.1.
fn neoforge_game_version(version: &str) -> Option<String> {
    let numbers: Vec<&str> = version.split(['.', '-']).collect();
    let major: u32 = numbers.first()?.parse().ok()?;
    let (minor, patch) = (numbers.get(1)?, numbers.get(2)?);
    if major >= 25 {
        numbers.get(3)?;
        Some(if *patch == "0" {
            format!("{major}.{minor}")
        } else {
            format!("{major}.{minor}.{patch}")
        })
    } else if major >= 20 {
        Some(if *minor == "0" {
            format!("1.{major}")
        } else {
            format!("1.{major}.{minor}")
        })
    } else {
        None
    }
}

fn has_headless_installer(game_version: &str, build: &str) -> bool {
    let Some(rest) = game_version.strip_prefix("1.") else {
        return true;
    };
    let minor = rest
        .split('.')
        .next()
        .and_then(|minor| minor.parse::<u32>().ok());
    match minor {
        Some(minor) if minor >= 13 => true,
        Some(12) if game_version == "1.12.2" => build
            .strip_prefix("14.23.5.")
            .and_then(|number| number.parse::<u32>().ok())
            .is_some_and(|number| number >= FIRST_HEADLESS_1_12_2_BUILD),
        _ => false,
    }
}

fn installer_url(loader: &LoaderVersion, game_version: &str) -> String {
    let version = &loader.version;
    match loader.loader {
        Loader::NeoForge if game_version == "1.20.1" => {
            format!(
                "{NEOFORGE_1_20_1_MAVEN_URL}/1.20.1-{version}/forge-1.20.1-{version}-installer.jar"
            )
        }
        Loader::NeoForge => {
            format!("{NEOFORGE_MAVEN_URL}/{version}/neoforge-{version}-installer.jar")
        }
        _ => format!(
            "{FORGE_MAVEN_URL}/{game_version}-{version}/forge-{game_version}-{version}-installer.jar"
        ),
    }
}

/// The folder name the installer gives its version.
fn version_id(loader: &LoaderVersion, game_version: &str) -> String {
    match loader.loader {
        Loader::NeoForge if game_version != "1.20.1" => format!("neoforge-{}", loader.version),
        _ => format!("{game_version}-forge-{}", loader.version),
    }
}

/// Installs the loader unless an earlier run already did, and returns what it adds to the
/// game version. `java` must be able to run the installer; the game's own runtime can.
pub async fn install<F>(
    client: &Client,
    dirs: &GameDirs,
    loader: &LoaderVersion,
    game_version: &str,
    java: &Path,
    on_progress: F,
) -> Result<ResolvedPatch>
where
    F: Fn(Progress),
{
    let id = version_id(loader, game_version);
    if let Some(installed) = read_installed(dirs, &id).await {
        return Ok(installed.into());
    }

    let work_dir = dirs.root.join(".installers");
    let installer = work_dir.join(format!("{id}-installer.jar"));
    let url = installer_url(loader, game_version);
    let sha1 = fetch_sha1(client, &url).await?;
    download::download_all(
        client,
        vec![Download {
            url,
            path: installer.clone(),
            sha1,
            size: 0,
        }],
        1,
        |_| {},
    )
    .await?;

    let result = run_installer(dirs, &installer, java, &on_progress).await;
    let _ = tokio::fs::remove_file(&installer).await;
    let _ = tokio::fs::remove_file(installer.with_extension("jar.log")).await;
    let _ = tokio::fs::remove_dir(&work_dir).await;
    let id = result?;

    read_installed(dirs, &id)
        .await
        .map(Into::into)
        .ok_or_else(|| Error::LoaderInstaller(format!("no version {id} after installing")))
}

/// The installed version, if its JSON is there with every library it lists.
async fn read_installed(dirs: &GameDirs, id: &str) -> Option<InstalledVersion> {
    let path = dirs.versions().join(id).join(format!("{id}.json"));
    let bytes = tokio::fs::read(path).await.ok()?;
    let version: InstalledVersion = serde_json::from_slice(&bytes).ok()?;
    let libraries = dirs.libraries();
    let complete = version
        .libraries
        .iter()
        .filter_map(|library| library.downloads.artifact.as_ref()?.path.as_ref())
        .all(|path| libraries.join(path).is_file());
    complete.then_some(version)
}

/// Runs the installer without a window and returns the id of the version it installed.
async fn run_installer<F>(
    dirs: &GameDirs,
    installer: &Path,
    java: &Path,
    on_progress: &F,
) -> Result<String>
where
    F: Fn(Progress),
{
    let (id, total_steps) = read_profile(installer)?;
    let profiles = dirs.root.join(LAUNCHER_PROFILES);
    if !profiles.exists() {
        tokio::fs::write(&profiles, r#"{"profiles":{}}"#)
            .await
            .map_err(|err| Error::io(&profiles, err))?;
    }

    let mut command = Command::new(java);
    command
        .arg("-Djava.awt.headless=true")
        .arg("-jar")
        .arg(installer)
        .arg("--installClient")
        .arg(&dirs.root)
        .current_dir(installer.parent().unwrap_or(&dirs.root))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let mut child = command.spawn().map_err(|err| Error::io(java, err))?;

    let stdout = child.stdout.take().expect("stdout is piped");
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let errors = tokio::spawn(async move {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes).await;
        String::from_utf8_lossy(&bytes).into_owned()
    });

    // Java prints paths in the system code page, so lines are not always valid UTF-8.
    let mut stdout = BufReader::new(stdout);
    let mut buffer = Vec::new();
    let mut tail = Vec::new();
    let mut steps = 0;
    while stdout.read_until(b'\n', &mut buffer).await.unwrap_or(0) > 0 {
        let line = String::from_utf8_lossy(&buffer).trim_end().to_owned();
        buffer.clear();
        if line.starts_with("Considering library") || line.trim_start().starts_with("MainClass:") {
            steps += 1;
            on_progress(Progress {
                bytes_done: steps.min(total_steps),
                bytes_total: total_steps,
            });
        }
        if tail.len() == OUTPUT_TAIL_LINES {
            tail.remove(0);
        }
        tail.push(line);
    }

    let status = child.wait().await.map_err(|err| Error::io(java, err))?;
    let errors = errors.await.unwrap_or_default();
    if status.success() {
        Ok(id)
    } else {
        let mut output = tail.join("\n");
        output.push('\n');
        output.push_str(errors.trim());
        Err(Error::LoaderInstaller(output.trim().to_owned()))
    }
}

/// The version id the installer will write and how many steps it reports along the way: one
/// per library it checks and one per processor it runs for the client.
fn read_profile(installer: &Path) -> Result<(String, u64)> {
    let file = std::fs::File::open(installer).map_err(|err| Error::io(installer, err))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|err| Error::io(installer, err.into()))?;
    let profile: InstallProfile = serde_json::from_str(&read_entry(
        &mut archive,
        installer,
        "install_profile.json",
    )?)?;
    let version: LibraryList = serde_json::from_str(&read_entry(
        &mut archive,
        installer,
        profile.json.trim_start_matches('/'),
    )?)?;

    let processors = profile
        .processors
        .iter()
        .filter(|processor| {
            processor.sides.is_empty() || processor.sides.iter().any(|side| side == "client")
        })
        .count();
    let steps = profile.libraries.len() + version.libraries.len() + processors;
    Ok((profile.version, steps as u64))
}

fn read_entry(
    archive: &mut zip::ZipArchive<std::fs::File>,
    installer: &Path,
    name: &str,
) -> Result<String> {
    let mut entry = archive
        .by_name(name)
        .map_err(|err| Error::io(installer.join(name), err.into()))?;
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .map_err(|err| Error::io(installer.join(name), err))?;
    Ok(text)
}

impl From<InstalledVersion> for ResolvedPatch {
    fn from(version: InstalledVersion) -> Self {
        Self {
            main_class: version.main_class,
            arguments: version.arguments,
            minecraft_arguments: version.minecraft_arguments,
            libraries: version.libraries,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_neoforge_builds_to_game_versions() {
        assert_eq!(neoforge_game_version("21.1.265").as_deref(), Some("1.21.1"));
        assert_eq!(neoforge_game_version("21.0.167").as_deref(), Some("1.21"));
        assert_eq!(
            neoforge_game_version("20.2.3-beta").as_deref(),
            Some("1.20.2")
        );
        assert_eq!(
            neoforge_game_version("26.1.2.124").as_deref(),
            Some("26.1.2")
        );
        assert_eq!(
            neoforge_game_version("26.1.0.19-beta").as_deref(),
            Some("26.1")
        );
        assert_eq!(neoforge_game_version("0.25w14craftmine.3-beta"), None);
    }

    #[test]
    fn skips_forge_builds_that_need_a_window() {
        assert!(has_headless_installer("1.20.1", "47.4.10"));
        assert!(has_headless_installer("26.1.2", "64.1.5"));
        assert!(has_headless_installer("1.12.2", "14.23.5.2860"));
        assert!(!has_headless_installer("1.12.2", "14.23.5.2847"));
        assert!(!has_headless_installer("1.8.9", "11.15.1.2318"));
    }

    #[test]
    fn names_installers_and_versions() {
        let forge = LoaderVersion {
            loader: Loader::Forge,
            version: "47.4.10".into(),
        };
        let neoforge = LoaderVersion {
            loader: Loader::NeoForge,
            version: "21.1.265".into(),
        };

        assert_eq!(
            installer_url(&forge, "1.20.1"),
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.4.10/forge-1.20.1-47.4.10-installer.jar"
        );
        assert_eq!(
            installer_url(&neoforge, "1.21.1"),
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.265/neoforge-21.1.265-installer.jar"
        );
        assert_eq!(version_id(&forge, "1.20.1"), "1.20.1-forge-47.4.10");
        assert_eq!(version_id(&neoforge, "1.21.1"), "neoforge-21.1.265");

        let old_neoforge = LoaderVersion {
            loader: Loader::NeoForge,
            version: "47.1.106".into(),
        };
        assert_eq!(
            installer_url(&old_neoforge, "1.20.1"),
            "https://maven.neoforged.net/releases/net/neoforged/forge/1.20.1-47.1.106/forge-1.20.1-47.1.106-installer.jar"
        );
        assert_eq!(version_id(&old_neoforge, "1.20.1"), "1.20.1-forge-47.1.106");
    }
}
