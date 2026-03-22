mod commands;
mod state;

use state::AppState;
use std::path::PathBuf;

/// Cross-platform base directory
fn get_base_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("ai-launcher"))
            .unwrap_or_else(|_| PathBuf::from("C:\\ai-launcher-data"))
    } else {
        std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".ai-launcher"))
            .unwrap_or_else(|_| PathBuf::from("/tmp/.ai-launcher"))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let base_dir = get_base_dir();
    let app_state = AppState::new(base_dir).expect("Failed to initialize AppState");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            // system
            commands::system::health_check,
            commands::system::get_system_info,
            // catalog
            commands::catalog::get_catalog,
            // apps
            commands::apps::list_apps,
            commands::apps::get_app,
            commands::apps::install_app,
            commands::apps::uninstall_app,
            // lifecycle
            commands::lifecycle::launch_app,
            commands::lifecycle::stop_app,
            commands::lifecycle::open_app_browser,
            // sandbox
            commands::sandbox::verify_sandbox,
            commands::sandbox::get_disk_usage,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
