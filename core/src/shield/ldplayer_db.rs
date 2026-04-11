use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

// ─── Data Types ────────────────────────────────────────────────────

/// Persisted metadata for an LDPlayer instance.
/// This augments the live data from `ldconsole list2` with user-provided
/// notes, tags, proxy config, and linked shield profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LdProfileMeta {
    /// Primary key — matches the LDPlayer instance index.
    pub ld_index: u32,
    /// Cached instance name (may drift from live state).
    pub name: String,
    /// User-facing notes.
    #[serde(default)]
    pub notes: Option<String>,
    /// Comma-separated tags for grouping / filtering.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Linked Shield Browser profile ID (for proxy sync).
    #[serde(default)]
    pub linked_shield_profile_id: Option<String>,
    /// Proxy host to push to the emulator.
    #[serde(default)]
    pub proxy_host: Option<String>,
    /// Proxy port.
    #[serde(default)]
    pub proxy_port: Option<u16>,
    /// Configured CPU cores (from last modify).
    #[serde(default)]
    pub cpu: Option<u32>,
    /// Configured memory in MB (from last modify).
    #[serde(default)]
    pub memory: Option<u32>,
    /// Configured resolution string.
    #[serde(default)]
    pub resolution: Option<String>,
    /// Epoch timestamp of creation in our DB.
    pub created_at: u64,
    /// Epoch timestamp of last update.
    pub updated_at: u64,
}

// ─── LDPlayer Profile Store ───────────────────────────────────────

/// SQLite-backed store for LDPlayer instance metadata.
pub struct LdPlayerStore {
    conn: Mutex<Connection>,
}

impl LdPlayerStore {
    /// Open (or create) the SQLite database for LDPlayer metadata.
    pub fn new(base_dir: &Path) -> Result<Self> {
        let db_dir = base_dir.join("shield-data");
        std::fs::create_dir_all(&db_dir).ok();
        let db_path = db_dir.join("ldplayer_profiles.db");

        let conn =
            Connection::open(&db_path).context("Failed to open LDPlayer profiles database")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS ldplayer_profiles (
                ld_index            INTEGER PRIMARY KEY,
                name                TEXT NOT NULL,
                notes               TEXT,
                tags                TEXT NOT NULL DEFAULT '',
                linked_shield_profile_id TEXT,
                proxy_host          TEXT,
                proxy_port          INTEGER,
                cpu                 INTEGER,
                memory              INTEGER,
                resolution          TEXT,
                created_at          INTEGER NOT NULL,
                updated_at          INTEGER NOT NULL
            );",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// List all stored LDPlayer profile metadata.
    pub fn list(&self) -> Result<Vec<LdProfileMeta>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT ld_index, name, notes, tags, linked_shield_profile_id,
                        proxy_host, proxy_port, cpu, memory, resolution,
                        created_at, updated_at
                 FROM ldplayer_profiles
                 ORDER BY ld_index",
            )
            .context("Failed to prepare list query")?;

        let rows = stmt
            .query_map([], |row| {
                let tags_str: String = row.get(3)?;
                let tags: Vec<String> = if tags_str.is_empty() {
                    Vec::new()
                } else {
                    tags_str.split(',').map(|s| s.trim().to_string()).collect()
                };
                Ok(LdProfileMeta {
                    ld_index: row.get(0)?,
                    name: row.get(1)?,
                    notes: row.get(2)?,
                    tags,
                    linked_shield_profile_id: row.get(4)?,
                    proxy_host: row.get(5)?,
                    proxy_port: row.get::<_, Option<i32>>(6)?.map(|p| p as u16),
                    cpu: row.get::<_, Option<i32>>(7)?.map(|c| c as u32),
                    memory: row.get::<_, Option<i32>>(8)?.map(|m| m as u32),
                    resolution: row.get(9)?,
                    created_at: row.get::<_, i64>(10)? as u64,
                    updated_at: row.get::<_, i64>(11)? as u64,
                })
            })
            .context("Failed to query LDPlayer profiles")?;

        let mut profiles = Vec::new();
        for row in rows {
            profiles.push(row?);
        }
        Ok(profiles)
    }

    /// Get metadata for a specific instance by index.
    pub fn get(&self, ld_index: u32) -> Result<Option<LdProfileMeta>> {
        let profiles = self.list()?;
        Ok(profiles.into_iter().find(|p| p.ld_index == ld_index))
    }

    /// Insert or update metadata for an instance.
    pub fn upsert(&self, meta: &LdProfileMeta) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tags_str = meta.tags.join(",");

        conn.execute(
            "INSERT INTO ldplayer_profiles
                (ld_index, name, notes, tags, linked_shield_profile_id,
                 proxy_host, proxy_port, cpu, memory, resolution,
                 created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(ld_index) DO UPDATE SET
                name = excluded.name,
                notes = excluded.notes,
                tags = excluded.tags,
                linked_shield_profile_id = excluded.linked_shield_profile_id,
                proxy_host = excluded.proxy_host,
                proxy_port = excluded.proxy_port,
                cpu = excluded.cpu,
                memory = excluded.memory,
                resolution = excluded.resolution,
                updated_at = excluded.updated_at",
            params![
                meta.ld_index as i64,
                meta.name,
                meta.notes,
                tags_str,
                meta.linked_shield_profile_id,
                meta.proxy_host,
                meta.proxy_port.map(|p| p as i32),
                meta.cpu.map(|c| c as i32),
                meta.memory.map(|m| m as i32),
                meta.resolution,
                meta.created_at as i64,
                meta.updated_at as i64,
            ],
        )
        .context("Failed to upsert LDPlayer profile")?;

        Ok(())
    }

    /// Delete metadata for an instance by index.
    pub fn delete(&self, ld_index: u32) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let changed = conn
            .execute(
                "DELETE FROM ldplayer_profiles WHERE ld_index = ?1",
                params![ld_index as i64],
            )
            .context("Failed to delete LDPlayer profile")?;
        Ok(changed > 0)
    }

    /// Update only the mutable user-metadata fields (notes, tags, proxy, linked profile).
    pub fn update_meta(
        &self,
        ld_index: u32,
        notes: Option<&str>,
        tags: &[String],
        linked_shield_profile_id: Option<&str>,
        proxy_host: Option<&str>,
        proxy_port: Option<u16>,
    ) -> Result<()> {
        let now = epoch_now();
        let conn = self.conn.lock().unwrap();
        let tags_str = tags.join(",");

        let changed = conn
            .execute(
                "UPDATE ldplayer_profiles
                 SET notes = ?1, tags = ?2, linked_shield_profile_id = ?3,
                     proxy_host = ?4, proxy_port = ?5, updated_at = ?6
                 WHERE ld_index = ?7",
                params![
                    notes,
                    tags_str,
                    linked_shield_profile_id,
                    proxy_host,
                    proxy_port.map(|p| p as i32),
                    now as i64,
                    ld_index as i64,
                ],
            )
            .context("Failed to update LDPlayer profile metadata")?;

        if changed == 0 {
            anyhow::bail!("LDPlayer profile with index {} not found in DB", ld_index);
        }

        Ok(())
    }

    /// Sync DB entries with live instances from `ldconsole list2`.
    ///
    /// - Inserts new entries for instances not yet in the DB.
    /// - Updates the `name` field for existing entries if it changed.
    /// - Optionally prunes DB entries whose index no longer exists in the live list.
    pub fn sync_with_live(
        &self,
        live_instances: &[super::ldplayer::LdInstance],
        prune_orphans: bool,
    ) -> Result<()> {
        let existing = self.list()?;
        let now = epoch_now();

        // Index set from live data
        let live_indices: std::collections::HashSet<u32> =
            live_instances.iter().map(|i| i.index).collect();

        // Insert/update live instances
        for inst in live_instances {
            if let Some(existing_meta) = existing.iter().find(|e| e.ld_index == inst.index) {
                // Update name if it drifted
                if existing_meta.name != inst.name {
                    let mut updated = existing_meta.clone();
                    updated.name = inst.name.clone();
                    updated.updated_at = now;
                    self.upsert(&updated)?;
                }
            } else {
                // New instance — insert with defaults
                let meta = LdProfileMeta {
                    ld_index: inst.index,
                    name: inst.name.clone(),
                    notes: None,
                    tags: Vec::new(),
                    linked_shield_profile_id: None,
                    proxy_host: None,
                    proxy_port: None,
                    cpu: None,
                    memory: None,
                    resolution: None,
                    created_at: now,
                    updated_at: now,
                };
                self.upsert(&meta)?;
            }
        }

        // Prune orphans if requested
        if prune_orphans {
            for existing_meta in &existing {
                if !live_indices.contains(&existing_meta.ld_index) {
                    self.delete(existing_meta.ld_index)?;
                }
            }
        }

        Ok(())
    }
}

// ─── Helpers ───────────────────────────────────────────────────────

fn epoch_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_store() -> (LdPlayerStore, TempDir) {
        let tmp = TempDir::new().unwrap();
        let store = LdPlayerStore::new(tmp.path()).unwrap();
        (store, tmp)
    }

    fn sample_meta(index: u32, name: &str) -> LdProfileMeta {
        let now = epoch_now();
        LdProfileMeta {
            ld_index: index,
            name: name.to_string(),
            notes: None,
            tags: Vec::new(),
            linked_shield_profile_id: None,
            proxy_host: None,
            proxy_port: None,
            cpu: None,
            memory: None,
            resolution: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_insert_and_list() {
        let (store, _tmp) = test_store();

        store.upsert(&sample_meta(0, "LDPlayer")).unwrap();
        store.upsert(&sample_meta(1, "LDPlayer-1")).unwrap();

        let profiles = store.list().unwrap();
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].ld_index, 0);
        assert_eq!(profiles[0].name, "LDPlayer");
        assert_eq!(profiles[1].ld_index, 1);
        assert_eq!(profiles[1].name, "LDPlayer-1");
    }

    #[test]
    fn test_upsert_updates_existing() {
        let (store, _tmp) = test_store();

        let mut meta = sample_meta(0, "Original");
        store.upsert(&meta).unwrap();

        meta.name = "Updated".to_string();
        meta.notes = Some("A note".into());
        store.upsert(&meta).unwrap();

        let profiles = store.list().unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "Updated");
        assert_eq!(profiles[0].notes.as_deref(), Some("A note"));
    }

    #[test]
    fn test_delete() {
        let (store, _tmp) = test_store();

        store.upsert(&sample_meta(0, "ToDelete")).unwrap();
        assert!(store.delete(0).unwrap());
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn test_delete_nonexistent() {
        let (store, _tmp) = test_store();
        assert!(!store.delete(999).unwrap());
    }

    #[test]
    fn test_update_meta() {
        let (store, _tmp) = test_store();

        store.upsert(&sample_meta(0, "Test")).unwrap();

        store
            .update_meta(
                0,
                Some("Updated notes"),
                &["tag1".into(), "tag2".into()],
                Some("shield-profile-123"),
                Some("127.0.0.1"),
                Some(8080),
            )
            .unwrap();

        let profile = store.get(0).unwrap().unwrap();
        assert_eq!(profile.notes.as_deref(), Some("Updated notes"));
        assert_eq!(profile.tags, vec!["tag1", "tag2"]);
        assert_eq!(
            profile.linked_shield_profile_id.as_deref(),
            Some("shield-profile-123")
        );
        assert_eq!(profile.proxy_host.as_deref(), Some("127.0.0.1"));
        assert_eq!(profile.proxy_port, Some(8080));
    }

    #[test]
    fn test_tags_serialization() {
        let (store, _tmp) = test_store();

        let mut meta = sample_meta(0, "Tagged");
        meta.tags = vec!["farming".into(), "bot".into(), "account-3".into()];
        store.upsert(&meta).unwrap();

        let loaded = store.get(0).unwrap().unwrap();
        assert_eq!(loaded.tags, vec!["farming", "bot", "account-3"]);
    }

    #[test]
    fn test_sync_with_live() {
        use super::super::ldplayer::LdInstance;

        let (store, _tmp) = test_store();

        // Pre-existing orphan
        store.upsert(&sample_meta(99, "Orphan")).unwrap();

        let live = vec![
            LdInstance {
                index: 0,
                name: "LDPlayer".into(),
                top_hwnd: 0,
                bind_hwnd: 0,
                is_running: false,
                pid: -1,
                vbox_pid: -1,
            },
            LdInstance {
                index: 1,
                name: "LDPlayer-1".into(),
                top_hwnd: 0,
                bind_hwnd: 0,
                is_running: true,
                pid: 1234,
                vbox_pid: 5678,
            },
        ];

        store.sync_with_live(&live, true).unwrap();

        let profiles = store.list().unwrap();
        assert_eq!(profiles.len(), 2); // orphan pruned
        assert_eq!(profiles[0].name, "LDPlayer");
        assert_eq!(profiles[1].name, "LDPlayer-1");
    }
}
