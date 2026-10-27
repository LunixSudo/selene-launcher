mod account;
mod commands;
mod error;
mod state;

use tauri::Manager;

use crate::state::Launcher;

/// Starts the launcher.
///
/// # Panics
///
/// Panics if the Tauri runtime fails to initialize.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
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
            commands::delete_instance,
            commands::memory_info,
            commands::set_instance_memory,
            commands::launch_instance,
            commands::mods::search_mods,
            commands::mods::installed_mods,
            commands::mods::install_mod,
            commands::mods::set_mod_enabled,
            commands::mods::remove_mod,
            commands::mods::check_mod_updates,
            commands::mods::update_mods,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run tauri application");
}
