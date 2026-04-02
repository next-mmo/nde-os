mod commands;
mod state;

use ai_launcher_core::shield::launcher::BrowserLauncher;
use commands::freecut::FreeCutState;
use commands::shield::ShieldLauncherState;
use state::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

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
    let app_state = AppState::new(base_dir.clone()).expect("Failed to initialize AppState");

    // Shield browser launcher (async-safe)
    let shield_launcher = ShieldLauncherState {
        launcher: Arc::new(Mutex::new(BrowserLauncher::new(&base_dir))),
    };

    // FreeCut video editor state
    let freecut_state =
        FreeCutState::new(&base_dir).expect("Failed to initialize FreeCut state");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;

                // In dev mode, disable always-on-top so you can switch to IDE / other windows.
                // Production keeps alwaysOnTop: true from tauri.conf.json.
                if let Some(win) = app.handle().get_webview_window("main") {
                    let _ = win.set_always_on_top(false);
                }
            }
            Ok(())
        })
        .manage(app_state)
        .manage(shield_launcher)
        .manage(freecut_state)
        .manage(commands::pty::PtyState::new())
        .invoke_handler(tauri::generate_handler![
            // system
            commands::system::health_check,
            commands::system::get_system_info,
            commands::system::get_resource_usage,
            // catalog
            commands::catalog::get_catalog,
            // apps
            commands::apps::list_apps,
            commands::apps::get_app,
            commands::apps::install_app,
            commands::apps::uninstall_app,
            commands::apps::upload_app,
            // lifecycle
            commands::lifecycle::launch_app,
            commands::lifecycle::stop_app,
            commands::lifecycle::open_app_browser,
            // sandbox
            commands::sandbox::verify_sandbox,
            commands::sandbox::get_disk_usage,
            // filesystem
            commands::filesystem::list_directory,
            commands::filesystem::get_home_dir,
            commands::filesystem::open_file,
            commands::filesystem::create_folder,
            commands::filesystem::delete_entry,
            commands::filesystem::rename_entry,
            commands::filesystem::read_file_content,
            commands::filesystem::write_file_content,
            // shield browser
            commands::shield::list_shield_profiles,
            commands::shield::get_shield_profile,
            commands::shield::create_shield_profile,
            commands::shield::delete_shield_profile,
            commands::shield::rename_shield_profile,
            commands::shield::get_shield_status,
            commands::shield::launch_shield_profile,
            commands::shield::stop_shield_profile,
            commands::shield::download_shield_engine,
            commands::shield::is_shield_engine_downloaded,
            commands::shield::remove_shield_engine,
            commands::shield::resolve_engine_version,
            commands::shield::get_available_engines,
            // figma json
            commands::figma_json::convert_figma_json,
            commands::figma_json::resolve_document_styles,
            commands::figma_json::fetch_figma_file,
            commands::figma_json::build_figma_llm_prompt,
            commands::figma_json::get_figma_sample,
            // screenshot
            commands::screenshot::capture_screenshot,
            // agent tasks
            commands::agent_tasks::get_agent_tasks,
            commands::agent_tasks::update_agent_task_status,
            commands::agent_tasks::create_agent_task,
            commands::agent_tasks::delete_agent_task,
            commands::agent_tasks::get_agent_task_content,
            commands::agent_tasks::update_agent_task_content,
            commands::agent_tasks::watch_tasks_dir,
            // git
            commands::git::git_status,
            commands::git::git_show_head,
            commands::git::git_add,
            commands::git::git_commit,
            commands::git::git_discard,
            // pty
            commands::pty::spawn_pty,
            commands::pty::write_pty,
            // freecut video editor
            commands::freecut::freecut_create_project,
            commands::freecut::freecut_list_projects,
            commands::freecut::freecut_get_project,
            commands::freecut::freecut_save_project,
            commands::freecut::freecut_delete_project,
            commands::freecut::freecut_import_media,
            commands::freecut::freecut_probe_media,
            commands::freecut::freecut_generate_thumbnails,
            commands::freecut::freecut_generate_waveform,
            commands::freecut::freecut_list_media,
            commands::freecut::freecut_delete_media,
            commands::freecut::freecut_render_frame,
            commands::freecut::freecut_get_hw_encoders,
            commands::freecut::freecut_export_video,
            commands::freecut::freecut_detect_dubbing_tools,
            commands::freecut::freecut_import_dubbing_srt,
            commands::freecut::freecut_generate_dub_assets,
            commands::freecut::freecut_install_dubbing_runtime,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
