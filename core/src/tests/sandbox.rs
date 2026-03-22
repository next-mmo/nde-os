use crate::sandbox::Sandbox;
use super::temp_base;
use std::path::Path;

#[test]
fn creation() {
    let tmp = temp_base();
    let ws = tmp.path().join("test-app").join("workspace");
    let sandbox = Sandbox::new(&ws).unwrap();
    assert!(sandbox.root().exists());
}

#[test]
fn init_creates_dirs() {
    let tmp = temp_base();
    let ws = tmp.path().join("init-test").join("workspace");
    let sandbox = Sandbox::new(&ws).unwrap();
    sandbox.init_workspace().unwrap();
    for dir in &["tmp", "config", "data", "models", "outputs", "logs"] {
        assert!(ws.join(dir).exists(), "Missing dir: {}", dir);
    }
}

#[test]
fn init_creates_info_file() {
    let tmp = temp_base();
    let ws = tmp.path().join("info-test").join("workspace");
    let sandbox = Sandbox::new(&ws).unwrap();
    sandbox.init_workspace().unwrap();
    let info_path = ws.join(".sandbox_info");
    assert!(info_path.exists());
    let content = std::fs::read_to_string(&info_path).unwrap();
    let val: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(val.get("platform").is_some());
    assert!(val.get("created_at").is_some());
}

#[test]
fn resolve_valid_path() {
    let tmp = temp_base();
    let ws = tmp.path().join("resolve-test").join("workspace");
    let sandbox = Sandbox::new(&ws).unwrap();
    sandbox.init_workspace().unwrap();
    assert!(sandbox.resolve(Path::new("data/file.txt")).is_ok());
}

#[test]
fn blocks_path_traversal() {
    let tmp = temp_base();
    let ws = tmp.path().join("traversal").join("workspace");
    let sandbox = Sandbox::new(&ws).unwrap();
    sandbox.init_workspace().unwrap();
    assert!(sandbox.resolve(Path::new("../../etc/passwd")).is_err());
    assert!(sandbox.resolve(Path::new("../../../etc/shadow")).is_err());
}

#[test]
fn blocks_absolute_path() {
    let tmp = temp_base();
    let ws = tmp.path().join("absolute").join("workspace");
    let sandbox = Sandbox::new(&ws).unwrap();
    sandbox.init_workspace().unwrap();
    let escape = if cfg!(windows) { "C:\\Windows\\System32" } else { "/etc/passwd" };
    assert!(sandbox.resolve(Path::new(escape)).is_err());
}

#[test]
fn blocks_windows_traversal() {
    let tmp = temp_base();
    let ws = tmp.path().join("win-trav").join("workspace");
    let sandbox = Sandbox::new(&ws).unwrap();
    sandbox.init_workspace().unwrap();
    assert!(sandbox.resolve(Path::new("..\\..\\Windows")).is_err());
}

#[test]
fn blocks_sensitive_filenames() {
    let tmp = temp_base();
    let ws = tmp.path().join("blocked").join("workspace");
    let sandbox = Sandbox::new(&ws).unwrap();
    sandbox.init_workspace().unwrap();
    assert!(sandbox.resolve(Path::new("data/.ssh")).is_err());
    assert!(sandbox.resolve(Path::new("data/.env")).is_err());
    assert!(sandbox.resolve(Path::new("data/.git")).is_err());
}

#[test]
fn verify_all_pass() {
    let tmp = temp_base();
    let ws = tmp.path().join("verify").join("workspace");
    let sandbox = Sandbox::new(&ws).unwrap();
    sandbox.init_workspace().unwrap();
    let r = sandbox.verify();
    assert!(r.path_traversal_blocked, "traversal should be blocked");
    assert!(r.absolute_escape_blocked, "absolute should be blocked");
    assert!(r.valid_path_works, "valid path should work");
    assert_eq!(r.platform, std::env::consts::OS);
}

#[test]
fn env_vars_present() {
    let tmp = temp_base();
    let ws = tmp.path().join("env").join("workspace");
    let sandbox = Sandbox::new(&ws).unwrap();
    sandbox.init_workspace().unwrap();
    let env = sandbox.env_vars();
    let keys: Vec<&str> = env.iter().map(|(k, _)| k.as_str()).collect();
    assert!(keys.contains(&"HOME"));
    assert!(keys.contains(&"USERPROFILE"));
    assert!(keys.contains(&"TMPDIR"));
    assert!(keys.contains(&"TEMP"));
    assert!(keys.contains(&"SANDBOX_ROOT"));
}

#[test]
fn disk_usage_grows() {
    let tmp = temp_base();
    let ws = tmp.path().join("disk").join("workspace");
    let sandbox = Sandbox::new(&ws).unwrap();
    sandbox.init_workspace().unwrap();
    let before = sandbox.disk_usage().unwrap();
    std::fs::write(ws.join("testfile.bin"), vec![42u8; 4096]).unwrap();
    let after = sandbox.disk_usage().unwrap();
    assert!(after > before, "expected growth: {} -> {}", before, after);
}

#[test]
fn root_path_correct() {
    let tmp = temp_base();
    let ws = tmp.path().join("root-test").join("workspace");
    let sandbox = Sandbox::new(&ws).unwrap();
    assert!(sandbox.root().to_string_lossy().contains("workspace"));
}
