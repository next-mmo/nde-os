# NDE-OS — Agent Rules

> **⚠️ Heavy Development** — APIs, manifests, and UI change without notice. Not production-ready.

## What This Is

**NDE-OS is a sandboxed virtual operating system for AI applications** — a self-contained desktop environment that **never touches the host OS**. All files, env vars, and processes are jailed to the NDE-OS workspace. Think Pinokio / Openclaw, but with a macOS-style virtual desktop, autonomous agent gateway, and zero host footprint. Users install & run AI apps (Stable Diffusion, ComfyUI, Ollama, Whisper, etc.) from a manifest catalog — each app gets a filesystem-jailed workspace with its own `uv`-managed Python venv.

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

#### Dev Mode Behavior (Important)

- **Always expanded**: In `import.meta.env.DEV`, the desktop always starts expanded (`collapsed = false`). `collapseDesktop()` is a no-op. No FAB-expansion logic needed in E2E specs.
- **Test hook**: `window.__svelteDesktop` is exposed in dev mode only (`main.ts`). Use `window.__svelteDesktop.openStaticApp("app-id")` to open windows programmatically in specs.

#### Known Gotchas

- **Tasks directory CWD**: `cargo tauri dev` runs the binary from `desktop/src-tauri/`. `tasks_dir()` in `agent_tasks.rs` now walks up from CWD to find `.agents/tasks/` (the repo root). E2E specs that write task files must target the path that `tasks_dir()` resolves to — confirmed by checking `desktop/src-tauri/.agents/tasks/` in dev mode.
- **`data-card-id` includes `.md`**: The Rust backend returns full filenames including extension (`task-name.md`). Playwright selectors must use `[data-card-id="task-name.md"]`, not `[data-card-id="task-name"]`.
- **HTML5 drag in WebView2**: `page.dragTo()` uses Pointer/Mouse events which do NOT fire `DragEvent` in WebView2. Always dispatch drag events manually via `page.evaluate()`: `dragstart` → `dragover` → `drop` → `dragend`.
- **`aria-hidden` elements**: The desktop wallpaper has `aria-hidden="true"`, making `toBeVisible()` fail. Use `waitForSelector('[data-testid="dock"], [data-window]')` to check if the desktop shell is ready.
- **Window remount for fresh data**: Svelte's `onMount` only fires once per component lifecycle. If a window stays open between tests, `get_agent_tasks` won't re-fetch. Close the window before reopening to force an `onMount` refresh:
  ```ts
  // Close existing window to force onMount re-run
  await page.evaluate(() => {
    document.querySelector('[data-window="vibe-studio"] button[aria-label="Close Desktop Window"]')?.click();
  });
  await page.waitForTimeout(500);
  // Re-open
  await page.evaluate(() => window.__svelteDesktop?.openStaticApp("vibe-studio"));
  ```
- **Tauri event channels**: Frontend `emit("event://name")` does NOT trigger Svelte `listen("event://name")` — those listen for backend-emitted events only. Triggering board refresh from tests requires either closing/reopening the window or using `window.__svelteDesktop.refreshKanban()`.

## 7. Global Definition of Done (DoD)

Before marking any task, ticket, or feature as complete, the following system-wide criteria must be met to ensure zero-mistake execution, flawless user experience, and deep backend performance:

### 7.1 Deep Rust Backend & Performance

- **Production Ready**: Zero mocks, zero hacks, zero `TODO`s. Code must be robust and completely error-handled.
- **Core Tests & Panics**: If `core/` or `server/` crates are modified, scoped unit tests (`cargo test -p <changed-crate> -- <test_name>`) MUST pass. Absolutely no panics allowed; use `anyhow::Result` everywhere.
- **Deep Optimization**: Rust logic must be deeply optimized for memory and speed. Use the most efficient data structures. Keep `Arc<Mutex<>>` locks scoped to the absolute minimum necessary duration.
- **Sandbox Integrity**: The NDE-OS sandbox constraints must be respected (verify with `curl -s http://localhost:8080/api/sandbox/test/verify` if touching sandbox domains). Canonicalize all paths to prevent traversal.

### 7.2 Desktop UI & End-User Experience

- **Goal Alignment**: Does the implemented feature perfectly align with the user's initial goal? Validate the end-to-end functionality from the user's perspective before declaring it done.
- **UI/UX Quality (Svelte/Tailwind)**: Ensure the Svelte 5 frontend adheres to the macOS Ventura aesthetic (blur effects, smooth dock animations, shadcn-svelte components). No custom raw CSS.
- **Desktop E2E Tests (Playwright)**: If `desktop/` UI or Tauri commands are modified, Playwright E2E tests MUST be written/updated and passing _inside_ the Tauri WebView2 sandbox.
- **Tauri IPC Bridge**: IPC must be lightning-fast. Batch commands aggressively. Return serialized structs, never raw primitives. Ensure events stream progress instead of polling.
- **Cross-Platform Parity**: Absolutely no hardcoded path strings (enforce `PathBuf::join()`). Safe and tested `cfg!(windows)` vs Unix subprocess execution gating. Both `HOME` and `USERPROFILE` explicitly accounted for.

## 8. Ticket & Task Status Flow

When working on markdown tasks or tickets in `.agents/tasks/`, always adhere to this global status sequence. Do not deviate from these exact statuses.

- **Status Flow**: `🔴 plan` ➔ `🟡 yolo mode` ➔ `🟢 done by AI` ➔ `✅ verified` OR `🔴 re-open`
  - `🔴 plan`: The initial planning phase. Wait for user approval before starting (unless YOLO mode is requested/active).
  - `🟡 yolo mode`: AI is actively generating code, working through the checklist autonomously without stopping.
  - `🟢 done by AI`: AI has completed all checklist tasks and DoD items. Human manually reviews and tests. 
  - `✅ verified`: Human has manually verified the feature works correctly.
  - `🔴 re-open`: Human found issues; AI needs to pick the task back up and fix them.
