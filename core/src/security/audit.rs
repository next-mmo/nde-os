use anyhow::Result;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Append-only audit trail with SHA-256 hash chain (Merkle-style).
/// Every action, tool call, and LLM request is logged with integrity.
pub struct AuditTrail {
    log_path: PathBuf,
    last_hash: String,
    enabled: bool,
}

impl AuditTrail {
    pub fn new(log_dir: impl AsRef<Path>, enabled: bool) -> Result<Self> {
        if !enabled {
            return Ok(Self {
                log_path: PathBuf::new(),
                last_hash: String::new(),
                enabled: false,
            });
        }

        let log_dir = log_dir.as_ref();
        fs::create_dir_all(log_dir)?;

        let log_path = log_dir.join("audit.jsonl");
        let last_hash = if log_path.exists() {
            // Read last line to get the chain hash
            let content = fs::read_to_string(&log_path)?;
            content
                .lines()
                .last()
                .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                .and_then(|v| v.get("hash").and_then(|h| h.as_str()).map(String::from))
                .unwrap_or_else(|| "genesis".into())
        } else {
            "genesis".into()
        };

        Ok(Self {
            log_path,
            last_hash,
            enabled,
        })
    }

    /// Log an event to the audit trail with hash chain integrity.
    pub fn log(&mut self, event_type: &str, raw_data: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let timestamp = chrono::Utc::now().to_rfc3339();

        // Scrub secrets from the data payload before storing/hashing
        let data = crate::security::policy::scrub_output(raw_data);

        // Hash: SHA256(previous_hash + timestamp + event + data)
        let mut hasher = Sha256::new();
        hasher.update(&self.last_hash);
        hasher.update(&timestamp);
        hasher.update(event_type);
        hasher.update(&data);
        let hash = hasher.finalize();
        let hash = hash.iter().map(|b| format!("{:02x}", b)).collect::<String>();

        let entry = serde_json::json!({
            "timestamp": timestamp,
            "event": event_type,
            "data": data,
            "prev_hash": self.last_hash,
            "hash": hash,
        });

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        writeln!(file, "{}", serde_json::to_string(&entry)?)?;
        self.last_hash = hash;
        Ok(())
    }

    /// Verify the integrity of the audit trail.
    pub fn verify(&self) -> Result<bool> {
        if !self.enabled || !self.log_path.exists() {
            return Ok(true);
        }

        let content = fs::read_to_string(&self.log_path)?;
        let mut expected_prev = "genesis".to_string();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let entry: serde_json::Value = serde_json::from_str(line)?;

            let prev = entry
                .get("prev_hash")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if prev != expected_prev {
                return Ok(false);
            }

            // Recompute hash
            let timestamp = entry
                .get("timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let event = entry.get("event").and_then(|v| v.as_str()).unwrap_or("");
            let data = entry.get("data").and_then(|v| v.as_str()).unwrap_or("");

            let mut hasher = Sha256::new();
            hasher.update(prev);
            hasher.update(timestamp);
            hasher.update(event);
            hasher.update(data);
            let computed = hasher.finalize();
            let computed = computed.iter().map(|b| format!("{:02x}", b)).collect::<String>();

            let stored = entry.get("hash").and_then(|v| v.as_str()).unwrap_or("");
            if computed != stored {
                return Ok(false);
            }

            expected_prev = computed;
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_trail() {
        let dir = tempfile::tempdir().unwrap();
        let mut trail = AuditTrail::new(dir.path(), true).unwrap();

        trail.log("test_event", "hello world").unwrap();
        trail.log("another_event", "some data").unwrap();

        assert!(trail.verify().unwrap());
    }
}
