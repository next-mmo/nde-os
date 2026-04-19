//! Tauri IPC commands for checking NDE-OS application updates.
//!
//! Checks the GitHub Releases API for the latest version and compares it
//! against the current running version (from Cargo.toml at compile time).

use serde::Serialize;

/// The current app version baked in at compile time.
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub owner/repo for release checks.
const GITHUB_OWNER: &str = "next-mmo";
const GITHUB_REPO: &str = "nde-os";

/// Result of a "check for updates" call.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    /// The currently running version (e.g. "0.3.0").
    pub current_version: String,
    /// The latest version available on GitHub (e.g. "0.4.0"), if check succeeded.
    pub latest_version: Option<String>,
    /// Whether an update is available (latest > current).
    pub update_available: bool,
    /// The release name / title from GitHub.
    pub release_name: Option<String>,
    /// The URL to the GitHub release page so the user can download.
    pub release_url: Option<String>,
    /// The release body / changelog markdown.
    pub release_body: Option<String>,
    /// ISO-8601 published date.
    pub published_at: Option<String>,
    /// Human-readable error if the check failed (network, rate-limit, etc).
    pub error: Option<String>,
}

/// Check for updates by querying the GitHub Releases API.
#[tauri::command]
pub async fn check_for_updates() -> Result<UpdateCheckResult, String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        GITHUB_OWNER, GITHUB_REPO
    );

    let client = reqwest::Client::builder()
        .user_agent(format!("NDE-OS/{}", CURRENT_VERSION))
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            return Ok(UpdateCheckResult {
                current_version: CURRENT_VERSION.to_string(),
                latest_version: None,
                update_available: false,
                release_name: None,
                release_url: None,
                release_body: None,
                published_at: None,
                error: Some(format!("Failed to reach GitHub: {e}")),
            });
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        return Ok(UpdateCheckResult {
            current_version: CURRENT_VERSION.to_string(),
            latest_version: None,
            update_available: false,
            release_name: None,
            release_url: None,
            release_body: None,
            published_at: None,
            error: Some(format!("GitHub API returned HTTP {status}")),
        });
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read GitHub response: {e}"))?;

    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse GitHub response: {e}"))?;

    // Extract tag_name (e.g. "v0.4.0") and strip the leading 'v'
    let tag = json["tag_name"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();

    let latest_version = if tag.is_empty() { None } else { Some(tag.clone()) };
    let update_available = !tag.is_empty() && is_newer(&tag, CURRENT_VERSION);

    let str_field = |key: &str| -> Option<String> {
        json.get(key).and_then(|v: &serde_json::Value| v.as_str()).map(String::from)
    };

    Ok(UpdateCheckResult {
        current_version: CURRENT_VERSION.to_string(),
        latest_version,
        update_available,
        release_name: str_field("name"),
        release_url: str_field("html_url"),
        release_body: str_field("body"),
        published_at: str_field("published_at"),
        error: None,
    })
}

/// Simple semver comparison: returns true if `latest` is strictly greater than `current`.
/// Handles versions like "0.3.0", "1.2.3", etc.
fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.split('.')
            .filter_map(|s| s.parse::<u64>().ok())
            .collect()
    };

    let l = parse(latest);
    let c = parse(current);

    // Compare component by component
    for i in 0..l.len().max(c.len()) {
        let lv = l.get(i).copied().unwrap_or(0);
        let cv = c.get(i).copied().unwrap_or(0);
        if lv > cv {
            return true;
        }
        if lv < cv {
            return false;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer() {
        assert!(is_newer("0.4.0", "0.3.0"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(is_newer("0.3.1", "0.3.0"));
        assert!(!is_newer("0.3.0", "0.3.0"));
        assert!(!is_newer("0.2.0", "0.3.0"));
        assert!(!is_newer("0.3.0", "0.3.1"));
    }

    #[test]
    fn test_current_version_is_set() {
        assert!(!CURRENT_VERSION.is_empty());
    }
}
