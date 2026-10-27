//! Downloads a game version, its assets and its Java runtime into a directory.
//!
//! ```sh
//! cargo run -p selene-core --example prepare -- 1.21.10 ./game
//! ```

use std::path::PathBuf;
use std::time::Instant;

use selene_core::download::{self, Progress};
use selene_core::manifest::VersionManifest;
use selene_core::rules::{Environment, Features};
use selene_core::version::{AssetIndex, VersionInfo};
use selene_core::{http, java};

const CONCURRENCY: usize = 32;

#[tokio::main]
async fn main() -> selene_core::Result<()> {
    let mut args = std::env::args().skip(1);
    let version_id = args.next().unwrap_or_else(|| "release".into());
    let root = PathBuf::from(args.next().unwrap_or_else(|| "game".into()));

    let started = Instant::now();
    let client = http::client()?;
    let manifest = VersionManifest::fetch(&client).await?;
    let id = if version_id == "release" {
        manifest.latest.release.clone()
    } else {
        version_id
    };
    let entry = manifest.get(&id).expect("unknown version");
    let version = VersionInfo::fetch(&client, entry).await?;
    let env = Environment::current(Features::default());
    println!(
        "{} ({} libraries)",
        version.id,
        version.libraries(&env).count()
    );

    let assets_dir = root.join("assets");
    let index_download = version.asset_index.download(&assets_dir);
    download::download_all(&client, vec![index_download.clone()], 1, |_| {}).await?;
    let index: AssetIndex = serde_json::from_slice(&std::fs::read(&index_download.path).unwrap())?;

    let mut downloads = version.downloads(&env, &root.join("versions"), &root.join("libraries"));
    downloads.extend(index.downloads(&assets_dir));
    println!("{} files", downloads.len());
    download::download_all(&client, downloads, CONCURRENCY, print_progress).await?;

    if let Some(java_version) = &version.java_version {
        let runtimes = java::fetch_index(&client).await?;
        let runtime = java::install(
            &client,
            &runtimes,
            &java_version.component,
            &root.join("runtimes"),
            print_progress,
        )
        .await?;
        println!(
            "\njava {} at {}",
            runtime.version,
            runtime.executable().display()
        );
    }

    println!("done in {:.1?}", started.elapsed());
    Ok(())
}

#[allow(clippy::cast_precision_loss)]
fn print_progress(progress: Progress) {
    let percent = progress.bytes_done as f64 / progress.bytes_total.max(1) as f64 * 100.0;
    print!("\r{percent:5.1}%");
}
