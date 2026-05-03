# Status
🟢 done by AI
Persistent Memory — Remove OpenViking, Embed openfang-memory SQLite Substrate

# Purpose
Replace the external OpenViking Python sidecar with a native Rust memory substrate ported from [`openfang-memory`](https://github.com/RightNow-AI/openfang/tree/main/crates/openfang-memory). This gives NDE-OS:

- **SQLite-backed storage** with vector embeddings for agent memory
- **Cross-channel canonical sessions** — what a user says on Telegram is remembered on Discord
- **Automatic LLM-based compaction** — older messages are summarized, not truncated
- **JSONL session mirroring** — human-readable conversation logs on disk
- **Zero external dependencies** — no Python, no sidecar process, no `uv pip install`

OpenViking was a Python server that required installation, process management, and health checks. The new system is a pure-Rust SQLite-backed crate compiled directly into `ai-launcher-core`, eliminating an entire class of startup failures and cross-platform issues.

# Architecture (from openfang-memory)

The `MemorySubstrate` composes five specialized stores behind a single API:

| Store | Purpose |
|-------|---------|
| `StructuredStore` | Key-value pairs, agent state persistence |
| `SemanticStore` | Text + embedding search (SQLite FTS + cosine similarity) |
| `KnowledgeStore` | Entity-relation graph (triples) |
| `SessionStore` | Conversation sessions, canonical cross-channel sessions, JSONL mirroring |
| `ConsolidationEngine` | Time-decay of old memories, confidence reduction |
| `UsageStore` | Token/usage tracking |

Dependencies to add to `core/Cargo.toml`:
- `rusqlite` (with `bundled` feature) — already in workspace
- `rmp-serde` — MessagePack for session blob serialization
- `chrono`, `uuid` — already in workspace

# Inputs & Outputs
- **Inputs:**
  - Agent messages from chat, Telegram gateway, or any channel
  - Memory recall queries (text search, semantic search, graph queries)
  - KV get/set operations for agent state
- **Outputs:**
  - Recalled memory fragments with confidence scores
  - Session history with compacted summaries
  - JSONL mirror files under `{data_dir}/sessions/`
  - Consolidation reports (decayed/merged counts)

# Edge Cases & Security
- **Migration path:** Existing OpenViking data (if any) will NOT be migrated — this is a clean break. Document this in release notes.
- **SQLite concurrency:** Use WAL mode + busy_timeout (already in openfang-memory pattern). Wrap Connection in `Arc<Mutex<>>` with minimal lock scope per AGENTS.md rules.
- **Embedding dimension mismatch:** Default to 512-dim (Qwen2.5-0.5B), but make dimension configurable. Phase 1 uses LIKE matching; Phase 2 adds true vector search.
- **Session blob corruption:** MessagePack deserialization must handle version mismatches gracefully — return empty session, log warning, never panic.
- **Cross-platform paths:** All session mirror paths via `PathBuf::join()`, never hardcode separators.
- **Sandbox integrity:** Memory DB must live inside the NDE-OS workspace (sandbox-jailed), never in system temp.

# Task Checklist

## Phase 1: Remove OpenViking
- [x] Delete `core/src/openviking/` module entirely (client.rs, config.rs, process.rs, mod.rs)
- [x] Remove `openviking` from `core/src/lib.rs` module declarations
- [x] Remove `VikingProcess` from `server/src/router.rs` AppState
- [x] Remove `/api/viking/*` routes from `server/src/router.rs`
- [x] Remove `openviking` handlers from `server/src/subsystems/` (viking_status, viking_install, viking_start, viking_stop)
- [x] Remove `openviking` from `server/src/openapi/` (viking.rs, schemas.rs references)
- [x] Remove `openviking` service from `core/src/services/config.rs` (config fields + tests)
- [x] Remove `openviking` service from `core/src/services/registry.rs` (detection + install logic + tests)
- [x] Remove OpenViking initialization from `server/src/main.rs`
- [x] Remove OpenViking from `desktop/src-tauri/src/lib.rs` (import + state management)
- [x] Remove `desktop/src-tauri/src/commands/viking.rs` Tauri commands
- [x] Remove OpenViking references from `desktop/src/main.ts` (app definitions)
- [x] Remove OpenViking featured card + actions from `desktop/src/components/apps/ServiceHub/ServiceHub.svelte`
- [x] Remove `/openviking` chat command from `desktop/src/components/apps/Chat/Chat.svelte`
- [x] Remove OpenViking types from `desktop/src/lib/api/types.ts`
- [x] Remove OpenViking API functions from `desktop/src/lib/api/backend.ts`
- [x] Remove OpenViking MCP tools from `core/src/mcp/builtin.rs` (nde_viking_find, nde_viking_read, nde_viking_ls)
- [x] Verify `cargo build` succeeds with no OpenViking references
- [x] Verify `cargo test -p ai-launcher-core` passes (320/321 pass, 1 pre-existing failure in security::policy unrelated to this change)

## Phase 2: Port openfang-memory Substrate
- [x] Create `core/src/memory/` module directory
- [x] Port `migration.rs` — SQLite schema creation (memories, sessions, canonical_sessions, entities, relations, task_queue tables)
- [x] Port `structured.rs` — KV store (agent state, get/set/delete/list_kv)
- [x] Port `semantic.rs` — Memory storage with LIKE-based recall (Phase 1) and embedding column for future vector search
- [x] Port `knowledge.rs` — Entity-relation graph store (add_entity, add_relation, query_graph)
- [x] Port `session.rs` — Session CRUD, canonical sessions, cross-channel append, compaction, JSONL mirroring
- [x] Port `consolidation.rs` — Time-decay engine (reduce confidence of stale memories)
- [x] Port `usage.rs` — Token usage tracking
- [x] Port `substrate.rs` — Unified `MemorySubstrate` composing all stores
- [x] Adapt types: Replace `openfang_types::*` imports with local equivalents or inline structs (MemoryId, AgentId, SessionId, MemorySource, MemoryFilter, MemoryFragment, Entity, Relation, etc.)
- [x] Add `rmp-serde` dependency to `core/Cargo.toml`
- [x] Ensure `rusqlite` has `bundled` feature enabled
- [x] Export `memory` module from `core/src/lib.rs`

## Phase 3: Integrate into Server + Desktop
- [x] Replace `VikingProcess` in `AppState` with `Arc<MemorySubstrate>` (or `Arc<Mutex<MemorySubstrate>>`)
- [x] Initialize `MemorySubstrate::open()` in `server/src/main.rs` at startup (DB path: `{data_dir}/memory.db`)
- [x] Create new REST routes under `/api/memory/*`:
  - `GET /api/memory/status` — DB stats (row counts, size)
  - `POST /api/memory/remember` — Store a memory fragment
  - `POST /api/memory/recall` — Search/recall memories
  - `GET /api/memory/sessions` — List sessions
  - `POST /api/memory/sessions` — Create session
  - `GET /api/memory/sessions/{id}` — Get session with messages
  - `DELETE /api/memory/sessions/{id}` — Delete session
  - `POST /api/memory/consolidate` — Trigger consolidation
- [x] Wire agent chat to use `MemorySubstrate` for session persistence instead of in-memory history
- [x] Wire Telegram gateway to append messages to canonical session for cross-channel memory
- [x] Create Tauri IPC commands for memory operations (memory_status, memory_recall, memory_remember)
- [x] Update ServiceHub UI: Replace OpenViking featured card with "Persistent Memory" info card (no install needed — it's built-in)
- [x] Update Chat.svelte: Replace `/openviking` command with `/memory` command
- [x] Update OpenAPI spec with new `/api/memory/*` routes

## Phase 4: Tests & Verification
- [x] Write unit tests for `MemorySubstrate::open_in_memory()` — KV round-trip
- [x] Write unit tests for session create/save/load/delete
- [x] Write unit tests for canonical session append + compaction
- [x] Write unit tests for semantic remember/recall
- [x] Write unit tests for consolidation decay
- [x] Write unit tests for JSONL mirror output
- [x] Run `cargo test -p ai-launcher-core -- memory` — all pass
- [x] Run `cargo test -p ai-launcher-server` — all pass (no OpenViking references)
- [x] Run `cargo build` — clean compilation
- [x] Implement ConsolidationEngine loop in server/src/main.rs (calls compact on threshold)
- [x] Manual smoke test: Start NDE-OS, verify ServiceHub shows built-in memory, send chat messages, verify sessions persist across restart

# Files Likely Affected

### Deleted
- `core/src/openviking/mod.rs`
- `core/src/openviking/client.rs`
- `core/src/openviking/config.rs`
- `core/src/openviking/process.rs`
- `desktop/src-tauri/src/commands/viking.rs`
- `server/src/openapi/viking.rs`

### New
- `core/src/memory/mod.rs`
- `core/src/memory/migration.rs`
- `core/src/memory/structured.rs`
- `core/src/memory/semantic.rs`
- `core/src/memory/knowledge.rs`
- `core/src/memory/session.rs`
- `core/src/memory/consolidation.rs`
- `core/src/memory/usage.rs`
- `core/src/memory/substrate.rs`
- `core/src/memory/types.rs` (local type definitions replacing openfang_types)
- `desktop/src-tauri/src/commands/memory.rs`

### Modified
- `core/src/lib.rs` — remove `openviking`, add `memory`
- `core/Cargo.toml` — add `rmp-serde`
- `core/src/services/config.rs` — remove openviking fields
- `core/src/services/registry.rs` — remove openviking service
- `server/src/main.rs` — init MemorySubstrate, remove VikingProcess
- `server/src/router.rs` — replace viking routes with memory routes, update AppState
- `server/src/openapi/mod.rs` — replace viking tag
- `server/src/openapi/schemas.rs` — replace viking schemas
- `desktop/src-tauri/src/lib.rs` — replace viking commands with memory commands
- `desktop/src/main.ts` — remove OpenViking app definitions
- `desktop/src/lib/api/backend.ts` — replace viking API calls with memory API calls
- `desktop/src/lib/api/types.ts` — replace viking types with memory types
- `desktop/src/components/apps/ServiceHub/ServiceHub.svelte` — replace viking card
- `desktop/src/components/apps/Chat/Chat.svelte` — replace `/openviking` command
- `status.md` — update row

# Definition of Done
- **Local DoD:**
  - OpenViking is completely removed — zero references in codebase.
  - `MemorySubstrate` initializes at startup and persists data to `{data_dir}/memory.db`.
  - Agent chat sessions persist across NDE-OS restarts.
  - Cross-channel canonical sessions work (Telegram → Desktop recall).
  - `/api/memory/*` REST routes return correct data.
  - ServiceHub shows "Persistent Memory" as a built-in capability (no install step).
- **Global DoD:**
  - No mocks, no TODOs, no panics — `anyhow::Result` everywhere.
  - All `Arc<Mutex<>>` locks scoped to minimum duration.
  - All paths via `PathBuf::join()`, cross-platform safe.
  - Scoped unit tests pass: `cargo test -p ai-launcher-core -- memory`.
  - Full build: `cargo build` succeeds cleanly.
  - Feature aligns with user's goal of a zero-dependency persistent memory system.
