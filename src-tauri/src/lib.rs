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
            commands::versions,
            commands::instances,
            commands::create_instance,
            commands::delete_instance,
            commands::launch_instance,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run tauri application");
}
