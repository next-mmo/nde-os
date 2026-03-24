# Project Status & Purpose

## 🎯 Our Purpose

**NDE-OS** is a **sandboxed AI app launcher** (akin to Pinokio or Openclaw) and **autonomous agent looping gateway chat** wrapped in a macOS-Ventura style web desktop.

The core mission is to provide a secure, native, cross-platform (Windows & Linux) environment where users can install and run AI apps (Stable Diffusion, ComfyUI, Ollama, Gradio, Whisper, etc.) from a manifest catalog. Every app is isolated in its own **filesystem-jailed workspace** leveraging a blazing-fast, auto-bootstrapped `uv`-managed Python virtual environment.

Looking forward, the system is transforming into a full **Agent Operating System (FangFlow)** featuring a looping runtime (IDLE → THINK → EXECUTE → OBSERVE), multi-provider LLM drivers, SQLite-backed persistent memory, and a sophisticated workflow DAG execution engine—all running securely within our Rust sandbox without the need for Docker.

---

## 🚀 Current Status: Real Project Output

Based on the actual project repository state (`core`, `desktop`, `server`, and documentation):

### ✅ Completed Milestones

- **Phase 0 - Core Architecture & Safety**
  - `core/sandbox` filesystem jail with path validation, traverse defense, and symlink validation.
  - `core/uv_env` creation for ultra-fast, isolated per-app Python virtual environments.
  - `core/app_manager` handling manifest-driven install/launch/stop app lifecycles.
  - Open API 3.0.3 compliant REST API backend (Rust).
  - Extensible Plugin System supporting hooks, monitors, providers, middleware, and UI dashboard panels.
- **Phase 1 & Phase 2 - Agent Runtime & Capabilities**
  - Agent loop state machine and LLM multi-drivers integrated (Anthropic, OpenAI, Codex, Groq, Together, Ollama).
  - Embedded Rust binding for `llama.cpp` (`llama-cpp-2` crate) for in-process GGUF inference (eliminating external llama-server process dependency).
  - 20+ built-in core tools (filesystem, memory, system integrations, skills) plus MCP client configuration.
  - Persistent memory using SQLite and Vector embeddings.

### 🔄 In-Progress / Current (Phase 0.5)

- **Desktop UI Migration (Tauri 2 & Svelte 5)**
  - Svelte 5 IDE integration and macOS-style environment (dock animations, blur limits).
  - Top bar metrics for system resources (RAM %, disk %, GPU indicators built and running).
  - Robust E2E testing framework via Playwright (`desktop/e2e`).
  - Kubb implementation for API client generation (`api-pos` schema mappings, API hooks).

### 📋 after a few months Upcoming Planned Not now

- **License Gateway Server ("Local Muscle + Server Brain")**
  - Axum + Ed25519 Rust server implementation.
  - Feature gating, telemetry, usage analytics, and Ed25519 challenge-response token protocols.
  - Separation of cloud auth from local GPU inference capabilities.
- **Advanced Autonomy (Phase 3 & Phase 4)**
  - Finalized orchestration for supervisor pattern sub-agents.
  - Agent channel routing (Telegram/Discord integrations) and cron-driven autonomy.

---

## 🎯 Definition of Done (End-User Perspective)

From the perspective of an everyday user (e.g., non-developer/designer), the project reaches its true "Beta/Release" state when:

1. **One-Click Delivery:** The user downloads a single, unified cross-platform binary executable (.exe/.dmg/.AppImage) without needing Docker, WSL, system-wide Python, or `pip` gymnastics to get started.
2. **Seamless Onboarding & Licensing:** The user successfully authenticates via the remote Gateway/License server, receiving local tier entitlements and access to a rich App Catalog instantly.
3. **Painless AI App Installation:** Clicking "Install" on apps like Stable Diffusion or ComfyUI seamlessly and securely provisions a jailed Python/uv environment and resolves complex GPU dependencies silently in the background.
4. **App Encapsulation (Sandboxing):** The user can run these complex AI apps confidently, knowing path boundaries, API dependencies, and variables stay trapped strictly in per-app sub-workspaces without compromising their host machine.
5. **Fluid Mac-Style Navigation:** The user interacts inside a familiar, buttery smooth macOS Ventura-style desktop (Dock, system tray metrics, web windows), blurring the lines between a browser window and a native high-end OS interface.
6. **Agentic Autonomy:** The user can spin up an intelligent "Assistant/Agent" from the dashboard that natively uses all local 20+ tools (filesystem, memory) and handles web scraping, task execution, and system manipulation without hand-holding.
7. **Offline Grace:** The user can rip out their ethernet cord and still seamlessly run local `llama.cpp` inference and workflows via their GPU cache without server ping failures.
8. **E2E Verified Reliability:** Every single core user journey listed above is fully covered and automatically verified by robust End-to-End (E2E Playwright) tests, ensuring zero regressions on release.

---

## 📦 Production Readiness Checklist

To definitively know when the API (`server/`) and Desktop (`desktop/`) are complete and "Product Ready" for public release, they must meet the following criteria:

### 1. The API (Backend / Rust Core)

- **Stable Contracts:** The OpenAPI 3.0.3 specification (`openapi.json`) is locked, and frontend TypeScript clients generate via Kubb perfectly without schema mismatches.
- **100% Test Coverage:** `cargo test` successfully clears all backend tests (especially sandbox security checks, symlink escapes, and `uv` environments).
- **Zero Panic Tolerance:** The Rust server handles malformed requests and app crashes gracefully with JSON error responses instead of panicking or crashing.

### 2. The Desktop (Frontend / Tauri + Svelte 5)

- **The "Green" Run:** 100% pass rate in the `npx playwright test` automated E2E pipeline for all core journeys (install, launch, stop apps, view metrics).
- **Release Build Success:** Executing `pnpm tauri build` successfully generates a standalone executable (`.exe`/`.dmg`/`.AppImage`) that boots instantly without requiring `npm run dev` or active terminal watchers.
- **Fluid Performance:** Svelte 5 + `shadcn-svelte` components deliver a smooth, glitch-free macOS Ventura-style experience (no blinking blurs or layout shifts).
- **Accurate Top-Bar Realities:** RAM, Disk, and GPU indicators correspond accurately to the host system's Task Manager without lagging the UI thread.

---

## 🧪 Demo User Test Scenarios

Concrete, real-world test scenarios that a default/demo user performs to verify the product works end-to-end. Scenarios requiring external API keys or tokens are marked 🔑 and can be **skipped** during a keyless demo run.

### Desktop & Navigation (No Key Required)

| # | Scenario | Expected Result | Status |
|---|----------|-----------------|--------|
| 1 | Launch the app (`.exe` or `pnpm dev`) | macOS-style desktop loads with dock, top bar, and wallpaper | ⬜ |
| 2 | Check top-bar system metrics | RAM %, Disk %, GPU indicator display real-time values | ⬜ |
| 3 | Click dock icons | Windows open/close/minimize with smooth animations | ⬜ |
| 4 | Open Settings panel | Settings UI renders, theme/wallpaper options work | ⬜ |
| 5 | Resize and drag windows | Windows respond fluidly with no layout shifts | ⬜ |

### App Catalog & Installation (No Key Required)

| # | Scenario | Expected Result | Status |
|---|----------|-----------------|--------|
| 6 | Open App Catalog from dock | Full catalog of available AI apps renders with icons and descriptions | ⬜ |
| 7 | Install a lightweight app (e.g., `gradio-demo`) | `uv` silently bootstraps a `.venv`, installs pip deps, shows progress | ⬜ |
| 8 | Launch the installed app | App subprocess starts, port opens, app iframe/window loads in desktop | ⬜ |
| 9 | Stop the running app | Process terminates cleanly, port released, status updated | ⬜ |
| 10 | Uninstall the app | Workspace directory cleaned up, app removed from installed list | ⬜ |
| 11 | Verify sandbox isolation | `GET /api/sandbox/{id}/verify` → all attack vectors blocked ✅ | ⬜ |

### Agent Chat via UI (Default: GGUF only)

> **Default test model:** `core/models/Qwen3.5-9B-Q4_K_M.gguf` (bundled, no download needed)

| # | Scenario | Key? | Expected Result | Status |
|---|----------|------|-----------------|--------|
| 12 | Open Agent Chat window from dock | ❌ | Chat UI renders with input box and message history | ⬜ |
| 13 | Chat using bundled GGUF model (`Qwen3.5-9B-Q4_K_M.gguf`) | ❌ | Agent responds via embedded `llama.cpp`, fully offline, no internet | ⬜ |
| 14 | Agent uses built-in tools (e.g., `file_write`, `list_dir`) | ❌ | Agent autonomously writes files or lists directories inside sandbox | ⬜ |
| 15 | Agent uses `memory_store` / `memory_recall` | ❌ | Agent persists and retrieves facts from SQLite memory | ⬜ |
| 16 | Chat using Ollama provider | 🔑 Skip | Requires local Ollama install — streams responses | ⬜ |
| 17 | Chat using OpenAI provider | 🔑 Skip | Requires `OPENAI_API_KEY` — streams GPT response | ⬜ |
| 18 | Chat using Anthropic provider | 🔑 Skip | Requires `ANTHROPIC_API_KEY` — streams Claude response | ⬜ |
| 19 | Chat using Groq provider | 🔑 Skip | Requires `GROQ_API_KEY` — fast Llama inference | ⬜ |
| 20 | Chat using Codex provider | 🔑 Skip | Requires OAuth flow — code-oriented responses | ⬜ |

### System & API Health (No Key Required)

| # | Scenario | Expected Result | Status |
|---|----------|-----------------|--------|
| 21 | `GET /api/health` | Returns `200 OK` with uptime | ⬜ |
| 22 | `GET /api/system` | Returns OS, Python version, GPU info, uv version | ⬜ |
| 23 | `GET /api/catalog` | Returns full list of available app manifests | ⬜ |
| 24 | `GET /api/apps` | Returns currently installed apps with statuses | ⬜ |
| 25 | Open Swagger UI (`/swagger-ui/`) | Interactive API docs render and all endpoints are testable | ⬜ |

### Advanced / Automation (No Key Required)

| # | Scenario | Expected Result | Status |
|---|----------|-----------------|--------|
| 26 | Agent runs a multi-step tool chain | Agent chains `web_fetch` → `file_write` → `memory_store` autonomously | ⬜ |
| 27 | Disk usage check per app | `GET /api/sandbox/{id}/disk` returns accurate byte counts | ⬜ |
| 28 | Install + Launch + Stop via API only (headless) | Full lifecycle via `curl` commands without touching the UI | ⬜ |

> **Rule:** If a scenario is marked 🔑 Skip, the E2E test should gracefully skip it (not fail) when no key/token is present in the environment.
