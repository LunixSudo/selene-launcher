mod account;
mod backdrop;
mod commands;
mod config;
mod discord;
mod error;
mod state;

use std::path::PathBuf;

use tauri::{AppHandle, Emitter, Manager, RunEvent, WebviewWindowBuilder};

use crate::commands::shortcut::launch_arg;
use crate::state::Launcher;

pub(crate) const MAIN_WINDOW: &str = "main";

/// A `portable.txt` next to the executable keeps everything in a `data` folder beside it,
/// so the launcher can live on a USB stick. Prism Launcher uses the same marker.
pub(crate) fn portable_data_dir() -> Option<PathBuf> {
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
        // A second start, say from a desktop shortcut, hands its request to the open window.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(id) = launch_arg(&args) {
                let _ = app.emit("launch-request", id);
            }
            show_main_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .on_page_load(|webview, _| backdrop::keep_backdrop(&webview.window()))
        .setup(|app| {
            let data_dir = match portable_data_dir() {
                Some(dir) => dir,
                None => app.path().app_data_dir()?,
            };
            // World icons and screenshots are shown straight from the instance folders.
            app.asset_protocol_scope()
                .allow_directory(&data_dir, true)?;
            let args: Vec<String> = std::env::args().collect();
            app.manage(Launcher::new(&data_dir, launch_arg(&args))?);
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
            commands::prepare_instance,
            commands::update_instance,
            commands::duplicate_instance,
            commands::delete_instance,
            commands::open_instance_folder,
            commands::open_data_folder,
            commands::config,
            commands::take_startup_launch,
            commands::take_missed_crashes,
            commands::set_discord_presence,
            commands::privacy::privacy_status,
            commands::privacy::accept_privacy,
            commands::shortcut::create_shortcut,
            commands::shortcut::update_shortcut,
            commands::skins::skin_profile,
            commands::skins::apply_skin,
            commands::skins::reset_skin,
            commands::skins::set_cape,
            commands::skins::saved_skins,
            commands::skins::add_skin,
            commands::skins::set_skin_variant,
            commands::skins::remove_skin,
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
            commands::update::can_uninstall,
            commands::update::uninstall_launcher,
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
        .build(tauri::generate_context!())
        .expect("failed to build tauri application")
        .run(keep_alive_while_playing);
}

/// Closing the window while a game runs keeps the launcher alive in the background, so play
/// time is counted and the window comes back afterwards.
fn keep_alive_while_playing(app: &AppHandle, event: RunEvent) {
    if let RunEvent::ExitRequested {
        code: None, api, ..
    } = event
        && app.state::<Launcher>().any_running()
    {
        api.prevent_exit();
    }
}

/// Brings the main window to the front, opening it again if it was closed.
pub(crate) fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == MAIN_WINDOW);
    if let Some(config) = config {
        let _ =
            WebviewWindowBuilder::from_config(app, config).and_then(WebviewWindowBuilder::build);
    }
}
