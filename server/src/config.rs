/// Centralized configuration — single source of truth for paths and settings.
use std::path::PathBuf;

/// Cross-platform base directory for all NDE-OS data.
/// Called once at startup and passed around via `AppState`.
pub fn base_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("ai-launcher"))
            .unwrap_or_else(|_| PathBuf::from("C:\\ai-launcher-data"))
    } else {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"))
            .join(".ai-launcher")
    }
}
