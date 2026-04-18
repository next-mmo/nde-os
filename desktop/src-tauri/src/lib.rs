mod commands;
mod state;

use ai_launcher_core::openviking::{config::VikingConfig, VikingProcess};
use ai_launcher_core::shield::launcher::BrowserLauncher;
use ai_launcher_core::shield::ldplayer_db::LdPlayerStore;
use ai_launcher_core::voice::runtime::VoiceRuntime;
use commands::freecut::FreeCutState;
use commands::service_hub::{VikingState, VoiceRuntimeState};
use commands::shield::{ShieldLauncherState, ShieldLdPlayerState};
use commands::workspace::WorkspaceState;
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

    // Global voice runtime state (shared across all apps)
    let voice_runtime_state = VoiceRuntimeState {
        runtime: VoiceRuntime::new(&base_dir),
    };

    // FreeCut video editor state
    let freecut_state = FreeCutState::new(&base_dir).expect("Failed to initialize FreeCut state");

    // Viking context database process state
    let viking_config = VikingConfig::from_service_config(&base_dir);
    let viking_state = VikingState {
        process: Arc::new(Mutex::new(VikingProcess::new(viking_config, &base_dir))),
    };

    // LDPlayer emulator DB store
    let ld_player_state = ShieldLdPlayerState {
        store: std::sync::Mutex::new(
            LdPlayerStore::new(&base_dir).expect("Failed to initialize LDPlayer DB"),
        ),
    };

    // Workspace state — manages active workspace path and recent workspaces
    let workspace_state = WorkspaceState::new(&base_dir);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
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
        .manage(voice_runtime_state)
        .manage(freecut_state)
        .manage(viking_state)
        .manage(workspace_state)
        .manage(ld_player_state)
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
            // shield android devices
            commands::shield::shield_adb_status,
            commands::shield::shield_list_android_devices,
            commands::shield::shield_list_avds,
            commands::shield::shield_launch_avd,
            commands::shield::shield_stop_device,
            commands::shield::shield_adb_connect,
            commands::shield::shield_configure_proxy,
            commands::shield::shield_clear_proxy,
            commands::shield::shield_device_screenshot,
            commands::shield::shield_open_url_on_device,
            // shield ldplayer emulator management
            commands::shield::shield_detect_ldplayer,
            commands::shield::shield_list_ldplayer_instances,
            commands::shield::shield_launch_ldplayer,
            commands::shield::shield_quit_ldplayer,
            commands::shield::shield_quit_all_ldplayer,
            commands::shield::shield_create_ldplayer,
            commands::shield::shield_clone_ldplayer,
            commands::shield::shield_remove_ldplayer,
            commands::shield::shield_modify_ldplayer,
            commands::shield::shield_update_ldplayer_meta,
            commands::shield::shield_download_ldplayer,
            // shield extensions
            commands::shield::shield_list_extensions,
            commands::shield::shield_install_extension_from_dir,
            commands::shield::shield_install_extension_from_file,
            commands::shield::shield_install_extension_from_url,
            commands::shield::shield_uninstall_extension,
            commands::shield::shield_bind_extension_to_profile,
            commands::shield::shield_unbind_extension_from_profile,
            commands::shield::shield_set_extension_enabled,
            commands::shield::shield_list_profile_extensions,
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
            commands::freecut::freecut_remove_background,
            commands::freecut::freecut_get_setting,
            commands::freecut::freecut_set_setting,
            commands::freecut::freecut_delete_setting,
            // service hub
            commands::service_hub::service_hub_status,
            commands::service_hub::service_hub_install,
            commands::service_hub::service_hub_get_config,
            commands::service_hub::service_hub_set_config,
            commands::service_hub::voice_runtime_status,
            commands::service_hub::voice_runtime_install,
            // openviking
            commands::viking::viking_status,
            commands::viking::viking_install,
            commands::viking::viking_start,
            commands::viking::viking_stop,
            // workspace
            commands::workspace::get_workspace,
            commands::workspace::set_workspace,
            commands::workspace::list_workspaces,
            // chat persistence
            commands::workspace::load_chat_history,
            commands::workspace::save_chat_history,
            commands::workspace::clear_chat_history,
            // vibe studio chat persistence
            commands::workspace::load_vibe_chat_history,
            commands::workspace::save_vibe_chat_history,
            // global settings
            commands::settings::get_global_settings,
            commands::settings::set_global_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
