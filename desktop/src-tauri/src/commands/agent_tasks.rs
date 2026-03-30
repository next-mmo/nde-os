use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentTask {
    pub filename: String,
    pub title: String,
    pub status: String,
    pub locked: bool,
}

/// Resolve the `.agents/tasks/` directory from CWD.
/// During `cargo tauri dev`, the binary runs from `desktop/src-tauri/`.
/// In production the binary runs from wherever it's installed.
/// We walk up until we find a directory containing `.agents/`.
fn tasks_dir() -> Result<PathBuf, String> {
    let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;

    // Walk up the directory tree to find the repo root (contains .agents/)
    let mut candidate = current_dir.as_path();
    loop {
        let agents_path = candidate.join(".agents").join("tasks");
        if agents_path.is_dir() {
            return Ok(agents_path);
        }
        match candidate.parent() {
            Some(parent) => candidate = parent,
            None => break,
        }
    }

    // Fallback: use CWD/.agents/tasks (will be created if needed)
    Ok(current_dir.join(".agents").join("tasks"))
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
pub async fn update_agent_task_status(
    app: AppHandle,
    filename: String,
    new_status: String,
) -> Result<(), String> {
    let tasks_dir = tasks_dir()?;
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
    title: String,
    description: String,
    checklist: Vec<String>,
    content: Option<String>,
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

    // Use LLM-provided content if available, otherwise minimal skeleton
    let file_content = if let Some(ref c) = content {
        if !c.trim().is_empty() {
            c.clone()
        } else {
            build_minimal_ticket(&title, &description, &checklist)
        }
    } else {
        build_minimal_ticket(&title, &description, &checklist)
    };

    let file_path = tasks_dir.join(&filename);
    fs::write(&file_path, &file_content).map_err(|e| format!("Failed to write task: {e}"))?;

    let _ = app.emit("tasks://updated", ());

    Ok(AgentTask {
        filename,
        title,
        status: "Plan".to_string(),
        locked: false,
    })
}

/// Minimal fallback ticket template — used ONLY when no LLM-generated content is provided
/// (e.g. quick-add from the UI + button). The LLM owns the real template format.
fn build_minimal_ticket(title: &str, description: &str, checklist: &[String]) -> String {
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

    format!(
        "# {title}\n\n- **Status:** 🔴 `plan`\n\n## Description\n\n{desc}\n\n## Checklist\n\n{checklist_str}"
    )
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

#[tauri::command]
pub async fn get_agent_task_content(filename: String) -> Result<String, String> {
    let tasks_dir = tasks_dir()?;
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
    filename: String,
    content: String,
) -> Result<(), String> {
    let tasks_dir = tasks_dir()?;
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
pub async fn watch_tasks_dir(app: AppHandle) -> Result<(), String> {
    use notify::{EventKind, RecursiveMode, Watcher};
    use std::sync::atomic::Ordering;

    if WATCHER_STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Ok(()); // Already started
    }

    let tasks_dir = tasks_dir()?;
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
