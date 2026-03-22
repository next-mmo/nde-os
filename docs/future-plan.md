# Future Plan: Agent OS, Gateway & Licensing

> **Status**: Planned (post-desktop)  
> **Codename**: FangFlow  
> **Timeframe**: 3–6 months after desktop release

---

## 1. Agent Looping Runtime

Transform AI Launcher into a full **Agent Operating System** with autonomous agent execution.

### Agent Loop State Machine

```
IDLE → THINK → EXECUTE (tool call) → OBSERVE → THINK → ... → REPLY → DONE
```

### Core Components

| Module | Purpose |
|--------|---------|
| `runtime/` | Agent loop state machine, config, conversation state |
| `llm/` | Multi-provider LLM drivers (Anthropic, OpenAI, Ollama, Groq) |
| `tools/` | 20+ built-in tools + MCP client for external tool servers |
| `memory/` | SQLite-backed persistent memory, key-value store, vector embeddings |
| `workflow/` | DAG execution engine — parallel, conditional, fan-out |
| `agents/` | Agent manifests, supervisor pattern, sub-agent spawning |
| `channels/` | Telegram, Discord, Slack, webhooks |
| `skills/` | OpenFang/DeerFlow-compatible skill format, marketplace |

### Agent Manifest (TOML)

```toml
[agent]
name = "researcher"
description = "Web research + report generation"

[model]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
fallback = ["openai:gpt-4o", "groq:llama-3.3-70b"]

[capabilities]
tools = ["web_search", "web_fetch", "file_write", "python_exec"]
max_loop_iterations = 50

[sandbox]
workspace = true
network = "filtered"    # Outbound HTTPS only
filesystem = "jailed"   # Uses existing Sandbox module
```

### Key Differentiator

Agents run inside our **existing Rust sandbox** — no Docker required. Cross-platform (Windows + Linux native). Single binary distribution.

---

## 2. Gateway (License Server)

A standalone Rust server that handles auth, licensing, feature gating, and app catalog distribution.

### Architecture: Server Brain + Local Muscle

```
┌─────────────────────────┐     ┌─────────────────────────┐
│     LICENSE SERVER       │     │     LOCAL MACHINE       │
│     (cloud/VPS)          │     │     (user's GPU)        │
│                          │     │                         │
│  • User auth             │◄───►│  • AI Launcher (Rust)   │
│  • License validation    │     │  • GPU inference        │
│  • App catalog serving   │     │  • Model loading        │
│  • Feature flags/tiers   │     │  • Sandbox / venv       │
│  • Usage analytics       │     │  • Offline cache (72h)  │
│  • Config sync           │     │  • Desktop UI           │
└─────────────────────────┘     └─────────────────────────┘
```

### Server Tech Stack

| Component | Choice |
|-----------|--------|
| Language | Rust (Axum) |
| Database | SQLite → Postgres |
| Auth | JWT (ES256) |
| License signing | Ed25519 |
| Deploy | Single binary, any VPS ($5/mo) |

### Server API Surface

```
# Auth
POST /api/auth/register, /login, /refresh, /logout

# License
POST /api/license/activate, /validate, /deactivate
GET  /api/license/entitlements

# App Catalog (replaces local hardcoded manifests)
GET  /api/catalog, /catalog/:id, /catalog/:id/download

# Config Sync
GET/PUT /api/sync/config

# Admin
GET  /api/admin/users, /analytics
POST /api/admin/licenses
```

### Challenge-Response Protocol

Every sensitive action (install, launch, download) requires a server round-trip:
1. Client sends signed `ACTION_REQUEST` (action + hardware ID + HMAC)
2. Server validates license/tier → returns encrypted `ACTION_RESPONSE` with manifest, deps, signed download URL
3. Client executes locally with one-time-use token

---

## 3. Licensing & Tiers

### Feature Matrix

| Feature | Free | Pro | Enterprise |
|---------|------|-----|------------|
| Max installed apps | 2 | 10 | Unlimited |
| Concurrent running | 1 | 3 | Unlimited |
| App catalog | Basic | Full | Full + Custom |
| GPU priority mode | ❌ | ✅ | ✅ |
| Config sync | ❌ | ✅ | ✅ |
| Custom models | ❌ | ✅ | ✅ |
| Agent runtime | 1 agent | 5 agents | Unlimited |
| License devices | 1 | 3 | 10 |

### License Token (Ed25519 Signed)

```json
{
  "sub": "user-uuid",
  "lic": "FL-A1B2-C3D4-E5F6",
  "tier": "pro",
  "hw": "sha256:...",
  "features": ["unlimited_apps", "gpu_priority", "custom_models"],
  "exp": 1711195200
}
```

- **Hardware-bound**: Won't validate on different machine
- **Time-limited**: 24h expiry, must heartbeat every 15 min
- **Offline grace**: 72h without server contact

### Client Hardening

| Layer | Technique |
|-------|-----------|
| Transport | TLS 1.3 + certificate pinning |
| Binary | SHA256 integrity self-check |
| Strings | Compile-time encryption |
| Debug | Anti-debug detection |
| Protocol | Rotating session keys |
| Updates | Forced update channel |

---

## Roadmap Sequence

```
Phase 0  ✅  Sandbox + uv + REST API + plugins
Phase 0.5    Desktop migration (Tauri 2) ← CURRENT
Phase 0.6    License server (Axum + Ed25519)
Phase 0.7    Server-side logic split + client hardening
Phase 1      Agent runtime (loop, LLM drivers, tools)
Phase 2      Memory & tools (SQLite, MCP, 20+ built-in)
Phase 3      Workflows & orchestration (DAG, sub-agents)
Phase 4      Channels & autonomy (Telegram, Discord, cron)
Phase 5      Compatibility & ecosystem (OpenFang/DeerFlow skills)
```
