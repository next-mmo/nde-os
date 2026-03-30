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
        "nde_kanban_create_task",
        "Create a new Kanban task ticket (markdown file in .agents/tasks/).",
        json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "The task title" },
                "description": { "type": "string", "description": "Detailed description of the task" },
                "checklist": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional checklist items for the task"
                }
            },
            "required": ["title"]
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

    server.register_tool(
        "nde_kanban_delete_task",
        "Delete a Kanban task ticket.",
        json!({
            "type": "object",
            "properties": {
                "filename": { "type": "string", "description": "The Markdown file name of the task to delete" }
            },
            "required": ["filename"]
        }),
    );

    server.register_tool(
        "nde_kanban_get_task_content",
        "Retrieve the full markdown content of a Kanban task.",
        json!({
            "type": "object",
            "properties": {
                "filename": { "type": "string", "description": "The Markdown file name of the task" }
            },
            "required": ["filename"]
        }),
    );

    server.register_tool(
        "nde_kanban_update_task_content",
        "Update the full markdown content of a Kanban task.",
        json!({
            "type": "object",
            "properties": {
                "filename": { "type": "string", "description": "The Markdown file name of the task" },
                "content": { "type": "string", "description": "The new Markdown content for the task" }
            },
            "required": ["filename", "content"]
        }),
    );
}

fn get_tasks_dir() -> PathBuf {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let mut candidate = current_dir.as_path();
    loop {
        let agents_path = candidate.join(".agents").join("tasks");
        if agents_path.is_dir() {
            return agents_path;
        }
        match candidate.parent() {
            Some(parent) => candidate = parent,
            None => break,
        }
    }

    current_dir.join(".agents").join("tasks")
}

fn classify_status(val: &str) -> &'static str {
    if val.contains('🟢') || val.to_lowercase().contains("done by ai") {
        "Done by AI"
    } else if val.contains('🟡') || val.to_lowercase().contains("yolo") {
        "YOLO mode"
    } else if val.to_lowercase().contains("waiting") {
        "Waiting Approval"
    } else if val.contains('✅') || val.to_lowercase().contains("verified") {
        "Verified"
    } else if val.to_lowercase().contains("re-open") {
        "Re-open"
    } else {
        "Plan"
    }
}

fn parse_status(content: &str) -> &'static str {
    if content.starts_with("---") {
        let after = &content[3..];
        if let Some(end) = after.find("\n---") {
            let frontmatter = &after[..end];
            for line in frontmatter.lines() {
                let line = line.trim();
                if let Some(val) = line.strip_prefix("status:") {
                    return classify_status(val.trim());
                }
            }
        }
    }
    if let Some(line) = content.lines().find(|l| l.starts_with("- **Status:**")) {
        return classify_status(line);
    }
    "Plan"
}

fn update_legacy_status_line(content: &str, fm_status: &str) -> String {
    let replacement = format!("- **Status:** {fm_status}");
    let mut updated: String = content
        .lines()
        .map(|line| {
            if line.starts_with("- **Status:**") {
                replacement.clone()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if content.ends_with('\n') && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated
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
                    if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            let filename = path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();

                            let title = content
                                .lines()
                                .find(|line| line.starts_with("# "))
                                .map(|line| line.trim_start_matches("# ").trim().to_string())
                                .unwrap_or_else(|| filename.clone());

                            let status = parse_status(&content).to_string();
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
        "nde_kanban_create_task" => {
            let args = params.get("arguments").unwrap_or(params);

            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing required field: title"))?;

            let description = args
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let checklist: Vec<String> = args
                .get("checklist")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let dir = get_tasks_dir();
            fs::create_dir_all(&dir).map_err(|e| anyhow!("Failed to create tasks dir: {e}"))?;

            // Generate slug filename
            let slug: String = title
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect::<String>()
                .split('-')
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("-");

            let mut filename = format!("{}.md", slug);
            let mut counter = 2u32;
            while dir.join(&filename).exists() {
                filename = format!("{}-{}.md", slug, counter);
                counter += 1;
            }

            // Build checklist markdown
            let checklist_md = if checklist.is_empty() {
                "- [ ] Implement the feature\n- [ ] Test the implementation\n".to_string()
            } else {
                checklist
                    .iter()
                    .map(|item| format!("- [ ] {}", item))
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n"
            };

            let desc = if description.is_empty() {
                title
            } else {
                description
            };
            let epoch_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let days = epoch_secs / 86400;
            let now = format!(
                "{}-{:02}-{:02}",
                1970 + days / 365,
                (days % 365 / 30 + 1).min(12),
                (days % 365 % 30 + 1).min(31)
            );

            let content = format!(
                "# {}\n\n- **Status:** 🔴 `plan`\n- **Created:** {}\n\n## Description\n\n{}\n\n## Checklist\n\n{}",
                title, now, desc, checklist_md
            );

            let filepath = dir.join(&filename);
            fs::write(&filepath, &content)?;

            Ok(json!({
                "success": true,
                "filename": filename,
                "title": title,
                "message": format!("Created task: {}", title)
            })
            .to_string())
        }
        "nde_kanban_update_task" => {
            let args = params.get("arguments").unwrap_or(params);

            let filename = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
            let new_status = args.get("status").and_then(|v| v.as_str()).unwrap_or("");

            if filename.is_empty()
                || filename.contains("..")
                || filename.contains("/")
                || filename.contains("\\")
            {
                return Err(anyhow!("Invalid filename"));
            }

            let filepath = get_tasks_dir().join(filename);
            if !filepath.exists() {
                return Err(anyhow!("File not found"));
            }

            let content = fs::read_to_string(&filepath)?;
            let fm_status = match new_status {
                "Waiting Approval" => "🔴 waiting approval",
                "YOLO mode" => "🟡 yolo mode",
                "Done by AI" => "🟢 done by AI",
                "Verified" | "Verified by Human" => "✅ verified",
                "Re-open" => "🔴 re-open",
                _ => "🔴 plan",
            };

            let updated_content = if content.starts_with("---") {
                let after = &content[3..];
                if let Some(end_offset) = after.find("\n---") {
                    let frontmatter = &after[..end_offset];
                    let rest = &after[end_offset..]; // starts with \n---

                    let updated_fm: String = frontmatter
                        .lines()
                        .map(|line| {
                            if line.trim_start().starts_with("status:") {
                                format!("status: {fm_status}")
                            } else {
                                line.to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    let mut out = format!("---\n{updated_fm}{rest}");
                    if content.ends_with('\n') && !out.ends_with('\n') {
                        out.push('\n');
                    }
                    out
                } else {
                    update_legacy_status_line(&content, fm_status)
                }
            } else {
                update_legacy_status_line(&content, fm_status)
            };

            fs::write(&filepath, updated_content)?;
            Ok(json!({"success": true, "message": format!("Updated {} to {}", filename, new_status)}).to_string())
        }
        "nde_kanban_delete_task" => {
            let args = params.get("arguments").unwrap_or(params);
            let filename = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");

            if filename.is_empty()
                || filename.contains("..")
                || filename.contains("/")
                || filename.contains("\\")
            {
                return Err(anyhow!("Invalid filename"));
            }

            let filepath = get_tasks_dir().join(filename);
            if filepath.exists() {
                fs::remove_file(&filepath)?;
                Ok(
                    json!({"success": true, "message": format!("Deleted task: {}", filename)})
                        .to_string(),
                )
            } else {
                Err(anyhow!("Task not found: {}", filename))
            }
        }
        "nde_kanban_get_task_content" => {
            let args = params.get("arguments").unwrap_or(params);
            let filename = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");

            if filename.is_empty()
                || filename.contains("..")
                || filename.contains("/")
                || filename.contains("\\")
            {
                return Err(anyhow!("Invalid filename"));
            }

            let filepath = get_tasks_dir().join(filename);
            if filepath.exists() {
                let content = fs::read_to_string(&filepath)?;
                Ok(content)
            } else {
                Err(anyhow!("Task not found: {}", filename))
            }
        }
        "nde_kanban_update_task_content" => {
            let args = params.get("arguments").unwrap_or(params);
            let filename = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
            let new_content = args
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing content"))?;

            if filename.is_empty()
                || filename.contains("..")
                || filename.contains("/")
                || filename.contains("\\")
            {
                return Err(anyhow!("Invalid filename"));
            }

            let filepath = get_tasks_dir().join(filename);
            if !filepath.exists() {
                return Err(anyhow!("Task not found: {}", filename));
            }

            fs::write(&filepath, new_content)?;
            Ok(
                json!({"success": true, "message": format!("Updated content of {}", filename)})
                    .to_string(),
            )
        }
        _ => Err(anyhow!("Unknown tool: {}", tool_name)),
    }
}
