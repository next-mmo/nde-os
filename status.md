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
