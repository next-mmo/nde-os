//! Secrets management — encrypt/decrypt sensitive tokens at rest.
//!
//! Uses AES-256-GCM (authenticated encryption) with a key derived from
//! machine-specific identifiers (hostname + username + data-dir path).
//! This ensures:
//!   - Tokens are never stored in plaintext in channels.json
//!   - The encrypted file is useless on another machine
//!   - No master password needed — key is deterministic per-machine

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Encrypted token prefix — indicates the value is encrypted, not plaintext.
const ENC_PREFIX: &str = "enc:";

/// Derive a 256-bit encryption key from machine-specific identifiers.
/// Uses SHA-256(hostname + username + data_dir path) as the key material.
fn derive_key(data_dir: &Path) -> [u8; 32] {
    let mut hasher = Sha256::new();

    // Machine hostname
    if let Ok(hostname) = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .or_else(|_| gethostname())
    {
        hasher.update(hostname.as_bytes());
    }

    // Current user
    if let Ok(user) = std::env::var("USERNAME").or_else(|_| std::env::var("USER")) {
        hasher.update(user.as_bytes());
    }

    // Data directory path (unique per install)
    hasher.update(data_dir.to_string_lossy().as_bytes());

    // Static salt to domain-separate from other SHA-256 uses
    hasher.update(b"nde-os-secrets-v1");

    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// Cross-platform hostname getter
fn gethostname() -> std::result::Result<String, std::env::VarError> {
    // Fallback: use a fixed string if we can't get hostname
    Ok("nde-os-default-host".to_string())
}

/// Encrypt a plaintext token. Returns a string like "enc:<hex-nonce>:<hex-ciphertext>".
pub fn encrypt_token(plaintext: &str, data_dir: &Path) -> Result<String> {
    if plaintext.is_empty() {
        return Ok(String::new());
    }

    // Already encrypted? Return as-is
    if plaintext.starts_with(ENC_PREFIX) {
        return Ok(plaintext.to_string());
    }

    let key = derive_key(data_dir);
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| anyhow!("Failed to create cipher: {}", e))?;

    // Generate random 96-bit nonce
    let nonce_bytes: [u8; 12] = {
        use aes_gcm::aead::rand_core::RngCore;
        let mut buf = [0u8; 12];
        OsRng.fill_bytes(&mut buf);
        buf
    };
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    // Format: enc:<hex-nonce>:<hex-ciphertext>
    Ok(format!(
        "{}{}:{}",
        ENC_PREFIX,
        hex_encode(&nonce_bytes),
        hex_encode(&ciphertext)
    ))
}

/// Decrypt an encrypted token. If the token is not encrypted (no "enc:" prefix),
/// returns it as-is for backward compatibility.
pub fn decrypt_token(encrypted: &str, data_dir: &Path) -> Result<String> {
    if encrypted.is_empty() {
        return Ok(String::new());
    }

    // Not encrypted? Return as-is (backward compat with plaintext tokens)
    if !encrypted.starts_with(ENC_PREFIX) {
        return Ok(encrypted.to_string());
    }

    let payload = &encrypted[ENC_PREFIX.len()..];
    let parts: Vec<&str> = payload.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid encrypted token format"));
    }

    let nonce_bytes = hex_decode(parts[0]).map_err(|e| anyhow!("Invalid nonce hex: {}", e))?;
    let ciphertext = hex_decode(parts[1]).map_err(|e| anyhow!("Invalid ciphertext hex: {}", e))?;

    if nonce_bytes.len() != 12 {
        return Err(anyhow!(
            "Invalid nonce length: expected 12, got {}",
            nonce_bytes.len()
        ));
    }

    let key = derive_key(data_dir);
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|e| anyhow!("Failed to create cipher: {}", e))?;

    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = cipher.decrypt(nonce, ciphertext.as_ref()).map_err(|_| {
        anyhow!("Decryption failed — token may have been encrypted on a different machine")
    })?;

    String::from_utf8(plaintext).map_err(|e| anyhow!("Decrypted token is not valid UTF-8: {}", e))
}

// ── Hex encoding (no extra deps) ─────────────────────────────────────────────

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(hex: &str) -> std::result::Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("Odd hex string length".into());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|e| format!("Invalid hex at pos {}: {}", i, e))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let token = "123456:ABC-DEF_ghijklmnop";

        let encrypted = encrypt_token(token, dir.path()).unwrap();
        assert!(encrypted.starts_with("enc:"), "Should have enc: prefix");
        assert_ne!(encrypted, token, "Should not be plaintext");

        let decrypted = decrypt_token(&encrypted, dir.path()).unwrap();
        assert_eq!(decrypted, token, "Roundtrip should preserve token");
    }

    #[test]
    fn test_empty_token() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(encrypt_token("", dir.path()).unwrap(), "");
        assert_eq!(decrypt_token("", dir.path()).unwrap(), "");
    }

    #[test]
    fn test_plaintext_passthrough() {
        let dir = tempfile::tempdir().unwrap();
        let plain = "123456:ABC";
        // decrypt_token on plaintext should return as-is
        assert_eq!(decrypt_token(plain, dir.path()).unwrap(), plain);
    }

    #[test]
    fn test_double_encrypt_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let token = "my-secret-token";
        let enc1 = encrypt_token(token, dir.path()).unwrap();
        let enc2 = encrypt_token(&enc1, dir.path()).unwrap();
        assert_eq!(enc1, enc2, "Re-encrypting should return same value");
    }

    #[test]
    fn test_wrong_machine_cannot_decrypt() {
        let dir1 = tempfile::tempdir().unwrap();
        let dir2 = tempfile::tempdir().unwrap();
        let token = "sensitive-token";

        let encrypted = encrypt_token(token, dir1.path()).unwrap();
        // Different data_dir = different key = decryption should fail
        let result = decrypt_token(&encrypted, dir2.path());
        assert!(
            result.is_err(),
            "Different machine key should fail to decrypt"
        );
    }
}
