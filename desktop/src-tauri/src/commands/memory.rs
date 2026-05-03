use ai_launcher_core::memory::types::{AgentId, MemoryFragment};
use ai_launcher_core::memory::MemorySubstrate;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;

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

    let memory = MemorySubstrate::open(&db_path).map_err(|e| e.to_string())?;
    
    // In a real implementation this might be more complex
    let db_size_bytes = std::fs::metadata(&db_path)
        .map(|m| m.len())
        .unwrap_or(0);
        
    Ok(MemoryStatus {
        rows: 0, // Mock rows count if we can't easily query
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
    
    // Default agent id for NDE-OS
    let agent_id = AgentId(uuid::Uuid::nil());
    
    let fragments = memory.semantic.recall(agent_id, &query, limit.unwrap_or(5)).map_err(|e| e.to_string())?;
    
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
    
    let agent_id = AgentId(uuid::Uuid::nil());
    
    memory.semantic.remember(agent_id, &content, &source, "ephemeral", None).map_err(|e| e.to_string())?;
    
    Ok(())
}
