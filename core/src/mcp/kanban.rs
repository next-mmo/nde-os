use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

use super::server::McpServer;

#[derive(Serialize)]
pub struct KanbanTask {
    pub filename: String,
    pub title: String,
    pub status: String,
    pub locked: bool,
}

pub fn register(server: &mut McpServer) {
    server.register_tool(
        "nde_kanban_get_tasks",
        "Retrieve all Vibe Code Studio Kanban tasks.",
        json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
    );

    server.register_tool(
        "nde_kanban_update_task",
        "Update the status of a specific Kanban task.",
        json!({
            "type": "object",
            "properties": {
                "filename": { "type": "string", "description": "The Markdown file name of the task" },
                "status": { "type": "string", "enum": ["Plan", "YOLO mode", "Done by AI", "Verified", "Re-open"], "description": "The new status" }
            },
            "required": ["filename", "status"]
        }),
    );
}

fn get_tasks_dir() -> PathBuf {
    // Relative to the NDE-OS root where core/ runs
    Path::new(".agents").join("tasks")
}

pub fn execute(tool_name: &str, params: &serde_json::Value) -> Result<String> {
    match tool_name {
        "nde_kanban_get_tasks" => {
            let dir = get_tasks_dir();
            if !dir.exists() {
                return Ok(json!([]).to_string());
            }

            let mut tasks = Vec::new();
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("md") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                            let mut title = filename.clone();
                            let mut status = "Plan".to_string();

                            for line in content.lines() {
                                if line.starts_with("# ") {
                                    title = line[2..].trim().to_string();
                                } else if line.contains("- **Status:**") {
                                    let l = line.to_lowercase();
                                    if l.contains("🟢") || l.contains("done") {
                                        status = "Done by AI".to_string();
                                    } else if l.contains("🟡") || l.contains("yolo") {
                                        status = "YOLO mode".to_string();
                                    } else if l.contains("✅") || l.contains("verified") {
                                        status = "Verified".to_string();
                                    } else if l.contains("re-open") {
                                        status = "Re-open".to_string();
                                    } else {
                                        status = "Plan".to_string();
                                    }
                                }
                            }

                            let locked = status == "YOLO mode";
                            tasks.push(KanbanTask {
                                filename,
                                title,
                                status,
                                locked,
                            });
                        }
                    }
                }
            }

            Ok(serde_json::to_string(&tasks)?)
        }
        "nde_kanban_update_task" => {
            let args = params.get("arguments").ok_or_else(|| anyhow!("Missing arguments"))?;
            
            let filename = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
            let new_status = args.get("status").and_then(|v| v.as_str()).unwrap_or("");

            if filename.is_empty() || filename.contains("..") || filename.contains("/") || filename.contains("\\") {
                return Err(anyhow!("Invalid filename"));
            }

            let filepath = get_tasks_dir().join(filename);
            if !filepath.exists() {
                return Err(anyhow!("File not found"));
            }

            let content = fs::read_to_string(&filepath)?;
            let emoji_status = match new_status {
                "YOLO mode" => "🟡 `yolo mode`",
                "Done by AI" => "🟢 `done by AI`",
                "Verified" => "✅ `verified`",
                "Re-open" => "🔴 `re-open`",
                _ => "🔴 `plan`",
            };

            let replacement = format!("- **Status:** {}", emoji_status);
            let mut lines: Vec<&str> = content.lines().collect();
            let mut modified = false;

            for line in lines.iter_mut() {
                if line.starts_with("- **Status:**") {
                    *line = replacement.as_str();
                    modified = true;
                }
            }

            if modified {
                let mut new_content = lines.join("\n");
                if content.ends_with("\n") {
                    new_content.push('\n');
                }
                fs::write(&filepath, new_content)?;
                Ok(json!({"success": true, "message": format!("Updated {} to {}", filename, new_status)}).to_string())
            } else {
                Err(anyhow!("Status line not found in file"))
            }
        }
        _ => Err(anyhow!("Unknown tool: {}", tool_name)),
    }
}
