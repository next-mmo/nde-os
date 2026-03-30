use super::{temp_base, test_manifest};
use crate::app_manager::AppManager;
use crate::manifest::{SourceType, StoreUploadRequest};
use std::fs;
use std::path::Path;

/// Helper: create a folder with a valid manifest.json for upload testing
fn create_test_app_folder(base: &Path, app_id: &str) -> std::path::PathBuf {
    let folder = base.join(format!("{}-upload-src", app_id));
    fs::create_dir_all(&folder).unwrap();

    let manifest = serde_json::json!({
        "id": app_id,
        "name": format!("Test Upload {}", app_id),
        "description": "Test app for upload",
        "author": "test",
        "python_version": "3",
        "needs_gpu": false,
        "pip_deps": [],
        "launch_cmd": if cfg!(windows) { "python -c \"print('hello')\"" } else { "python3 -c \"print('hello')\"" },
        "port": 19999,
        "env": [],
        "disk_size": "1MB",
        "tags": ["test"]
    });

    fs::write(
        folder.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    // Write a dummy app file
    fs::write(folder.join("app.py"), "print('hello from upload test')").unwrap();

    folder
}

// ── Validation tests (no real install needed) ────────────────────────────

#[test]
fn folder_upload_rejects_missing_source_path() {
    let req = StoreUploadRequest {
        source_type: SourceType::Folder,
        source_path: None,
        git_url: None,
    };
    let errors = AppManager::validate_upload(&req);
    assert!(!errors.is_empty());
    assert_eq!(errors[0].field, "source_path");
}

#[test]
fn folder_upload_rejects_nonexistent_path() {
    let req = StoreUploadRequest {
        source_type: SourceType::Folder,
        source_path: Some("/nonexistent/path/does-not-exist".into()),
        git_url: None,
    };
    let errors = AppManager::validate_upload(&req);
    assert!(!errors.is_empty());
    assert!(errors[0].message.contains("does not exist"));
}

#[test]
fn zip_upload_rejects_missing_source_path() {
    let req = StoreUploadRequest {
        source_type: SourceType::Zip,
        source_path: None,
        git_url: None,
    };
    let errors = AppManager::validate_upload(&req);
    assert!(!errors.is_empty());
    assert_eq!(errors[0].field, "source_path");
}

#[test]
fn zip_upload_rejects_wrong_extension() {
    let tmp = temp_base();
    let bad_file = tmp.path().join("not-a-zip.tar");
    fs::write(&bad_file, "fake").unwrap();

    let req = StoreUploadRequest {
        source_type: SourceType::Zip,
        source_path: Some(bad_file.to_string_lossy().into()),
        git_url: None,
    };
    let errors = AppManager::validate_upload(&req);
    assert!(!errors.is_empty());
    assert!(errors[0].message.contains(".zip extension"));
}

#[test]
fn git_url_rejects_missing_url() {
    let req = StoreUploadRequest {
        source_type: SourceType::GitUrl,
        source_path: None,
        git_url: None,
    };
    let errors = AppManager::validate_upload(&req);
    assert!(!errors.is_empty());
    assert_eq!(errors[0].field, "git_url");
}

#[test]
fn git_url_rejects_non_http() {
    let req = StoreUploadRequest {
        source_type: SourceType::GitUrl,
        source_path: None,
        git_url: Some("ftp://github.com/user/repo.git".into()),
    };
    let errors = AppManager::validate_upload(&req);
    assert!(!errors.is_empty());
    assert!(errors[0].message.contains("http://"));
}

#[test]
fn git_url_rejects_unknown_host_without_git_ext() {
    let req = StoreUploadRequest {
        source_type: SourceType::GitUrl,
        source_path: None,
        git_url: Some("https://random-server.example.com/repo".into()),
    };
    let errors = AppManager::validate_upload(&req);
    assert!(!errors.is_empty());
    assert!(errors[0].message.contains("known host"));
}

#[test]
fn git_url_accepts_known_host() {
    let req = StoreUploadRequest {
        source_type: SourceType::GitUrl,
        source_path: None,
        git_url: Some("https://github.com/user/my-app".into()),
    };
    let errors = AppManager::validate_upload(&req);
    assert!(errors.is_empty());
}

#[test]
fn git_url_accepts_dot_git_extension() {
    let req = StoreUploadRequest {
        source_type: SourceType::GitUrl,
        source_path: None,
        git_url: Some("https://my-server.example.com/repos/app.git".into()),
    };
    let errors = AppManager::validate_upload(&req);
    assert!(errors.is_empty());
}

// ── Integration test: folder upload happy path ──────────────────────────

#[test]
fn folder_upload_validates_and_installs() {
    let tmp = temp_base();
    let base = tmp.path().join("store-upload-test");
    fs::create_dir_all(&base).unwrap();

    let mgr = AppManager::new(&base).unwrap();
    let app_folder = create_test_app_folder(tmp.path(), "upload-folder-test");

    let req = StoreUploadRequest {
        source_type: SourceType::Folder,
        source_path: Some(app_folder.to_string_lossy().into()),
        git_url: None,
    };

    let result = mgr.upload_to_store(&req).unwrap();
    assert!(result.accepted, "Upload should be accepted: {:?}", result);
    assert_eq!(result.app_id.as_deref(), Some("upload-folder-test"));
    assert!(result.validation_errors.is_empty());

    // App should be in registry now
    let app = mgr.get_app("upload-folder-test");
    assert!(app.is_some());

    // Clean up
    mgr.uninstall("upload-folder-test").ok();
}

#[test]
fn upload_rejects_missing_manifest_json() {
    let tmp = temp_base();
    let base = tmp.path().join("store-upload-no-manifest");
    fs::create_dir_all(&base).unwrap();

    let mgr = AppManager::new(&base).unwrap();

    // Create a folder without manifest.json
    let folder = tmp.path().join("no-manifest-app");
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("app.py"), "print('no manifest')").unwrap();

    let req = StoreUploadRequest {
        source_type: SourceType::Folder,
        source_path: Some(folder.to_string_lossy().into()),
        git_url: None,
    };

    let errors = AppManager::validate_upload(&req);
    assert!(!errors.is_empty());
    assert!(errors[0].message.contains("manifest.json"));
}
