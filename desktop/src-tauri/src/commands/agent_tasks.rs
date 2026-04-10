use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use super::workspace::WorkspaceState;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentTask {
    pub id: u32,
    pub filename: String,
    pub title: String,
    pub status: String,
    pub locked: bool,
}

/// Extract the `status:` value from YAML frontmatter (between the first pair of `---` lines).
/// Falls back to scanning `- **Status:**` markdown lines for legacy compatibility.
fn parse_status(content: &str) -> &'static str {
    // Try YAML frontmatter first
    if content.starts_with("---") {
        let after = &content[3..];
        if let Some(end) = after.find("\n---") {
            let frontmatter = &after[..end];
            for line in frontmatter.lines() {
                let line = line.trim();
                if let Some(val) = line.strip_prefix("status:") {
                    let val = val.trim();
                    return classify_status(val);
                }
            }
        }
    }
    // Legacy: `- **Status:** 🟢 ...`
    if let Some(line) = content.lines().find(|l| l.starts_with("- **Status:**")) {
        return classify_status(line);
    }
    "Plan"
}

fn classify_status(val: &str) -> &'static str {
    if val.contains('🟢') || val.to_lowercase().contains("done by ai") {
        "Done by AI"
    } else if val.contains('🟡') || val.to_lowercase().contains("yolo") {
        "YOLO mode"
    } else if val.to_lowercase().contains("waiting") {
        "Waiting Approval"
    } else if val.contains('✅') || val.to_lowercase().contains("verified") {
        "Verified by Human"
    } else if val.to_lowercase().contains("re-open") {
        "Re-open"
    } else {
        "Plan"
    }
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
fn next_task_id(tasks_dir: &std::path::Path) -> Result<u32, String> {
    let counter_path = tasks_dir.join(".next_id");
    let current = if counter_path.exists() {
        fs::read_to_string(&counter_path)
            .map_err(|e| e.to_string())?
            .trim()
            .parse::<u32>()
            .unwrap_or(1)
    } else {
        1
    };
    fs::write(&counter_path, (current + 1).to_string())
        .map_err(|e| format!("Failed to write .next_id: {e}"))?;
    Ok(current)
}

/// Inject `- **ID:** NDE-<n>` after the title line in a task file.
fn inject_id_into_content(content: &str, id: u32) -> String {
    let id_line = format!("- **ID:** NDE-{}", id);
    let mut lines: Vec<&str> = content.lines().collect();

    // Find the title line and insert ID right after it (and any blank line)
    if let Some(title_pos) = lines.iter().position(|l| l.starts_with("# ")) {
        let insert_at = if title_pos + 1 < lines.len() && lines[title_pos + 1].is_empty() {
            title_pos + 2
        } else {
            title_pos + 1
        };
        lines.insert(insert_at, &id_line);
    } else {
        // No title found, prepend
        lines.insert(0, &id_line);
    }

    let mut result = lines.join("\n");
    if content.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

#[tauri::command]
pub async fn get_agent_tasks(
    workspace_state: tauri::State<'_, WorkspaceState>,
) -> Result<Vec<AgentTask>, String> {
    let mut tasks = Vec::new();
    let tasks_dir = workspace_state.tasks_dir();

    if !tasks_dir.exists() {
        return Ok(tasks);
    }

    let entries = fs::read_dir(&tasks_dir).map_err(|e| format!("read_dir failed: {e}"))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
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

                let status = parse_status(&content);
                let locked = status == "YOLO mode";

                // Parse or auto-assign ID
                let id = match parse_id(&content) {
                    Some(id) => id,
                    None => {
                        // Auto-assign ID to legacy tasks
                        let new_id = next_task_id(&tasks_dir)?;
                        let updated = inject_id_into_content(&content, new_id);
                        let _ = fs::write(&path, updated);
                        new_id
                    }
                };

                tasks.push(AgentTask {
                    id,
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
pub async fn update_agent_task_status(
    app: AppHandle,
    workspace_state: tauri::State<'_, WorkspaceState>,
    filename: String,
    new_status: String,
) -> Result<(), String> {
    let tasks_dir = workspace_state.tasks_dir();
    let file_path = tasks_dir.join(&filename);

    // Sandbox: prevent path traversal
    if !file_path.starts_with(&tasks_dir) {
        return Err("Invalid filename traversal attempt".into());
    }

    let content = fs::read_to_string(&file_path).map_err(|e| e.to_string())?;

    // Map UI column name → frontmatter status string
    let fm_status = match new_status.as_str() {
        "Waiting Approval" => "🔴 waiting approval",
        "YOLO mode" => "🟡 yolo mode",
        "Done by AI" => "🟢 done by AI",
        "Verified by Human" => "✅ verified by human",
        "Re-open" => "🔴 re-open",
        _ => "🔴 plan",
    };

    let new_content = if content.starts_with("---") {
        // Update YAML frontmatter `status:` line
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

            // Preserve trailing newline
            let mut out = format!("---\n{updated_fm}{rest}");
            if content.ends_with('\n') && !out.ends_with('\n') {
                out.push('\n');
            }
            out
        } else {
            // Malformed frontmatter — fall through to legacy update
            update_legacy_status_line(&content, fm_status)
        }
    } else {
        // Legacy markdown `- **Status:**` lines
        update_legacy_status_line(&content, fm_status)
    };

    fs::write(&file_path, new_content).map_err(|e| e.to_string())?;
    let _ = app.emit("tasks://updated", ());

    Ok(())
}

/// Fallback: rewrite a `- **Status:**` markdown line.
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

/// Create a new agent task ticket file in `.agents/tasks/`.
///
/// If `content` is non-empty, it is written as-is (LLM-generated full markdown).
/// Otherwise a minimal skeleton is generated from title/description/checklist.
#[tauri::command]
pub async fn create_agent_task(
    app: AppHandle,
    workspace_state: tauri::State<'_, WorkspaceState>,
    title: String,
    description: String,
    checklist: Vec<String>,
    content: Option<String>,
) -> Result<AgentTask, String> {
    let tasks_dir = workspace_state.tasks_dir();
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

    // Use LLM-provided content if available, otherwise minimal skeleton
    let file_content = if let Some(ref c) = content {
        if !c.trim().is_empty() {
            c.clone()
        } else {
            build_minimal_ticket(&title, &description, &checklist, &tasks_dir)
        }
    } else {
        build_minimal_ticket(&title, &description, &checklist, &tasks_dir)
    };

    let file_path = tasks_dir.join(&filename);
    fs::write(&file_path, &file_content).map_err(|e| format!("Failed to write task: {e}"))?;

    let _ = app.emit("tasks://updated", ());

    // Parse the assigned ID from the content we just wrote
    let id = parse_id(&file_content).unwrap_or(0);

    Ok(AgentTask {
        id,
        filename,
        title,
        status: "Plan".to_string(),
        locked: false,
    })
}

/// Minimal fallback ticket template — used ONLY when no LLM-generated content is provided
/// (e.g. quick-add from the UI + button). The LLM owns the real template format.
fn build_minimal_ticket(title: &str, description: &str, checklist: &[String], tasks_dir: &std::path::Path) -> String {
    let desc = if description.is_empty() {
        title
    } else {
        description
    };
    let checklist_str = if checklist.is_empty() {
        "- [ ] Implement the feature\n- [ ] Test the implementation\n".to_string()
    } else {
        checklist
            .iter()
            .map(|item| format!("- [ ] {}", item))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    };

    // Assign a sequential ID
    let id = next_task_id(tasks_dir).unwrap_or(0);

    format!(
        "# {title}\n\n- **ID:** NDE-{id}\n- **Status:** 🔴 `plan`\n\n## Description\n\n{desc}\n\n## Checklist\n\n{checklist_str}"
    )
}

#[tauri::command]
pub async fn delete_agent_task(
    app: AppHandle,
    workspace_state: tauri::State<'_, WorkspaceState>,
    filename: String,
) -> Result<(), String> {
    let tasks_dir = workspace_state.tasks_dir();
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

#[tauri::command]
pub async fn get_agent_task_content(
    workspace_state: tauri::State<'_, WorkspaceState>,
    filename: String,
) -> Result<String, String> {
    let tasks_dir = workspace_state.tasks_dir();
    let file_path = tasks_dir.join(&filename);

    if !file_path.starts_with(&tasks_dir) {
        return Err("Invalid filename traversal attempt".into());
    }

    if file_path.exists() {
        fs::read_to_string(&file_path).map_err(|e| e.to_string())
    } else {
        Err(format!("Task not found: {}", filename))
    }
}

#[tauri::command]
pub async fn update_agent_task_content(
    app: AppHandle,
    workspace_state: tauri::State<'_, WorkspaceState>,
    filename: String,
    content: String,
) -> Result<(), String> {
    let tasks_dir = workspace_state.tasks_dir();
    let file_path = tasks_dir.join(&filename);

    if !file_path.starts_with(&tasks_dir) {
        return Err("Invalid filename traversal attempt".into());
    }

    if !file_path.exists() {
        return Err(format!("Task not found: {}", filename));
    }

    fs::write(&file_path, content).map_err(|e| e.to_string())?;
    let _ = app.emit("tasks://updated", ());

    Ok(())
}

static WATCHER_STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[tauri::command]
pub async fn watch_tasks_dir(
    app: AppHandle,
    workspace_state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    use notify::{EventKind, RecursiveMode, Watcher};
    use std::sync::atomic::Ordering;

    if WATCHER_STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Ok(()); // Already started
    }

    let tasks_dir = workspace_state.tasks_dir();
    fs::create_dir_all(&tasks_dir)
        .map_err(|e| format!("Failed to create tasks dir for watcher: {e}"))?;

    let app_clone = app.clone();

    // Spawn a thread to keep the watcher alive and receive events
    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = match notify::recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                log::error!("Failed to create tasks watcher: {:?}", e);
                WATCHER_STARTED.store(false, Ordering::SeqCst);
                return;
            }
        };

        if let Err(e) = watcher.watch(&tasks_dir, RecursiveMode::NonRecursive) {
            log::error!("Failed to watch tasks directory: {:?}", e);
            WATCHER_STARTED.store(false, Ordering::SeqCst);
            return;
        }

        while let Ok(res) = rx.recv() {
            match res {
                Ok(event) => match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        let _ = app_clone.emit("tasks://updated", ());
                    }
                    _ => {}
                },
                Err(e) => log::error!("Watch error: {:?}", e),
            }
        }
    });

    Ok(())
}
