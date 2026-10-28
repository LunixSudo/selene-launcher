//! Downloads a game version, its assets and its Java runtime into a directory.
//!
//! ```sh
//! cargo run -p selene-core --example prepare -- 1.21.10 ./game fabric
//! ```
//!
//! The optional loader is `fabric`, `quilt`, `forge` or `neoforge`, with a version after a colon to pin one.

use std::time::Instant;

use selene_core::download::Progress;
use selene_core::game::{self, GameDirs, Stage};
use selene_core::http;
use selene_core::loader::{self, Loader, LoaderVersion};
use selene_core::manifest::VersionManifest;

#[tokio::main]
async fn main() -> selene_core::Result<()> {
    let mut args = std::env::args().skip(1);
    let version_id = args.next().unwrap_or_else(|| "release".into());
    let dirs = GameDirs::new(args.next().unwrap_or_else(|| "game".into()));
    let loader_arg = args.next();

    let started = Instant::now();
    let client = http::client()?;
    let manifest = VersionManifest::fetch(&client).await?;
    let id = if version_id == "release" {
        manifest.latest.release.clone()
    } else {
        version_id
    };
    let entry = manifest.get(&id).expect("unknown version");

    let loader = match loader_arg {
        Some(arg) => Some(pick_loader(&client, &arg, &id).await?),
        None => None,
    };

    let installed = game::install(&client, &dirs, entry, loader.as_ref(), print_progress).await?;
    println!(
        "\n{} with java {} at {}",
        installed.version.id,
        installed.java.version,
        installed.java.executable().display()
    );
    println!("done in {:.1?}", started.elapsed());
    Ok(())
}

async fn pick_loader(
    client: &reqwest::Client,
    arg: &str,
    game_version: &str,
) -> selene_core::Result<LoaderVersion> {
    let (name, version) = arg.split_once(':').unwrap_or((arg, ""));
    let loader = match name {
        "fabric" => Loader::Fabric,
        "quilt" => Loader::Quilt,
        "forge" => Loader::Forge,
        "neoforge" => Loader::NeoForge,
        other => panic!("unknown loader {other}"),
    };
    let version = if version.is_empty() {
        loader::releases(client, loader, game_version)
            .await?
            .into_iter()
            .find(|release| release.stable)
            .expect("no stable loader for this version")
            .version
    } else {
        version.to_owned()
    };
    println!("{loader:?} {version}");
    Ok(LoaderVersion { loader, version })
}

#[allow(clippy::cast_precision_loss)]
fn print_progress(stage: Stage, progress: Progress) {
    let percent = progress.bytes_done as f64 / progress.bytes_total.max(1) as f64 * 100.0;
    print!("\r{stage:?} {percent:5.1}%");
}
