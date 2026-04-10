use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tauri::Emitter;

/// A single entry in the recents list.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WorkspaceEntry {
    pub path: String,
    pub name: String,
    pub last_used: String,
}

/// Managed Tauri state for the active workspace.
pub struct WorkspaceState {
    /// The active workspace root (directory containing `.agents/`).
    pub current: RwLock<PathBuf>,
    /// Recently used workspaces.
    pub recents: RwLock<Vec<WorkspaceEntry>>,
    /// Where we persist the recents list across app restarts.
    config_path: PathBuf,
}

impl WorkspaceState {
    /// Bootstrap workspace state:
    /// 1. Load recents from `<base_dir>/workspaces.json`
    /// 2. Auto-detect current workspace with CWD walk-up (backward compat)
    pub fn new(base_dir: &Path) -> Self {
        let config_path = base_dir.join("workspaces.json");
        let recents = Self::load_recents(&config_path);

        // Auto-detect workspace from CWD (existing behavior as default)
        let current = Self::detect_workspace_from_cwd()
            .or_else(|| recents.first().map(|e| PathBuf::from(&e.path)))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        WorkspaceState {
            current: RwLock::new(current),
            recents: RwLock::new(recents),
            config_path,
        }
    }

    /// Walk up from CWD to find a directory with `.agents/` and a repo-root marker.
    fn detect_workspace_from_cwd() -> Option<PathBuf> {
        let current_dir = std::env::current_dir().ok()?;
        let mut candidate = current_dir.as_path();

        // First pass: require repo-root marker
        loop {
            let agents_path = candidate.join(".agents");
            let is_repo_root = candidate.join(".git").exists()
                || (candidate.join("Cargo.toml").exists() && candidate.join("server").exists());
            if agents_path.is_dir() && is_repo_root {
                return Some(candidate.to_path_buf());
            }
            candidate = candidate.parent()?;
        }
    }

    fn load_recents(path: &Path) -> Vec<WorkspaceEntry> {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save_recents(&self) {
        let recents = self.recents.read().unwrap();
        if let Ok(json) = serde_json::to_string_pretty(&*recents) {
            let _ = fs::create_dir_all(self.config_path.parent().unwrap_or(Path::new(".")));
            let _ = fs::write(&self.config_path, json);
        }
    }

    /// Switch to a new workspace and add to recents.
    pub fn set_workspace(&self, path: PathBuf) {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let path_str = path.to_string_lossy().to_string();
        let now = chrono_now();

        {
            let mut current = self.current.write().unwrap();
            *current = path;
        }

        // Update recents: remove old entry, push to front, cap at 20
        {
            let mut recents = self.recents.write().unwrap();
            recents.retain(|e| e.path != path_str);
            recents.insert(
                0,
                WorkspaceEntry {
                    path: path_str,
                    name,
                    last_used: now,
                },
            );
            recents.truncate(20);
        }
        self.save_recents();
    }

    /// Get the tasks directory for the current workspace.
    pub fn tasks_dir(&self) -> PathBuf {
        let current = self.current.read().unwrap();
        current.join(".agents").join("tasks")
    }

    /// Get the chat directory for the current workspace.
    pub fn chat_dir(&self) -> PathBuf {
        let current = self.current.read().unwrap();
        current.join(".agents").join("chat")
    }
}

/// Simple ISO-ish timestamp without chrono crate.
fn chrono_now() -> String {
    let epoch_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = epoch_secs / 86400;
    let secs_in_day = epoch_secs % 86400;
    format!(
        "{}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        1970 + days / 365,
        (days % 365 / 30 + 1).min(12),
        (days % 365 % 30 + 1).min(31),
        secs_in_day / 3600,
        (secs_in_day % 3600) / 60,
        secs_in_day % 60,
    )
}

// ── Tauri IPC Commands ──────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct WorkspaceInfo {
    pub path: String,
    pub name: String,
    pub has_tasks: bool,
    pub has_chat: bool,
}

#[tauri::command]
pub async fn get_workspace(
    state: tauri::State<'_, WorkspaceState>,
) -> Result<WorkspaceInfo, String> {
    let current = state.current.read().map_err(|e| e.to_string())?;
    let tasks_dir = current.join(".agents").join("tasks");
    let chat_dir = current.join(".agents").join("chat");
    Ok(WorkspaceInfo {
        path: current.to_string_lossy().to_string(),
        name: current
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        has_tasks: tasks_dir.is_dir(),
        has_chat: chat_dir.is_dir(),
    })
}

#[tauri::command]
pub async fn set_workspace(
    path: String,
    state: tauri::State<'_, WorkspaceState>,
    app: tauri::AppHandle,
) -> Result<WorkspaceInfo, String> {
    let workspace_path = PathBuf::from(&path);
    if !workspace_path.is_dir() {
        return Err(format!("Directory does not exist: {}", path));
    }

    // Ensure .agents/ exists
    let agents_dir = workspace_path.join(".agents");
    let tasks_dir = agents_dir.join("tasks");
    let chat_dir = agents_dir.join("chat");
    fs::create_dir_all(&tasks_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&chat_dir).map_err(|e| e.to_string())?;

    state.set_workspace(workspace_path.clone());

    // Emit event so frontend refreshes
    let _ = app.emit("workspace://changed", &path);

    Ok(WorkspaceInfo {
        path,
        name: workspace_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        has_tasks: tasks_dir.is_dir(),
        has_chat: chat_dir.is_dir(),
    })
}

#[tauri::command]
pub async fn list_workspaces(
    state: tauri::State<'_, WorkspaceState>,
) -> Result<Vec<WorkspaceEntry>, String> {
    let recents = state.recents.read().map_err(|e| e.to_string())?;
    Ok(recents.clone())
}

// ── Chat Persistence Commands ───────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub id: u32,
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatHistory {
    pub messages: Vec<ChatMessage>,
    pub conversation_id: Option<String>,
    pub msg_counter: u32,
}

#[tauri::command]
pub async fn load_chat_history(
    state: tauri::State<'_, WorkspaceState>,
) -> Result<ChatHistory, String> {
    let chat_dir = state.chat_dir();
    let history_path = chat_dir.join("history.json");

    if !history_path.exists() {
        return Ok(ChatHistory {
            messages: vec![],
            conversation_id: None,
            msg_counter: 0,
        });
    }

    let content = fs::read_to_string(&history_path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| format!("Invalid chat history: {}", e))
}

#[tauri::command]
pub async fn save_chat_history(
    history: ChatHistory,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let chat_dir = state.chat_dir();
    fs::create_dir_all(&chat_dir).map_err(|e| e.to_string())?;

    let history_path = chat_dir.join("history.json");
    let json = serde_json::to_string_pretty(&history).map_err(|e| e.to_string())?;
    fs::write(&history_path, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_chat_history(
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let chat_dir = state.chat_dir();
    let history_path = chat_dir.join("history.json");
    if history_path.exists() {
        fs::remove_file(&history_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── Vibe Studio Chat Persistence ───────────────────────────────────────────

#[tauri::command]
pub async fn load_vibe_chat_history(
    state: tauri::State<'_, WorkspaceState>,
) -> Result<ChatHistory, String> {
    let chat_dir = state.chat_dir();
    let history_path = chat_dir.join("vibe-history.json");

    if !history_path.exists() {
        return Ok(ChatHistory {
            messages: vec![],
            conversation_id: None,
            msg_counter: 0,
        });
    }

    let content = fs::read_to_string(&history_path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| format!("Invalid vibe chat history: {}", e))
}

#[tauri::command]
pub async fn save_vibe_chat_history(
    history: ChatHistory,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let chat_dir = state.chat_dir();
    fs::create_dir_all(&chat_dir).map_err(|e| e.to_string())?;

    let history_path = chat_dir.join("vibe-history.json");
    let json = serde_json::to_string_pretty(&history).map_err(|e| e.to_string())?;
    fs::write(&history_path, json).map_err(|e| e.to_string())
}
