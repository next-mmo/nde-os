use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

use super::server::McpServer;

#[derive(Serialize)]
pub struct KanbanTask {
    pub id: u32,
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
                "filename": { "type": "string", "description": "Task identifier: either a NDE ID (e.g. 'NDE-5') or the .md filename (e.g. 'my-task.md')" },
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
                "filename": { "type": "string", "description": "Task identifier: either a NDE ID (e.g. 'NDE-5') or the .md filename" }
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
                "filename": { "type": "string", "description": "Task identifier: either a NDE ID (e.g. 'NDE-5') or the .md filename" }
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
                "filename": { "type": "string", "description": "Task identifier: either a NDE ID (e.g. 'NDE-5') or the .md filename" },
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

/// Parse `- **ID:** NDE-<n>` from task content. Returns None if not found.
fn parse_id(content: &str) -> Option<u32> {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("- **ID:** NDE-") {
            if let Ok(n) = rest.trim().parse::<u32>() {
                return Some(n);
            }
        }
    }
    None
}

/// Read the next available ID from `.next_id` counter file, increment and save.
fn next_task_id(tasks_dir: &std::path::Path) -> Result<u32> {
    let counter_path = tasks_dir.join(".next_id");
    let current = if counter_path.exists() {
        fs::read_to_string(&counter_path)?
            .trim()
            .parse::<u32>()
            .unwrap_or(1)
    } else {
        1
    };
    fs::write(&counter_path, (current + 1).to_string())
        .map_err(|e| anyhow!("Failed to write .next_id: {e}"))?;
    Ok(current)
}

/// Inject `- **ID:** NDE-<n>` after the title line in a task file.
fn inject_id_into_content(content: &str, id: u32) -> String {
    let id_line = format!("- **ID:** NDE-{}", id);
    let mut lines: Vec<&str> = content.lines().collect();

    if let Some(title_pos) = lines.iter().position(|l| l.starts_with("# ")) {
        let insert_at = if title_pos + 1 < lines.len() && lines[title_pos + 1].is_empty() {
            title_pos + 2
        } else {
            title_pos + 1
        };
        lines.insert(insert_at, &id_line);
    } else {
        lines.insert(0, &id_line);
    }

    let mut result = lines.join("\n");
    if content.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Resolve a filename or NDE-ID (e.g. "NDE-5") to the actual .md filename on disk.
fn resolve_filename(input: &str, tasks_dir: &std::path::Path) -> Result<String> {
    // If it already looks like a .md filename and exists, use it directly
    if input.ends_with(".md") {
        if tasks_dir.join(input).exists() {
            return Ok(input.to_string());
        }
        return Err(anyhow!("Task not found: {}", input));
    }

    // Try to parse NDE-N or just N as an ID
    let id_str = input
        .strip_prefix("NDE-")
        .or_else(|| input.strip_prefix("nde-"))
        .unwrap_or(input);

    if let Ok(target_id) = id_str.parse::<u32>() {
        // Scan all .md files for one containing `- **ID:** NDE-{target_id}`
        if let Ok(entries) = fs::read_dir(tasks_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Some(id) = parse_id(&content) {
                            if id == target_id {
                                return Ok(path.file_name().unwrap_or_default().to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }
        return Err(anyhow!("No task found with ID: NDE-{}", target_id));
    }

    // Fallback: try appending .md
    let with_ext = format!("{}.md", input);
    if tasks_dir.join(&with_ext).exists() {
        return Ok(with_ext);
    }

    Err(anyhow!("Task not found: {}", input))
}

pub fn execute(tool_name: &str, params: &serde_json::Value) -> Result<String> {
    match tool_name {
        "nde_kanban_get_tasks" => {
            let dir = get_tasks_dir();
            if !dir.exists() {
                return Ok(json!([]).to_string());
            }

            let mut tasks = Vec::new();
            if let Ok(entries) = fs::read_dir(&dir) {
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

                            // Parse or auto-assign ID
                            let id = match parse_id(&content) {
                                Some(id) => id,
                                None => {
                                    let new_id = next_task_id(&dir).unwrap_or(0);
                                    let updated = inject_id_into_content(&content, new_id);
                                    let _ = fs::write(&path, updated);
                                    new_id
                                }
                            };

                            tasks.push(KanbanTask {
                                id,
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
                "# {}\n\n- **ID:** NDE-{}\n- **Status:** 🔴 `plan`\n- **Created:** {}\n\n## Description\n\n{}\n\n## Checklist\n\n{}",
                title, next_task_id(&dir).unwrap_or(0), now, desc, checklist_md
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

            let raw_filename = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
            let new_status = args.get("status").and_then(|v| v.as_str()).unwrap_or("");

            if raw_filename.is_empty() {
                return Err(anyhow!("Missing filename or task ID"));
            }

            let dir = get_tasks_dir();
            let filename = resolve_filename(raw_filename, &dir)?;

            if filename.contains("..") || filename.contains("/") || filename.contains("\\") {
                return Err(anyhow!("Invalid filename"));
            }

            let filepath = dir.join(&filename);

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
            let raw_filename = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");

            if raw_filename.is_empty() {
                return Err(anyhow!("Missing filename or task ID"));
            }

            let dir = get_tasks_dir();
            let filename = resolve_filename(raw_filename, &dir)?;

            if filename.contains("..") || filename.contains("/") || filename.contains("\\") {
                return Err(anyhow!("Invalid filename"));
            }

            let filepath = dir.join(&filename);
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
            let raw_filename = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");

            if raw_filename.is_empty() {
                return Err(anyhow!("Missing filename or task ID"));
            }

            let dir = get_tasks_dir();
            let filename = resolve_filename(raw_filename, &dir)?;

            let filepath = dir.join(&filename);
            if filepath.exists() {
                let content = fs::read_to_string(&filepath)?;
                Ok(content)
            } else {
                Err(anyhow!("Task not found: {}", filename))
            }
        }
        "nde_kanban_update_task_content" => {
            let args = params.get("arguments").unwrap_or(params);
            let raw_filename = args.get("filename").and_then(|v| v.as_str()).unwrap_or("");
            let new_content = args
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Missing content"))?;

            if raw_filename.is_empty() {
                return Err(anyhow!("Missing filename or task ID"));
            }

            let dir = get_tasks_dir();
            let filename = resolve_filename(raw_filename, &dir)?;

            let filepath = dir.join(&filename);
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
