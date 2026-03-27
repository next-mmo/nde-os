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

/// Resolve the `.agents/tasks/` directory from CWD.
fn tasks_dir() -> Result<PathBuf, String> {
    let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
    let base_dir = if current_dir.ends_with("desktop") {
        current_dir.parent().unwrap().to_path_buf()
    } else {
        current_dir
    };
    Ok(base_dir.join(".agents").join("tasks"))
}

#[tauri::command]
pub async fn get_agent_tasks() -> Result<Vec<AgentTask>, String> {
    let mut tasks = Vec::new();
    let tasks_dir = tasks_dir()?;

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
    let tasks_dir = tasks_dir()?;
    let file_path = tasks_dir.join(&filename);
    
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
    let _ = app.emit("tasks://updated", ());
    
    Ok(())
}

/// Create a new agent task ticket file in `.agents/tasks/`.
#[tauri::command]
pub async fn create_agent_task(
    app: AppHandle,
    title: String,
    description: String,
    checklist: Vec<String>,
) -> Result<AgentTask, String> {
    let tasks_dir = tasks_dir()?;
    fs::create_dir_all(&tasks_dir).map_err(|e| format!("Failed to create tasks dir: {e}"))?;

    // Generate a slug filename from the title
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    // Avoid collisions
    let mut filename = format!("{}.md", slug);
    let mut counter = 2u32;
    while tasks_dir.join(&filename).exists() {
        filename = format!("{}-{}.md", slug, counter);
        counter += 1;
    }

    // Build checklist markdown
    let checklist_str = if checklist.is_empty() {
        "- [ ] Implement the feature\n- [ ] Test the implementation\n".to_string()
    } else {
        checklist.iter().map(|item| format!("- [ ] {}", item)).collect::<Vec<_>>().join("\n") + "\n"
    };

    let now = {
        let d = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // Simple UTC date string (good enough for a ticket timestamp)
        let secs_per_day = 86400u64;
        let days = d / secs_per_day;
        // Approximate year/month/day (not accounting for leap seconds, but fine for tickets)
        let year = 1970 + days / 365;
        let remaining = days % 365;
        let month = remaining / 30 + 1;
        let day = remaining % 30 + 1;
        format!("{}-{:02}-{:02}", year, month.min(12), day.min(31))
    };
    let desc = if description.is_empty() { title.clone() } else { description };

    let content = format!(
        "# {}\n\n- **Status:** 🔴 `plan`\n- **Created:** {}\n\n## Description\n\n{}\n\n## Checklist\n\n{}",
        title, now, desc, checklist_str
    );

    let file_path = tasks_dir.join(&filename);
    fs::write(&file_path, &content).map_err(|e| format!("Failed to write task: {e}"))?;

    let _ = app.emit("tasks://updated", ());

    Ok(AgentTask {
        filename,
        title,
        status: "Plan".to_string(),
        locked: false,
    })
}

/// Delete an agent task file.
#[tauri::command]
pub async fn delete_agent_task(app: AppHandle, filename: String) -> Result<(), String> {
    let tasks_dir = tasks_dir()?;
    let file_path = tasks_dir.join(&filename);

    if !file_path.starts_with(&tasks_dir) {
        return Err("Invalid filename traversal attempt".into());
    }

    if file_path.exists() {
        fs::remove_file(&file_path).map_err(|e| e.to_string())?;
        let _ = app.emit("tasks://updated", ());
    }

    Ok(())
}
