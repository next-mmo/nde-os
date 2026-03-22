use ai_launcher_core::app_manager::AppManager;
use std::path::PathBuf;
use std::sync::Mutex;

/// Shared application state managed by Tauri.
/// Wraps AppManager in a Mutex for thread-safe access from commands.
pub struct AppState {
    pub manager: Mutex<AppManager>,
    pub base_dir: PathBuf,
}

impl AppState {
    pub fn new(base_dir: PathBuf) -> anyhow::Result<Self> {
        std::fs::create_dir_all(&base_dir)?;
        let manager = AppManager::new(&base_dir)?;
        Ok(Self {
            manager: Mutex::new(manager),
            base_dir,
        })
    }
}
