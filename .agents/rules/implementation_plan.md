---
trigger: model_decision
---

# Agent Core v2 — Enterprise-Grade 24/7 Agent Runtime

> **Rules**: No mocks. No fakes. No TODOs. No placeholders. Production-ready only.  
> **Testing**: E2E required. `cargo test` + Playwright CDP. Tests fail = not done.  
> **Standard**: AGENTS.md rules strictly enforced.

## What Exists Today (Honest Audit)

| Module                                                                                                               | LOC  | Verdict                                                                                                                               |
| -------------------------------------------------------------------------------------------------------------------- | ---- | ------------------------------------------------------------------------------------------------------------------------------------- |
| [agent/mod.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/mod.rs)                   | 108  | ⚠️ Basic sync loop, no streaming, no cancel                                                                                           |
| [agent/config.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/config.rs)             | 192  | ✅ Solid TOML config                                                                                                                  |
| [llm/streaming.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/llm/streaming.rs)           | 185  | ✅ ChunkStream types exist — **unused by agent**                                                                                      |
| [llm/mod.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/llm/mod.rs)                       | 326  | ✅ 9 providers, [chat_stream()](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/llm/mod.rs#94-115) trait exists |
| [security/metering.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/security/metering.rs)   | 110  | ✅ ComputeMeter exists — **not wired to agent**                                                                                       |
| [security/audit.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/security/audit.rs)         | ~100 | ✅ SHA-256 audit — **not wired to agent**                                                                                             |
| [security/injection.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/security/injection.rs) | ~100 | ✅ Injection scanner — **not wired to agent**                                                                                         |
| `server/stream_handler.rs`                                                                                           | 134  | ❌ **Fake streaming** — runs sync, splits words                                                                                       |
| `server/agent_handler.rs`                                                                                            | 185  | ⚠️ Creates new AgentRuntime per request (no persistence)                                                                              |

> [!CAUTION]
> The agent loop exists but is disconnected from security (metering, audit, injection scan), streaming, and persistence. Every request creates a fresh runtime. Nothing survives a restart.

## Proposed Changes (8 Modules)

### Layer 1: Foundation Types (no deps on each other)

#### [MODIFY] [models.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/models.rs) — already created

Task state machine. [AgentTask](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/models.rs#67-114) with 7 states, builder pattern, lifecycle transitions.

#### [MODIFY] [protocol.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/protocol.rs) — already created

12 typed SSE events. [to_sse()](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/protocol.rs#138-143) serialization. Terminal detection.

#### [MODIFY] [store.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/store.rs) — already created

SQLite WAL task persistence + checkpoint save/restore for crash recovery.

---

### Layer 2: Core Engine (depends on Layer 1)

#### [NEW] [executor.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/executor.rs)

The real agent loop. Replaces the fake streaming and disconnected sync loop.

**Requirements (strict)**:

- Uses `provider.chat_stream()` — real token-level SSE, not word-split
- Emits `AgentEvent` via `tokio::sync::mpsc` channel
- Integrates [ComputeMeter](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/security/metering.rs#5-15) — checks budget after every iteration
- Integrates `InjectionScanner` — scans user input before LLM call
- Integrates `AuditLog` — records every tool execution with SHA-256
- Respects `CancellationToken` (`tokio_util::CancellationToken`)
- Saves checkpoint every N iterations for crash recovery
- Retry with exponential backoff on transient LLM errors (429, 500, timeout)
- Per-task timeout enforcement via `tokio::time::timeout`

**What makes this 10/10**:

- Every security module wired in (not bolted on later)
- Every tool call audited with hash chain
- Every LLM call metered
- Every user input scanned for injection
- Crash at any point → resume from checkpoint

---

#### [NEW] [heartbeat.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/heartbeat.rs)

Background 24/7 guardian — **the reason the agent never dies**.

**Requirements**:

- Spawns as `tokio::spawn` background task
- Emits `AgentEvent::Heartbeat` every 10s for each running task
- Detects stuck tasks — no event emitted for >60s → mark as stale
- Auto-recovery: stale tasks get cancelled + re-queued if [can_retry()](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/models.rs#200-204)
- Tracks per-task memory via `sysinfo` crate (already in deps)
- Emits via Tauri `app.emit("agent-heartbeat", ...)` for frontend indicators
- Runs cleanup: `store.cleanup_old(7)` once per hour

**What makes this 10/10**:

- Self-healing: stuck agent auto-recovers without user intervention
- Resource monitoring: tracks actual memory per task
- Integrates with Tauri events for live UI

---

### Layer 3: Orchestration (depends on Layer 2)

#### [NEW] [manager.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/manager.rs)

The single entry point for all agent operations — **replaces creating AgentRuntime per request**.

**Requirements**:

- Singleton managed via `tauri::manage()` with `Arc<Mutex<>>`
- `spawn(input, config) -> TaskId` — creates task, persists to store, spawns executor
- [cancel(task_id)](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/protocol.rs#234-240) — signals CancellationToken, waits for clean shutdown
- [pause(task_id)](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/models.rs#168-172) — saves checkpoint, stops executor, marks Paused
- `resume(task_id)` — loads checkpoint, re-spawns executor from saved messages
- [list_tasks(filter)](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/store.rs#152-189) — queries store
- `subscribe(task_id) -> broadcast::Receiver<AgentEvent>` — for SSE streaming
- `on_boot()` — loads incomplete tasks from store, marks stale Running→Failed or re-queues
- Concurrent task limit: configurable (default 3)
- Owns HeartbeatMonitor — starts/stops with manager lifecycle

**What makes this 10/10**:

- Survives any crash: boot → restore → continue
- Pause/resume: no other agent framework does this (not DeerFlow, not CoPaw, not OpenFang)
- Concurrent task queue with backpressure

---

#### [NEW] [scheduler.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/scheduler.rs)

Autonomous scheduled tasks — **the 24/7 personal AI**.

**Requirements**:

- Cron-like schedule definitions stored in SQLite
- `add_schedule(cron_expr, input, config)` — registers a recurring task
- `remove_schedule(id)` — stops future executions
- `list_schedules()` — shows all active schedules
- Background tick loop: checks schedules every 60s, spawns tasks via Manager
- Persisted: schedules survive restart

**What makes this 10/10**:

- CoPaw has heartbeat-based scheduling — ours is cron-level precise
- DeerFlow has no scheduling at all
- OpenFang's "Hands" is manual — ours is autonomous

---

#### [NEW] [guardian.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/guardian.rs)

Security middleware — wires ALL existing security modules into the agent pipeline.

**Requirements**:

- `Guardian::check_input(input)` — runs injection scan, returns Ok or Err
- `Guardian::authorize_tool(tool_name, args)` — checks tool policy (deny > allow > profile)
- `Guardian::record_action(action)` — appends to audit log with SHA-256 chain
- `Guardian::check_budget(meter)` — enforces compute limits
- All methods return `Result<()>` — agent loop stops on any [Err](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/desktop/src/lib/api/types.ts#60-64)

**What makes this 10/10**:

- Single security facade — impossible to forget a check
- IronClaw-level security without WASM overhead
- Every action audited, every input scanned, every tool authorized

---

### Layer 4: Integration

#### [MODIFY] [mod.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/mod.rs)

- Re-export all 8 modules
- Keep [AgentRuntime](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/mod.rs#10-17) for backward compat (CLI single-turn use)
- `AgentManager` is the primary interface for server + desktop

#### [MODIFY] [stream_handler.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/server/src/stream_handler.rs)

**Delete fake streaming entirely.** Replace with:

- `POST /api/agent/tasks` → spawn task, return `{ task_id }`
- `GET /api/agent/tasks/{id}/stream` → real SSE from `manager.subscribe(id)`
- `POST /api/agent/tasks/{id}/cancel` → cancel
- `GET /api/agent/tasks` → list tasks
- `GET /api/agent/tasks/{id}` → task status

#### [MODIFY] [agent_handler.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/server/src/agent_handler.rs)

- [AgentState](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/server/src/agent_handler.rs#14-19) holds `AgentManager` (not raw config)
- `AgentManager::on_boot()` called at server startup
- Keep `POST /api/agent/chat` as synchronous wrapper (spawns task, awaits completion)

---

## Verification Plan (Strict — No Exceptions)

### Unit Tests (Rust)

```bash
# Every module gets its own test suite
cargo test -p ai-launcher-core -- agent::models      # state machine transitions
cargo test -p ai-launcher-core -- agent::protocol     # SSE serialization
cargo test -p ai-launcher-core -- agent::store        # SQLite CRUD + checkpoint
cargo test -p ai-launcher-core -- agent::executor     # loop with mock provider (test-only)
cargo test -p ai-launcher-core -- agent::heartbeat    # stuck task detection
cargo test -p ai-launcher-core -- agent::manager      # spawn/cancel/pause/resume
cargo test -p ai-launcher-core -- agent::scheduler    # cron parsing + tick
cargo test -p ai-launcher-core -- agent::guardian     # security checks

# Full workspace
cargo check --workspace
cargo test -p ai-launcher-core
```

### E2E Tests (Playwright CDP — Tauri WebView2)

Per AGENTS.md rules: tests run inside Tauri WebView2 via CDP, not standalone browser.

#### [NEW] `e2e/agent-streaming.spec.ts`

- Open Chat app → send message → verify real SSE tokens arrive (not word-split)
- Verify [text_delta](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/agent/protocol.rs#174-180) events appear incrementally
- Verify `task_completed` event contains final output

#### [NEW] `e2e/agent-cancel.spec.ts`

- Spawn a long task → click cancel → verify `task_cancelled` event
- Verify task state shows "cancelled" in task list

#### [NEW] `e2e/agent-heartbeat.spec.ts`

- Spawn task → wait 15s → verify heartbeat events received
- Verify Architecture app shows live task count

#### [NEW] `e2e/agent-persistence.spec.ts`

- Spawn task → kill server → restart → verify task recovered

```bash
# Run E2E (requires dev.sh running)
cd desktop && npx playwright test e2e/agent-streaming.spec.ts --reporter=list
cd desktop && npx playwright test e2e/agent-cancel.spec.ts --reporter=list
```

### Integration Test (API)

```bash
# Health + task lifecycle via curl
curl -s http://localhost:8080/api/health
curl -X POST http://localhost:8080/api/agent/tasks -d '{"message":"hello"}'
curl -s http://localhost:8080/api/agent/tasks
```

## Implementation Order

1. `guardian.rs` — security facade (wires existing modules)
2. `executor.rs` — real streaming loop with guardian integrated
3. `heartbeat.rs` — background monitor
4. [manager.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/llm/manager.rs) — task lifecycle orchestrator
5. `scheduler.rs` — cron-based autonomous tasks
6. Update [mod.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/llm/mod.rs) — re-exports
7. Update [stream_handler.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/server/src/stream_handler.rs) — delete fake streaming, add real SSE endpoints
8. Update [agent_handler.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/server/src/agent_handler.rs) — use AgentManager
9. Unit tests for all modules
10. E2E specs for streaming, cancel, heartbeat, persistence
