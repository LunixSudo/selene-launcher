use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::{Path, PathBuf};

use crate::rules::{self, Environment, Features};
use crate::version::{Argument, ArgumentValue, VersionInfo};
use crate::{Error, Result};

/// Same garbage collector tuning the official launcher passes.
const DEFAULT_JVM_ARGS: &[&str] = &[
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+UseG1GC",
    "-XX:G1NewSizePercent=20",
    "-XX:G1ReservePercent=20",
    "-XX:MaxGCPauseMillis=50",
    "-XX:G1HeapRegionSize=32M",
];

#[derive(Debug, Clone)]
pub struct LaunchOptions {
    pub java: PathBuf,
    pub game_dir: PathBuf,
    pub assets_dir: PathBuf,
    /// Usually `assets_dir`; old versions read assets by name from another folder.
    pub game_assets_dir: PathBuf,
    pub libraries_dir: PathBuf,
    pub versions_dir: PathBuf,
    pub natives_dir: PathBuf,
    pub account: LaunchAccount,
    pub max_memory_mb: u32,
    pub resolution: Option<(u32, u32)>,
    pub fullscreen: bool,
    pub quick_play: Option<QuickPlay>,
    pub extra_jvm_args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LaunchAccount {
    pub name: String,
    pub uuid: String,
    pub access_token: String,
    pub xuid: Option<String>,
}

#[derive(Debug, Clone)]
pub enum QuickPlay {
    Singleplayer(String),
    Multiplayer(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
}

impl LaunchOptions {
    fn features(&self) -> Features {
        Features {
            has_custom_resolution: Some(self.resolution.is_some()),
            has_quick_plays_support: Some(false),
            is_quick_play_singleplayer: Some(matches!(
                self.quick_play,
                Some(QuickPlay::Singleplayer(_))
            )),
            is_quick_play_multiplayer: Some(matches!(
                self.quick_play,
                Some(QuickPlay::Multiplayer(_))
            )),
            ..Features::default()
        }
    }
}

impl LaunchCommand {
    pub fn to_command(&self) -> std::process::Command {
        let mut command = std::process::Command::new(&self.program);
        command.args(&self.args).current_dir(&self.working_dir);
        command
    }
}

/// Builds the full `java` invocation for a version that is already downloaded.
pub fn build_command(version: &VersionInfo, options: &LaunchOptions) -> LaunchCommand {
    let env = Environment::current(options.features());
    build_command_for(version, options, &env)
}

fn build_command_for(
    version: &VersionInfo,
    options: &LaunchOptions,
    env: &Environment,
) -> LaunchCommand {
    let variables = variables(version, options, env);
    let substitute = |arg: &str| substitute(arg, &variables);

    let mut args = vec![format!("-Xmx{}M", options.max_memory_mb)];
    args.extend(DEFAULT_JVM_ARGS.iter().map(ToString::to_string));
    args.extend(options.extra_jvm_args.iter().cloned());

    if let Some(arguments) = &version.arguments {
        args.extend(resolve(&arguments.jvm, env).map(substitute));
        args.push(version.main_class.clone());
        args.extend(resolve(&arguments.game, env).map(substitute));
    } else {
        args.push(substitute("-Djava.library.path=${natives_directory}"));
        args.push("-cp".into());
        args.push(substitute("${classpath}"));
        args.push(version.main_class.clone());
        let game = version.minecraft_arguments.as_deref().unwrap_or_default();
        args.extend(game.split_whitespace().map(substitute));
        if let Some((width, height)) = options.resolution {
            args.extend([
                "--width".into(),
                width.to_string(),
                "--height".into(),
                height.to_string(),
            ]);
        }
    }
    if let Some(QuickPlay::Multiplayer(address)) = &options.quick_play
        && !args.iter().any(|arg| arg == "--quickPlayMultiplayer")
    {
        args.extend(legacy_server_args(address));
    }
    if options.fullscreen {
        args.push("--fullscreen".into());
    }

    LaunchCommand {
        program: options.java.clone(),
        args,
        working_dir: options.game_dir.clone(),
    }
}

/// Versions before 1.20 only know `--server` and `--port`.
fn legacy_server_args(address: &str) -> Vec<String> {
    match address.rsplit_once(':') {
        Some((host, port)) if port.parse::<u16>().is_ok() => {
            vec!["--server".into(), host.into(), "--port".into(), port.into()]
        }
        _ => vec!["--server".into(), address.into()],
    }
}

/// Unpacks native libraries for versions that still ship them as separate archives (before 1.19).
pub fn extract_natives(version: &VersionInfo, options: &LaunchOptions) -> Result<()> {
    let env = Environment::current(options.features());
    for library in version.libraries(&env) {
        let Some(path) = library
            .native_artifact(&env)
            .and_then(|artifact| artifact.path.as_ref())
        else {
            continue;
        };
        let jar = options.libraries_dir.join(path);
        extract_jar(&jar, &options.natives_dir)?;
    }
    Ok(())
}

fn extract_jar(jar: &Path, target: &Path) -> Result<()> {
    let file = File::open(jar).map_err(|err| Error::io(jar, err))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|err| Error::io(jar, err.into()))?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|err| Error::io(jar, err.into()))?;
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        if entry.is_dir() || name.starts_with("META-INF") {
            continue;
        }
        let out_path = target.join(name);
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| Error::io(parent, err))?;
        }
        let mut out = File::create(&out_path).map_err(|err| Error::io(&out_path, err))?;
        std::io::copy(&mut entry, &mut out).map_err(|err| Error::io(&out_path, err))?;
    }
    Ok(())
}

fn resolve<'a>(
    arguments: &'a [Argument],
    env: &'a Environment,
) -> impl Iterator<Item = &'a str> + 'a {
    arguments.iter().flat_map(move |argument| {
        let values: Vec<&str> = match argument {
            Argument::Plain(value) => vec![value],
            Argument::Conditional { rules, value } if rules::allowed(rules, env) => match value {
                ArgumentValue::One(value) => vec![value],
                ArgumentValue::Many(values) => values.iter().map(String::as_str).collect(),
            },
            Argument::Conditional { .. } => Vec::new(),
        };
        values
    })
}

fn classpath(version: &VersionInfo, options: &LaunchOptions, env: &Environment) -> String {
    let mut seen = HashSet::new();
    let libraries = version
        .libraries(env)
        .filter_map(|library| library.downloads.artifact.as_ref()?.path.as_ref())
        .map(|path| options.libraries_dir.join(path));
    let client_jar = options
        .versions_dir
        .join(&version.id)
        .join(format!("{}.jar", version.id));

    libraries
        .chain(std::iter::once(client_jar))
        .filter(|path| seen.insert(path.clone()))
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(classpath_separator(env))
}

fn classpath_separator(env: &Environment) -> &'static str {
    if env.os == "windows" { ";" } else { ":" }
}

fn variables(
    version: &VersionInfo,
    options: &LaunchOptions,
    env: &Environment,
) -> HashMap<&'static str, String> {
    let account = &options.account;
    let (width, height) = options.resolution.unwrap_or((854, 480));
    let (singleplayer, multiplayer) = match &options.quick_play {
        Some(QuickPlay::Singleplayer(world)) => (world.clone(), String::new()),
        Some(QuickPlay::Multiplayer(address)) => (String::new(), address.clone()),
        None => (String::new(), String::new()),
    };
    let display = |path: &Path| path.display().to_string();

    HashMap::from([
        ("auth_player_name", account.name.clone()),
        ("auth_uuid", account.uuid.clone()),
        ("auth_access_token", account.access_token.clone()),
        (
            "auth_session",
            format!("token:{}:{}", account.access_token, account.uuid),
        ),
        ("auth_xuid", account.xuid.clone().unwrap_or_default()),
        ("user_type", "msa".into()),
        ("user_properties", "{}".into()),
        ("clientid", String::new()),
        ("version_name", version.id.clone()),
        ("version_type", version.kind.as_str().into()),
        ("game_directory", display(&options.game_dir)),
        ("assets_root", display(&options.assets_dir)),
        ("game_assets", display(&options.game_assets_dir)),
        ("assets_index_name", version.asset_index.id.clone()),
        ("library_directory", display(&options.libraries_dir)),
        ("natives_directory", display(&options.natives_dir)),
        ("classpath", classpath(version, options, env)),
        ("classpath_separator", classpath_separator(env).into()),
        ("launcher_name", "selene-launcher".into()),
        ("launcher_version", env!("CARGO_PKG_VERSION").into()),
        ("resolution_width", width.to_string()),
        ("resolution_height", height.to_string()),
        ("quickPlaySingleplayer", singleplayer),
        ("quickPlayMultiplayer", multiplayer),
        ("quickPlayPath", String::new()),
        ("quickPlayRealms", String::new()),
    ])
}

/// Replaces every `${name}` with its value; unknown names are left as they are.
fn substitute(template: &str, variables: &HashMap<&'static str, String>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            out.push_str(&rest[start..]);
            return out;
        };
        match variables.get(&after[..end]) {
            Some(value) => out.push_str(value),
            None => out.push_str(&rest[start..start + 3 + end]),
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODERN: &str = r#"{
        "id": "1.21.10",
        "type": "release",
        "mainClass": "net.minecraft.client.main.Main",
        "assetIndex": { "id": "27", "sha1": "a", "size": 1, "totalSize": 1, "url": "https://example.com" },
        "downloads": { "client": { "sha1": "c", "size": 1, "url": "https://example.com" } },
        "arguments": {
            "game": [
                "--username", "${auth_player_name}",
                "--version", "${version_name}",
                { "rules": [{ "action": "allow", "features": { "has_custom_resolution": true } }],
                  "value": ["--width", "${resolution_width}", "--height", "${resolution_height}"] },
                { "rules": [{ "action": "allow", "features": { "is_quick_play_multiplayer": true } }],
                  "value": ["--quickPlayMultiplayer", "${quickPlayMultiplayer}"] }
            ],
            "jvm": [
                { "rules": [{ "action": "allow", "os": { "name": "osx" } }], "value": "-XstartOnFirstThread" },
                "-Djava.library.path=${natives_directory}",
                "-cp", "${classpath}"
            ]
        },
        "libraries": [
            { "name": "a:a:1", "downloads": { "artifact": { "path": "a/a-1.jar", "sha1": "1", "size": 1, "url": "https://example.com" } } },
            { "name": "a:a:1", "downloads": { "artifact": { "path": "a/a-1.jar", "sha1": "1", "size": 1, "url": "https://example.com" } } }
        ]
    }"#;

    const LEGACY: &str = r#"{
        "id": "1.8.9",
        "type": "release",
        "mainClass": "net.minecraft.client.main.Main",
        "minecraftArguments": "--username ${auth_player_name} --session ${auth_session} --userType ${user_type}",
        "assetIndex": { "id": "1.8", "sha1": "a", "size": 1, "totalSize": 1, "url": "https://example.com" },
        "downloads": { "client": { "sha1": "c", "size": 1, "url": "https://example.com" } }
    }"#;

    fn options() -> LaunchOptions {
        LaunchOptions {
            java: PathBuf::from("java"),
            game_dir: PathBuf::from("game"),
            assets_dir: PathBuf::from("assets"),
            game_assets_dir: PathBuf::from("assets"),
            libraries_dir: PathBuf::from("libraries"),
            versions_dir: PathBuf::from("versions"),
            natives_dir: PathBuf::from("natives"),
            account: LaunchAccount {
                name: "Steve".into(),
                uuid: "uuid".into(),
                access_token: "token".into(),
                xuid: None,
            },
            max_memory_mb: 4096,
            resolution: None,
            fullscreen: false,
            quick_play: None,
            extra_jvm_args: Vec::new(),
        }
    }

    fn windows(options: &LaunchOptions) -> Environment {
        Environment {
            os: "windows",
            arch: "x86_64",
            features: options.features(),
        }
    }

    fn command(json: &str, options: &LaunchOptions) -> Vec<String> {
        let version: VersionInfo = serde_json::from_str(json).unwrap();
        build_command_for(&version, options, &windows(options)).args
    }

    #[test]
    fn substitutes_known_variables_only() {
        let variables = HashMap::from([("name", "Steve".to_string())]);

        assert_eq!(substitute("${name}!", &variables), "Steve!");
        assert_eq!(substitute("${other}", &variables), "${other}");
        assert_eq!(substitute("a ${name", &variables), "a ${name");
    }

    #[test]
    fn builds_modern_command() {
        let args = command(MODERN, &options());
        let separator = std::path::MAIN_SEPARATOR;

        assert_eq!(args[0], "-Xmx4096M");
        assert!(!args.contains(&"-XstartOnFirstThread".into()));
        let classpath = &args[args.iter().position(|arg| arg == "-cp").unwrap() + 1];
        assert_eq!(
            *classpath,
            format!(
                "libraries{separator}a/a-1.jar;versions{separator}1.21.10{separator}1.21.10.jar"
            )
        );
        let main = args
            .iter()
            .position(|arg| arg == "net.minecraft.client.main.Main")
            .unwrap();
        assert_eq!(
            args[main + 1..],
            ["--username", "Steve", "--version", "1.21.10"]
        );
    }

    #[test]
    fn joins_servers_on_old_versions() {
        let mut options = options();
        options.quick_play = Some(QuickPlay::Multiplayer("mc.example.com:25570".into()));
        let args = command(LEGACY, &options);

        assert_eq!(
            &args[args.len() - 4..],
            ["--server", "mc.example.com", "--port", "25570"]
        );
    }

    #[test]
    fn asks_for_fullscreen() {
        let mut options = options();
        options.fullscreen = true;

        assert_eq!(command(MODERN, &options).last().unwrap(), "--fullscreen");
    }

    #[test]
    fn adds_optional_arguments_when_enabled() {
        let mut options = options();
        options.resolution = Some((1280, 720));
        options.quick_play = Some(QuickPlay::Multiplayer("mc.example.com".into()));
        let args = command(MODERN, &options);

        let tail = &args[args.len() - 6..];
        assert_eq!(
            tail,
            [
                "--width",
                "1280",
                "--height",
                "720",
                "--quickPlayMultiplayer",
                "mc.example.com"
            ]
        );
    }

    #[test]
    fn builds_legacy_command() {
        let args = command(LEGACY, &options());

        assert!(args.contains(&"-Djava.library.path=natives".into()));
        let main = args
            .iter()
            .position(|arg| arg == "net.minecraft.client.main.Main")
            .unwrap();
        assert_eq!(
            args[main + 1..],
            [
                "--username",
                "Steve",
                "--session",
                "token:token:uuid",
                "--userType",
                "msa"
            ]
        );
    }
}
