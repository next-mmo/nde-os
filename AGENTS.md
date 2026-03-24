# AI Launcher (NDE-OS) — Agent Rules

## What This Is

Sandboxed AI app launcher (like Pinokio, Openclaw) autonomous agent looping gateway chat with a macOS-style web desktop. Users install & run AI apps (Stable Diffusion, ComfyUI, Ollama, Whisper, etc.) from a manifest catalog — each app gets a filesystem-jailed workspace with its own `uv`-managed Python venv.

**Stack**: Rust backend (sandbox + app lifecycle + REST API) → Tauri 2 bridge, Tanstack Query → Svelte 5 desktop UI.

**Key subsystems**:

- `core/sandbox` — filesystem jail: path canonicalization, symlink defense, env-var jailing (`HOME`, `TMPDIR`, `XDG_*`, `APPDATA` all scoped to workspace).
- `core/uv_env` — auto-bootstraps `uv`, creates per-app `.venv`, installs pip deps 10-100× faster than pip.
- `core/app_manager` — app lifecycle: install → launch (subprocess) → stop → uninstall. Manifest-driven.
- `core/manifest` — JSON app manifest schema (id, python_version, pip_deps, launch_cmd, port, gpu, tags).
- `server/` — REST API (health, catalog, apps CRUD, launch/stop, sandbox verify, disk usage). OpenAPI 3.0.3.
- `desktop/` — macOS Ventura desktop shell (dock, windows, top bar, app catalog UI, settings).
- `plugins/` — extensible via hooks, monitors, providers, middleware, UI panels.

Monorepo: `core/` (Rust sandbox), `server/` (Rust API), `desktop/` (Tauri+Svelte5). `core` has ZERO deps on `server`/`desktop`.

## 1. Quality

- Run tests on unsure changes. No guessing.
- Production-ready only. No TODOs, no hacks, no mocks.

## 2. UI

- shadcn-svelte + Tailwind only. No custom `<style>` or raw CSS.
- macOS Ventura aesthetic: blur, traffic-light controls (`@neodrag/svelte`), dock animations.

## 3. Cross-Platform (No WSL)

- Paths: `PathBuf::join()` only, never hardcode separators.
- Processes: `cfg!(windows)` → `cmd /C`, Unix → `sh -c`.
- Env: set both `HOME` + `USERPROFILE` concurrently.
- Modular: Provider pattern, dependency injection, decoupled modules.

## 4. Rust

- `anyhow::Result` everywhere, no panics.
- Minimize `Arc<Mutex<>>` lock scopes.
- Canonicalize paths (symlink/traversal defense). Readonly stays readonly.
- `uv` not `pip`. One venv per workspace (`workspace/.venv`).

## 5. Tauri Performance & Security

Follow `.agents/skills/tauri-best-practices/SKILL.md` for all Tauri 2 code:

- **IPC**: Batch data in single commands, return structs not primitives. All commands `async`.
- **Events**: Stream progress via `app.emit()` — never poll from frontend.
- **Payloads**: Keep under 1 MB. Large data → write to disk, pass path.
- **State**: Use `tauri::manage()` with `Mutex`/`RwLock`. Minimize lock scope.
- **Security**: CSP configured, `freezePrototype: true`, per-window capabilities, path canonicalization.
- **App Size**: Release profile with `lto = true`, `strip = true`, `opt-level = "s"`, `panic = "abort"`, `codegen-units = 1`.
- **Frontend**: Tree-shake `@tauri-apps/api` imports, code-split routes, compress assets.
- **Errors**: `anyhow` internally, `String` or typed `Serialize` errors at the IPC boundary.

## 6. Testing

Tests fail = task NOT done. **Only test what you touched** — run scoped tests for changed crates/files, not the full suite.

- Scoped: `cargo test -p <changed-crate> -- <test_name>` | `npx playwright test <changed-spec>`
- Full suite only before final merge/PR.
- Sandbox verify: `curl -s http://localhost:8080/api/sandbox/test/verify`

### E2E (Playwright + Tauri)

**Tests MUST run inside the Tauri WebView2, not a standalone browser.** The app is a Tauri 2 desktop — behavior differs from browser (drag regions, window APIs, permissions). Always use the CDP fixtures.

- **CDP**: `dev.sh` sets `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9222"` on the Tauri process. Playwright connects via `chromium.connectOverCDP()`.
- **Fixtures**: All E2E specs import from `e2e/fixtures.ts` (provides CDP-connected `page`) and `e2e/helpers.ts`.
- **Run**: `cd desktop && npx playwright test e2e/<spec>.spec.ts --reporter=list` (requires `./dev.sh` running).
- **Never** use `page.goto("http://localhost:5174")` — the Tauri webview loads its own URL via the bridge.
