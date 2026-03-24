---
name: tauri-best-practices
description: Tauri 2 best practices for performance, security, IPC, and cross-platform development. Use when writing Tauri commands, configuring permissions, optimizing app size, designing IPC patterns, or reviewing Tauri code for performance and security issues.
---

# Tauri 2 Best Practices

## Quick Start

When writing Tauri 2 code, always:
1. Use **typed IPC commands** with serde — never pass raw strings
2. **Minimize IPC calls** — batch data, return structs not primitives
3. **Use async commands** for I/O — never block the main thread
4. **Scope permissions** per-window via capabilities
5. **Strip release binaries** and enable LTO for smallest app size

## Architecture Principles

### Process Model
Tauri uses a **multi-process architecture**: one Rust "core" process and one or more WebView processes. The core process owns all privileged operations. The WebView is untrusted by default.

```
┌─────────────────────────────────────┐
│           Core Process (Rust)        │
│  ┌──────────┐  ┌──────────────────┐ │
│  │ Commands  │  │  Event System    │ │
│  │ (IPC)     │  │  (pub/sub)       │ │
│  └──────────┘  └──────────────────┘ │
│  ┌──────────┐  ┌──────────────────┐ │
│  │ Plugins   │  │  State Manager   │ │
│  └──────────┘  └──────────────────┘ │
└─────────────┬───────────────────────┘
              │ IPC Bridge
┌─────────────┴───────────────────────┐
│        WebView Process (JS/TS)       │
│  ┌──────────┐  ┌──────────────────┐ │
│  │ invoke()  │  │  listen/emit     │ │
│  └──────────┘  └──────────────────┘ │
└─────────────────────────────────────┘
```

### Key Rule: Core Owns State
All persistent state, filesystem access, process spawning, and network requests **must** go through the core process via IPC commands. The WebView should only handle UI rendering and user interaction.

## IPC Performance

### DO: Batch Data in Single Commands

```rust
// ✅ GOOD: Return a struct with all needed data
#[tauri::command]
async fn get_dashboard(state: State<'_, AppState>) -> Result<Dashboard, String> {
    let apps = state.get_apps().await;
    let stats = state.get_stats().await;
    let notifications = state.get_notifications().await;
    Ok(Dashboard { apps, stats, notifications })
}
```

```rust
// ❌ BAD: Multiple separate calls from frontend
#[tauri::command]
async fn get_apps() -> Result<Vec<App>, String> { /* ... */ }
#[tauri::command]
async fn get_stats() -> Result<Stats, String> { /* ... */ }
#[tauri::command]
async fn get_notifications() -> Result<Vec<Notification>, String> { /* ... */ }
```

### DO: Use Async Commands for I/O

```rust
// ✅ GOOD: Async — won't block the main thread
#[tauri::command]
async fn read_config(path: PathBuf) -> Result<Config, String> {
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}
```

```rust
// ❌ BAD: Sync I/O blocks the main thread, freezes UI
#[tauri::command]
fn read_config_sync(path: PathBuf) -> Result<Config, String> {
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}
```

### DO: Use Events for Streaming Data

```rust
// ✅ GOOD: Stream progress via events, not polling
#[tauri::command]
async fn install_app(
    app: &tauri::AppHandle,
    manifest: Manifest,
) -> Result<(), String> {
    let total = manifest.deps.len();
    for (i, dep) in manifest.deps.iter().enumerate() {
        install_dep(dep).await?;
        app.emit("install-progress", InstallProgress {
            current: i + 1,
            total,
            dep: dep.name.clone(),
        }).map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

```typescript
// Frontend: listen for events
import { listen } from '@tauri-apps/api/event';

const unlisten = await listen<InstallProgress>('install-progress', (event) => {
  updateProgressBar(event.payload.current, event.payload.total);
});
```

### DO: Type Everything with Serde

```rust
// ✅ GOOD: Strongly typed request/response
#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchRequest {
    pub app_id: String,
    pub gpu: bool,
    pub env_overrides: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LaunchResult {
    pub pid: u32,
    pub port: u16,
    pub log_path: PathBuf,
}

#[tauri::command]
async fn launch_app(req: LaunchRequest) -> Result<LaunchResult, String> {
    // ...
}
```

### IPC Payload Size

- Keep payloads **under 1 MB**. For large data (images, files), write to disk and pass the path.
- Use `Vec<u8>` for binary data — Tauri serializes it efficiently as base64.
- For very large binary transfers, use the asset protocol or custom protocol instead of IPC.

## State Management

### DO: Use Managed State

```rust
// ✅ GOOD: Tauri-managed state with interior mutability
pub struct AppState {
    db: Mutex<Database>,
    config: RwLock<Config>,
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            db: Mutex::new(Database::connect()),
            config: RwLock::new(Config::load()),
        })
        .invoke_handler(tauri::generate_handler![get_config, update_config])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
async fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    Ok(state.config.read().await.clone())
}
```

### DO: Minimize Lock Scope

```rust
// ✅ GOOD: Lock only for the critical section
#[tauri::command]
async fn update_app_status(
    state: State<'_, AppState>,
    app_id: String,
    status: AppStatus,
) -> Result<(), String> {
    let result = {
        let mut apps = state.apps.lock().await;
        apps.update_status(&app_id, status)?
        // Lock drops here
    };
    // Do expensive work outside the lock
    notify_watchers(&app_id, &result).await;
    Ok(())
}
```

## Security

### Capability-Based Permissions

Always define **per-window capabilities** — never grant blanket access.

```json
// src-tauri/capabilities/main-window.json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "main-window",
  "description": "Capabilities for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "shell:allow-open",
    {
      "identifier": "fs:allow-read-text-file",
      "allow": [{ "path": "$APPDATA/**" }]
    }
  ]
}
```

### Content Security Policy

Always set CSP — it's your primary defense against XSS in the WebView.

```json
// tauri.conf.json → security
{
  "security": {
    "csp": {
      "default-src": "'self' customprotocol: asset:",
      "connect-src": "ipc: http://ipc.localhost",
      "font-src": ["https://fonts.gstatic.com"],
      "img-src": "'self' asset: http://asset.localhost blob: data:",
      "style-src": "'unsafe-inline' 'self' https://fonts.googleapis.com"
    },
    "freezePrototype": true
  }
}
```

### Input Validation in Commands

```rust
// ✅ GOOD: Validate all inputs from the WebView
#[tauri::command]
async fn read_file(
    state: State<'_, AppState>,
    path: PathBuf,
) -> Result<String, String> {
    // Canonicalize to prevent path traversal
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    let allowed_root = state.workspace_root.canonicalize().map_err(|e| e.to_string())?;

    if !canonical.starts_with(&allowed_root) {
        return Err("Access denied: path outside workspace".into());
    }

    tokio::fs::read_to_string(&canonical)
        .await
        .map_err(|e| e.to_string())
}
```

### Freeze Prototype

Always enable `freezePrototype: true` in production — prevents prototype pollution attacks.

## App Size Optimization

### Cargo Profile for Release

```toml
# Cargo.toml
[profile.release]
panic = "abort"       # Remove unwinding code
codegen-units = 1     # Better optimization, slower compile
lto = true            # Link-Time Optimization
opt-level = "s"       # Optimize for size ("z" for even smaller)
strip = true          # Strip debug symbols
```

### Frontend UI Stack

- **shadcn-svelte + Tailwind only**. No custom `<style>` blocks or raw CSS.
- **macOS Ventura aesthetic**: backdrop blur, traffic-light window controls (`@neodrag/svelte`), dock magnification animations, vibrancy layers.
- Use shadcn-svelte components as the base — extend via Tailwind utilities, never override with inline styles.
- Prefer `cn()` utility for conditional class merging.

```svelte
<!-- ✅ GOOD: shadcn-svelte + Tailwind -->
<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { cn } from '$lib/utils';
</script>

<Button
  variant="ghost"
  class={cn(
    "backdrop-blur-xl bg-white/10 rounded-xl",
    "hover:bg-white/20 transition-all duration-200"
  )}
>
  Launch App
</Button>
```

```svelte
<!-- ❌ BAD: Custom <style> or raw CSS -->
<button class="my-button">Launch App</button>

<style>
  .my-button { background: rgba(255,255,255,0.1); }
</style>
```

### Frontend Optimization

- Use **tree-shaking** — import only what you use from `@tauri-apps/api`
- **Code-split** routes — lazy load heavy components
- **Compress assets** — use WebP/AVIF, minify CSS/JS
- Use `vite build` with `rollupOptions.output.manualChunks` for optimal chunking

```typescript
// ✅ GOOD: Import only what you need
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// ❌ BAD: Import everything
import * as tauri from '@tauri-apps/api';
```

### Exclude Unused Features

```toml
# Cargo.toml — only enable what you use
[dependencies]
tauri = { version = "2", features = [] }
# Don't enable "devtools" in production
```

```json
// tauri.conf.json — disable unused built-in APIs
{
  "app": {
    "withGlobalTauri": false
  }
}
```

## Cross-Platform

### Path Handling

```rust
// ✅ GOOD: Use PathBuf::join(), never hardcode separators
let config_path = app_handle
    .path()
    .app_config_dir()
    .map_err(|e| e.to_string())?
    .join("settings.json");
```

```rust
// ❌ BAD: Hardcoded separator
let config_path = format!("{}/settings.json", config_dir);
```

### Process Spawning

```rust
// ✅ GOOD: Platform-aware shell commands
fn shell_command(script: &str) -> Command {
    if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", script]);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", script]);
        cmd
    }
}
```

### Environment Variables

```rust
// ✅ GOOD: Set both Unix and Windows home vars
fn set_home_env(cmd: &mut Command, home: &Path) {
    cmd.env("HOME", home);           // Unix
    cmd.env("USERPROFILE", home);     // Windows
}
```

## Plugin Development

### Best Practices

```rust
// ✅ GOOD: Plugin with proper init, state, and commands
use tauri::plugin::{Builder, TauriPlugin};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("my-plugin")
        .setup(|app, _api| {
            app.manage(MyPluginState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            my_command_1,
            my_command_2,
        ])
        .build()
}
```

### Plugin Permissions

```toml
# permissions/default.toml
"$schema" = "schemas/schema.json"

[default]
description = "Default permissions for my-plugin"
permissions = ["allow-my-command-1", "allow-my-command-2"]
```

## Error Handling

### Consistent Error Types

```rust
// ✅ GOOD: Use anyhow in internals, convert at the boundary
use anyhow::{Context, Result};

#[tauri::command]
async fn install_app(manifest: Manifest) -> Result<(), String> {
    install_app_inner(manifest)
        .await
        .map_err(|e| format!("{e:#}"))
}

async fn install_app_inner(manifest: Manifest) -> Result<()> {
    let workspace = create_workspace(&manifest.id)
        .context("Failed to create workspace")?;
    install_deps(&workspace, &manifest.pip_deps)
        .await
        .context("Failed to install dependencies")?;
    Ok(())
}
```

### Typed Errors for Frontend

```rust
// ✅ GOOD: Structured errors the frontend can handle
#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
pub enum AppError {
    NotFound { id: String },
    AlreadyRunning { id: String, pid: u32 },
    InstallFailed { id: String, reason: String },
    PermissionDenied { path: String },
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { id } => write!(f, "App '{id}' not found"),
            Self::AlreadyRunning { id, pid } => write!(f, "App '{id}' already running (PID {pid})"),
            Self::InstallFailed { id, reason } => write!(f, "Install of '{id}' failed: {reason}"),
            Self::PermissionDenied { path } => write!(f, "Permission denied: {path}"),
        }
    }
}
```

## Testing

### Unit Test Commands

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_command_validates_input() {
        let state = AppState::test_default();
        let result = read_file(
            State::new(&state),
            PathBuf::from("../../etc/passwd"),
        ).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Access denied"));
    }
}
```

### E2E with Playwright + CDP

For Tauri desktop apps, always connect via CDP — never navigate to `localhost`.

```typescript
// e2e/fixtures.ts
import { chromium, type Page } from '@playwright/test';

export async function connectToTauri(): Promise<Page> {
    const browser = await chromium.connectOverCDP('http://localhost:9222');
    const context = browser.contexts()[0];
    return context.pages()[0];
}
```

## Quick Reference Checklist

- [ ] All IPC commands are `async`
- [ ] Payloads are typed with `Serialize`/`Deserialize`
- [ ] Lock scopes are minimized (`Arc<Mutex<>>`)
- [ ] CSP is configured (not disabled)
- [ ] `freezePrototype: true` in production
- [ ] Capabilities scoped per-window
- [ ] Path traversal prevented via canonicalization
- [ ] `profile.release` has `lto`, `strip`, `opt-level = "s"`
- [ ] Frontend tree-shakes `@tauri-apps/api` imports
- [ ] Cross-platform: `PathBuf::join()`, `cfg!(windows)`, both `HOME` + `USERPROFILE`
- [ ] Errors use `anyhow` internally, `String`/typed errors at boundary
- [ ] Events used for streaming data (not polling)
