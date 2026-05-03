use ai_launcher_core::memory::types::{AgentId, MemoryFragment, MemorySource};
use ai_launcher_core::memory::MemorySubstrate;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Serialize)]
pub struct MemoryStatus {
    pub rows: usize,
    pub db_size_bytes: u64,
}

#[tauri::command]
pub async fn memory_status(
    app_handle: tauri::AppHandle,
) -> Result<MemoryStatus, String> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("data");

    let db_path = data_dir.join("memory.db");

    if !db_path.exists() {
        return Ok(MemoryStatus {
            rows: 0,
            db_size_bytes: 0,
        });
    }

    let db_size_bytes = std::fs::metadata(&db_path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(MemoryStatus {
        rows: 0,
        db_size_bytes,
    })
}

#[tauri::command]
pub async fn memory_recall(
    query: String,
    limit: Option<usize>,
    app_handle: tauri::AppHandle,
) -> Result<Vec<MemoryFragment>, String> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("data");

    let db_path = data_dir.join("memory.db");

    let memory = MemorySubstrate::open(&db_path).map_err(|e| e.to_string())?;

    let fragments = memory.semantic.recall(&query, limit.unwrap_or(5), None).map_err(|e| e.to_string())?;

    Ok(fragments)
}

#[tauri::command]
pub async fn memory_remember(
    content: String,
    source: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("data");

    let db_path = data_dir.join("memory.db");

    let memory = MemorySubstrate::open(&db_path).map_err(|e| e.to_string())?;

    let agent_id = AgentId::default();
    let mem_source = match source.as_str() {
        "user" => MemorySource::User,
        "conversation" => MemorySource::Conversation,
        _ => MemorySource::System,
    };

    memory.semantic.remember(agent_id, &content, mem_source, "ephemeral", HashMap::new()).map_err(|e| e.to_string())?;

    Ok(())
}
