# Movie Dub → NDE-OS Core Integration

- **ID:** NDE-11
**Status:** 🟢 done by AI  
**Feature:** Integrate the standalone `movie-dub` (FangDub) pipeline into `core/src/freecut/` as a native NDE-OS module  
**Purpose:** Unify the full end-to-end movie dubbing pipeline (STT → Translate → TTS → Sync → Mix → Remux) into the NDE-OS sandbox, reusing the existing Service Hub infrastructure for all external tool dependencies (FFmpeg, Whisper, edge-tts, demucs) instead of directly shelling out to host OS binaries.

## Context

The `movie-dub/` standalone project (FangDub) is a Rust-based dubbing pipeline for movies:
- **7 crates**: `dub-core` (types), `dub-stt` (Whisper), `dub-translate` (Lingva + Claude LLM), `dub-tts` (edge-tts), `dub-sync` (WSOLA time-stretch), `dub-mix` (audio mix + video remux), `dub-cli` (CLI binary)
- Currently shells out directly to `ffmpeg`, `whisper`, `edge-tts`, `demucs` on the host OS with hardcoded paths

FreeCut already has:
- `core/src/freecut/dubbing.rs` — SRT-based dubbing (import SRT → TTS → assets)
- `core/src/voice/` — Global voice runtime (Edge TTS + RVC + Whisper detection via Service Hub)
- `core/src/media/ffmpeg.rs` — Sandboxed FFmpeg bootstrap (auto-download into `base_dir/.ffmpeg/`)
- `core/src/services/registry.rs` — Service Hub with install/detect for all deps

The movie-dub pipeline adds **5 new capabilities** that FreeCut doesn't have:
1. **Translation engine** — Lingva (free) + Claude/Ollama LLM with length-aware dubbing prompts
2. **WSOLA time-stretch** — Pure Rust pitch-preserved audio stretching
3. **Khmer text analysis** — Syllable counting, duration estimation for Khmer script
4. **Audio mixing** — Background separation (demucs) + multi-segment mixing
5. **Video remux** — Replace/dual-audio MP4 output, subtitle burn

## Architecture Decision

Integrate as **new submodules under `core/src/freecut/`** (not separate crates):

```
core/src/freecut/
├── mod.rs              ← add new modules
├── dubbing.rs          ← existing: extend with movie dub pipeline
├── movie_dub/          ← NEW: movie dubbing pipeline
│   ├── mod.rs          ← public API: MovieDubPipeline
│   ├── config.rs       ← DubConfig (merged from dub-core config)
│   ├── lang.rs         ← Lang enum (En, Zh, Km) + syllable rates
│   ├── segment.rs      ← Segment, TimedText, DubbedSegment types
│   ├── pipeline.rs     ← Phase enum, ProgressFn, orchestrator
│   ├── stt.rs          ← STT engine (delegates to VoiceRuntime whisper)
│   ├── translate/      ← Translation subsystem
│   │   ├── mod.rs
│   │   ├── engine.rs   ← TranslateEngine trait
│   │   ├── lingva.rs   ← Free Lingva translation
│   │   ├── llm.rs      ← Claude/Ollama LLM translation
│   │   ├── khmer.rs    ← Khmer syllable analysis
│   │   └── translator.rs ← Smart routing (free → LLM fallback)
│   ├── sync.rs         ← WSOLA time-stretch (pure Rust)
│   └── mix.rs          ← Audio mixing, remux, subtitle generation
```

**Key integration points:**
- All ffmpeg calls → use `crate::media::ffmpeg::ensure_ffmpeg()` for sandboxed binary paths
- All whisper calls → use `crate::voice::runtime::VoiceRuntime::resolve_tool("whisper")`
- All edge-tts calls → use `crate::voice::tts::synthesize()` via VoiceRuntime
- All demucs calls → use VoiceRuntime resolve + sandboxed PATH
- Config → stored per-project in FreeCut project JSON, not standalone `config.toml`
- No `hound` dependency — use existing WAV reading or add `hound` to `core/Cargo.toml`

## Inputs & Outputs

**Inputs:**
- Video file path (mp4, mkv, etc.) within NDE-OS sandbox
- Source language (en, zh, auto)
- Target language (km — Khmer)
- Optional: LLM API key, voice selection, dual-audio flag, subtitle options

**Outputs:**
- Dubbed MP4 (video copy + AAC dubbed audio)
- Optional: dual-audio MP4, SRT subtitle file, burned-subtitles MP4

## Edge Cases & Security

- All file paths must be within the NDE-OS sandbox (canonicalize + validate)
- FFmpeg binary resolved from `base_dir/.ffmpeg/` first, never raw `"ffmpeg"` on host
- Whisper/edge-tts resolved via VoiceRuntime (sandboxed venv)
- LLM API keys stored via NDE-OS secrets management, never in plaintext config files
- Temp files created inside sandbox workspace, cleaned up after pipeline
- Large model files (whisper medium ~1.5GB) managed via Service Hub, not CLI pip install
- Cross-platform path handling via `PathBuf::join()`, never hardcoded separators

## Task Checklist

### Phase 1: Core Types & Config
- [x] Create `core/src/freecut/movie_dub/mod.rs` — module exports
- [x] Create `core/src/freecut/movie_dub/lang.rs` — `Lang` enum (En, Zh, Km) with syllable rates
- [x] Create `core/src/freecut/movie_dub/segment.rs` — `Segment`, `TimedText`, `DubbedSegment`
- [x] Create `core/src/freecut/movie_dub/config.rs` — `MovieDubConfig` (translation, sync, tts, stt, output settings)
- [x] Create `core/src/freecut/movie_dub/pipeline.rs` — `Phase` enum, `ProgressFn`, `MovieDubPipeline` struct

### Phase 2: Translation Engine
- [x] Create `core/src/freecut/movie_dub/translate/mod.rs` — module re-exports
- [x] Create `core/src/freecut/movie_dub/translate/engine.rs` — `TranslateEngine` trait, `TranslateRequest`, `TranslateResult`
- [x] Create `core/src/freecut/movie_dub/translate/khmer.rs` — Khmer syllable estimation (port from dub-translate, all tests)
- [x] Create `core/src/freecut/movie_dub/translate/lingva.rs` — Lingva free translation (port with fallback URLs)
- [x] Create `core/src/freecut/movie_dub/translate/llm.rs` — Claude/Ollama LLM translation with duration-aware prompting
- [x] Create `core/src/freecut/movie_dub/translate/translator.rs` — Smart router: free → LLM when stretch is bad

### Phase 3: Audio Processing (Sandbox-Aware)
- [x] Create `core/src/freecut/movie_dub/stt.rs` — STT engine using VoiceRuntime for whisper + sandboxed ffmpeg for audio extraction
- [x] Create `core/src/freecut/movie_dub/sync.rs` — WSOLA time-stretch (pure Rust, port from dub-sync)
- [x] Create `core/src/freecut/movie_dub/mix.rs` — Audio mixing, demucs separation (via VoiceRuntime), remux (via sandboxed ffmpeg), SRT generation
- [x] Add `hound = "3.5"` to `core/Cargo.toml` for WAV reading/writing

### Phase 4: Pipeline Orchestrator
- [x] Implement `MovieDubPipeline::dub_video()` — full end-to-end: extract → transcribe → translate → separate → TTS → sync → mix → remux
- [x] Implement `MovieDubPipeline::dub_audio()` — segments JSON → dubbed WAV (no video)
- [x] Implement `MovieDubPipeline::transcribe()` — video/audio → segments JSON
- [x] Implement `MovieDubPipeline::translate()` — segments → translated segments
- [x] Wire all subprocess calls through sandboxed binaries (ffmpeg, whisper, edge-tts, demucs)

### Phase 5: Integration
- [x] Register `movie_dub` module in `core/src/freecut/mod.rs`
- [x] Add `demucs` to Service Hub registry as an optional service
- [x] Add `"MovieDub"` to `used_by` lists for voice-runtime, ffmpeg, whisper in service registry
- [x] Update service registry to detect demucs availability
- [x] Ensure all temp files use sandbox-scoped directories

### Phase 6: Tests
- [x] Port Khmer syllable tests from dub-translate
- [x] Port WSOLA time-stretch tests from dub-sync
- [x] Unit test for `MovieDubConfig` serialization
- [x] Unit test for sandboxed ffmpeg resolution in STT/mix
- [x] Run `cargo test -p ai-launcher-core -- movie_dub` — all pass

## Definition of Done

- [x] All movie-dub functionality available as `core::freecut::movie_dub::MovieDubPipeline`
- [x] Zero host-OS dependencies — everything resolved through NDE-OS sandbox (Service Hub)
- [x] FFmpeg calls use `crate::media::ffmpeg::ensure_ffmpeg()` — never raw `"ffmpeg"`
- [x] Whisper/edge-tts/demucs resolved via `VoiceRuntime` — never raw subprocess names
- [x] All tests pass: `cargo test -p ai-launcher-core -- movie_dub`
- [x] No panics, all error handling via `anyhow::Result`
- [x] Cross-platform paths: `PathBuf::join()` only, no hardcoded separators
- [x] No TODOs, no mocks, production-ready code
- [x] `hound` dependency added to `core/Cargo.toml`
- [x] Existing FreeCut dubbing functionality unaffected
