use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

// ─── Actor Dataset (JSONL) ─────────────────────────────────────────

/// Local dataset storage for actor runs.
///
/// Each run gets a JSONL file where each line is one data item.
/// Mirrors Apify's Dataset API: push items, read items, export.
pub struct ActorDataset {
    path: PathBuf,
}

impl ActorDataset {
    /// Create or open a dataset at the given file path.
    pub fn new(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create dataset directory")?;
        }
        Ok(Self { path })
    }

    /// Append data items to the dataset.
    pub fn push_data(&self, items: &[serde_json::Value]) -> Result<()> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("Failed to open dataset: {}", self.path.display()))?;

        let mut writer = BufWriter::new(file);
        for item in items {
            let line = serde_json::to_string(item)
                .context("Failed to serialize dataset item")?;
            writeln!(writer, "{}", line)?;
        }
        writer.flush()?;
        Ok(())
    }

    /// Read items from the dataset with pagination.
    pub fn get_items(&self, offset: usize, limit: usize) -> Result<Vec<serde_json::Value>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = std::fs::File::open(&self.path)
            .context("Failed to open dataset for reading")?;
        let reader = BufReader::new(file);

        let items: Vec<serde_json::Value> = reader
            .lines()
            .filter_map(|line| line.ok())
            .filter(|line| !line.trim().is_empty())
            .skip(offset)
            .take(limit)
            .filter_map(|line| serde_json::from_str(&line).ok())
            .collect();

        Ok(items)
    }

    /// Count total items in the dataset.
    pub fn count(&self) -> Result<usize> {
        if !self.path.exists() {
            return Ok(0);
        }

        let file = std::fs::File::open(&self.path)
            .context("Failed to open dataset for counting")?;
        let reader = BufReader::new(file);
        let count = reader
            .lines()
            .filter_map(|l| l.ok())
            .filter(|l| !l.trim().is_empty())
            .count();

        Ok(count)
    }

    /// Export all items as a JSON array.
    pub fn export_json(&self) -> Result<Vec<serde_json::Value>> {
        self.get_items(0, usize::MAX)
    }

    /// Export all items as CSV to the given writer.
    /// Uses the keys from the first item as column headers.
    pub fn export_csv(&self, writer: &mut dyn Write) -> Result<()> {
        let items = self.export_json()?;
        if items.is_empty() {
            return Ok(());
        }

        // Collect all unique keys across all items for headers
        let mut headers: Vec<String> = Vec::new();
        for item in &items {
            if let Some(obj) = item.as_object() {
                for key in obj.keys() {
                    if !headers.contains(key) {
                        headers.push(key.clone());
                    }
                }
            }
        }

        // Write header row
        writeln!(writer, "{}", headers.join(","))?;

        // Write data rows
        for item in &items {
            let row: Vec<String> = headers
                .iter()
                .map(|h| {
                    item.get(h).map_or_else(
                        String::new,
                        |v| match v {
                            serde_json::Value::String(s) => {
                                // Escape CSV values containing commas or quotes
                                if s.contains(',') || s.contains('"') || s.contains('\n') {
                                    format!("\"{}\"", s.replace('"', "\"\""))
                                } else {
                                    s.clone()
                                }
                            }
                            serde_json::Value::Null => String::new(),
                            other => other.to_string(),
                        },
                    )
                })
                .collect();
            writeln!(writer, "{}", row.join(","))?;
        }

        Ok(())
    }

    /// Get the dataset file path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

// ─── Actor Key-Value Store ─────────────────────────────────────────

/// Local key-value store for actor runs.
///
/// Each key maps to a file in the store directory.
/// Mirrors Apify's Key-Value Store API: set, get, list keys.
pub struct ActorKvStore {
    dir: PathBuf,
}

impl ActorKvStore {
    /// Create or open a KV store at the given directory.
    pub fn new(dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create KV store directory: {}", dir.display()))?;
        Ok(Self { dir })
    }

    /// Store a value under the given key.
    /// Content type is stored as `{key}.content-type` metadata file.
    pub fn set(&self, key: &str, value: &[u8], content_type: &str) -> Result<()> {
        let sanitized = sanitize_key(key);
        let value_path = self.dir.join(&sanitized);
        let meta_path = self.dir.join(format!("{}.content-type", sanitized));

        std::fs::write(&value_path, value)
            .with_context(|| format!("Failed to write KV entry: {}", key))?;
        std::fs::write(&meta_path, content_type)
            .with_context(|| format!("Failed to write KV content-type: {}", key))?;

        Ok(())
    }

    /// Retrieve a value by key. Returns None if not found.
    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let sanitized = sanitize_key(key);
        let value_path = self.dir.join(&sanitized);

        if !value_path.exists() {
            return Ok(None);
        }

        let data = std::fs::read(&value_path)
            .with_context(|| format!("Failed to read KV entry: {}", key))?;
        Ok(Some(data))
    }

    /// Get the content type for a stored key.
    pub fn get_content_type(&self, key: &str) -> Option<String> {
        let sanitized = sanitize_key(key);
        let meta_path = self.dir.join(format!("{}.content-type", sanitized));
        std::fs::read_to_string(&meta_path).ok()
    }

    /// Store a JSON value under the given key.
    pub fn set_json(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(value)
            .context("Failed to serialize JSON for KV store")?;
        self.set(key, &bytes, "application/json")
    }

    /// Retrieve a JSON value by key.
    pub fn get_json(&self, key: &str) -> Result<Option<serde_json::Value>> {
        match self.get(key)? {
            Some(data) => {
                let value: serde_json::Value = serde_json::from_slice(&data)
                    .with_context(|| format!("Failed to parse JSON from KV entry: {}", key))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// List all keys in the store (excluding metadata files).
    pub fn list_keys(&self) -> Result<Vec<String>> {
        let mut keys = Vec::new();

        if !self.dir.exists() {
            return Ok(keys);
        }

        for entry in std::fs::read_dir(&self.dir)
            .context("Failed to read KV store directory")?
        {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip content-type metadata files
            if !name.ends_with(".content-type") {
                keys.push(name);
            }
        }

        keys.sort();
        Ok(keys)
    }

    /// Delete a key from the store.
    pub fn delete(&self, key: &str) -> Result<()> {
        let sanitized = sanitize_key(key);
        let value_path = self.dir.join(&sanitized);
        let meta_path = self.dir.join(format!("{}.content-type", sanitized));

        if value_path.exists() {
            std::fs::remove_file(&value_path)?;
        }
        if meta_path.exists() {
            std::fs::remove_file(&meta_path)?;
        }

        Ok(())
    }

    /// Get the store directory path.
    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

/// Sanitize a key for safe filesystem use.
/// Replaces path separators and special chars with underscores.
fn sanitize_key(key: &str) -> String {
    key.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

// ─── Run Storage Layout ────────────────────────────────────────────

/// Creates the storage layout for an actor run.
///
/// ```text
/// actors/{actor_id}/runs/{run_id}/
/// ├── dataset.jsonl          ← ActorDataset
/// ├── key-value-store/       ← ActorKvStore
/// │   ├── INPUT              ← serialized input
/// │   └── OUTPUT             ← final output (optional)
/// └── log.txt                ← stdout/stderr capture
/// ```
pub struct RunStorage {
    pub run_dir: PathBuf,
    pub dataset: ActorDataset,
    pub kv_store: ActorKvStore,
    pub log_path: PathBuf,
}

impl RunStorage {
    /// Initialize storage for a new actor run.
    pub fn create(actors_dir: &Path, actor_id: &str, run_id: &str) -> Result<Self> {
        let run_dir = actors_dir.join(actor_id).join("runs").join(run_id);
        std::fs::create_dir_all(&run_dir)
            .context("Failed to create run directory")?;

        let dataset_path = run_dir.join("dataset.jsonl");
        let kv_dir = run_dir.join("key-value-store");
        let log_path = run_dir.join("log.txt");

        let dataset = ActorDataset::new(dataset_path)?;
        let kv_store = ActorKvStore::new(kv_dir)?;

        Ok(Self {
            run_dir,
            dataset,
            kv_store,
            log_path,
        })
    }

    /// Store the input JSON in the KV store under the "INPUT" key.
    pub fn store_input(&self, input: &serde_json::Value) -> Result<()> {
        self.kv_store.set_json("INPUT", input)
    }

    /// Open or create the run storage for an existing run.
    pub fn open(actors_dir: &Path, actor_id: &str, run_id: &str) -> Result<Self> {
        let run_dir = actors_dir.join(actor_id).join("runs").join(run_id);
        if !run_dir.exists() {
            anyhow::bail!(
                "Run directory not found: {}",
                run_dir.display()
            );
        }

        let dataset_path = run_dir.join("dataset.jsonl");
        let kv_dir = run_dir.join("key-value-store");
        let log_path = run_dir.join("log.txt");

        let dataset = ActorDataset::new(dataset_path)?;
        let kv_store = ActorKvStore::new(kv_dir)?;

        Ok(Self {
            run_dir,
            dataset,
            kv_store,
            log_path,
        })
    }
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── Dataset Tests ──────────────────────────────────────────────

    #[test]
    fn test_dataset_push_and_read() {
        let tmp = TempDir::new().unwrap();
        let ds = ActorDataset::new(tmp.path().join("data.jsonl")).unwrap();

        ds.push_data(&[
            serde_json::json!({"url": "https://a.com", "title": "A"}),
            serde_json::json!({"url": "https://b.com", "title": "B"}),
        ])
        .unwrap();

        assert_eq!(ds.count().unwrap(), 2);

        let items = ds.get_items(0, 10).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["url"], "https://a.com");
        assert_eq!(items[1]["title"], "B");
    }

    #[test]
    fn test_dataset_pagination() {
        let tmp = TempDir::new().unwrap();
        let ds = ActorDataset::new(tmp.path().join("data.jsonl")).unwrap();

        for i in 0..20 {
            ds.push_data(&[serde_json::json!({"i": i})]).unwrap();
        }

        assert_eq!(ds.count().unwrap(), 20);

        let page1 = ds.get_items(0, 5).unwrap();
        assert_eq!(page1.len(), 5);
        assert_eq!(page1[0]["i"], 0);

        let page2 = ds.get_items(5, 5).unwrap();
        assert_eq!(page2.len(), 5);
        assert_eq!(page2[0]["i"], 5);

        let page_end = ds.get_items(18, 10).unwrap();
        assert_eq!(page_end.len(), 2);
    }

    #[test]
    fn test_dataset_empty() {
        let tmp = TempDir::new().unwrap();
        let ds = ActorDataset::new(tmp.path().join("no.jsonl")).unwrap();
        assert_eq!(ds.count().unwrap(), 0);
        assert!(ds.get_items(0, 10).unwrap().is_empty());
    }

    #[test]
    fn test_dataset_export_csv() {
        let tmp = TempDir::new().unwrap();
        let ds = ActorDataset::new(tmp.path().join("data.jsonl")).unwrap();

        ds.push_data(&[
            serde_json::json!({"name": "Alice", "age": 30}),
            serde_json::json!({"name": "Bob", "age": 25}),
        ])
        .unwrap();

        let mut buf = Vec::new();
        ds.export_csv(&mut buf).unwrap();
        let csv = String::from_utf8(buf).unwrap();
        // Headers could be either order, but both values should be present
        assert!(csv.contains("name"));
        assert!(csv.contains("age"));
        assert!(csv.contains("Alice"));
        assert!(csv.contains("Bob"));
    }

    #[test]
    fn test_dataset_append() {
        let tmp = TempDir::new().unwrap();
        let ds = ActorDataset::new(tmp.path().join("data.jsonl")).unwrap();

        ds.push_data(&[serde_json::json!({"a": 1})]).unwrap();
        ds.push_data(&[serde_json::json!({"b": 2})]).unwrap();

        assert_eq!(ds.count().unwrap(), 2);
    }

    // ── KV Store Tests ─────────────────────────────────────────────

    #[test]
    fn test_kv_store_set_get() {
        let tmp = TempDir::new().unwrap();
        let kv = ActorKvStore::new(tmp.path().join("kv")).unwrap();

        kv.set("hello", b"world", "text/plain").unwrap();
        let result = kv.get("hello").unwrap().unwrap();
        assert_eq!(result, b"world");
        assert_eq!(kv.get_content_type("hello").unwrap(), "text/plain");
    }

    #[test]
    fn test_kv_store_json() {
        let tmp = TempDir::new().unwrap();
        let kv = ActorKvStore::new(tmp.path().join("kv")).unwrap();

        let data = serde_json::json!({"key": "value", "n": 42});
        kv.set_json("config", &data).unwrap();

        let result = kv.get_json("config").unwrap().unwrap();
        assert_eq!(result["key"], "value");
        assert_eq!(result["n"], 42);
    }

    #[test]
    fn test_kv_store_not_found() {
        let tmp = TempDir::new().unwrap();
        let kv = ActorKvStore::new(tmp.path().join("kv")).unwrap();
        assert!(kv.get("nonexistent").unwrap().is_none());
    }

    #[test]
    fn test_kv_store_list_keys() {
        let tmp = TempDir::new().unwrap();
        let kv = ActorKvStore::new(tmp.path().join("kv")).unwrap();

        kv.set("alpha", b"a", "text/plain").unwrap();
        kv.set("beta", b"b", "text/plain").unwrap();

        let keys = kv.list_keys().unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"alpha".to_string()));
        assert!(keys.contains(&"beta".to_string()));
    }

    #[test]
    fn test_kv_store_delete() {
        let tmp = TempDir::new().unwrap();
        let kv = ActorKvStore::new(tmp.path().join("kv")).unwrap();

        kv.set("deleteme", b"data", "text/plain").unwrap();
        assert!(kv.get("deleteme").unwrap().is_some());

        kv.delete("deleteme").unwrap();
        assert!(kv.get("deleteme").unwrap().is_none());
    }

    #[test]
    fn test_sanitize_key() {
        assert_eq!(sanitize_key("hello/world"), "hello_world");
        assert_eq!(sanitize_key("path\\to\\file"), "path_to_file");
        assert_eq!(sanitize_key("normal-key_123"), "normal-key_123");
        assert_eq!(sanitize_key("a:b*c?d"), "a_b_c_d");
    }

    // ── Run Storage Tests ──────────────────────────────────────────

    #[test]
    fn test_run_storage_create() {
        let tmp = TempDir::new().unwrap();
        let storage = RunStorage::create(tmp.path(), "my-actor", "run-001").unwrap();

        assert!(storage.run_dir.exists());
        assert!(storage.kv_store.dir().exists());

        // Store input and verify
        let input = serde_json::json!({"url": "https://example.com"});
        storage.store_input(&input).unwrap();

        let stored = storage.kv_store.get_json("INPUT").unwrap().unwrap();
        assert_eq!(stored["url"], "https://example.com");
    }

    #[test]
    fn test_run_storage_open_nonexistent() {
        let tmp = TempDir::new().unwrap();
        let result = RunStorage::open(tmp.path(), "no-actor", "no-run");
        assert!(result.is_err());
    }
}
