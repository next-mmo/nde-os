use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ─── Browser Engine Types ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrowserEngine {
    Wayfern,   // Chromium-based with C++ fingerprint patches
    Camoufox,  // Firefox-based with C++ fingerprint patches
}

impl BrowserEngine {
    pub fn as_str(&self) -> &'static str {
        match self {
            BrowserEngine::Wayfern => "wayfern",
            BrowserEngine::Camoufox => "camoufox",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "wayfern" => Ok(BrowserEngine::Wayfern),
            "camoufox" => Ok(BrowserEngine::Camoufox),
            _ => anyhow::bail!("Unknown browser engine: {s}"),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            BrowserEngine::Wayfern => "Wayfern (Chromium)",
            BrowserEngine::Camoufox => "Camoufox (Firefox)",
        }
    }
}

// ─── Proxy Settings ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub proxy_type: ProxyType,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProxyType {
    Http,
    Https,
    Socks4,
    Socks5,
}

impl ProxyConfig {
    /// Format as a URL string: <type>://[user:pass@]host:port
    pub fn to_url(&self) -> String {
        let scheme = match self.proxy_type {
            ProxyType::Http => "http",
            ProxyType::Https => "https",
            ProxyType::Socks4 => "socks4",
            ProxyType::Socks5 => "socks5",
        };

        match (&self.username, &self.password) {
            (Some(user), Some(pass)) => {
                format!("{scheme}://{user}:{pass}@{}:{}", self.host, self.port)
            }
            _ => format!("{scheme}://{}:{}", self.host, self.port),
        }
    }

    /// Format as Chromium --proxy-server argument
    pub fn to_chromium_arg(&self) -> String {
        format!("--proxy-server={}://{}:{}", 
            match self.proxy_type {
                ProxyType::Http | ProxyType::Https => "http",
                ProxyType::Socks4 | ProxyType::Socks5 => "socks5",
            },
            self.host, 
            self.port
        )
    }
}

// ─── Browser Executable Discovery ──────────────────────────────────

/// Find the browser executable in a given installation directory.
/// Cross-platform: handles Windows (.exe), macOS (.app/Contents/MacOS/), Linux (binary).
pub fn find_executable(engine: &BrowserEngine, install_dir: &Path) -> Result<PathBuf> {
    match engine {
        BrowserEngine::Wayfern => find_chromium_executable(install_dir),
        BrowserEngine::Camoufox => find_firefox_executable(install_dir),
    }
}

fn find_chromium_executable(install_dir: &Path) -> Result<PathBuf> {
    let candidates = if cfg!(target_os = "windows") {
        vec![
            install_dir.join("chrome.exe"),
            install_dir.join("chromium.exe"),
            install_dir.join("wayfern.exe"),
            install_dir.join("chrome-win").join("chrome.exe"),
        ]
    } else if cfg!(target_os = "macos") {
        // On macOS, find .app bundle
        if let Ok(entries) = std::fs::read_dir(install_dir) {
            let app_dir = entries
                .filter_map(|e| e.ok())
                .find(|e| {
                    e.path().extension().is_some_and(|ext| ext == "app")
                })
                .map(|e| e.path().join("Contents").join("MacOS"));

            if let Some(macos_dir) = app_dir {
                vec![
                    macos_dir.join("Chromium"),
                    macos_dir.join("Wayfern"),
                    macos_dir.join("chrome"),
                ]
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    } else {
        // Linux
        vec![
            install_dir.join("chromium"),
            install_dir.join("chrome"),
            install_dir.join("wayfern"),
            install_dir.join("chrome-linux").join("chrome"),
        ]
    };

    candidates
        .into_iter()
        .find(|p| p.exists() && p.is_file())
        .context("Wayfern/Chromium executable not found in installation directory")
}

fn find_firefox_executable(install_dir: &Path) -> Result<PathBuf> {
    let candidates = if cfg!(target_os = "windows") {
        vec![
            install_dir.join("firefox.exe"),
            install_dir.join("camoufox.exe"),
        ]
    } else if cfg!(target_os = "macos") {
        if let Ok(entries) = std::fs::read_dir(install_dir) {
            let app_dir = entries
                .filter_map(|e| e.ok())
                .find(|e| {
                    e.path().extension().is_some_and(|ext| ext == "app")
                })
                .map(|e| e.path().join("Contents").join("MacOS"));

            if let Some(macos_dir) = app_dir {
                vec![
                    macos_dir.join("camoufox"),
                    macos_dir.join("firefox"),
                ]
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    } else {
        // Linux
        vec![
            install_dir.join("camoufox-bin"),
            install_dir.join("camoufox"),
            install_dir.join("firefox"),
        ]
    };

    candidates
        .into_iter()
        .find(|p| p.exists() && p.is_file())
        .context("Camoufox/Firefox executable not found in installation directory")
}

/// Build launch arguments for the browser engine.
pub fn build_launch_args(
    engine: &BrowserEngine,
    profile_data_dir: &Path,
    proxy: Option<&ProxyConfig>,
    url: Option<&str>,
    debugging_port: Option<u16>,
    headless: bool,
) -> Vec<String> {
    match engine {
        BrowserEngine::Wayfern => build_chromium_args(profile_data_dir, proxy, url, debugging_port, headless),
        BrowserEngine::Camoufox => build_firefox_args(profile_data_dir, proxy, url, debugging_port, headless),
    }
}

fn build_chromium_args(
    profile_data_dir: &Path,
    proxy: Option<&ProxyConfig>,
    url: Option<&str>,
    debugging_port: Option<u16>,
    headless: bool,
) -> Vec<String> {
    let mut args = vec![
        format!("--user-data-dir={}", profile_data_dir.display()),
        "--no-default-browser-check".into(),
        "--disable-background-mode".into(),
        "--disable-component-update".into(),
        "--disable-background-timer-throttling".into(),
        "--crash-server-url=".into(),
        "--disable-updater".into(),
        "--disable-session-crashed-bubble".into(),
        "--hide-crash-restore-bubble".into(),
        "--disable-infobars".into(),
        "--disable-features=DialMediaRouteProvider".into(),
        "--use-mock-keychain".into(),
        "--password-store=basic".into(),
    ];

    if let Some(port) = debugging_port {
        args.push("--remote-debugging-address=127.0.0.1".into());
        args.push(format!("--remote-debugging-port={port}"));
    }

    if headless {
        args.push("--headless=new".into());
    }

    if let Some(proxy) = proxy {
        args.push(proxy.to_chromium_arg());
    }

    if let Some(url) = url {
        args.push(url.to_string());
    }

    args
}

fn build_firefox_args(
    profile_data_dir: &Path,
    _proxy: Option<&ProxyConfig>,
    url: Option<&str>,
    debugging_port: Option<u16>,
    headless: bool,
) -> Vec<String> {
    let mut args = vec![
        "-profile".into(),
        profile_data_dir.display().to_string(),
        "-no-remote".into(),
    ];

    if let Some(port) = debugging_port {
        args.push("--start-debugger-server".into());
        args.push(port.to_string());
    }

    if headless {
        args.push("--headless".into());
    }

    if let Some(url) = url {
        args.push(url.to_string());
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_engine_roundtrip() {
        assert_eq!(BrowserEngine::from_str("wayfern").unwrap(), BrowserEngine::Wayfern);
        assert_eq!(BrowserEngine::from_str("camoufox").unwrap(), BrowserEngine::Camoufox);
        assert!(BrowserEngine::from_str("unknown").is_err());
        assert_eq!(BrowserEngine::Wayfern.as_str(), "wayfern");
        assert_eq!(BrowserEngine::Camoufox.as_str(), "camoufox");
    }

    #[test]
    fn test_proxy_config_to_url() {
        let proxy = ProxyConfig {
            proxy_type: ProxyType::Socks5,
            host: "127.0.0.1".into(),
            port: 9050,
            username: Some("user".into()),
            password: Some("pass".into()),
        };
        assert_eq!(proxy.to_url(), "socks5://user:pass@127.0.0.1:9050");

        let proxy_no_auth = ProxyConfig {
            proxy_type: ProxyType::Http,
            host: "proxy.example.com".into(),
            port: 8080,
            username: None,
            password: None,
        };
        assert_eq!(proxy_no_auth.to_url(), "http://proxy.example.com:8080");
    }

    #[test]
    fn test_chromium_args_include_profile_dir() {
        let dir = std::path::PathBuf::from("/profiles/test-uuid/profile");
        let args = build_chromium_args(&dir, None, None, Some(9222), false);
        assert!(args.iter().any(|a| a.contains("--user-data-dir=")));
        assert!(args.iter().any(|a| a.contains("--remote-debugging-port=9222")));
    }

    #[test]
    fn test_firefox_args_include_profile_dir() {
        let dir = std::path::PathBuf::from("/profiles/test-uuid/profile");
        let args = build_firefox_args(&dir, None, Some("https://example.com"), None, false);
        assert!(args.contains(&"-profile".to_string()));
        assert!(args.contains(&"https://example.com".to_string()));
    }
}
