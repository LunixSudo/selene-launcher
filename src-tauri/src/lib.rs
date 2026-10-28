mod account;
mod commands;
mod config;
mod error;
mod state;

use std::path::PathBuf;

use tauri::Manager;

use crate::state::Launcher;

/// A `portable.txt` next to the executable keeps everything in a `data` folder beside it,
/// so the launcher can live on a USB stick. Prism Launcher uses the same marker.
fn portable_data_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    dir.join("portable.txt").is_file().then(|| dir.join("data"))
}

/// Starts the launcher.
///
/// # Panics
///
/// Panics if the Tauri runtime fails to initialize.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = match portable_data_dir() {
                Some(dir) => dir,
                None => app.path().app_data_dir()?,
            };
            // World icons and screenshots are shown straight from the instance folders.
            app.asset_protocol_scope()
                .allow_directory(&data_dir, true)?;
            app.manage(Launcher::new(&data_dir)?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::accounts,
            commands::sign_in,
            commands::cancel_sign_in,
            commands::sign_out,
            commands::set_active_account,
            commands::versions,
            commands::loader_versions,
            commands::instances,
            commands::create_instance,
            commands::update_instance,
            commands::duplicate_instance,
            commands::delete_instance,
            commands::open_instance_folder,
            commands::open_data_folder,
            commands::config,
            commands::set_config,
            commands::game_log,
            commands::memory_info,
            commands::set_instance_memory,
            commands::set_launch_options,
            commands::launch_instance,
            commands::stop_instance,
            commands::content::search_catalog,
            commands::content::catalog_categories,
            commands::content::installed_projects,
            commands::content::installed_content,
            commands::content::preview_install,
            commands::content::install_content,
            commands::content::set_content_enabled,
            commands::content::remove_content,
            commands::content::check_updates,
            commands::content::update_content,
            commands::content::project_versions,
            commands::content::curseforge_changelog,
            commands::content::install_version,
            commands::content::project_page,
            commands::content::preview_version_change,
            commands::content::change_version,
            commands::packs::import_pack,
            commands::packs::export_pack,
            commands::update::check_update,
            commands::update::install_update,
            commands::saves::worlds,
            commands::saves::open_world_folder,
            commands::saves::back_up_world,
            commands::saves::delete_world,
            commands::saves::servers,
            commands::saves::save_servers,
            commands::saves::screenshots,
            commands::saves::open_screenshot,
            commands::saves::delete_screenshot,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run tauri application");
}
