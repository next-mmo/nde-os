//! SQLite-backed project persistence for FreeCut.
//!
//! Projects are stored as JSON blobs in SQLite, following the same pattern
//! as the agent store. Media metadata is stored in a separate table for
//! fast lookups without deserializing entire projects.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::Path;

use super::project::{MediaMetadata, Project};

/// FreeCut project + media storage backed by SQLite.
pub struct FreeCutStore {
    conn: Connection,
}

impl FreeCutStore {
    /// Open (or create) the FreeCut database at the given path.
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("failed to open FreeCut DB at {}", db_path.display()))?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;",
        )?;

        let store = Self { conn };
        store.init_tables()?;
        Ok(store)
    }

    fn init_tables(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS projects (
                id          TEXT PRIMARY KEY,
                name        TEXT NOT NULL,
                data        TEXT NOT NULL,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS media (
                id          TEXT PRIMARY KEY,
                project_id  TEXT NOT NULL,
                file_name   TEXT NOT NULL,
                file_path   TEXT NOT NULL,
                media_type  TEXT NOT NULL,
                data        TEXT NOT NULL,
                imported_at TEXT NOT NULL,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_media_project ON media(project_id);

            CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    // ── Projects ───────────────────────────────────────────────────────────

    /// Save a project (insert or replace).
    pub fn save_project(&self, project: &Project) -> Result<()> {
        let json = serde_json::to_string(project)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO projects (id, name, data, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                project.id,
                project.name,
                json,
                project.created_at.to_rfc3339(),
                project.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Load a project by ID.
    pub fn get_project(&self, id: &str) -> Result<Option<Project>> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM projects WHERE id = ?1")?;
        let result = stmt
            .query_row(params![id], |row| {
                let json: String = row.get(0)?;
                Ok(json)
            })
            .optional()?;

        match result {
            Some(json) => {
                let project: Project = serde_json::from_str(&json)?;
                Ok(Some(project))
            }
            None => Ok(None),
        }
    }

    /// List all projects (id, name, updated_at) — lightweight summary.
    pub fn list_projects(&self) -> Result<Vec<ProjectSummary>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, updated_at FROM projects ORDER BY updated_at DESC")?;
        let rows = stmt.query_map([], |row| {
            Ok(ProjectSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                updated_at: row.get(2)?,
            })
        })?;
        let mut projects = Vec::new();
        for row in rows {
            projects.push(row?);
        }
        Ok(projects)
    }

    /// Delete a project and its associated media records.
    pub fn delete_project(&self, id: &str) -> Result<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    // ── Media ──────────────────────────────────────────────────────────────

    /// Save media metadata linked to a project.
    pub fn save_media(&self, project_id: &str, media: &MediaMetadata) -> Result<()> {
        let json = serde_json::to_string(media)?;
        self.conn.execute(
            "INSERT OR REPLACE INTO media (id, project_id, file_name, file_path, media_type, data, imported_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                media.id,
                project_id,
                media.file_name,
                media.file_path,
                serde_json::to_string(&media.media_type)?,
                json,
                media.imported_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// List all media for a project.
    pub fn list_media(&self, project_id: &str) -> Result<Vec<MediaMetadata>> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM media WHERE project_id = ?1 ORDER BY imported_at DESC")?;
        let rows = stmt.query_map(params![project_id], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;
        let mut media = Vec::new();
        for row in rows {
            let json = row?;
            let m: MediaMetadata = serde_json::from_str(&json)?;
            media.push(m);
        }
        Ok(media)
    }

    /// Update the thumbnail_path in an existing media record.
    ///
    /// Called after async thumbnail generation so that `list_media` returns
    /// the correct path on the next call (e.g. after a hard reload).
    pub fn update_media_thumbnail(&self, media_id: &str, thumbnail_path: &str) -> Result<bool> {
        // Read the current JSON blob, patch it, and write it back to keep
        // the denormalised `data` column in sync with the individual columns.
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM media WHERE id = ?1")?;
        let existing: Option<String> = stmt
            .query_row(params![media_id], |row| row.get(0))
            .optional()?;

        let Some(json) = existing else {
            return Ok(false);
        };

        let mut meta: super::project::MediaMetadata = serde_json::from_str(&json)?;
        meta.thumbnail_path = Some(thumbnail_path.to_string());
        let updated_json = serde_json::to_string(&meta)?;

        let affected = self.conn.execute(
            "UPDATE media SET data = ?1 WHERE id = ?2",
            params![updated_json, media_id],
        )?;
        Ok(affected > 0)
    }

    /// Delete a media entry by ID.
    pub fn delete_media(&self, media_id: &str) -> Result<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM media WHERE id = ?1", params![media_id])?;
        Ok(affected > 0)
    }

    // ── Settings ───────────────────────────────────────────────────────────

    /// Read a setting value by key. Returns `None` if the key does not exist.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key = ?1")?;
        let result = stmt
            .query_row(params![key], |row| row.get::<_, String>(0))
            .optional()?;
        Ok(result)
    }

    /// Insert or replace a setting value.
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    /// Delete a setting by key. Returns `true` if the key existed.
    pub fn delete_setting(&self, key: &str) -> Result<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(affected > 0)
    }
}

/// Lightweight project listing (no full timeline deserialization).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub updated_at: String,
}

// We need the optional extension trait for rusqlite.
use rusqlite::OptionalExtension;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::freecut::project::{MediaType, ProjectResolution};
    use chrono::Utc;
    use tempfile::NamedTempFile;

    fn temp_store() -> FreeCutStore {
        let f = NamedTempFile::new().unwrap();
        FreeCutStore::new(f.path()).unwrap()
    }

    fn sample_project(id: &str) -> Project {
        Project {
            id: id.to_string(),
            name: format!("Project {id}"),
            description: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            duration: 300,
            schema_version: 1,
            metadata: ProjectResolution::default(),
            timeline: None,
            dubbing: None,
        }
    }

    #[test]
    fn save_and_get_project() {
        let store = temp_store();
        let proj = sample_project("p1");
        store.save_project(&proj).unwrap();

        let loaded = store.get_project("p1").unwrap().unwrap();
        assert_eq!(loaded.name, "Project p1");
        assert_eq!(loaded.metadata.width, 1920);
    }

    #[test]
    fn list_projects() {
        let store = temp_store();
        store.save_project(&sample_project("a")).unwrap();
        store.save_project(&sample_project("b")).unwrap();

        let list = store.list_projects().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn delete_project() {
        let store = temp_store();
        store.save_project(&sample_project("d")).unwrap();
        assert!(store.delete_project("d").unwrap());
        assert!(store.get_project("d").unwrap().is_none());
    }

    #[test]
    fn get_nonexistent_project() {
        let store = temp_store();
        assert!(store.get_project("nope").unwrap().is_none());
    }

    #[test]
    fn save_and_list_media() {
        let store = temp_store();
        store.save_project(&sample_project("mp")).unwrap();

        let media = MediaMetadata {
            id: "m1".to_string(),
            file_name: "clip.mp4".to_string(),
            file_path: "/tmp/clip.mp4".to_string(),
            file_size: 1024,
            media_type: MediaType::Video,
            width: Some(1920),
            height: Some(1080),
            duration_secs: Some(10.0),
            fps: Some(30.0),
            codec: Some("h264".to_string()),
            audio_codec: None,
            sample_rate: None,
            channels: None,
            thumbnail_path: None,
            imported_at: Utc::now(),
        };

        store.save_media("mp", &media).unwrap();
        let list = store.list_media("mp").unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].file_name, "clip.mp4");
    }

    #[test]
    fn cascade_delete_media() {
        let store = temp_store();
        store.save_project(&sample_project("cd")).unwrap();

        let media = MediaMetadata {
            id: "m2".to_string(),
            file_name: "test.wav".to_string(),
            file_path: "/tmp/test.wav".to_string(),
            file_size: 512,
            media_type: MediaType::Audio,
            width: None,
            height: None,
            duration_secs: Some(5.0),
            fps: None,
            codec: None,
            audio_codec: Some("pcm".to_string()),
            sample_rate: Some(44100),
            channels: Some(2),
            thumbnail_path: None,
            imported_at: Utc::now(),
        };

        store.save_media("cd", &media).unwrap();
        store.delete_project("cd").unwrap();
        let list = store.list_media("cd").unwrap();
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn update_thumbnail_path() {
        let store = temp_store();
        store.save_project(&sample_project("tp")).unwrap();

        let media = MediaMetadata {
            id: "m3".to_string(),
            file_name: "thumb_test.mp4".to_string(),
            file_path: "/tmp/thumb_test.mp4".to_string(),
            file_size: 2048,
            media_type: MediaType::Video,
            width: Some(1280),
            height: Some(720),
            duration_secs: Some(30.0),
            fps: Some(30.0),
            codec: Some("h264".to_string()),
            audio_codec: None,
            sample_rate: None,
            channels: None,
            thumbnail_path: None,
            imported_at: Utc::now(),
        };

        store.save_media("tp", &media).unwrap();

        // Initially no thumbnail.
        let before = store.list_media("tp").unwrap();
        assert!(before[0].thumbnail_path.is_none());

        // Update the thumbnail path.
        let updated = store.update_media_thumbnail("m3", "/tmp/thumbs/thumb0.jpg").unwrap();
        assert!(updated);

        // list_media now returns the thumbnail path.
        let after = store.list_media("tp").unwrap();
        assert_eq!(after[0].thumbnail_path.as_deref(), Some("/tmp/thumbs/thumb0.jpg"));

        // Updating a nonexistent ID returns false.
        let not_found = store.update_media_thumbnail("does-not-exist", "/x").unwrap();
        assert!(!not_found);
    }

    #[test]
    fn settings_get_set_delete() {
        let store = temp_store();

        // Key missing initially.
        assert!(store.get_setting("last_project_id").unwrap().is_none());

        // Set then get.
        store.set_setting("last_project_id", "proj-abc").unwrap();
        assert_eq!(
            store.get_setting("last_project_id").unwrap().as_deref(),
            Some("proj-abc")
        );

        // Overwrite via set (INSERT OR REPLACE).
        store.set_setting("last_project_id", "proj-xyz").unwrap();
        assert_eq!(
            store.get_setting("last_project_id").unwrap().as_deref(),
            Some("proj-xyz")
        );

        // Delete returns true when key existed.
        assert!(store.delete_setting("last_project_id").unwrap());

        // Returns false when key is already gone.
        assert!(!store.delete_setting("last_project_id").unwrap());

        // Gone after delete.
        assert!(store.get_setting("last_project_id").unwrap().is_none());
    }
}
