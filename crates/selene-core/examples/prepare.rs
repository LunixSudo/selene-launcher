//! Downloads a game version, its assets and its Java runtime into a directory.
//!
//! ```sh
//! cargo run -p selene-core --example prepare -- 1.21.10 ./game
//! ```

use std::time::Instant;

use selene_core::download::Progress;
use selene_core::game::{self, GameDirs, Stage};
use selene_core::http;
use selene_core::manifest::VersionManifest;

#[tokio::main]
async fn main() -> selene_core::Result<()> {
    let mut args = std::env::args().skip(1);
    let version_id = args.next().unwrap_or_else(|| "release".into());
    let dirs = GameDirs::new(args.next().unwrap_or_else(|| "game".into()));

    let started = Instant::now();
    let client = http::client()?;
    let manifest = VersionManifest::fetch(&client).await?;
    let id = if version_id == "release" {
        manifest.latest.release.clone()
    } else {
        version_id
    };
    let entry = manifest.get(&id).expect("unknown version");

    let installed = game::install(&client, &dirs, entry, print_progress).await?;
    println!(
        "\n{} with java {} at {}",
        installed.version.id,
        installed.java.version,
        installed.java.executable().display()
    );
    println!("done in {:.1?}", started.elapsed());
    Ok(())
}

#[allow(clippy::cast_precision_loss)]
fn print_progress(stage: Stage, progress: Progress) {
    let percent = progress.bytes_done as f64 / progress.bytes_total.max(1) as f64 * 100.0;
    print!("\r{stage:?} {percent:5.1}%");
}
