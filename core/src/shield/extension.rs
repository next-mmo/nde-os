use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::browser::BrowserEngine;

// ─── Extension Types ──────────────────────────────────────────────

/// Source of an extension installation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionSource {
    /// Uploaded via developer mode (sideloaded unpacked or packed)
    Developer,
    /// Installed from URL (store or direct download)
    Store,
}

/// Browser compatibility for an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionFormat {
    /// Chromium extension (CRX / unpacked with manifest.json v2/v3)
    Chromium,
    /// Firefox extension (XPI / unpacked with manifest.json)
    Firefox,
}

/// Metadata for an installed browser extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub format: ExtensionFormat,
    pub source: ExtensionSource,
    pub source_url: Option<String>,
    pub permissions: Vec<String>,
    pub icon_relative: Option<String>,
    pub installed_at: u64,
    pub updated_at: u64,
}

/// Per-profile extension binding: which extensions are enabled for which profiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileExtensionBinding {
    pub extension_id: String,
    pub enabled: bool,
}

/// Per-profile extension configuration stored alongside profile metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileExtensions {
    pub bindings: Vec<ProfileExtensionBinding>,
}

// ─── Extension Manager ────────────────────────────────────────────

/// Manages the global extension pool and per-profile bindings.
///
/// Storage layout:
/// ```text
/// {base_dir}/shield-extensions/
/// ├── {ext-id}/
/// │   ├── meta.json        # ExtensionMeta
/// │   └── extension/       # Unpacked extension files
/// ```
///
/// Per-profile bindings are stored in:
/// ```text
/// {base_dir}/shield-profiles/{profile-id}/extensions.json
/// ```
pub struct ExtensionManager {
    extensions_dir: PathBuf,
    profiles_dir: PathBuf,
}

impl ExtensionManager {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            extensions_dir: base_dir.join("shield-extensions"),
            profiles_dir: base_dir.join("shield-profiles"),
        }
    }

    // ─── Global Extension Pool ────────────────────────────────

    /// List all installed extensions.
    pub fn list_extensions(&self) -> Result<Vec<ExtensionMeta>> {
        if !self.extensions_dir.exists() {
            return Ok(Vec::new());
        }

        let mut extensions = Vec::new();
        for entry in fs::read_dir(&self.extensions_dir).context("Failed to read extensions dir")? {
            let entry = entry?;
            let meta_path = entry.path().join("meta.json");
            if meta_path.exists() {
                let content = fs::read_to_string(&meta_path)
                    .with_context(|| format!("Failed to read {}", meta_path.display()))?;
                let meta: ExtensionMeta = serde_json::from_str(&content)
                    .with_context(|| format!("Failed to parse {}", meta_path.display()))?;
                extensions.push(meta);
            }
        }

        extensions.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(extensions)
    }

    /// Get a single extension by ID.
    pub fn get_extension(&self, ext_id: &str) -> Result<ExtensionMeta> {
        let meta_path = self.extensions_dir.join(ext_id).join("meta.json");
        let content = fs::read_to_string(&meta_path)
            .with_context(|| format!("Extension '{ext_id}' not found"))?;
        serde_json::from_str(&content).context("Failed to parse extension metadata")
    }

    /// Get the unpacked extension directory for a given extension ID.
    pub fn extension_dir(&self, ext_id: &str) -> PathBuf {
        self.extensions_dir.join(ext_id).join("extension")
    }

    /// Install an extension from an unpacked directory (developer mode sideload).
    ///
    /// Copies the directory contents into the global extension pool.
    pub fn install_from_directory(&self, source_dir: &Path) -> Result<ExtensionMeta> {
        let manifest_path = source_dir.join("manifest.json");
        if !manifest_path.exists() {
            anyhow::bail!(
                "No manifest.json found in '{}'. Not a valid browser extension.",
                source_dir.display()
            );
        }

        let (name, version, description, author, permissions, icon, format) =
            parse_extension_manifest(&manifest_path)?;

        let ext_id = uuid::Uuid::new_v4().to_string();
        let dest_dir = self.extensions_dir.join(&ext_id);
        let ext_dir = dest_dir.join("extension");

        fs::create_dir_all(&ext_dir).context("Failed to create extension directory")?;
        copy_dir_recursive(source_dir, &ext_dir)?;

        let now = epoch_secs();
        let meta = ExtensionMeta {
            id: ext_id,
            name,
            version,
            description,
            author,
            format,
            source: ExtensionSource::Developer,
            source_url: None,
            permissions,
            icon_relative: icon,
            installed_at: now,
            updated_at: now,
        };

        let meta_json =
            serde_json::to_string_pretty(&meta).context("Failed to serialize extension meta")?;
        fs::write(dest_dir.join("meta.json"), meta_json)
            .context("Failed to write extension meta")?;

        Ok(meta)
    }

    /// Install an extension from a packed file (CRX or XPI).
    ///
    /// Extracts the archive and installs into the global pool.
    pub fn install_from_packed(&self, packed_path: &Path) -> Result<ExtensionMeta> {
        let ext_name = packed_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();

        // Create a temp extraction directory
        let temp_dir = self
            .extensions_dir
            .join(format!(".tmp-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir).context("Failed to create temp extraction dir")?;

        let result = (|| -> Result<ExtensionMeta> {
            // CRX files are ZIP archives with a header prefix; XPI files are plain ZIPs
            extract_extension_archive(packed_path, &temp_dir)
                .with_context(|| format!("Failed to extract '{ext_name}'"))?;

            // The extracted content might be in a subdirectory or directly in temp_dir
            let manifest_dir = find_manifest_dir(&temp_dir)?;
            self.install_from_directory(&manifest_dir)
        })();

        // Clean up temp dir regardless of success/failure
        let _ = fs::remove_dir_all(&temp_dir);

        result
    }

    /// Install an extension from a URL (download + install).
    pub async fn install_from_url(&self, url: &str) -> Result<ExtensionMeta> {
        let client = reqwest::Client::builder()
            .user_agent("NDE-OS-Shield/1.0")
            .build()
            .context("Failed to build HTTP client")?;

        let resp = client
            .get(url)
            .send()
            .await
            .with_context(|| format!("Failed to download extension from {url}"))?;

        if !resp.status().is_success() {
            anyhow::bail!("Download failed with status {}", resp.status());
        }

        // Determine filename from URL or content-disposition
        let filename = url
            .rsplit('/')
            .next()
            .unwrap_or("extension.zip")
            .to_string();

        let bytes = resp.bytes().await.context("Failed to read response body")?;

        // Write to temp file
        let temp_path = self
            .extensions_dir
            .join(format!(".download-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&self.extensions_dir)
            .context("Failed to create extensions directory")?;

        let file_path = temp_path.with_extension(
            Path::new(&filename)
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .as_ref(),
        );
        fs::write(&file_path, &bytes).context("Failed to write downloaded file")?;

        let result = self.install_from_packed(&file_path);

        // Clean up downloaded file
        let _ = fs::remove_file(&file_path);

        let mut meta = result?;
        meta.source = ExtensionSource::Store;
        meta.source_url = Some(url.to_string());

        // Update meta on disk
        let meta_path = self.extensions_dir.join(&meta.id).join("meta.json");
        let json = serde_json::to_string_pretty(&meta)?;
        fs::write(&meta_path, json)?;

        Ok(meta)
    }

    /// Uninstall an extension globally and remove it from all profiles.
    pub fn uninstall(&self, ext_id: &str) -> Result<()> {
        let ext_dir = self.extensions_dir.join(ext_id);
        if !ext_dir.exists() {
            anyhow::bail!("Extension '{ext_id}' not found");
        }

        // Remove from all profile bindings
        if self.profiles_dir.exists() {
            if let Ok(entries) = fs::read_dir(&self.profiles_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let bindings_path = entry.path().join("extensions.json");
                    if bindings_path.exists() {
                        if let Ok(mut pe) = self.read_profile_extensions_file(&bindings_path) {
                            pe.bindings.retain(|b| b.extension_id != ext_id);
                            let _ = self.write_profile_extensions_file(&bindings_path, &pe);
                        }
                    }
                }
            }
        }

        fs::remove_dir_all(&ext_dir)
            .with_context(|| format!("Failed to remove extension directory for '{ext_id}'"))?;

        Ok(())
    }

    /// Update extension metadata (e.g., after editing name/description).
    pub fn update_meta(&self, ext_id: &str, name: Option<&str>, description: Option<&str>) -> Result<ExtensionMeta> {
        let mut meta = self.get_extension(ext_id)?;

        if let Some(n) = name {
            meta.name = n.to_string();
        }
        if let Some(d) = description {
            meta.description = d.to_string();
        }
        meta.updated_at = epoch_secs();

        let meta_path = self.extensions_dir.join(ext_id).join("meta.json");
        let json = serde_json::to_string_pretty(&meta)?;
        fs::write(&meta_path, json)?;

        Ok(meta)
    }

    // ─── Per-Profile Bindings ─────────────────────────────────

    /// Get extensions bound to a profile (with enabled/disabled state).
    pub fn get_profile_extensions(&self, profile_id: &str) -> Result<ProfileExtensions> {
        let path = self.profile_extensions_path(profile_id);
        if !path.exists() {
            return Ok(ProfileExtensions::default());
        }
        self.read_profile_extensions_file(&path)
    }

    /// Bind an extension to a profile (initially enabled).
    pub fn bind_to_profile(&self, profile_id: &str, ext_id: &str) -> Result<()> {
        // Verify extension exists
        let _ = self.get_extension(ext_id)?;

        let mut pe = self.get_profile_extensions(profile_id)?;

        // Don't duplicate
        if pe.bindings.iter().any(|b| b.extension_id == ext_id) {
            return Ok(());
        }

        pe.bindings.push(ProfileExtensionBinding {
            extension_id: ext_id.to_string(),
            enabled: true,
        });

        self.save_profile_extensions(profile_id, &pe)
    }

    /// Unbind an extension from a profile.
    pub fn unbind_from_profile(&self, profile_id: &str, ext_id: &str) -> Result<()> {
        let mut pe = self.get_profile_extensions(profile_id)?;
        pe.bindings.retain(|b| b.extension_id != ext_id);
        self.save_profile_extensions(profile_id, &pe)
    }

    /// Enable or disable an extension for a profile.
    pub fn set_extension_enabled(
        &self,
        profile_id: &str,
        ext_id: &str,
        enabled: bool,
    ) -> Result<()> {
        let mut pe = self.get_profile_extensions(profile_id)?;

        let binding = pe
            .bindings
            .iter_mut()
            .find(|b| b.extension_id == ext_id)
            .context("Extension not bound to this profile")?;

        binding.enabled = enabled;
        self.save_profile_extensions(profile_id, &pe)
    }

    /// Get the list of extension directories that should be loaded for a profile launch.
    /// Only returns enabled extensions that are compatible with the given engine.
    pub fn get_launch_extensions(
        &self,
        profile_id: &str,
        engine: &BrowserEngine,
    ) -> Result<Vec<PathBuf>> {
        let pe = self.get_profile_extensions(profile_id)?;
        let target_format = match engine {
            BrowserEngine::Wayfern => ExtensionFormat::Chromium,
            BrowserEngine::Camoufox => ExtensionFormat::Firefox,
        };

        let mut paths = Vec::new();
        for binding in &pe.bindings {
            if !binding.enabled {
                continue;
            }
            if let Ok(meta) = self.get_extension(&binding.extension_id) {
                if meta.format == target_format {
                    let ext_path = self.extension_dir(&binding.extension_id);
                    if ext_path.exists() {
                        paths.push(ext_path);
                    }
                }
            }
        }
        Ok(paths)
    }

    /// Get all extensions with their binding state for a specific profile.
    /// Returns (meta, bound, enabled) tuples for UI display.
    pub fn list_extensions_for_profile(
        &self,
        profile_id: &str,
    ) -> Result<Vec<(ExtensionMeta, bool, bool)>> {
        let all = self.list_extensions()?;
        let pe = self.get_profile_extensions(profile_id)?;

        let binding_map: HashMap<&str, bool> = pe
            .bindings
            .iter()
            .map(|b| (b.extension_id.as_str(), b.enabled))
            .collect();

        Ok(all
            .into_iter()
            .map(|meta| {
                let bound = binding_map.contains_key(meta.id.as_str());
                let enabled = binding_map.get(meta.id.as_str()).copied().unwrap_or(false);
                (meta, bound, enabled)
            })
            .collect())
    }

    // ─── Helpers ──────────────────────────────────────────────

    fn profile_extensions_path(&self, profile_id: &str) -> PathBuf {
        self.profiles_dir.join(profile_id).join("extensions.json")
    }

    fn save_profile_extensions(&self, profile_id: &str, pe: &ProfileExtensions) -> Result<()> {
        let path = self.profile_extensions_path(profile_id);
        let parent = path.parent().context("Invalid profile path")?;
        fs::create_dir_all(parent)?;
        let json = serde_json::to_string_pretty(pe)?;
        fs::write(&path, json).context("Failed to write profile extensions")
    }

    fn read_profile_extensions_file(&self, path: &Path) -> Result<ProfileExtensions> {
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content).context("Failed to parse profile extensions")
    }

    fn write_profile_extensions_file(&self, path: &Path, pe: &ProfileExtensions) -> Result<()> {
        let json = serde_json::to_string_pretty(pe)?;
        fs::write(path, json).context("Failed to write profile extensions")
    }
}

// ─── Manifest Parsing ─────────────────────────────────────────────

/// Parse a browser extension manifest.json and extract metadata.
/// Supports both Chrome Manifest V2/V3 and Firefox WebExtension manifests.
fn parse_extension_manifest(
    manifest_path: &Path,
) -> Result<(
    String,            // name
    String,            // version
    String,            // description
    String,            // author
    Vec<String>,       // permissions
    Option<String>,    // icon relative path
    ExtensionFormat,   // format
)> {
    let content = fs::read_to_string(manifest_path).context("Failed to read manifest.json")?;
    let manifest: serde_json::Value =
        serde_json::from_str(&content).context("Failed to parse manifest.json")?;

    let name = manifest["name"]
        .as_str()
        .unwrap_or("Unknown Extension")
        .to_string();

    let version = manifest["version"]
        .as_str()
        .unwrap_or("0.0.0")
        .to_string();

    let description = manifest["description"].as_str().unwrap_or("").to_string();

    let author = manifest["author"]
        .as_str()
        .or_else(|| {
            manifest["developer"]["name"].as_str()
        })
        .unwrap_or("Unknown")
        .to_string();

    // Collect permissions from both Chrome and Firefox fields
    let mut permissions = Vec::new();
    if let Some(perms) = manifest["permissions"].as_array() {
        for p in perms {
            if let Some(s) = p.as_str() {
                permissions.push(s.to_string());
            }
        }
    }
    if let Some(perms) = manifest["host_permissions"].as_array() {
        for p in perms {
            if let Some(s) = p.as_str() {
                permissions.push(s.to_string());
            }
        }
    }

    // Find the largest icon
    let icon = if let Some(icons) = manifest["icons"].as_object() {
        // Pick the largest icon by numeric key
        icons
            .iter()
            .filter_map(|(k, v)| {
                let size: u32 = k.parse().ok()?;
                let path = v.as_str()?;
                Some((size, path.to_string()))
            })
            .max_by_key(|(size, _)| *size)
            .map(|(_, path)| path)
    } else {
        None
    };

    // Detect format: Firefox extensions use "browser_specific_settings" or
    // "applications" (legacy). Also check for "gecko" key inside those.
    let is_firefox = manifest.get("browser_specific_settings").is_some()
        || manifest
            .get("applications")
            .and_then(|a| a.get("gecko"))
            .is_some();

    let format = if is_firefox {
        ExtensionFormat::Firefox
    } else {
        ExtensionFormat::Chromium
    };

    Ok((name, version, description, author, permissions, icon, format))
}

// ─── Archive Extraction ───────────────────────────────────────────

/// Extract a CRX or XPI (both are ZIP-based) into a target directory.
///
/// CRX v3 files have a binary header before the ZIP data — we detect and skip it.
fn extract_extension_archive(packed_path: &Path, dest: &Path) -> Result<()> {
    let data = fs::read(packed_path).context("Failed to read packed extension file")?;

    // CRX v3 header detection: starts with "Cr24" magic bytes
    let zip_data = if data.len() > 16 && &data[0..4] == b"Cr24" {
        // CRX3 format:
        //   4 bytes: magic "Cr24"
        //   4 bytes: version (3)
        //   4 bytes: header length
        //   N bytes: header proto
        //   rest:    ZIP data
        let header_len = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
        let zip_offset = 12 + header_len;
        if zip_offset >= data.len() {
            anyhow::bail!("Invalid CRX file: header extends past file boundary");
        }
        &data[zip_offset..]
    } else {
        // Plain ZIP (XPI or unpacked-then-zipped)
        &data[..]
    };

    let cursor = std::io::Cursor::new(zip_data);
    let mut archive = zip::ZipArchive::new(cursor).context("Failed to open as ZIP archive")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let out_path = dest.join(
            file.enclosed_name()
                .context("Invalid file path in archive")?,
        );

        if file.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out_file = fs::File::create(&out_path)?;
            std::io::copy(&mut file, &mut out_file)?;
        }
    }

    Ok(())
}

/// Find the directory containing manifest.json (may be top-level or one level down).
fn find_manifest_dir(dir: &Path) -> Result<PathBuf> {
    // Check top level
    if dir.join("manifest.json").exists() {
        return Ok(dir.to_path_buf());
    }

    // Check one level deep
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            if p.is_dir() && p.join("manifest.json").exists() {
                return Ok(p);
            }
        }
    }

    anyhow::bail!(
        "No manifest.json found in extracted extension at '{}'",
        dir.display()
    )
}

// ─── Utilities ────────────────────────────────────────────────────

fn epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Recursively copy a directory tree.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src).with_context(|| format!("Failed to read {}", src.display()))? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).with_context(|| {
                format!(
                    "Failed to copy {} -> {}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_manager() -> (ExtensionManager, TempDir) {
        let tmp = TempDir::new().unwrap();
        let mgr = ExtensionManager::new(tmp.path());
        (mgr, tmp)
    }

    fn create_fake_extension(dir: &Path, name: &str, is_firefox: bool) {
        fs::create_dir_all(dir).unwrap();
        let mut manifest = serde_json::json!({
            "name": name,
            "version": "1.0.0",
            "description": "Test extension",
            "manifest_version": 3,
            "permissions": ["storage", "tabs"],
            "icons": { "48": "icon.png" }
        });
        if is_firefox {
            manifest["browser_specific_settings"] = serde_json::json!({
                "gecko": { "id": "test@example.com" }
            });
        }
        fs::write(
            dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(dir.join("icon.png"), b"fake-png").unwrap();
    }

    #[test]
    fn test_install_from_directory_chromium() {
        let (mgr, tmp) = test_manager();
        let ext_src = tmp.path().join("my-ext-src");
        create_fake_extension(&ext_src, "My Chrome Ext", false);

        let meta = mgr.install_from_directory(&ext_src).unwrap();
        assert_eq!(meta.name, "My Chrome Ext");
        assert_eq!(meta.format, ExtensionFormat::Chromium);
        assert_eq!(meta.source, ExtensionSource::Developer);
        assert!(meta.permissions.contains(&"storage".to_string()));

        // Verify files exist
        let ext_dir = mgr.extension_dir(&meta.id);
        assert!(ext_dir.join("manifest.json").exists());
        assert!(ext_dir.join("icon.png").exists());
    }

    #[test]
    fn test_install_from_directory_firefox() {
        let (mgr, tmp) = test_manager();
        let ext_src = tmp.path().join("my-ff-ext");
        create_fake_extension(&ext_src, "My Firefox Ext", true);

        let meta = mgr.install_from_directory(&ext_src).unwrap();
        assert_eq!(meta.format, ExtensionFormat::Firefox);
    }

    #[test]
    fn test_list_extensions() {
        let (mgr, tmp) = test_manager();

        let ext1 = tmp.path().join("ext1");
        create_fake_extension(&ext1, "Beta Ext", false);
        mgr.install_from_directory(&ext1).unwrap();

        let ext2 = tmp.path().join("ext2");
        create_fake_extension(&ext2, "Alpha Ext", true);
        mgr.install_from_directory(&ext2).unwrap();

        let list = mgr.list_extensions().unwrap();
        assert_eq!(list.len(), 2);
        // Sorted alphabetically
        assert_eq!(list[0].name, "Alpha Ext");
        assert_eq!(list[1].name, "Beta Ext");
    }

    #[test]
    fn test_uninstall() {
        let (mgr, tmp) = test_manager();
        let ext_src = tmp.path().join("removable");
        create_fake_extension(&ext_src, "Removable", false);

        let meta = mgr.install_from_directory(&ext_src).unwrap();
        let id = meta.id.clone();

        mgr.uninstall(&id).unwrap();
        assert!(mgr.list_extensions().unwrap().is_empty());
    }

    #[test]
    fn test_profile_binding() {
        let (mgr, tmp) = test_manager();

        // Create a fake profile directory
        let profile_id = "test-profile-123";
        fs::create_dir_all(tmp.path().join("shield-profiles").join(profile_id)).unwrap();

        let ext_src = tmp.path().join("bindable");
        create_fake_extension(&ext_src, "Bindable", false);
        let meta = mgr.install_from_directory(&ext_src).unwrap();

        // Bind
        mgr.bind_to_profile(profile_id, &meta.id).unwrap();
        let pe = mgr.get_profile_extensions(profile_id).unwrap();
        assert_eq!(pe.bindings.len(), 1);
        assert!(pe.bindings[0].enabled);

        // Disable
        mgr.set_extension_enabled(profile_id, &meta.id, false)
            .unwrap();
        let pe = mgr.get_profile_extensions(profile_id).unwrap();
        assert!(!pe.bindings[0].enabled);

        // Launch extensions (disabled => none returned)
        let launch = mgr
            .get_launch_extensions(profile_id, &BrowserEngine::Wayfern)
            .unwrap();
        assert!(launch.is_empty());

        // Re-enable
        mgr.set_extension_enabled(profile_id, &meta.id, true)
            .unwrap();
        let launch = mgr
            .get_launch_extensions(profile_id, &BrowserEngine::Wayfern)
            .unwrap();
        assert_eq!(launch.len(), 1);

        // Unbind
        mgr.unbind_from_profile(profile_id, &meta.id).unwrap();
        let pe = mgr.get_profile_extensions(profile_id).unwrap();
        assert!(pe.bindings.is_empty());
    }

    #[test]
    fn test_uninstall_removes_profile_bindings() {
        let (mgr, tmp) = test_manager();

        let profile_id = "cleanup-profile";
        fs::create_dir_all(tmp.path().join("shield-profiles").join(profile_id)).unwrap();

        let ext_src = tmp.path().join("cleanup-ext");
        create_fake_extension(&ext_src, "Cleanup", false);
        let meta = mgr.install_from_directory(&ext_src).unwrap();

        mgr.bind_to_profile(profile_id, &meta.id).unwrap();
        mgr.uninstall(&meta.id).unwrap();

        let pe = mgr.get_profile_extensions(profile_id).unwrap();
        assert!(pe.bindings.is_empty());
    }

    #[test]
    fn test_no_manifest_fails() {
        let (mgr, tmp) = test_manager();
        let empty_dir = tmp.path().join("empty");
        fs::create_dir_all(&empty_dir).unwrap();

        assert!(mgr.install_from_directory(&empty_dir).is_err());
    }

    #[test]
    fn test_format_detection() {
        let (mgr, tmp) = test_manager();

        // Firefox with applications.gecko
        let ff_dir = tmp.path().join("ff-legacy");
        fs::create_dir_all(&ff_dir).unwrap();
        fs::write(
            ff_dir.join("manifest.json"),
            r#"{"name":"FF Legacy","version":"1.0","applications":{"gecko":{"id":"x@y.com"}}}"#,
        )
        .unwrap();

        let meta = mgr.install_from_directory(&ff_dir).unwrap();
        assert_eq!(meta.format, ExtensionFormat::Firefox);
    }
}
