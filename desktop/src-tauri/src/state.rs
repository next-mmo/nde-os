use ai_launcher_core::app_manager::AppManager;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub type SharedAppManager = Arc<Mutex<AppManager>>;

/// Shared application state managed by Tauri.
/// Wraps AppManager in an Arc<Mutex> for thread-safe access from commands.
pub struct AppState {
    pub manager: SharedAppManager,
    pub base_dir: PathBuf,
}

impl AppState {
    pub fn new(base_dir: PathBuf) -> anyhow::Result<Self> {
        std::fs::create_dir_all(&base_dir)?;
        let manager = AppManager::new(&base_dir)?;
        Ok(Self {
            manager: Arc::new(Mutex::new(manager)),
            base_dir,
        })
    }
}

pub async fn with_manager<T, F>(manager: SharedAppManager, op: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&AppManager) -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let guard = manager.lock().map_err(|e| e.to_string())?;
        op(&guard)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::{with_manager, SharedAppManager};
    use ai_launcher_core::app_manager::AppManager;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_base() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ai-launcher-desktop-state-{stamp}"));
        fs::create_dir_all(&path).expect("temp test dir should be created");
        path
    }

    #[test]
    fn with_manager_runs_operation() {
        let base_dir = temp_base();
        let manager: SharedAppManager = Arc::new(Mutex::new(
            AppManager::new(&base_dir).expect("manager should be created"),
        ));

        let count = tauri::async_runtime::block_on(with_manager(manager, |mgr| Ok(mgr.total_count())))
            .expect("operation should succeed");

        assert_eq!(count, 0);
        fs::remove_dir_all(base_dir).ok();
    }

    #[test]
    fn with_manager_propagates_operation_error() {
        let base_dir = temp_base();
        let manager: SharedAppManager = Arc::new(Mutex::new(
            AppManager::new(&base_dir).expect("manager should be created"),
        ));

        let result: Result<(), String> = tauri::async_runtime::block_on(with_manager(manager, |_| {
            Err("expected failure".to_string())
        }));

        assert_eq!(result.unwrap_err(), "expected failure");
        fs::remove_dir_all(base_dir).ok();
    }
}
