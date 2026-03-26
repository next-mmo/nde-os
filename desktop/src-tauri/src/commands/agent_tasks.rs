use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Emitter};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentTask {
    pub filename: String,
    pub title: String,
    pub status: String,
    pub locked: bool,
}

#[tauri::command]
pub async fn get_agent_tasks() -> Result<Vec<AgentTask>, String> {
    let mut tasks = Vec::new();
    let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
    let base_dir = if current_dir.ends_with("desktop") {
        current_dir.parent().unwrap().to_path_buf()
    } else {
        current_dir
    };
    
    let tasks_dir = base_dir.join(".agents").join("tasks");

    if !tasks_dir.exists() {
        return Ok(tasks);
    }

    let entries = fs::read_dir(&tasks_dir).map_err(|e| format!("read_dir failed: {e}"))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            if let Ok(content) = fs::read_to_string(&path) {
                let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                
                let title = content.lines()
                    .find(|line| line.starts_with("# "))
                    .map(|line| line.trim_start_matches("# ").to_string())
                    .unwrap_or_else(|| filename.clone());
                
                let status_line = content.lines().find(|line| line.starts_with("- **Status:**"));
                let status = if let Some(line) = status_line {
                    if line.contains("🟢") || line.contains("done") { "Done by AI" }
                    else if line.contains("🟡") || line.contains("yolo") { "YOLO mode" }
                    else if line.contains("waiting") { "Waiting Approval" }
                    else if line.contains("✅") || line.contains("verified") { "Verified by Human" }
                    else if line.contains("re-open") { "Re-open" }
                    else { "Plan" }
                } else {
                    "Plan"
                };

                // 4.1 If status matches 🟡 yolo mode, tag task state as locked: true
                let locked = status == "YOLO mode";

                tasks.push(AgentTask {
                    filename,
                    title,
                    status: status.to_string(),
                    locked,
                });
            }
        }
    }

    Ok(tasks)
}

#[tauri::command]
pub async fn update_agent_task_status(app: AppHandle, filename: String, new_status: String) -> Result<(), String> {
    let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
    let base_dir = if current_dir.ends_with("desktop") {
        current_dir.parent().unwrap().to_path_buf()
    } else {
        current_dir
    };
    
    let tasks_dir = base_dir.join(".agents").join("tasks");
        
    let file_path = tasks_dir.join(&filename);
    
    // Basic sandboxing/traversal protection
    if !file_path.starts_with(&tasks_dir) {
        return Err("Invalid filename traversal attempt".into());
    }

    let content = fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
    
    let emoji_status = match new_status.as_str() {
        "Waiting Approval" => "🔴 `waiting-approval`",
        "YOLO mode" => "🟡 `yolo mode`",
        "Done by AI" => "🟢 `done by AI`",
        "Verified by Human" => "✅ `verified by human`",
        "Re-open" => "🔴 `re-open`",
        _ => "🔴 `plan`",
    };
    
    let replacement = format!("- **Status:** {}", emoji_status);
    
    let new_content = content.lines().map(|line| {
        if line.starts_with("- **Status:**") {
            replacement.clone()
        } else {
            line.to_string()
        }
    }).collect::<Vec<_>>().join("\n");
    
    let mut final_content = new_content;
    if content.ends_with('\n') {
        final_content.push('\n');
    }

    fs::write(&file_path, final_content).map_err(|e| e.to_string())?;
    
    // Emit event to update all frontend clients
    let _ = app.emit("tasks://updated", ());
    
    Ok(())
}
