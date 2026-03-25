---
name: enterprise-verify
description: Verifies that NDE-OS agent core code is production-ready with no mocks, no fakes, no TODOs, and matches all R&D comparison claims. Use after implementing any agent core module, before marking a task as complete, or when auditing code quality against the implementation plan.
---

# Enterprise Verification Skill

## Purpose

This skill verifies that every R&D claim in `r&d-agent-looping.md` is backed by real, production-grade code — not mocks, not fakes, not placeholders. It also validates API contracts, thread safety, error handling quality, dependency health, and performance baselines.

Run this after implementing any agent core module.

## When to Trigger

- After creating or modifying any file in `core/src/agent/`
- After modifying `server/src/stream_handler.rs` or `server/src/main.rs`
- Before marking any implementation task as `[x]` done
- When auditing the codebase against competitor claims
- After adding or updating dependencies in `Cargo.toml`
- Before any release or demo

---

## Step 1: Forbidden Patterns Scan

Scan every file in the agent core for forbidden patterns. **Any match in non-test code = fail**.

```bash
# Run from project root — ALL must return zero matches in non-test code:

# 1. No TODO/FIXME/HACK/XXX comments
rg -i "TODO|FIXME|HACK|XXX" core/src/agent/ --type rust -g '!*test*'

# 2. No mock/stub/dummy/placeholder/fake in production code
rg -i "mock|stub|dummy|placeholder|fake" core/src/agent/ --type rust -g '!*test*'

# 3. No unwrap() in production code (use ? or .unwrap_or)
rg "\.unwrap\(\)" core/src/agent/ --type rust -g '!*test*' -g '!*mod.rs'

# 4. No println!/dbg!/eprintln! (use tracing instead)
rg "println!|dbg!|eprintln!" core/src/agent/ --type rust -g '!*test*'

# 5. No unimplemented!(), todo!(), or unreachable!() in production paths
rg "unimplemented!\(|todo!\(|unreachable!\(" core/src/agent/ --type rust -g '!*test*'

# 6. No panic!() in production code (except explicit bounds checks)
rg "panic!\(" core/src/agent/ --type rust -g '!*test*'

# 7. No dead code / unused suppressions hiding incomplete work
rg "allow\(dead_code\)|allow\(unused" core/src/agent/ --type rust

# 8. No hardcoded secrets or credentials
rg -i "sk-|api_key\s*=\s*\"|password\s*=\s*\"" core/src/agent/ --type rust
```

**Verdict**: If any of the above return matches in non-test code, the module is **not production-ready**.

---

## Step 2: R&D Claims Verification

For each claim in `r&d-agent-looping.md`, verify the backing code exists and is functional.

### Agent Runtime Claims

| Claim | File to Check | What to Verify |
|---|---|---|
| Agent loop | `core/src/agent/mod.rs` | `AgentRuntime::run()` exists, compiles, loops up to `max_iterations` |
| 9 providers | `core/src/llm/mod.rs` | `create_provider()` match has 9+ arms (anthropic, openai, ollama, groq, deepseek, mistral, openrouter, local, gguf) |
| SSE streaming | `core/src/agent/protocol.rs` | `AgentEvent::to_sse()` returns valid `data: {...}\n\n` format |
| LLM hot-swap | `core/src/llm/manager.rs` | `LlmManager::switch()` or equivalent runtime model-change method exists |
| Task persistence | `core/src/agent/store.rs` | `TaskStore::save_task()` + `load_task()` use `rusqlite::Connection` with WAL mode |
| SQLite memory | `core/src/memory/` | `ConversationStore` and `KeyValueStore` exist with proper CRUD |

### Security Claims

| Claim | File to Check | What to Verify |
|---|---|---|
| Injection scan | `core/src/security/injection.rs` | `InjectionScanner::scan()` has 8 patterns (4 High, 2 Medium, 2 Low severity) |
| SHA-256 audit | `core/src/security/audit.rs` | `AuditTrail::log()` uses `Sha256::new()`, chains `prev_hash` |
| Compute metering | `core/src/security/metering.rs` | `ComputeMeter::check_budget()` checks 3 limits: tokens, time, tool_calls |
| Guardian facade | `core/src/agent/guardian.rs` | `Guardian::check_input()` calls `scanner.scan()` |
| Guardian wired | `core/src/agent/guardian.rs` | `Guardian::authorize_tool()` calls `meter.add_tool_call()` + `meter.check_budget()` |
| Guardian audit | `core/src/agent/guardian.rs` | `Guardian::record_action()` calls `audit.log()` |

### Agent Core v2 Claims

| Claim | File to Check | What to Verify |
|---|---|---|
| Task state machine | `core/src/agent/models.rs` | `TaskState` enum has exactly 7 variants: Pending, Running, Paused, Completed, Failed, Cancelled, TimedOut |
| 12 SSE events | `core/src/agent/protocol.rs` | `AgentEvent` enum has exactly 12 variants |
| Persistent store | `core/src/agent/store.rs` | `TaskStore` uses `rusqlite::Connection` with WAL journal mode |
| Checkpoint recovery | `core/src/agent/store.rs` | `save_checkpoint()` + `load_checkpoint()` exist, use `task_checkpoints` table |
| Executor with cancel | `core/src/agent/executor.rs` | Uses `CancellationToken` from `tokio_util`, checks `cancel.is_cancelled()` in loop |
| Heartbeat monitor | `core/src/agent/heartbeat.rs` | Emits `AgentEvent::Heartbeat` on interval, detects stale tasks via threshold |
| Task manager | `core/src/agent/manager.rs` | `spawn()`, `cancel()`, `pause()`, `resume()` all exist |
| Scheduler | `core/src/agent/scheduler.rs` | `ScheduleStore::add()` with cron expression, `start_scheduler()` background loop |
| Boot recovery | `core/src/agent/manager.rs` | `on_boot()` loads incomplete tasks, marks stale Running as Failed or re-queues |

```bash
# Quick verify: all agent modules compile
cargo check -p ai-launcher-core 2>&1 | tail -5
```

---

## Step 3: Unit Tests Must Pass

Every module MUST have its own `#[cfg(test)] mod tests` block with meaningful assertions.

```bash
# Run all agent tests
cargo test -p ai-launcher-core -- agent:: 2>&1

# Check test count
cargo test -p ai-launcher-core -- agent:: 2>&1 | grep "test result"
```

**Minimum test counts per module**:

| Module | Min Tests | What They Cover |
|---|---|---|
| `models` | 4 | lifecycle transitions, retry, timeout, chaining |
| `protocol` | 3 | serialization, task_id extraction, terminal detection |
| `store` | 5 | CRUD, state update, checkpoint save/load, filter, incomplete |
| `guardian` | 6 | safe input, injection block, tool auth, budget exceeded, disabled mode, audit integrity |
| `executor` | 2 | basic completion, cancellation (mock provider OK in test-only code) |
| `heartbeat` | 3 | register/unregister, activity tracking, stale detection |
| `manager` | 1 | manager creation (provider may fail in CI, graceful handling OK) |
| `scheduler` | 5 | cron every-minutes, every-hours, daily, invalid expr, store add/remove + due check |

```bash
# Verify minimum test counts per module
for mod in models protocol store guardian executor heartbeat manager scheduler; do
  count=$(cargo test -p ai-launcher-core -- "agent::${mod}::tests" 2>&1 | grep "^test " | wc -l)
  echo "${mod}: ${count} tests"
done
```

---

## Step 4: No Fake Streaming

The new streaming path MUST use real token-level streaming via `provider.chat_stream()` + mpsc events. The legacy fallback path MUST be clearly marked as deprecated.

```bash
# The NEW streaming path must NOT split words:
rg "split_inclusive|split\(' '\)" server/src/stream_handler.rs

# If matches found, verify they are ONLY in the fallback_stream_chat function (legacy compat).
# The primary handle_stream_chat path must use AgentManager.subscribe().

# Real SSE subscription MUST exist in the primary path:
rg "subscribe|broadcast|mpsc" server/src/stream_handler.rs

# Executor must use chat_stream (real streaming), not chat (sync):
rg "chat_stream" core/src/agent/executor.rs
```

**Legacy fallback**: `fallback_stream_chat()` in `stream_handler.rs` still uses `split_inclusive(' ')` for backward compatibility when `AgentManager` is unavailable. This is **acceptable** only if:
1. It is clearly gated behind `if let Some(manager)` — the primary path uses real streaming
2. The fallback is documented as deprecated
3. The primary `handle_stream_chat` calls `manager.subscribe()` for real SSE

---

## Step 5: Security Wiring Verification

Every security module must be called from the agent pipeline — not just existing in isolation.

```bash
# Guardian must be created and used in executor.rs:
rg "Guardian::new|guardian\." core/src/agent/executor.rs

# check_input must be called before LLM:
rg "check_input" core/src/agent/executor.rs

# authorize_tool must be called before each tool execution:
rg "authorize_tool" core/src/agent/executor.rs

# add_tokens must be called after each LLM response:
rg "add_tokens" core/src/agent/executor.rs

# check_budget must be called each iteration:
rg "check_budget" core/src/agent/executor.rs

# record_tool_result must be called after each tool:
rg "record_tool_result" core/src/agent/executor.rs

# AuditTrail SHA-256 must use sha2 crate:
rg "use sha2" core/src/security/audit.rs

# InjectionScanner must be called through Guardian (not bypassed):
rg "scanner\.scan" core/src/agent/guardian.rs

# ComputeMeter must be called through Guardian (not bypassed):
rg "meter\.(add_tool_call|check_budget|add_tokens)" core/src/agent/guardian.rs
```

---

## Step 6: Module Integration Wiring

Verify that all modules are properly wired into the module tree and re-exported.

```bash
# mod.rs must declare all 8 submodules:
for mod in config executor guardian heartbeat manager models protocol scheduler store; do
  rg "pub mod ${mod}" core/src/agent/mod.rs || echo "MISSING: ${mod}"
done

# Key types must be re-exported from mod.rs:
rg "pub use" core/src/agent/mod.rs

# AgentManager, AgentTask, TaskState, AgentEvent must be re-exported:
rg "pub use manager::AgentManager" core/src/agent/mod.rs
rg "pub use models::" core/src/agent/mod.rs
rg "pub use protocol::AgentEvent" core/src/agent/mod.rs
```

---

## Step 7: API Contract Verification

The server endpoints must match the implementation plan specification.

```bash
# Required REST endpoints (check server routing):
rg "agent/tasks" server/src/main.rs server/src/stream_handler.rs

# Must have these handlers:
# POST /api/agent/tasks         -> spawn_task
# GET  /api/agent/tasks         -> list_tasks
# GET  /api/agent/tasks/{id}    -> get_task
# GET  /api/agent/tasks/{id}/stream -> stream_task
# POST /api/agent/tasks/{id}/cancel -> cancel_task
# POST /api/agent/chat/stream   -> handle_stream_chat (backward-compat)

# Verify each handler function exists:
for fn in spawn_task list_tasks get_task stream_task cancel_task handle_stream_chat; do
  rg "pub fn ${fn}" server/src/stream_handler.rs || echo "MISSING HANDLER: ${fn}"
done
```

---

## Step 8: Thread Safety Verification

Agent core runs in an async multi-task environment. Verify safe concurrency patterns.

```bash
# TaskStore uses Mutex<Connection> (not raw Connection):
rg "Mutex<Connection>" core/src/agent/store.rs

# ScheduleStore uses Mutex<Connection>:
rg "Mutex<Connection>" core/src/agent/scheduler.rs

# AgentManager uses tokio::sync::Mutex for running tasks:
rg "Mutex<HashMap" core/src/agent/manager.rs

# HeartbeatTracker uses tokio::sync::Mutex:
rg "Mutex<HashMap" core/src/agent/heartbeat.rs

# Provider is Arc<dyn LlmProvider> (shared safely):
rg "Arc<dyn LlmProvider>" core/src/agent/executor.rs core/src/agent/manager.rs

# No raw static muts:
rg "static mut" core/src/agent/ --type rust
```

---

## Step 9: Error Handling Quality

Production code must not silently swallow errors on critical operations.

```bash
# Find all "let _ =" patterns — review each for appropriateness:
rg "let _ =" core/src/agent/ --type rust -g '!*test*' -n

# Acceptable: let _ = event_tx.send(...) — channel send failure is non-fatal
# Acceptable: let _ = store.save_checkpoint(...) — best-effort persistence
# NOT acceptable: let _ = store.save_task(...) in critical paths
# NOT acceptable: let _ = guardian.check_budget() — security check must not be ignored

# Verify security checks use ? (not let _ =):
rg "guardian\." core/src/agent/executor.rs -n
# check_input, authorize_tool, check_budget should all use ? operator
```

---

## Step 10: Dependency Health

```bash
# Check for known vulnerabilities (requires cargo-audit installed):
cargo audit 2>&1 | tail -10

# Verify key dependencies are present in Cargo.toml:
for dep in rusqlite tokio tokio-util chrono sha2 serde serde_json uuid anyhow async-trait futures sysinfo; do
  rg "^${dep}" core/Cargo.toml || echo "MISSING DEP: ${dep}"
done

# No wildcard version specs:
rg '\*' core/Cargo.toml | grep -v '#'
```

---

## Step 11: Workspace Build Health

```bash
# Full workspace must compile with zero errors
cargo check --workspace 2>&1 | tail -5

# Count warnings in agent modules (target: 0)
cargo check -p ai-launcher-core 2>&1 | rg "warning\[" | wc -l

# Binary size check (development build — target < 100MB debug, < 30MB release)
cargo build -p ai-launcher-core 2>&1 | tail -3
```

---

## Output Format

After running all steps, produce a verification report:

```
## Enterprise Verification Report

Date: YYYY-MM-DD
Modules verified: [list]

### Step 1: Forbidden Patterns
- [ ] No TODO/FIXME/HACK/XXX
- [ ] No mock/stub/fake in production code
- [ ] No unwrap() in production code
- [ ] No println!/dbg!/eprintln!
- [ ] No unimplemented!/todo!/unreachable!
- [ ] No panic!() in production code
- [ ] No dead_code/unused suppressions
- [ ] No hardcoded secrets

### Step 2: R&D Claims
- [ ] Agent runtime claims verified (list each)
- [ ] Security claims verified (list each)
- [ ] Agent core v2 claims verified (list each)

### Step 3: Unit Tests
- [ ] All tests pass
- [ ] Test counts meet minimums per module
| Module | Required | Actual | Pass? |
|--------|----------|--------|-------|
| models | 4 | N | Y/N |
| protocol | 3 | N | Y/N |
| store | 5 | N | Y/N |
| guardian | 6 | N | Y/N |
| executor | 2 | N | Y/N |
| heartbeat | 3 | N | Y/N |
| manager | 1 | N | Y/N |
| scheduler | 5 | N | Y/N |

### Step 4: Streaming
- [ ] Primary path uses real SSE (AgentManager.subscribe)
- [ ] Executor uses chat_stream (not chat sync)
- [ ] Legacy fallback clearly marked as deprecated

### Step 5: Security Wiring
- [ ] Guardian created in executor
- [ ] check_input called before LLM
- [ ] authorize_tool called before each tool
- [ ] add_tokens called after each LLM response
- [ ] check_budget called each iteration
- [ ] record_tool_result called after each tool
- [ ] SHA-256 chain in audit trail
- [ ] All security checks use ? (not let _ =)

### Step 6: Module Integration
- [ ] All 8+ submodules declared in mod.rs
- [ ] Key types re-exported (AgentManager, AgentTask, TaskState, AgentEvent)

### Step 7: API Contracts
- [ ] POST /api/agent/tasks (spawn)
- [ ] GET /api/agent/tasks (list)
- [ ] GET /api/agent/tasks/{id} (status)
- [ ] GET /api/agent/tasks/{id}/stream (SSE)
- [ ] POST /api/agent/tasks/{id}/cancel
- [ ] POST /api/agent/chat/stream (backward compat)

### Step 8: Thread Safety
- [ ] TaskStore: Mutex<Connection>
- [ ] ScheduleStore: Mutex<Connection>
- [ ] AgentManager: tokio::Mutex for running tasks
- [ ] HeartbeatTracker: tokio::Mutex for activities
- [ ] Provider: Arc<dyn LlmProvider>
- [ ] No static muts

### Step 9: Error Handling
- [ ] Security checks never swallowed (use ? operator)
- [ ] Event sends OK to swallow (non-fatal)
- [ ] Checkpoint saves OK to swallow (best-effort)

### Step 10: Dependencies
- [ ] cargo audit clean (or known exceptions documented)
- [ ] All required crates present

### Step 11: Build Health
- [ ] cargo check --workspace passes
- [ ] Zero warnings in agent modules
- [ ] Compiles successfully

**Overall Verdict**: PASS / FAIL (list any failures)
```

---

## Rules

1. **Tests fail = module not done.** Period.
2. **Any forbidden pattern found = module not done.** No exceptions.
3. **R&D claim without backing code = claim must be downgraded** from Done to Building or Missing.
4. **Fake streaming in primary path = immediate fail.** Legacy fallback is tolerated but must be marked deprecated.
5. **Security not wired = immediate fail.** Isolated modules don't count.
6. **Security checks swallowed with `let _ =` = immediate fail.** Guardian results must propagate via `?`.
7. **Thread safety violation = immediate fail.** Shared state without proper synchronization is a data race.
8. **Missing API endpoint = fail.** Every endpoint in the plan must be routed.
9. **Hardcoded credentials = immediate fail.** No exceptions.

---

## Quick Run (Copy-Paste)

Run the full verification in one shot:

```bash
echo "=== Step 1: Forbidden Patterns ==="
echo "--- TODO/FIXME ---"
rg -i "TODO|FIXME|HACK|XXX" core/src/agent/ --type rust -g '!*test*' -c 2>/dev/null || echo "CLEAN"
echo "--- mock/stub/fake ---"
rg -i "mock|stub|dummy|placeholder|fake" core/src/agent/ --type rust -g '!*test*' -c 2>/dev/null || echo "CLEAN"
echo "--- unwrap ---"
rg "\.unwrap\(\)" core/src/agent/ --type rust -g '!*test*' -g '!*mod.rs' -c 2>/dev/null || echo "CLEAN"
echo "--- println/dbg ---"
rg "println!|dbg!|eprintln!" core/src/agent/ --type rust -g '!*test*' -c 2>/dev/null || echo "CLEAN"
echo "--- unimplemented/todo/unreachable ---"
rg "unimplemented!\(|todo!\(|unreachable!\(" core/src/agent/ --type rust -g '!*test*' -c 2>/dev/null || echo "CLEAN"
echo "--- panic ---"
rg "panic!\(" core/src/agent/ --type rust -g '!*test*' -c 2>/dev/null || echo "CLEAN"
echo "--- dead_code/unused ---"
rg "allow\(dead_code\)|allow\(unused" core/src/agent/ --type rust -c 2>/dev/null || echo "CLEAN"

echo ""
echo "=== Step 3: Tests ==="
cargo test -p ai-launcher-core -- agent:: 2>&1 | grep "test result"

echo ""
echo "=== Step 4: Streaming ==="
echo "--- split_inclusive in primary path? ---"
rg "split_inclusive" server/src/stream_handler.rs -n 2>/dev/null || echo "CLEAN"
echo "--- chat_stream in executor? ---"
rg "chat_stream" core/src/agent/executor.rs -c 2>/dev/null || echo "MISSING"

echo ""
echo "=== Step 5: Security Wiring ==="
for check in "Guardian::new" "check_input" "authorize_tool" "add_tokens" "check_budget" "record_tool_result"; do
  rg "$check" core/src/agent/executor.rs -c 2>/dev/null && echo "${check}: WIRED" || echo "${check}: NOT WIRED"
done

echo ""
echo "=== Step 11: Build ==="
cargo check --workspace 2>&1 | tail -3
```
