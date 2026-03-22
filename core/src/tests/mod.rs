mod manifest;
mod sandbox;
mod manager;

use crate::manifest::AppManifest;

/// Create a minimal test manifest
pub fn test_manifest(id: &str) -> AppManifest {
    AppManifest {
        id: id.to_string(),
        name: format!("Test {}", id),
        description: "test app".to_string(),
        author: "test".to_string(),
        repo: None,
        python_version: "3".to_string(),
        needs_gpu: false,
        pip_deps: vec![],
        launch_cmd: if cfg!(windows) {
            "python -c \"import http.server\"".to_string()
        } else {
            "python3 -c \"import http.server\"".to_string()
        },
        port: 17860,
        env: vec![],
        disk_size: "1MB".to_string(),
        tags: vec!["test".to_string()],
    }
}

pub fn temp_base() -> tempfile::TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}
