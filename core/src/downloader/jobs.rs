//! Persistent download-job state.
//!
//! A `Job` corresponds to a single "Download" button click: one playlist +
//! a subset of its items. State is persisted as JSON in
//! `{data_dir}/downloads/jobs.json` so the Download Center can recover
//! across restarts.

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ItemStatus {
    Pending,
    Downloading,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadItem {
    pub id: String,
    pub title: String,
    pub index: u32,
    pub output_path: PathBuf,
    pub status: ItemStatus,
    #[serde(default)]
    pub progress: f32,
    #[serde(default)]
    pub bytes_downloaded: u64,
    #[serde(default)]
    pub bytes_total: u64,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub provider: String,
    pub source_url: String,
    pub playlist_id: String,
    pub title: String,
    #[serde(default)]
    pub cover: Option<String>,
    pub output_dir: PathBuf,
    pub status: JobStatus,
    pub items: Vec<DownloadItem>,
    /// Provider-specific context echoed back when resolving streams.
    #[serde(default)]
    pub context: serde_json::Value,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Job {
    pub fn touch(&mut self) {
        self.updated_at = Utc::now().timestamp();
    }
}

/// Thread-safe on-disk job store. Writes the full list atomically via
/// tempfile rename so a crash mid-write cannot corrupt state.
pub struct JobStore {
    path: PathBuf,
    inner: Mutex<HashMap<String, Job>>,
}

impl JobStore {
    /// Open or create the store at `{data_dir}/downloads/jobs.json`.
    pub fn open(data_dir: &Path) -> Result<Self> {
        let dir = data_dir.join("downloads");
        std::fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
        let path = dir.join("jobs.json");

        let inner = if path.exists() {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("read {}", path.display()))?;
            if raw.trim().is_empty() {
                HashMap::new()
            } else {
                serde_json::from_str::<HashMap<String, Job>>(&raw).unwrap_or_default()
            }
        } else {
            HashMap::new()
        };

        Ok(Self {
            path,
            inner: Mutex::new(inner),
        })
    }

    pub fn insert(&self, job: Job) -> Result<()> {
        {
            let mut map = self
                .inner
                .lock()
                .map_err(|_| anyhow::anyhow!("jobs lock poisoned"))?;
            map.insert(job.id.clone(), job);
        }
        self.flush()
    }

    pub fn get(&self, id: &str) -> Option<Job> {
        self.inner.lock().ok()?.get(id).cloned()
    }

    pub fn list(&self) -> Vec<Job> {
        match self.inner.lock() {
            Ok(map) => {
                let mut v: Vec<Job> = map.values().cloned().collect();
                v.sort_by_key(|j| -j.created_at);
                v
            }
            Err(_) => Vec::new(),
        }
    }

    pub fn update<F>(&self, id: &str, f: F) -> Result<Option<Job>>
    where
        F: FnOnce(&mut Job),
    {
        let updated = {
            let mut map = self
                .inner
                .lock()
                .map_err(|_| anyhow::anyhow!("jobs lock poisoned"))?;
            let Some(job) = map.get_mut(id) else {
                return Ok(None);
            };
            f(job);
            job.touch();
            Some(job.clone())
        };
        self.flush()?;
        Ok(updated)
    }

    pub fn remove(&self, id: &str) -> Result<bool> {
        let existed = {
            let mut map = self
                .inner
                .lock()
                .map_err(|_| anyhow::anyhow!("jobs lock poisoned"))?;
            map.remove(id).is_some()
        };
        if existed {
            self.flush()?;
        }
        Ok(existed)
    }

    fn flush(&self) -> Result<()> {
        let snapshot: HashMap<String, Job> = {
            let map = self
                .inner
                .lock()
                .map_err(|_| anyhow::anyhow!("jobs lock poisoned"))?;
            map.clone()
        };
        let tmp = self.path.with_extension("json.tmp");
        let body = serde_json::to_string_pretty(&snapshot)?;
        std::fs::write(&tmp, body).with_context(|| format!("write {}", tmp.display()))?;
        std::fs::rename(&tmp, &self.path)
            .with_context(|| format!("rename {} -> {}", tmp.display(), self.path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn mk_job(id: &str) -> Job {
        Job {
            id: id.to_string(),
            provider: "short_drama".to_string(),
            source_url: "https://example.com".to_string(),
            playlist_id: "pl".to_string(),
            title: "t".to_string(),
            cover: None,
            output_dir: PathBuf::from("/tmp"),
            status: JobStatus::Queued,
            items: vec![],
            context: serde_json::json!({}),
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
        }
    }

    #[test]
    fn persists_across_reopen() {
        let tmp = TempDir::new().unwrap();
        {
            let store = JobStore::open(tmp.path()).unwrap();
            store.insert(mk_job("a")).unwrap();
            store.insert(mk_job("b")).unwrap();
        }
        let store = JobStore::open(tmp.path()).unwrap();
        let mut ids: Vec<String> = store.list().into_iter().map(|j| j.id).collect();
        ids.sort();
        assert_eq!(ids, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn update_mutates_and_persists() {
        let tmp = TempDir::new().unwrap();
        let store = JobStore::open(tmp.path()).unwrap();
        store.insert(mk_job("a")).unwrap();
        store
            .update("a", |j| j.status = JobStatus::Completed)
            .unwrap();
        assert_eq!(store.get("a").unwrap().status, JobStatus::Completed);
    }

    #[test]
    fn remove_deletes() {
        let tmp = TempDir::new().unwrap();
        let store = JobStore::open(tmp.path()).unwrap();
        store.insert(mk_job("a")).unwrap();
        assert!(store.remove("a").unwrap());
        assert!(store.get("a").is_none());
    }
}
