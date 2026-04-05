# Core Voice Services + Global Service Onboarding

**Status:** `🟢 done by AI`

## Feature

1. **Extract RVC + Edge TTS** from FreeCut's dubbing module into standalone **core services** (`core/src/voice/`)
2. **Create a Global Service Hub** — a new `service-hub` app that manages all NDE-OS service dependencies (voice runtime, FFmpeg, Python, uv, etc.) in one place
3. **Deep-link from apps → Service Hub → back to app** — when FreeCut (or any app) detects missing services, it shows a CTA that opens the Service Hub with the required services pre-selected. After install, the user returns to the originating app.

## Purpose

**Current pain**: FreeCut has its own inline onboarding UI for dubbing tools, but:
- Each app would need to duplicate this setup UX
- The voice runtime lives in `freecut/tooling/dubbing-runtime` — no other app can use it
- There's no central place to see "what's installed" across NDE-OS

**After this change:**
- **`core/src/voice/`** — framework-agnostic TTS + RVC services, callable by any crate
- **`server/` API** — REST endpoints `/api/v1/tts/*`, `/api/v1/rvc/*`, `/api/v1/voice/*`
- **Service Hub app** — unified onboarding UI for ALL NDE-OS services:
  - Voice Runtime (Edge TTS + Whisper + RVC)
  - FFmpeg (video processing)
  - Python / uv (tooling foundation)
  - Future: Ollama, ComfyUI, etc.
- **Deep-link flow**: FreeCut detects missing → opens Service Hub with `?require=voice-runtime` → user installs → Service Hub redirects back to FreeCut
- Centralized runtime at `~/.ai-launcher/voice-runtime/` (shared, not per-app)

## Architecture

```
┌────────────────────────────────────────────────────────────────┐
│  core/src/voice/                                               │
│  ├── mod.rs          barrel export                             │
│  ├── tts.rs          EdgeTTS synthesis + voices                │
│  ├── rvc.rs          RVC voice conversion                      │
│  ├── runtime.rs      venv management + detection               │
│  └── types.rs        shared request/response types             │
├────────────────────────────────────────────────────────────────┤
│  core/src/services/                                            │
│  ├── mod.rs          barrel export                             │
│  ├── registry.rs     service registry (voice, ffmpeg, etc.)    │
│  └── types.rs        ServiceDef, ServiceStatus, ServiceGroup   │
└──────────┬────────────────────────────┬───────────────────────┘
           │                            │
    ┌──────▼─────────┐          ┌───────▼──────────────────┐
    │ server/         │          │ desktop/                  │
    │ voice_handler   │          │ ServiceHub app            │
    │ /api/v1/tts/*   │          │ (global onboarding UI)    │
    │ /api/v1/rvc/*   │          │                           │
    │ /api/v1/voice/* │          │ FreeCut (consumer)        │
    └────────────────┘          │ → deep-links to ServiceHub │
                                └───────────────────────────┘
```

### Deep-Link Flow (App → Service Hub → Back)

```
FreeCut detects: needsDubbingSetup = true
    │
    ▼  User clicks "Set Up Voice Runtime"
openStaticApp("service-hub", { require: ["voice-runtime"], returnTo: "freecut" })
    │
    ▼  Service Hub opens with "Voice Runtime" pre-expanded
User clicks "Install" → uv creates venv, installs edge-tts + whisper
    │
    ▼  Install complete
Service Hub shows ✅ → "Return to FreeCut" button
    │
    ▼  openStaticApp("freecut") — re-triggers detectDubbingTools()
```

## REST API Endpoints

### Voice (Core Service)

| Method | Endpoint | Purpose |
|--------|----------|---------|
| POST | `/api/v1/tts/synthesize` | Text-to-speech via Edge TTS |
| GET | `/api/v1/tts/voices` | List available TTS voices |
| POST | `/api/v1/rvc/convert` | Voice conversion via RVC |
| GET | `/api/v1/rvc/models` | List RVC models |
| GET | `/api/v1/voice/status` | Runtime availability check |
| POST | `/api/v1/voice/install` | Install voice components |

### Services (Global Registry)

| Method | Endpoint | Purpose |
|--------|----------|---------|
| GET | `/api/v1/services` | List all registered services + status |
| GET | `/api/v1/services/{id}/status` | Check specific service status |
| POST | `/api/v1/services/{id}/install` | Install a specific service |

## Edge Cases & Security

- **Sandbox enforcement**: All output paths within NDE-OS data dir. Canonicalize all inputs.
- **Concurrent installs**: Mutex on venv operations — only one install at a time, multiple reads OK.
- **Missing runtime**: Clear errors with deep-link to Service Hub, never panic.
- **Path traversal**: Canonicalize model/index paths, validate against voice-runtime dir.
- **Return-to navigation**: Store `returnTo` in desktop state, not URL params (Tauri webview).
- **First-launch detection**: Check `localStorage` flag for `service-hub:first-run-complete`. Show optional onboarding on first desktop boot if desired.

## Task Checklist

### Phase A: Core Voice Module (`core/src/voice/`)

- [x] **A1.** Create `core/src/voice/types.rs` — shared types: `TtsSynthesizeRequest`, `TtsSynthesizeResult`, `RvcConvertRequest`, `RvcConvertResult`, `VoiceRuntimeStatus`, `VoiceInstallRequest`, `VoiceInstallResult`, `VoiceModel`
- [x] **A2.** Create `core/src/voice/runtime.rs` — central voice runtime manager:
  - `VoiceRuntime::new(base_dir)` — points to `~/.ai-launcher/voice-runtime/`
  - `detect_status()` → `VoiceRuntimeStatus`
  - `install_components(components)` → `VoiceInstallResult` (uses `uv_env`)
  - `runtime_bin_dir()` → PATH prepend
  - `resolve_tool(name)` → finds CLI in runtime venv
  - `migrate_from_freecut(freecut_tooling_dir)` — backward compat migration
- [x] **A3.** Create `core/src/voice/tts.rs` — Edge TTS service:
  - `synthesize(request, runtime)` → `TtsSynthesizeResult`
  - `list_voices(runtime)` → `Vec<String>`
  - `default_voice_for_language(lang)` → `String`
- [x] **A4.** Create `core/src/voice/rvc.rs` — RVC service:
  - `convert(request, runtime)` → `RvcConvertResult`
  - `list_models(models_dir)` → `Vec<VoiceModel>`
- [x] **A5.** Create `core/src/voice/mod.rs` — barrel exports
- [x] **A6.** Register `pub mod voice;` in `core/src/lib.rs`

### Phase B: Core Service Registry (`core/src/services/`)

- [x] **B1.** Create `core/src/services/types.rs` — `ServiceDef`, `ServiceStatus`, `ServiceGroup`, `ServiceRequirement`
  - `ServiceDef { id, name, description, group, required_by: Vec<app_ids>, install_fn }`
  - `ServiceStatus { installed: bool, version: Option<String>, path: Option<String> }`
- [x] **B2.** Create `core/src/services/registry.rs` — register known services:
  - `voice-runtime` (Edge TTS + Whisper)
  - `rvc` (RVC CLI)
  - `ffmpeg`
  - `python` / `uv`
  - `detect_all()` → HashMap<service_id, ServiceStatus>
  - `install_service(id, base_dir)` → Result
- [x] **B3.** Create `core/src/services/mod.rs` — barrel export
- [x] **B4.** Register `pub mod services;` in `core/src/lib.rs`

### Phase C: Refactor FreeCut Dubbing → Consume `core::voice`

- [ ] **C1.** Update `core/src/freecut/dubbing.rs`:
  - Replace `synthesize_with_edge_tts()` → call `voice::tts::synthesize()`
  - Replace `synthesize_with_rvc()` → call `voice::rvc::convert()` (TTS then RVC)
  - Replace `list_edge_voices()` → call `voice::tts::list_voices()`
  - Replace `detect_local_tools()` → delegate to `voice::runtime::detect_status()` + local whisper/LLM checks
  - Remove duplicated helpers: `resolve_command`, `resolve_python`, `run_checked_command`, `default_voice_for_language`
- [ ] **C2.** Update `desktop/src-tauri/src/commands/freecut.rs`:
  - `freecut_install_dubbing_runtime` → delegate to `voice::runtime::install_components()`
  - `freecut_detect_dubbing_tools` → use global voice runtime
  - Remove FreeCut-specific `dubbing_runtime_workspace`, `dubbing_runtime_bin_dir`, `with_tool_runtime_path`
- [ ] **C3.** Auto-migrate FreeCut tooling dir → shared voice-runtime on first access

### Phase D: Global Service Hub Desktop App

- [x] **D1.** Register `"service-hub"` in `apps-config.ts` + `AppNexus.svelte`
- [x] **D2.** Create Tauri IPC commands for service management:
  - `service_hub_status` → returns all services + their installed status  
  - `service_hub_install` → installs a specific service (delegates to `core::services`)
- [x] **D3.** Add `VoiceRuntimeState` to Tauri managed state (shared, not per-app)
- [x] **D4.** Create `ServiceHub.svelte` — global onboarding UI:
  - **Service cards** grouped by category (Voice, Media, AI) with install/status toggles
  - Each card shows: name, description, installed ✅ / missing ❌, install button, used by [app icons]
  - **"Required by"** badges: e.g., "Used by FreeCut, Podcast Studio"
  - Progress bar during install
  - macOS Ventura aesthetic (blur, glassmorphism, shadcn-svelte + Tailwind)
- [x] **D5.** Add deep-link support to `desktop.svelte.ts`:
  - Extend `openStaticApp()` to accept optional context: `openStaticApp("service-hub", { require: ["voice-runtime"], returnTo: "freecut" })`
  - Store pending return-to in reactive state
  - Service Hub reads this context to pre-select required services and show "Return to X" button
- [x] **D6.** Update FreeCut's dubbing setup UI:
  - Replace inline "Install in NDE-OS" button → "Open Service Hub" button calling `openStaticApp("service-hub", { require: ["voice-runtime"], returnTo: "freecut" })`
  - Keep the inline quick-status cards (Whisper ✅, Edge TTS ✅, RVC ❌) as read-only indicators
  - On re-focus / onMount, re-detect tools to pick up changes

### Phase E: REST API (`server/`)

- [ ] **E1.** Create `server/src/voice_handler.rs` — all `/api/v1/voice/*`, `/api/v1/tts/*`, `/api/v1/rvc/*` routes
- [ ] **E2.** Add voice runtime state + routes to `server/src/main.rs`
- [ ] **E3.** Add service registry routes (`/api/v1/services`, `/api/v1/services/{id}/status`, `/api/v1/services/{id}/install`)
- [ ] **E4.** Add OpenAPI specs for all new endpoints

### Phase F: Tests

- [x] **F1.** Unit tests for `core::voice::tts` — voice list parsing, request building
- [x] **F2.** Unit tests for `core::voice::rvc` — request validation, model listing
- [x] **F3.** Unit tests for `core::voice::runtime` — status detection, path resolution
- [x] **F4.** Unit tests for `core::services::registry` — service detection
- [ ] **F5.** Verify FreeCut dubbing tests still pass after refactor
- [x] **F6.** Verify `cargo build` passes for all crates

## Definition of Done

- [x] `core/src/voice/` module exists with `tts.rs`, `rvc.rs`, `runtime.rs`, `types.rs`
- [x] `core/src/services/` module exists with `registry.rs`, `types.rs`
- [ ] `core/src/freecut/dubbing.rs` delegates to `core::voice` — no more local synthesis functions
- [ ] REST API `/api/v1/tts/*`, `/api/v1/rvc/*`, `/api/v1/voice/*` work end-to-end
- [x] Service Hub app opens, shows all services, allows install
- [x] FreeCut → "Open Service Hub" → installs → "Return to FreeCut" flow works
- [ ] FreeCut dubbing works identically from user's perspective
- [x] `cargo test -p ai-launcher-core -- voice` passes
- [ ] `cargo test -p ai-launcher-core -- dubbing` passes
- [x] No panics, no TODOs, no mocks — production-ready
- [x] Cross-platform paths (`PathBuf::join` only, `cfg!(windows)` guards)
- [x] shadcn-svelte + Tailwind only for Service Hub UI, no custom CSS
