//! Integration tests for the Shield Browser subsystem.
//! Tests profile lifecycle, engine management, and launcher download URLs.
//! Real filesystem, no mocks.

use ai_launcher_core::shield::browser::BrowserEngine;
use ai_launcher_core::shield::engine::EngineManager;
use ai_launcher_core::shield::launcher;
use ai_launcher_core::shield::profile::{FingerprintConfig, ProfileManager, ShieldProfile};
use std::path::PathBuf;
use std::time::SystemTime;

fn temp_dir(prefix: &str) -> PathBuf {
    let ns = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("nde-shield-{}-{}", prefix, ns));
    std::fs::create_dir_all(&path).unwrap();
    path
}

// ═══════════════════════════════════════════════════════════════════
// Profile lifecycle — create, list, rename, delete, running state
// ═══════════════════════════════════════════════════════════════════

#[test]
fn shield_profile_full_lifecycle() {
    let dir = temp_dir("lifecycle");
    let mgr = ProfileManager::new(&dir);

    // Create two profiles with different engines
    let p1 = ShieldProfile::new("US Chrome".into(), BrowserEngine::Wayfern, "133.0.0".into());
    let p2 = ShieldProfile::new("EU Firefox".into(), BrowserEngine::Camoufox, "132.0.2".into());
    let id1 = p1.id.clone();
    let id2 = p2.id.clone();

    mgr.create_profile(&p1).unwrap();
    mgr.create_profile(&p2).unwrap();

    // List returns both
    let profiles = mgr.list_profiles().unwrap();
    assert_eq!(profiles.len(), 2);

    // Get by ID
    let fetched = mgr.get_profile(&id1).unwrap();
    assert_eq!(fetched.name, "US Chrome");
    assert_eq!(fetched.engine, BrowserEngine::Wayfern);
    assert_eq!(fetched.engine_version, "133.0.0");
    assert!(!fetched.is_running());

    // Rename
    let renamed = mgr.rename_profile(&id1, "US Chrome Pro").unwrap();
    assert_eq!(renamed.name, "US Chrome Pro");

    // Verify rename persisted
    let fetched2 = mgr.get_profile(&id1).unwrap();
    assert_eq!(fetched2.name, "US Chrome Pro");

    // Set running + verify
    mgr.set_running(&id1, 54321).unwrap();
    let running = mgr.get_profile(&id1).unwrap();
    assert!(running.is_running());
    assert_eq!(running.process_id, Some(54321));
    assert!(running.last_launch.is_some());

    // Cannot delete while running
    assert!(mgr.delete_profile(&id1).is_err());

    // Stop + delete
    mgr.set_stopped(&id1).unwrap();
    let stopped = mgr.get_profile(&id1).unwrap();
    assert!(!stopped.is_running());

    mgr.delete_profile(&id1).unwrap();
    assert_eq!(mgr.list_profiles().unwrap().len(), 1);

    // Delete second profile
    mgr.delete_profile(&id2).unwrap();
    assert!(mgr.list_profiles().unwrap().is_empty());

    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn shield_profile_duplicate_name_case_insensitive() {
    let dir = temp_dir("dup-name");
    let mgr = ProfileManager::new(&dir);

    let p1 = ShieldProfile::new("My Profile".into(), BrowserEngine::Wayfern, "1.0".into());
    mgr.create_profile(&p1).unwrap();

    // Same name different case should fail
    let p2 = ShieldProfile::new("my profile".into(), BrowserEngine::Camoufox, "2.0".into());
    assert!(mgr.create_profile(&p2).is_err());

    // Different name should succeed
    let p3 = ShieldProfile::new("Other Profile".into(), BrowserEngine::Camoufox, "2.0".into());
    assert!(mgr.create_profile(&p3).is_ok());

    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn shield_profile_data_dir_structure() {
    let dir = temp_dir("data-dir");
    let mgr = ProfileManager::new(&dir);

    let profile = ShieldProfile::new("Test".into(), BrowserEngine::Wayfern, "1.0".into());
    let id = profile.id.clone();
    mgr.create_profile(&profile).unwrap();

    // data_dir should be {profiles_dir}/{id}/profile
    let data = profile.data_dir(mgr.profiles_dir());
    assert!(data.ends_with(format!("{}/profile", id)));

    // metadata path should be {profiles_dir}/{id}/metadata.json
    let meta = profile.metadata_path(mgr.profiles_dir());
    assert!(meta.exists(), "metadata.json should exist after create");

    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn shield_fingerprint_config_defaults_and_serialization() {
    let fp = FingerprintConfig::default();
    assert!(fp.os.is_none());
    assert!(!fp.randomize_on_launch);

    let fp_full = FingerprintConfig {
        os: Some("windows".into()),
        user_agent: Some("Mozilla/5.0".into()),
        screen: Some([1920, 1080]),
        webgl_vendor: Some("NVIDIA".into()),
        webgl_renderer: Some("RTX 3080".into()),
        timezone: Some("America/New_York".into()),
        locale: Some("en-US".into()),
        hardware_concurrency: Some(16),
        device_memory: Some(32),
        raw_fingerprint: None,
        randomize_on_launch: true,
    };

    // Round-trip through JSON
    let json = serde_json::to_string(&fp_full).unwrap();
    let parsed: FingerprintConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.os.as_deref(), Some("windows"));
    assert_eq!(parsed.screen, Some([1920, 1080]));
    assert!(parsed.randomize_on_launch);
}

// ═══════════════════════════════════════════════════════════════════
// Engine management — install dir, download state, list
// ═══════════════════════════════════════════════════════════════════

#[test]
fn shield_engine_install_dir_format() {
    let dir = temp_dir("engine-dir");
    let mgr = EngineManager::new(&dir);

    let wayfern_dir = mgr.engine_install_dir(&BrowserEngine::Wayfern, "133.0.6943.2");
    assert!(wayfern_dir.to_string_lossy().contains("shield-engines"));
    assert!(wayfern_dir.to_string_lossy().contains("wayfern"));
    assert!(wayfern_dir.to_string_lossy().contains("133.0.6943.2"));

    let camoufox_dir = mgr.engine_install_dir(&BrowserEngine::Camoufox, "132.0.2");
    assert!(camoufox_dir.to_string_lossy().contains("camoufox"));

    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn shield_engine_not_downloaded_by_default() {
    let dir = temp_dir("engine-empty");
    let mgr = EngineManager::new(&dir);

    assert!(!mgr.is_downloaded(&BrowserEngine::Wayfern, "133.0"));
    assert!(!mgr.is_downloaded(&BrowserEngine::Camoufox, "132.0"));

    // get_executable should fail for missing engine
    assert!(mgr.get_executable(&BrowserEngine::Wayfern, "133.0").is_err());

    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn shield_engine_list_downloaded_empty() {
    let dir = temp_dir("engine-list");
    let mgr = EngineManager::new(&dir);

    let result = mgr.list_downloaded().unwrap();
    assert!(result.is_empty());

    std::fs::remove_dir_all(dir).ok();
}

// ═══════════════════════════════════════════════════════════════════
// Browser engine — serialization, display names, launch args
// ═══════════════════════════════════════════════════════════════════

#[test]
fn shield_browser_engine_roundtrip() {
    for engine in &[BrowserEngine::Wayfern, BrowserEngine::Camoufox] {
        let s = engine.as_str();
        let parsed = BrowserEngine::from_str(s).unwrap();
        assert_eq!(&parsed, engine);
    }

    // Invalid engine name
    assert!(BrowserEngine::from_str("chrome").is_err());
    assert!(BrowserEngine::from_str("firefox").is_err());
}

#[test]
fn shield_browser_engine_display_names() {
    assert_eq!(BrowserEngine::Wayfern.display_name(), "Wayfern (Chromium)");
    assert_eq!(BrowserEngine::Camoufox.display_name(), "Camoufox (Firefox)");
}

// ═══════════════════════════════════════════════════════════════════
// Download URL builder — platform-aware URLs
// ═══════════════════════════════════════════════════════════════════

#[test]
fn shield_download_url_camoufox() {
    let url = launcher::get_download_url(&BrowserEngine::Camoufox, "132.0.2").unwrap();
    assert!(url.starts_with("https://github.com/daijro/camoufox/releases/"));
    assert!(url.contains("132.0.2"));
    // Should contain platform-specific identifiers
    assert!(
        url.contains("win") || url.contains("lin") || url.contains("mac"),
        "URL should contain platform: {url}"
    );
}

#[test]
fn shield_download_url_wayfern() {
    let url = launcher::get_download_url(&BrowserEngine::Wayfern, "133.0.6943.2").unwrap();
    assert!(url.starts_with("https://download.wayfern.com/"));
    assert!(url.contains("133.0.6943.2"));
    assert!(
        url.contains("windows") || url.contains("linux") || url.contains("macos"),
        "URL should contain platform: {url}"
    );
}

#[test]
fn shield_platform_suffix_not_empty() {
    let suffix = EngineManager::platform_suffix();
    assert!(!suffix.is_empty());
}
