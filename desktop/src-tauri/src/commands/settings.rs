//! Global user settings — secrets in OS keychain, preferences on disk.
//!
//! **Secrets** (API keys, tokens) are stored in the OS-native credential store:
//!   - macOS: Keychain
//!   - Windows: Credential Manager
//!   - Linux: Secret Service (GNOME Keyring / KDE Wallet)
//!
//! **Non-sensitive preferences** (model defaults, language) are stored as JSON
//! at `<base_dir>/settings.json`.
//!
//! **Migration**: On first read, if the old `settings.json` contains secret
//! fields (hfToken, etc.), they are automatically migrated to the keychain
//! and removed from the JSON file.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Once;
use tauri::State;

use crate::state::AppState;

/// The service name used for all keyring entries.
const KEYRING_SERVICE: &str = "nde-os";

static MIGRATION_ONCE: Once = Once::new();

/// What the frontend sends / receives — secrets are transparently encrypted.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSettings {
    // ── Secrets (stored in OS keychain, NOT on disk) ───────────────────
    /// HuggingFace API token (used by pyannote speaker diarization, etc.).
    #[serde(default)]
    pub hf_token: Option<String>,
    /// OpenAI API key.
    #[serde(default)]
    pub openai_api_key: Option<String>,
    /// Anthropic API key.
    #[serde(default)]
    pub anthropic_api_key: Option<String>,

    // ── Non-sensitive preferences (stored in settings.json) ───────────
    /// Default Whisper model size.
    #[serde(default)]
    pub whisper_default_model: Option<String>,
    /// Default target language for dubbing.
    #[serde(default)]
    pub default_target_language: Option<String>,
}

/// Non-sensitive preferences persisted to `settings.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Preferences {
    #[serde(default)]
    whisper_default_model: Option<String>,
    #[serde(default)]
    default_target_language: Option<String>,
}

/// Legacy settings.json shape (used only for one-time migration).
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct LegacySettings {
    #[serde(default)]
    hf_token: Option<String>,
    #[serde(default)]
    openai_api_key: Option<String>,
    #[serde(default)]
    anthropic_api_key: Option<String>,
    #[serde(default)]
    whisper_default_model: Option<String>,
    #[serde(default)]
    default_target_language: Option<String>,
}

fn prefs_path(base_dir: &std::path::Path) -> PathBuf {
    base_dir.join("settings.json")
}

// ─── Keyring helpers (with graceful fallback) ─────────────────────────────────

/// Read a secret from the OS keychain. Returns `None` if not found or on error.
fn keyring_get(key: &str) -> Option<String> {
    let entry = match keyring::Entry::new(KEYRING_SERVICE, key) {
        Ok(e) => e,
        Err(e) => {
            log::warn!("Keyring unavailable for '{key}': {e}");
            return None;
        }
    };
    match entry.get_password() {
        Ok(value) if !value.is_empty() => Some(value),
        Ok(_) => None,
        Err(keyring::Error::NoEntry) => None,
        Err(e) => {
            log::warn!("Failed to read '{key}' from keychain: {e}");
            None
        }
    }
}

/// Write a secret to the OS keychain. Returns false if keyring is unavailable.
fn keyring_set(key: &str, value: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, key)
        .map_err(|e| format!("Keyring unavailable for '{key}': {e}"))?;
    entry
        .set_password(value)
        .map_err(|e| format!("Failed to store '{key}' in OS keychain: {e}"))
}

/// Delete a secret from the OS keychain (best-effort).
fn keyring_delete(key: &str) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, key) {
        let _ = entry.delete_credential();
    }
}

// ─── Migration ────────────────────────────────────────────────────────────────

/// One-time migration: move secrets from legacy plaintext settings.json → keychain.
/// Then rewrite settings.json without the secret fields.
fn migrate_legacy_secrets(base_dir: &std::path::Path) {
    let path = prefs_path(base_dir);
    if !path.exists() {
        return;
    }

    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return,
    };

    let legacy: LegacySettings = match serde_json::from_str(&data) {
        Ok(l) => l,
        Err(_) => return,
    };

    let mut migrated_any = false;

    // Migrate each secret: if it exists in the JSON and NOT yet in keychain, move it.
    let secrets_to_migrate = [
        ("hf-token", &legacy.hf_token),
        ("openai-api-key", &legacy.openai_api_key),
        ("anthropic-api-key", &legacy.anthropic_api_key),
    ];

    for (keyring_key, json_value) in &secrets_to_migrate {
        if let Some(value) = json_value {
            if !value.trim().is_empty() && keyring_get(keyring_key).is_none() {
                match keyring_set(keyring_key, value.trim()) {
                    Ok(()) => {
                        log::info!("Migrated '{keyring_key}' from settings.json → OS keychain");
                        migrated_any = true;
                    }
                    Err(e) => {
                        log::warn!("Migration failed for '{keyring_key}': {e}");
                    }
                }
            }
        }
    }

    // Rewrite settings.json without secret fields (keep only preferences).
    if migrated_any || legacy.hf_token.is_some() || legacy.openai_api_key.is_some() || legacy.anthropic_api_key.is_some() {
        let clean_prefs = Preferences {
            whisper_default_model: legacy.whisper_default_model,
            default_target_language: legacy.default_target_language,
        };
        if let Ok(clean_data) = serde_json::to_string_pretty(&clean_prefs) {
            if std::fs::write(&path, clean_data).is_ok() {
                log::info!("Scrubbed secrets from settings.json");
            }
        }
    }
}

// ─── Tauri commands ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_global_settings(state: State<'_, AppState>) -> Result<GlobalSettings, String> {
    // Run one-time migration on first access.
    let base_dir = state.base_dir.clone();
    MIGRATION_ONCE.call_once(move || {
        migrate_legacy_secrets(&base_dir);
    });

    // 1. Load non-sensitive prefs from disk.
    let prefs = load_prefs(&state.base_dir);

    // 2. Load secrets from OS keychain.
    let hf_token = keyring_get("hf-token");
    let openai_api_key = keyring_get("openai-api-key");
    let anthropic_api_key = keyring_get("anthropic-api-key");

    Ok(GlobalSettings {
        hf_token,
        openai_api_key,
        anthropic_api_key,
        whisper_default_model: prefs.whisper_default_model,
        default_target_language: prefs.default_target_language,
    })
}

#[tauri::command]
pub async fn set_global_settings(
    state: State<'_, AppState>,
    settings: GlobalSettings,
) -> Result<(), String> {
    // 1. Store secrets in OS keychain (or delete if cleared).
    store_or_clear_secret("hf-token", &settings.hf_token)?;
    store_or_clear_secret("openai-api-key", &settings.openai_api_key)?;
    store_or_clear_secret("anthropic-api-key", &settings.anthropic_api_key)?;

    // 2. Store non-sensitive prefs to disk (no secrets!).
    let prefs = Preferences {
        whisper_default_model: settings.whisper_default_model,
        default_target_language: settings.default_target_language,
    };
    save_prefs(&state.base_dir, &prefs)?;

    Ok(())
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

fn store_or_clear_secret(key: &str, value: &Option<String>) -> Result<(), String> {
    match value {
        Some(v) if !v.trim().is_empty() => keyring_set(key, v.trim()),
        _ => {
            keyring_delete(key);
            Ok(())
        }
    }
}

fn load_prefs(base_dir: &std::path::Path) -> Preferences {
    let path = prefs_path(base_dir);
    if !path.exists() {
        return Preferences::default();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

fn save_prefs(base_dir: &std::path::Path, prefs: &Preferences) -> Result<(), String> {
    let path = prefs_path(base_dir);
    let data = serde_json::to_string_pretty(prefs)
        .map_err(|e| format!("Failed to serialize preferences: {e}"))?;
    std::fs::write(&path, data).map_err(|e| format!("Failed to write preferences: {e}"))?;
    Ok(())
}
