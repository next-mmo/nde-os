/// Kanban REST API handlers — direct execution via core::mcp::kanban.
///
/// Extracted from inline route logic in main.rs for modularity.
use crate::response::*;
use tiny_http::Request;

/// Handle a simple kanban tool call with optional request body.
pub fn execute_tool(
    tool_name: &str,
    req: Option<&mut Request>,
) -> HttpResponse {
    let params = if let Some(req) = req {
        match parse_body::<serde_json::Value>(req) {
            Ok(v) => v,
            Err(_) => {
                // No body or invalid — use empty params
                serde_json::json!({})
            }
        }
    } else {
        serde_json::json!({})
    };

    match ai_launcher_core::mcp::kanban::execute(tool_name, &params) {
        Ok(result) => {
            let data = serde_json::from_str::<serde_json::Value>(&result)
                .unwrap_or_else(|_| serde_json::json!({ "result": result }));
            ok("Kanban operation successful", data)
        }
        Err(e) => err(500, &format!("Kanban error: {}", e)),
    }
}

/// GET /api/kanban/tasks/{filename}/content
pub fn get_task_content(filename: &str) -> HttpResponse {
    let params = serde_json::json!({ "filename": filename });
    match ai_launcher_core::mcp::kanban::execute("nde_kanban_get_task_content", &params) {
        Ok(content) => ok("Task content", serde_json::json!({ "content": content })),
        Err(e) => err(404, &format!("Task not found: {}", e)),
    }
}

/// PUT /api/kanban/tasks/{filename}/content
pub fn update_task_content(filename: &str, req: &mut Request) -> HttpResponse {
    let body_json: serde_json::Value = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let params = serde_json::json!({
        "filename": filename,
        "content": body_json.get("content").and_then(|v| v.as_str()).unwrap_or("")
    });
    match ai_launcher_core::mcp::kanban::execute("nde_kanban_update_task_content", &params) {
        Ok(result) => ok("Task content updated", serde_json::json!({ "result": result })),
        Err(e) => err(500, &format!("Failed to update: {}", e)),
    }
}

/// PUT /api/kanban/tasks/{filename} — update task status
pub fn update_task(filename: &str, req: &mut Request) -> HttpResponse {
    let body_json: serde_json::Value = match parse_body(req) {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    let params = serde_json::json!({
        "filename": filename,
        "status": body_json.get("status").and_then(|v| v.as_str()).unwrap_or("Plan")
    });
    match ai_launcher_core::mcp::kanban::execute("nde_kanban_update_task", &params) {
        Ok(result) => ok("Task updated", serde_json::json!({ "result": result })),
        Err(e) => err(500, &format!("Failed to update: {}", e)),
    }
}

/// DELETE /api/kanban/tasks/{filename}
pub fn delete_task(filename: &str) -> HttpResponse {
    let params = serde_json::json!({ "filename": filename });
    match ai_launcher_core::mcp::kanban::execute("nde_kanban_delete_task", &params) {
        Ok(result) => ok("Task deleted", serde_json::json!({ "result": result })),
        Err(e) => err(404, &format!("Failed to delete: {}", e)),
    }
}
