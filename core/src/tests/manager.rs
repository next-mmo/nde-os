use crate::app_manager::AppManager;
use crate::manifest::AppStatus;
use super::{temp_base, test_manifest};

#[test]
fn creation() {
    let tmp = temp_base();
    assert!(AppManager::new(tmp.path()).is_ok());
}

#[test]
fn empty_initially() {
    let tmp = temp_base();
    let mgr = AppManager::new(tmp.path()).unwrap();
    assert_eq!(mgr.total_count(), 0);
    assert_eq!(mgr.running_count(), 0);
    assert!(mgr.list_apps().is_empty());
}

#[test]
fn catalog_has_4_apps() {
    let tmp = temp_base();
    let mgr = AppManager::new(tmp.path()).unwrap();
    let cat = mgr.catalog();
    assert_eq!(cat.len(), 4);
    let ids: Vec<&str> = cat.iter().map(|a| a.id.as_str()).collect();
    assert!(ids.contains(&"sample-node"));
    assert!(ids.contains(&"sample-gradio"));
    assert!(ids.contains(&"stable-diffusion-webui"));
    assert!(ids.contains(&"ollama"));
}

#[test]
fn base_dir() {
    let tmp = temp_base();
    let mgr = AppManager::new(tmp.path()).unwrap();
    assert_eq!(mgr.base_dir(), tmp.path());
}

#[test]
fn uv_info_has_path() {
    let tmp = temp_base();
    let mgr = AppManager::new(tmp.path()).unwrap();
    assert!(mgr.uv_info().get("uv_path").is_some());
}

#[test]
fn get_app_none_when_empty() {
    let tmp = temp_base();
    let mgr = AppManager::new(tmp.path()).unwrap();
    assert!(mgr.get_app("no-such-app").is_none());
}

#[test]
fn launch_not_installed() {
    let tmp = temp_base();
    let mgr = AppManager::new(tmp.path()).unwrap();
    let err = mgr.launch("ghost").unwrap_err();
    assert!(err.to_string().contains("not installed"));
}

#[test]
fn stop_not_running() {
    let tmp = temp_base();
    let mgr = AppManager::new(tmp.path()).unwrap();
    let err = mgr.stop("ghost").unwrap_err();
    assert!(err.to_string().contains("not running"));
}

#[test]
fn install_and_get() {
    let tmp = temp_base();
    let mgr = AppManager::new(tmp.path()).unwrap();
    mgr.install(&test_manifest("inst-1")).unwrap();
    assert_eq!(mgr.total_count(), 1);
    let app = mgr.get_app("inst-1").unwrap();
    assert_eq!(app.manifest.id, "inst-1");
    assert_eq!(app.status, AppStatus::Installed);
    assert!(std::path::Path::new(&app.workspace).exists());
}

#[test]
fn install_duplicate_fails() {
    let tmp = temp_base();
    let mgr = AppManager::new(tmp.path()).unwrap();
    mgr.install(&test_manifest("dup")).unwrap();
    let err = mgr.install(&test_manifest("dup")).unwrap_err();
    assert!(err.to_string().contains("already installed"));
}

#[test]
fn install_two_apps() {
    let tmp = temp_base();
    let mgr = AppManager::new(tmp.path()).unwrap();
    mgr.install(&test_manifest("a")).unwrap();
    mgr.install(&test_manifest("b")).unwrap();
    assert_eq!(mgr.total_count(), 2);
    assert_eq!(mgr.list_apps().len(), 2);
}

#[test]
fn uninstall() {
    let tmp = temp_base();
    let mgr = AppManager::new(tmp.path()).unwrap();
    mgr.install(&test_manifest("rm")).unwrap();
    let ws = mgr.get_app("rm").unwrap().workspace.clone();
    mgr.uninstall("rm").unwrap();
    assert_eq!(mgr.total_count(), 0);
    assert!(mgr.get_app("rm").is_none());
    assert!(!std::path::Path::new(&ws).exists());
}

#[test]
fn registry_persists() {
    let tmp = temp_base();
    let base = tmp.path().to_path_buf();
    {
        let mgr = AppManager::new(&base).unwrap();
        mgr.install(&test_manifest("persist")).unwrap();
    }
    {
        let mgr = AppManager::new(&base).unwrap();
        assert_eq!(mgr.total_count(), 1);
        assert!(mgr.get_app("persist").is_some());
    }
}
