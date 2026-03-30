use crate::manifest::{AppManifest, AppStatus, InstalledApp};

#[test]
fn factory_gradio() {
    let m = AppManifest::sample_counter();
    assert_eq!(m.id, "sample-gradio");
    assert_eq!(m.port, 7860);
    assert!(!m.needs_gpu);
    assert!(!m.pip_deps.is_empty());
}

#[test]
fn factory_stable_diffusion() {
    let m = AppManifest::stable_diffusion();
    assert_eq!(m.id, "stable-diffusion-webui");
    assert!(m.needs_gpu);
    assert!(m.tags.contains(&"gpu".to_string()));
}

#[test]
fn factory_ollama() {
    let m = AppManifest::ollama();
    assert_eq!(m.id, "ollama");
    assert_eq!(m.port, 11434);
}

#[test]
fn serialization_roundtrip() {
    let m = AppManifest::sample_counter();
    let json = serde_json::to_string(&m).unwrap();
    let m2: AppManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(m.id, m2.id);
    assert_eq!(m.port, m2.port);
    assert_eq!(m.pip_deps, m2.pip_deps);
}

#[test]
fn all_fields_serialize() {
    let m = AppManifest::sample_counter();
    let val: serde_json::Value = serde_json::to_value(&m).unwrap();
    assert!(val.get("id").is_some());
    assert!(val.get("name").is_some());
    assert!(val.get("port").is_some());
    assert!(val.get("launch_cmd").is_some());
    assert!(val.get("tags").is_some());
}

#[test]
fn status_installed() {
    let s = AppStatus::Installed;
    let json = serde_json::to_string(&s).unwrap();
    assert!(json.contains("Installed"));
}

#[test]
fn status_running() {
    let s = AppStatus::Running {
        pid: 1234,
        port: 7860,
    };
    let json = serde_json::to_string(&s).unwrap();
    assert!(json.contains("Running"));
    assert!(json.contains("1234"));
    assert!(json.contains("7860"));
}

#[test]
fn status_error() {
    let s = AppStatus::Error {
        message: "boom".into(),
    };
    let json = serde_json::to_string(&s).unwrap();
    assert!(json.contains("Error"));
    assert!(json.contains("boom"));
}

#[test]
fn installed_app_roundtrip() {
    let app = InstalledApp {
        manifest: AppManifest::sample_counter(),
        status: AppStatus::Installed,
        workspace: "/tmp/test".to_string(),
        installed_at: Some("2025-01-01T00:00:00Z".to_string()),
        last_run: None,
    };
    let json = serde_json::to_string(&app).unwrap();
    let app2: InstalledApp = serde_json::from_str(&json).unwrap();
    assert_eq!(app.manifest.id, app2.manifest.id);
    assert_eq!(app.workspace, app2.workspace);
}

#[test]
fn python_cmd_cross_platform() {
    let cmd = AppManifest::python_cmd();
    if cfg!(windows) {
        assert_eq!(cmd, "python");
    } else {
        assert_eq!(cmd, "python3");
    }
}
