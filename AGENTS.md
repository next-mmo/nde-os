# AGENTS.md — AI Launcher

Instructions for AI coding agents (Claude Code, Cursor, Copilot, Windsurf, etc.) working on this project.

---

## Project Overview

AI Launcher is a cross-platform (Linux + Windows native, no WSL) sandboxed AI application manager. Users install and run AI apps (Stable Diffusion, Ollama, ComfyUI, etc.) inside isolated filesystem jails with per-app Python virtual environments managed by **uv**.

**Tech stack**: Rust, tiny_http, serde_json, uv (Python package manager), OpenAPI 3.0.3  
**Desktop frontend**: Svelte 5, Vite, `@neodrag/svelte`, `popmotion`, `unplugin-icons`, lightningcss — macOS Ventura-style UI

---

## Desktop Frontend Rules — macOS Style (MANDATORY)

**The desktop UI MUST use the macOS Ventura-style design from `macos-web-main/`.** This is the single source of truth for all visual patterns. Do NOT use shadcn, Tailwind, Material UI, or any other component library.

### Reference Project

The `macos-web-main/` directory contains the reference implementation. Key architecture:

```
macos-web-main/src/
├── components/
│   ├── Desktop/          # Desktop shell, bootup screen, context menu, Window system
│   │   └── Window/       # Draggable windows with TrafficLights (@neodrag/svelte)
│   ├── Dock/             # Animated dock with magnification (popmotion interpolation)
│   ├── TopBar/           # Menu bar, ActionCenter, clock
│   ├── SystemUI/         # System dialogs
│   └── apps/             # App components rendered inside windows via AppNexus.svelte
├── configs/apps/         # App configs (title, size, resizable, dock_breaks_before)
├── state/                # Svelte 5 runes ($state, $effect, $derived)
├── css/                  # theme.css (CSS variables), global.css, reset.css
├── actions/              # Svelte actions (elevation, click-outside, portal, trap-focus)
└── helpers/              # create-app-config, fade, random, sleep
```

### Mandatory UI Patterns

| Pattern | Rule |
|---------|------|
| **Window management** | Every app view opens as a draggable macOS-style window via `Window.svelte` + `@neodrag/svelte`. Windows have traffic lights (red=close, yellow=minimize, green=fullscreen). |
| **Dock** | All apps appear in the animated Dock at the bottom. Dock items use `popmotion` interpolation for magnification on hover. Icons live in `public/app-icons/{app_id}/256.webp`. |
| **TopBar** | Always visible at top with translucent blur. Shows current app menu via `MenuBar.svelte` and `ActionCenter` (theme toggle, Wi-Fi, etc.). |
| **Theming** | Use CSS custom properties from `theme.css` (`--system-color-*`, `--system-font-family`, `--system-cursor-*`). Support light/dark via `body.dark` class. Never hardcode colors. |
| **State management** | Use Svelte 5 runes (`$state`, `$effect`, `$derived`). App open/close/z-index state lives in `state/apps.svelte.ts`. |
| **Path alias** | Use `🍎` alias for `src/` (configured in `vite.config.ts` → `resolve.alias`). |
| **No Tailwind** | Style with scoped `<style>` in `.svelte` files using CSS variables from the theme. |
| **No SvelteKit** | Plain Svelte 5 + Vite. No SSR, no routing — all navigation is via window open/close. |

### Adding a New App to the Desktop

1. Add config in `configs/apps/apps-config.ts` using `create_app_config({ title, width, height, resizable })`
2. Add app ID to `state/apps.svelte.ts` (open, z_indices, fullscreen maps)
3. Create component in `components/apps/MyApp/MyApp.svelte`
4. Register in `components/apps/AppNexus.svelte` with `{#if app_id === 'my-app'}` branch
5. Place icon at `public/app-icons/my-app/256.webp`

### Visual Standards

- **Blur effects**: Use `backdrop-filter: blur()` for TopBar, Dock, and window chrome
- **Rounded corners**: Windows `border-radius: 0.75rem`, Dock `1.2rem`
- **Shadows**: Elevated shadow for active windows, subtle for inactive
- **Typography**: Inter font family (`@fontsource/inter`), system font stack fallback
- **Animations**: Spring-based for Dock magnification, `sineInOut` easing for window transitions
- **Cursors**: Custom macOS cursors from `public/cursors/`

---

## Architecture Rules

### Module Responsibilities

| Module | File | Owns | Never Does |
|--------|------|------|------------|
| `sandbox` | `src/sandbox/mod.rs` | Path validation, symlink defense, env var generation, workspace init, security verification | Network calls, process management |
| `uv_env` | `src/uv_env/mod.rs` | uv binary bootstrap, Python installation, venv creation, pip deps, venv env vars | Sandbox enforcement, process tracking |
| `app_manager` | `src/app_manager/mod.rs` | App lifecycle (install/launch/stop/uninstall), registry persistence, process tracking | HTTP handling, path security (delegates to sandbox) |
| `manifest` | `src/manifest/mod.rs` | Type definitions (AppManifest, AppStatus, InstalledApp), built-in catalog | Business logic |
| `main` | `src/main.rs` | HTTP server, routing, JSON responses, OpenAPI spec, Swagger UI | Business logic (delegates to app_manager) |

### Dependency Direction

```
main.rs
  ├── app_manager (uses sandbox + uv_env + manifest)
  ├── manifest    (standalone types)
  ├── sandbox     (standalone, no deps on other modules)
  └── uv_env      (standalone, no deps on other modules)
```

**Never** create circular dependencies between modules.

### Data Flow

```
HTTP Request → main.rs (parse + route) → app_manager (orchestrate) → sandbox + uv_env (execute) → Response
```

---

## Cross-Platform Requirements

**Every code path must work on both Linux and Windows without WSL.**

### Mandatory Platform Patterns

```rust
// File paths — ALWAYS use std::path, never hardcode separators
let path = workspace.join("data").join("file.txt");  // ✅
let path = format!("{}/data/file.txt", workspace);    // ❌

// Process spawning — ALWAYS branch on platform
if cfg!(windows) {
    Command::new("cmd").args(["/C", &cmd])
} else {
    Command::new("sh").args(["-c", &cmd])
}

// Python binary — ALWAYS use platform-aware names
let python = if cfg!(windows) { "python" } else { "python3" };
let pip = if cfg!(windows) { "pip" } else { "pip3" };

// Permissions — ALWAYS use cfg(unix) / cfg(windows)
#[cfg(unix)]
fn set_perms(path: &Path) { /* chmod 700 */ }
#[cfg(windows)]
fn set_perms(path: &Path) { /* icacls */ }

// Environment variables — ALWAYS set BOTH Unix and Windows variants
env_vars.push(("HOME", &root));           // Unix
env_vars.push(("USERPROFILE", &root));     // Windows
env_vars.push(("TMPDIR", &tmp));           // Unix
env_vars.push(("TEMP", &tmp));             // Windows
env_vars.push(("TMP", &tmp));              // Windows
```

### Platform Testing Checklist

Before merging any change, verify:

- [ ] `cargo build` succeeds on Linux
- [ ] `cargo build` succeeds on Windows (or cross-compile with `--target x86_64-pc-windows-msvc`)
- [ ] No hardcoded `/` or `\` path separators
- [ ] No Linux-only crates (check `nix`, etc.)
- [ ] No `#[cfg(unix)]` without matching `#[cfg(windows)]`
- [ ] Sandbox verify passes on both platforms

---

## Coding Standards

### Rust Style

- **Error handling**: Use `anyhow::Result` everywhere. Return errors, don't panic.
- **Logging**: Use `println!` with prefix tags: `[sandbox]`, `[uv]`, `[proc]`, `[registry]`, `[api]`
- **Concurrency**: `Arc<Mutex<>>` for shared state. Lock scope must be minimal.
- **Strings**: Use `.to_string_lossy()` for all path-to-string conversions (handles non-UTF8 on Windows).

```rust
// Lock scope — ALWAYS minimize
{
    let mut reg = self.registry.lock().unwrap();
    reg.insert(id, app);
} // lock dropped here
self.save_registry()?; // this can take time, don't hold the lock
```

### API Design

- **Response format**: Always `{ "success": bool, "message": string, "data": T | null }`
- **Status codes**: 200 (ok), 201 (created), 400 (bad request), 404 (not found), 409 (conflict), 500 (error)
- **Content-Type**: Always `application/json`
- **CORS**: Always `Access-Control-Allow-Origin: *`

### Sandbox Rules

**CRITICAL — these are security invariants that must never be broken:**

1. `Sandbox::resolve()` must ALWAYS canonicalize paths before checking containment
2. Symlinks must be resolved (via `canonicalize`) before any `starts_with` check
3. Every process must have `HOME`/`USERPROFILE` jailed to the workspace
4. No readonly path may be writable — only the workspace root grants write access
5. `Sandbox::verify()` must test all 4 vectors: traversal, absolute, symlink, valid path
6. Sandbox verification runs on EVERY install, not just first time

### uv Integration Rules

1. **uv is best-effort**: If uv fails (no network, wrong platform), fall back gracefully. The sandbox is the security layer, uv is for convenience.
2. **Never shell out to pip directly**: Always go through uv (`uv pip install`), which is faster and handles venvs properly.
3. **One venv per app**: Each app gets `workspace/.venv/`. Never share venvs.
4. **Bootstrap on first run**: Auto-download uv if not found. Don't require manual install.
5. **Cache per workspace**: Set `UV_CACHE_DIR=workspace/.uv_cache/` so each app has isolated cache.

---

## File Descriptions

### `src/main.rs`
HTTP server using `tiny_http`. Synchronous request handling (one at a time, robust against crashes). Contains the full OpenAPI 3.0.3 spec as a `serde_json::Value`, and serves Swagger UI from CDN. All routes delegate to `AppManager`.

### `src/sandbox/mod.rs`
The security core. `Sandbox::new()` creates the workspace, sets permissions. `Sandbox::resolve()` is the path validator — handles traversal, absolute paths, symlink resolution. `Sandbox::verify()` runs all 4 security tests and returns a structured result. `Sandbox::env_vars()` generates the jailed environment variables (both Linux and Windows variants).

### `src/uv_env/mod.rs`
Manages uv binary lifecycle. `ensure_uv()` bootstraps uv (check bundled → check PATH → download). `UvEnv` wraps per-app operations: `ensure_python()`, `create_venv()`, `install_deps()`. `build_launch_cmd()` rewrites `python3 app.py` to use the venv python. `env_vars()` returns `VIRTUAL_ENV` and modified `PATH`.

### `src/app_manager/mod.rs`
Orchestrates install/launch/stop/uninstall. `install()` chains: sandbox creation → security verify → uv python → venv → deps. `launch()` builds combined env vars (sandbox + uv), rewrites the launch command, and spawns the process. Uses `Arc<Mutex<>>` for thread-safe registry and running process tracking.

### `src/manifest/mod.rs`
Pure data types: `AppManifest`, `AppStatus` (tagged enum), `InstalledApp`, `InstallRequest`. Also contains factory methods for built-in catalog apps (`gradio_demo()`, `stable_diffusion()`, `ollama()`). No business logic.

---

## Adding a New Feature

### Adding a new API endpoint

1. Add the handler function in the `match` block in `main.rs`
2. Add the path to the `openapi_spec()` function
3. Update the startup banner `println!` list
4. Test with curl

### Adding a new app to the catalog

1. Create `apps/my-app/manifest.json`
2. Add a factory method in `manifest/mod.rs`: `pub fn my_app() -> Self { ... }`
3. Add it to the `catalog()` method in `app_manager/mod.rs`

### Adding a new sandbox security test

1. Add the test to `Sandbox::verify()` in `sandbox/mod.rs`
2. Add the field to `SandboxVerifyResult`
3. Update the OpenAPI schema in `main.rs`
4. Update the dashboard UI verification panel

### Adding a new plugin type

1. Define the plugin schema in `docs/plugin-spec.md`
2. Create the plugin loader in a new `src/plugins/` module
3. Add lifecycle hooks to `app_manager`
4. Register plugin API routes in `main.rs`

---

## Testing

### Manual API Test Script

```bash
# Health
curl -s http://localhost:8080/api/health

# Install
curl -s -X POST http://localhost:8080/api/apps \
  -H "Content-Type: application/json" \
  -d '{"manifest":{"id":"test","name":"Test","description":"x","author":"x",
       "python_version":"3","needs_gpu":false,"pip_deps":[],"launch_cmd":
       "python3 -m http.server 7860","port":7860,"env":[],"disk_size":"1MB","tags":[]}}'

# Verify sandbox
curl -s http://localhost:8080/api/sandbox/test/verify

# Launch
curl -s -X POST http://localhost:8080/api/apps/test/launch

# Stop
curl -s -X POST http://localhost:8080/api/apps/test/stop

# Uninstall
curl -s -X DELETE http://localhost:8080/api/apps/test
```

### Expected Sandbox Verification

All four checks must return `true`:
```json
{
  "path_traversal_blocked": true,
  "absolute_escape_blocked": true,
  "symlink_escape_blocked": true,
  "valid_path_works": true
}
```

---

## Common Pitfalls

| Pitfall | Fix |
|---------|-----|
| Server crashes after N requests | Use `server.recv()` loop, not iterator. Handle errors gracefully. |
| Borrow checker on `format!` in match | Bind to `let` before the match, or use `.to_string()` |
| Windows paths fail | Use `PathBuf::join()`, never string concatenation |
| uv hangs on install | Network blocked. Make uv steps non-fatal (sandbox is the security layer). |
| `tiny_http` no async | Intentional. Synchronous is simpler and more robust for this use case. |
| Temporary `to_string_lossy()` dropped | Bind to `let binding = path.to_string_lossy().to_string();` first |

---

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `serde` | 1.0.197 | JSON serialization |
| `serde_json` | 1.0.114 | JSON parsing |
| `anyhow` | 1.0.81 | Error handling |
| `chrono` | 0.4.35 | Timestamps |
| `tiny_http` | 0.12 | HTTP server (zero async deps, works on Rust 1.75) |

**No platform-specific crates.** All platform branching uses `cfg!(windows)` / `cfg!(unix)` with stdlib.
