# NDE-OS Feature Status Tracker

> **Last Updated:** 2026-04-08  
> **Legend:** ✅ Working | ⚠️ Partial | ❌ Broken | 🔲 Not Tested  
> **Severity:** 🔴 Critical | 🟠 Major | 🟡 Minor | 🔵 Cosmetic

---

## 1. Desktop Shell

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 1.1 | Desktop wallpaper renders | ✅ | | Gradient-overlaid wallpaper with CSS custom property |
| 1.2 | Desktop icons show & open apps | ✅ | | DesktopIcons.svelte with full icon grid + click-to-open |
| 1.3 | Desktop right-click context menu | ✅ | | ContextMenu with Sort, Clean Up, Wallpaper, Dark Mode, Launchpad, Spotlight |
| 1.4 | Desktop icon drag & rearrange | ✅ | | Drag via `@neodrag/svelte`, positions saved |
| 1.5 | "Open With" menu on files | ✅ | | OpenWithMenu.svelte implemented |
| 1.6 | Lock Screen display / unlock | ✅ | | LockScreen.svelte renders when `desktop.is_locked` |
| 1.7 | Notification Center | ✅ | | NotificationCenter.svelte toggles via TopBar clock button |

## 2. Window Manager

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 2.1 | Window open / close | ✅ | | TrafficLights close + `closeWindow()` state action |
| 2.2 | Window minimize | ✅ | | `window.minimized` → opacity-0 + pointer-events-none |
| 2.3 | Window maximize / restore | ✅ | | Double-click title bar or traffic light → fullscreen toggle (`inset-0`) |
| 2.4 | Window drag (move) | ✅ | | `@neodrag/svelte` with `.window-drag-handle` titlebar control |
| 2.5 | Window resize | ✅ | | 8-direction pointer-based resize handles (n/s/e/w/ne/nw/se/sw) |
| 2.6 | Window focus / z-order stacking | ✅ | | `focusWindow()` sets z-index, active ring styling |
| 2.7 | Traffic-light controls (close/min/max) | ✅ | | TrafficLights.svelte with macOS-style red/yellow/green |
| 2.8 | Multiple windows at once | ✅ | | WindowsArea renders array of `desktop.windows` |

## 3. Dock

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 3.1 | Dock renders with app icons | ✅ | | 24 static apps + dynamic session/window icons via DockItem |
| 3.2 | Click dock icon → opens app | ✅ | | DockItem click opens static app or focuses window |
| 3.3 | Dock hover magnification animation | ✅ | | `mouseX` tracking passed to each DockItem for proximity magnification |
| 3.4 | Dock shows running-app dot indicator | ✅ | | Running dot rendered if app has open window/session |
| 3.5 | Dock separators between groups | ✅ | | `dock_breaks_before` config → 1px vertical divider |

## 4. Top Bar (Menu Bar)

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 4.1 | Top bar renders (clock, metrics) | ✅ | | TopBar.svelte with blur backdrop, drag region overlay |
| 4.2 | Top bar time/date display | ✅ | | TopBarTime.svelte with click → toggleNotificationCenter |
| 4.3 | System metrics (CPU/RAM/GPU) | ✅ | | TopBarMetrics.svelte polled every 5s via `refreshResourceUsage()` |
| 4.4 | Action Center toggle | ✅ | | ActionCenter.svelte with expandable panel |
| 4.5 | Menu bar shows active app name | ✅ | | MenuBar.svelte shows active window app name |

## 5. Launchpad

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 5.1 | Launchpad opens (grid of all apps) | ✅ | | Launchpad.svelte with fullscreen grid overlay |
| 5.2 | Click app icon → opens app window | ✅ | | Each grid icon calls openStaticApp() |
| 5.3 | Launchpad search/filter | ✅ | | Built-in search input filters app grid |
| 5.4 | Launchpad close on click outside | ✅ | | Backdrop click or Escape closes launchpad |

## 6. Spotlight Search

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 6.1 | Spotlight opens (keyboard shortcut / UI) | ✅ | | Ctrl+Space / ⌘Space global shortcut + context menu |
| 6.2 | Search apps by name | ✅ | | Spotlight.svelte with fuzzy search across all registered apps |
| 6.3 | Select result → opens app | ✅ | | Arrow-key navigation + Enter to open |

---

## 7. AI Launcher (App Catalog)

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 7.1 | Catalog loads & shows app cards | ✅ | | `GET /api/catalog` returns manifests; AppCard.svelte renders |
| 7.2 | Install an app from catalog | ✅ | | `POST /api/apps` via AppManager with uv venv |
| 7.3 | Uninstall an app | ✅ | | `DELETE /api/apps/{id}` removes workspace |
| 7.4 | Launch (start) an installed app | ✅ | | `POST /api/apps/{id}/launch` subprocess spawn |
| 7.5 | Stop a running app | ✅ | | `POST /api/apps/{id}/stop` process kill |
| 7.6 | App status indicator (running/stopped) | ✅ | | `GET /api/apps/{id}` returns runtime status |
| 7.7 | Install progress / streaming output | ⚠️ | 🟡 | uv install runs but no SSE progress stream to UI yet |

### Catalog Apps

| # | App | Install | Launch | Stop | Bug Description |
|---|-----|---------|--------|------|-----------------|
| 7.a | ComfyUI | 🔲 | 🔲 | 🔲 | |
| 7.b | Fooocus | 🔲 | 🔲 | 🔲 | |
| 7.c | Kohya-SS | 🔲 | 🔲 | 🔲 | |
| 7.d | Ollama | 🔲 | 🔲 | 🔲 | |
| 7.e | Open WebUI | 🔲 | 🔲 | 🔲 | |
| 7.f | Stable Diffusion | 🔲 | 🔲 | 🔲 | |
| 7.g | Whisper Web | 🔲 | 🔲 | 🔲 | |
| 7.h | Sample Gradio | 🔲 | 🔲 | 🔲 | |
| 7.i | Sample Node | 🔲 | 🔲 | 🔲 | |

## 8. NDE Chat (LLM Chat)

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 8.1 | Chat window opens | ✅ | | |
| 8.2 | Send message & get response | ✅ | | |
| 8.3 | Streaming response display | ✅ | | Fixed GGUF inference pipe deadlock |
| 8.4 | Conversation history | ✅ | | `GET /api/agent/conversations` + `GET /api/agent/conversations/{id}/messages` |
| 8.5 | Model selection / switching | ✅ | | `POST /api/models/switch` with `sync_agent_provider()` auto-propagation |

## 9. LLM Providers / Model Settings

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 9.1 | Model settings window opens | ✅ | | ModelSettings app in apps-config |
| 9.2 | Add / configure providers (API keys) | ✅ | | `POST /api/models/providers` + Codex OAuth flow |
| 9.3 | List available models | ✅ | | `GET /api/models` + `GET /api/models/local` + `GET /api/models/recommendations` |
| 9.4 | Test provider connection | ✅ | | `POST /api/models/verify` validates provider health |

## 10. Terminal

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 10.1 | Terminal window opens | ✅ | | Terminal app in apps-config |
| 10.2 | PTY session starts (interactive shell) | ✅ | | `spawn_pty` Tauri command → cmd.exe / bash with sandbox env |
| 10.3 | Execute commands | ✅ | | `write_pty` writes stdin; PTY reader thread streams output |
| 10.4 | Output rendering | ✅ | | `pty_read_{id}` event → bytes emitted to frontend |
| 10.5 | Multiple terminal tabs/instances | ⚠️ | 🟡 | PTY state supports multiple IDs; UI tab management not fully tested |

## 11. Code Editor

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 11.1 | Code editor window opens | ✅ | | CodeEditor app in apps-config |
| 11.2 | Open / edit files | ✅ | | `read_file_content` Tauri command (sandbox-jailed) |
| 11.3 | Syntax highlighting | ⚠️ | 🟡 | Frontend component exists, highlighting lib integration not verified |
| 11.4 | Save files | ✅ | | `write_file_content` Tauri command (sandbox-jailed) |

## 12. File Explorer

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 12.1 | File explorer window opens | ✅ | | FileExplorer app in apps-config |
| 12.2 | Browse sandbox filesystem | ✅ | | `list_directory` Tauri command with Sandbox jail enforcement |
| 12.3 | Create / delete files & folders | ✅ | | `create_folder`, `delete_entry` commands (jailed, tested) |
| 12.4 | Rename files | ✅ | | `rename_entry` command (old + new paths jailed) |
| 12.5 | Navigate directories | ✅ | | `get_home_dir` returns sandbox root; path navigation jailed |

## 13. Browser

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 13.1 | Browser window opens | ✅ | | Browser app in apps-config with `currentBrowserUrl` title |
| 13.2 | Navigate to URLs | ✅ | | Browser window with URL bar + navigation |
| 13.3 | URL bar works | ✅ | | Title bar shows current URL via `currentBrowserUrl()` |
| 13.4 | Page loads & renders | ⚠️ | 🟡 | Session-based iframe rendering; direct page load depends on CSP |

## 14. Shield Browser

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 14.1 | Shield browser opens | ✅ | | ShieldBrowser app in apps-config; full Tauri IPC commands |
| 14.2 | Shield protection features | ✅ | | ProfileManager, EngineManager, BrowserLauncher with fingerprint spoofing |
| 14.3 | Browsing works through shield | ✅ | | `launch_shield_profile` → Camoufox/Wayfern profile instance |
| 14.4 | Profile CRUD (create/list/rename/delete) | ✅ | | Full Tauri commands: create, list, get, rename, delete |
| 14.5 | Engine download with progress | ✅ | | `download_shield_engine` with % progress events to frontend |
| 14.6 | Android emulator management (ADB) | ✅ | | `core/shield/emulator.rs` — lifecycle, proxy, URL, screenshots |

## 15. Command Center

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 15.1 | Command center opens | ✅ | | CommandCenter app in apps-config |
| 15.2 | Run commands / agents | ✅ | | Agent tasks: `POST /api/agent/tasks` → spawn task with SSE stream |
| 15.3 | View command output | ✅ | | `GET /api/agent/tasks/{id}/stream` SSE + cancel support |

## 16. Vibe Code Studio (Kanban / Tasks)

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 16.1 | Vibe Studio window opens | ✅ | | VibeCodeStudio app in apps-config |
| 16.2 | Kanban board renders columns | ✅ | | `GET /api/kanban/tasks` returns grouped tasks |
| 16.3 | Task cards display | ✅ | | Cards with `data-card-id` including `.md` extension |
| 16.4 | Drag-and-drop cards between columns | ⚠️ | 🟡 | Manual drag dispatch needed in WebView2; basic DnD implemented |
| 16.5 | Create / edit / delete tasks | ✅ | | Full CRUD: POST create, PUT update status/content, DELETE |

## 17. FreeCut (Video Editor)

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 17.1 | FreeCut window opens | ✅ | | FreeCut app in apps-config; `core/freecut` + Tauri commands |
| 17.2 | Import media files | ✅ | | Tauri freecut commands for media import |
| 17.3 | Timeline renders | ✅ | | Multi-track timeline engine in `core/freecut` |
| 17.4 | Drag clips to timeline | ⚠️ | 🟡 | Backend engine supports it; frontend DnD UX not fully verified |
| 17.5 | Playback controls (play/pause/seek) | ✅ | | Tauri freecut commands for playback |
| 17.6 | Export / render video | ✅ | | GPU-accelerated FFmpeg pipeline in core |

## 18. Plugins

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 18.1 | Plugins window opens | ✅ | | Plugins app in apps-config |
| 18.2 | List installed plugins | ✅ | | `GET /api/plugins` via PluginEngine |
| 18.3 | Install / uninstall plugins | ✅ | | `POST /api/plugins/{id}/install` + discover |
| 18.4 | Plugin functionality works | ✅ | | Start/stop/logs: `POST .../start`, `POST .../stop`, `GET .../logs` |

### Plugin Subsystems (from `plugins/` directory)

| # | Plugin | Status | Bug Description |
|---|--------|--------|-----------------|
| 18.a | Backup & Restore | ✅ | Manifest discovered by PluginEngine |
| 18.b | Disk Cleaner | ✅ | Manifest discovered by PluginEngine |
| 18.c | GPU Monitor | ✅ | Manifest discovered by PluginEngine |
| 18.d | Log Viewer | ✅ | Manifest discovered by PluginEngine |
| 18.e | Model Downloader | ✅ | Manifest discovered by PluginEngine |
| 18.f | Port Manager | ✅ | Manifest discovered by PluginEngine |

## 19. Channels

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 19.1 | Channels window opens | ✅ | | Channels app in apps-config |
| 19.2 | Create / join channels | ✅ | | `POST /api/channels/{name}/configure` with AES-256-GCM token encryption |
| 19.3 | Send / receive messages | ✅ | | Telegram gateway: long-polling + tool execution + Kanban commands |

## 20. MCP Tools

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 20.1 | MCP Tools window opens | ✅ | | McpTools app in apps-config |
| 20.2 | List MCP servers / tools | ✅ | | `GET /api/mcp/tools` + `GET /api/mcp/servers` from builtin server |
| 20.3 | Invoke / test tools | ⚠️ | 🟡 | Tools registered in core; UI invocation flow not fully tested |

## 21. Skills

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 21.1 | Skills window opens | ✅ | | Skills app in apps-config |
| 21.2 | List available skills | ✅ | | `GET /api/skills` via real SkillLoader discovery |
| 21.3 | Execute / run a skill | ⚠️ | 🟡 | Core SkillExecutor exists; endpoint for execution not yet exposed |

## 22. Knowledge

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 22.1 | Knowledge window opens | ✅ | | Knowledge app in apps-config |
| 22.2 | Browse knowledge items | ✅ | | `GET /api/knowledge` via real KnowledgeGraph (SQLite-backed) |
| 22.3 | Search knowledge | ✅ | | `GET /api/knowledge/search?q=...` with LIKE search |

## 23. Architecture Viewer

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 23.1 | Architecture window opens | ✅ | | Architecture app in apps-config |
| 23.2 | System diagram / visualization | ⚠️ | 🟡 | Window opens; diagram content depends on Architecture component impl |

## 24. Screenshot

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 24.1 | Screenshot window opens | ✅ | | Screenshot app in apps-config + ScreenshotOverlay.svelte |
| 24.2 | Capture screenshot | ✅ | | `screenshot` Tauri command via `core/screenshot` (feature-gated) |
| 24.3 | View screenshot results | ✅ | | Screenshot results window displays captured image |

## 25. Service Hub

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 25.1 | Service Hub window opens | ✅ | | ServiceHub app in apps-config |
| 25.2 | List running services | ✅ | | `service_hub.rs` Tauri commands for service enumeration |
| 25.3 | Start / stop services | ✅ | | Lifecycle commands in `core/services` |
| 25.4 | View service logs | ⚠️ | 🟡 | Log viewing depends on plugin/service log output capture |

## 26. Settings

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 26.1 | Settings window opens | ✅ | | Settings app in apps-config |
| 26.2 | Change preferences | ⚠️ | 🟡 | Theme toggle works; full preferences panel scope not verified |
| 26.3 | Settings persist on restart | ⚠️ | 🟡 | Window geometry persists via localStorage; general prefs persistence TBD |

## 27. Logs

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 27.1 | Logs window opens | ✅ | | Logs app in apps-config + GlobalLogDrawer |
| 27.2 | Logs stream in real-time | ✅ | | `GET /api/logs?since={id}` polling with SharedLogBuffer; 3s poll interval |
| 27.3 | Filter / search logs | ⚠️ | 🟡 | Log entries returned; client-side filtering UI not fully verified |

## 28. Figma / JSON Playground

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 28.1 | Figma Render opens | ✅ | | FigmaRender app + `core/figma_json` engine + Tauri commands |
| 28.2 | JSON Playground opens | ✅ | | JsonPlayground app in apps-config |
| 28.3 | Render Figma JSON | ✅ | | `figma_json.rs` Tauri commands for rendering Figma JSON nodes |

---

## 29. Backend / Core (Rust)

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 29.1 | REST API server starts | ✅ | | `tiny_http` on :8080 with 8-thread worker pool + backpressure |
| 29.2 | Sandbox filesystem jail | ✅ | | Path canonicalization, symlink defense, blocked filenames, traversal protection |
| 29.3 | `uv` environment bootstrap | ✅ | | `core/uv_env` auto-bootstraps uv, creates per-app `.venv` |
| 29.4 | App install (pip deps via uv) | ✅ | | AppManager → uv pip install 10-100× faster than pip |
| 29.5 | App launch (subprocess spawn) | ✅ | | `POST /api/apps/{id}/launch` with jailed env vars |
| 29.6 | App stop (process kill) | ✅ | | `POST /api/apps/{id}/stop` terminates subprocess |
| 29.7 | Disk usage reporting | ✅ | | `GET /api/sandbox/{id}/disk` returns usage |
| 29.8 | OpenViking server lifecycle | ✅ | | Install/start/stop/status via `/api/viking/*` routes |
| 29.9 | Node.js environment bootstrap | ✅ | | `core/node_env` module for Node.js sandbox |
| 29.10 | Git clone / pull for app install | ✅ | | `git.rs` Tauri command for git operations |

## 30. Tauri IPC Bridge

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 30.1 | Tauri app launches | ✅ | | `main.rs` + `lib.rs` with state management and managed resources |
| 30.2 | Frontend ↔ backend IPC works | ✅ | | 16 command modules: apps, catalog, filesystem, freecut, git, lifecycle, pty, sandbox, screenshot, service_hub, shield, system, viking, figma_json, agent_tasks |
| 30.3 | Events streaming (progress, logs) | ✅ | | `app.emit()` for PTY output, shield download progress, SSE agent streams |
| 30.4 | Window management commands | ✅ | | Desktop state Svelte 5 store with full window CRUD + geometry persistence |

## 31. Gateway (Telegram)

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 31.1 | Telegram bot long-polling | ✅ | | `gateway/telegram.rs` (32KB) — robust long-polling with auto-reconnect |
| 31.2 | Kanban commands via Telegram | ✅ | | Direct-execution slash commands for task management |
| 31.3 | Token encryption (AES-256-GCM) | ✅ | | `core/secrets.rs` with per-install key derivation |
| 31.4 | Session isolation per chat | ✅ | | Context scoping via chat_id |
| 31.5 | Gateway status monitoring | ✅ | | `GatewayState` with atomic counters (messages_received/sent) |

## 32. Agent Runtime

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 32.1 | AgentManager lifecycle (boot/recover) | ✅ | | `on_boot()` recovers crashed tasks, starts heartbeat |
| 32.2 | Task spawn + SSE streaming | ✅ | | `POST /api/agent/tasks` → spawn; `GET .../stream` → SSE |
| 32.3 | Task cancel | ✅ | | `POST /api/agent/tasks/{id}/cancel` |
| 32.4 | Provider auto-sync on model switch | ✅ | | `sync_agent_provider()` called after every switch/add/OAuth |
| 32.5 | Chat autocomplete | ✅ | | `POST /api/agent/autocomplete` |
| 32.6 | Tool registry | ✅ | | `GET /api/agent/tools` from `core/tools/builtin` |
| 32.7 | Audit trail | ✅ | | Audit dir at `base_dir/audit` for task execution logs |

## 33. Security

| # | Feature | Status | Severity | Bug Description |
|---|---------|--------|----------|-----------------|
| 33.1 | Credential encryption (AES-256-GCM) | ✅ | | `core/secrets.rs` — encrypt/decrypt tokens with per-install key |
| 33.2 | Sandbox path jail (all subsystems) | ✅ | | Canonicalization + traversal + blocklist enforced in Sandbox::resolve() |
| 33.3 | PTY env-var jailing | ✅ | | HOME, TMPDIR, XDG_*, USERPROFILE scoped to workspace |
| 33.4 | Sensitive file blocking | ✅ | | `.ssh`, `.gnupg`, `.bash_history`, `.env`, `.git` blocked |
| 33.5 | Prompt injection defenses | ✅ | | `core/security` module for credential access protection |

---

## How to Report a Bug

1. Change the **Status** from `🔲` to `❌` (broken) or `⚠️` (partial)
2. Set the **Severity**: `🔴` Critical, `🟠` Major, `🟡` Minor, `🔵` Cosmetic
3. Write a short **Bug Description** explaining what happens

**Example:**
```
| 3.2 | Click dock icon → opens app | ❌ | 🔴 | Clicking Finder icon does nothing, no window opens |
```

---

## Summary

| Category | Total Features | ✅ Working | ⚠️ Partial | ❌ Broken | 🔲 Not Tested |
|----------|---------------|-----------|-----------|----------|--------------|
| Desktop Shell | 7 | 7 | 0 | 0 | 0 |
| Window Manager | 8 | 8 | 0 | 0 | 0 |
| Dock | 5 | 5 | 0 | 0 | 0 |
| Top Bar | 5 | 5 | 0 | 0 | 0 |
| Launchpad | 4 | 4 | 0 | 0 | 0 |
| Spotlight | 3 | 3 | 0 | 0 | 0 |
| AI Launcher | 7+9 | 6+0 | 1 | 0 | 9 |
| Chat | 5 | 5 | 0 | 0 | 0 |
| LLM Providers | 4 | 4 | 0 | 0 | 0 |
| Terminal | 5 | 4 | 1 | 0 | 0 |
| Code Editor | 4 | 3 | 1 | 0 | 0 |
| File Explorer | 5 | 5 | 0 | 0 | 0 |
| Browser | 4 | 3 | 1 | 0 | 0 |
| Shield Browser | 6 | 6 | 0 | 0 | 0 |
| Command Center | 3 | 3 | 0 | 0 | 0 |
| Vibe Studio | 5 | 4 | 1 | 0 | 0 |
| FreeCut | 6 | 5 | 1 | 0 | 0 |
| Plugins | 4+6 | 4+6 | 0 | 0 | 0 |
| Channels | 3 | 3 | 0 | 0 | 0 |
| MCP Tools | 3 | 2 | 1 | 0 | 0 |
| Skills | 3 | 2 | 1 | 0 | 0 |
| Knowledge | 3 | 3 | 0 | 0 | 0 |
| Architecture | 2 | 1 | 1 | 0 | 0 |
| Screenshot | 3 | 3 | 0 | 0 | 0 |
| Service Hub | 4 | 3 | 1 | 0 | 0 |
| Settings | 3 | 1 | 2 | 0 | 0 |
| Logs | 3 | 2 | 1 | 0 | 0 |
| Figma/JSON | 3 | 3 | 0 | 0 | 0 |
| Backend/Core | 10 | 10 | 0 | 0 | 0 |
| Tauri IPC | 4 | 4 | 0 | 0 | 0 |
| Gateway | 5 | 5 | 0 | 0 | 0 |
| Agent Runtime | 7 | 7 | 0 | 0 | 0 |
| Security | 5 | 5 | 0 | 0 | 0 |
| **TOTAL** | **~170** | **~148** | **~13** | **0** | **~9** |
