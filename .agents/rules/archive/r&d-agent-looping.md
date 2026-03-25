# R&D: Agent Framework Comparison

> **NDE-OS vs the entire landscape** — Rust agents, TypeScript agents, Python platforms, ByteDance superagent, personal AI assistants.
> 
> *Last updated: 2026-03-26 — Phase 3: DeerFlow + CoPaw added*

## Legend

✅ Done · 🔨 Building · 📋 Planned · ❌ Missing · ⭐ NDE-OS unique · 🥇 Best in class

---

## All Features at a Glance

| Feature                      |    OpenClaw    |   OpenFang    |    IronClaw    |  ZeroClaw   |   Moltis   |  PocketPaw   |   DeerFlow   |     CoPaw      |          **NDE-OS**           |
| :--------------------------- | :------------: | :-----------: | :------------: | :---------: | :--------: | :----------: | :----------: | :------------: | :---------------------------: |
| **IDENTITY**                 |                |               |                |             |            |              |              |                |                               |
| Language                     |   TypeScript   |     Rust      |      Rust      |    Rust     |    Rust    |    Python    |    Python    |     Python     |           **Rust**            |
| Type                         |  AI assistant  |   Agent OS    | Security agent | Lightweight |  Runtime   | Ext platform | SuperAgent   | Personal AI    |       **AI Desktop OS**       |
| Binary size                  | ~200 MB (Node) |     32 MB     |     ~20 MB     |  🥇 3.4 MB  |   44 MB    | ~50 MB (Py)  | ~100 MB (Py) |  ~80 MB (Py)   |        🔨 target <30MB        |
| RAM usage                    |    ~300 MB     |    ~50 MB     |     ~80 MB     |  🥇 <5 MB   |   ~60 MB   |   ~120 MB    |   ~200 MB    |    ~150 MB     |        🔨 target <50MB        |
| Boot time                    |      ~2s       |    <200ms     |       —        |  🥇 <10ms   |     —      |     ~3s      |     ~5s      |      ~3s       |       🔨 target <500ms        |
| GitHub stars                 |    🥇 40k+     |     12k+      |      3k+       |    2k+      |    5k+     |     1k+      |    35k+      |      8k+       |          ⭐ private           |
| **AGENT RUNTIME**            |                |               |                |             |            |              |              |                |                               |
| Agent loop                   | 🥇 ✅ original |      ✅       |       ✅       |     ✅      |     ✅     |      ❌      |   ✅ multi   |   ✅ single    |        ✅ **verified**         |
| Multi-agent orchestration    |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      | 🥇 ✅ graph  |    partial     |          📋 Phase 4           |
| Multi-provider LLM           |       ✅       |     🥇 26     |       ✅       |  ✅ trait   |     ✅     |      ✅      |  ✅ OpenAI   |    ✅ multi    | ⭐ ✅ **9 providers (tested)** |
| Local LLM (Ollama)           |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |    ✅ ext    |   ✅ local   |    ✅ local    |     ⭐ ✅ **auto-launch**     |
| Native LLM (llama.cpp)       |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |      ❌      |       ❌       |   ⭐ 🔨 **GGUF in-process**   |
| Streaming                    |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |    ✅ SSE    |      ✅      |       ✅       |     ✅ **SSE (word-level)**    |
| LLM hot-swap                 |       ❌       |     partial   |       ❌       |     ❌      |     ❌     |      ❌      |      ❌      |       ❌       | ⭐ ✅ **runtime switch**       |
| Autonomous scheduling        |       ❌       |   🥇 Hands    |       ❌       |     ✅      |     ❌     |      ❌      |      ❌      |  ✅ heartbeat  |          🔨 building          |
| Human-in-the-loop            |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |  🥇 ✅ HITL  |       ❌       |          📋 Phase 4           |
| Long-term memory             |       ❌       |      ✅       |       ❌       |     ❌      |     ❌     |      ❌      |  🥇 ✅ LTM   |  ✅ ReMe       |     ✅ **SQLite (verified)**   |
| Task persistence (24/7)      |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      | ✅ checkpoint|  ✅ heartbeat  |   ⭐ 🔨 **SQLite+checkpoint**  |
| **SANDBOX & SECURITY**       |                |               |                |             |            |              |              |                |                               |
| Sandbox type                 |  ❌ **none**   |   WASM dual   |  🥇 WASM cap   |    WASM     | WASM+Apple |  uv+tokens   | Docker/K8s   |    ❌ none     |      ⭐ ✅ **OS jail**        |
| Path traversal defense       |       ❌       |     WASM      |      WASM      |    WASM     |    WASM    |      ❌      |   Docker     |       ❌       |    ⭐ ✅ **canonicalize**     |
| Symlink defense              |       ❌       |     WASM      |      WASM      |    WASM     |    WASM    |      ❌      |   Docker     |       ❌       |  ⭐ ✅ **resolve+validate**   |
| Env-var jailing              |       ❌       |      ❌       |    partial     |     ❌      |     ❌     |      ❌      |   Docker     |       ❌       |   ⭐ ✅ **12 vars jailed**    |
| Prompt injection scan        |       ❌       |      ✅       |     🥇 ✅      |     ❌      |     ❌     |      ❌      |      ❌      |       ❌       |    ✅ **verified (28 tests)** |
| Credential isolation         |  ❌ plaintext  |      ❌       |    🥇 vault    |     ❌      |     ❌     |    ✅ JWT    |   ❌ env     |    ❌ env      |      ✅ env-per-sandbox       |
| Audit trail                  |       ❌       |   🥇 Merkle   |       ❌       |     ❌      |     ❌     |      ❌      |      ❌      |       ❌       |    ✅ **SHA-256 hash chain**  |
| Compute metering             |       ❌       |    ✅ fuel    |       ❌       |   ✅ fuel   |     ✅     |      ❌      |      ❌      |       ❌       |       ✅ **token+tool+time** |
| Known CVEs                   | ⚠️ **9.4+9.6** |   ✅ clean    |    ✅ clean    |  ✅ clean   |  ✅ clean  |   ✅ clean   |   ✅ clean   |    ✅ clean    |           ✅ clean            |
| Cross-platform               |    ✅ Node     |      ✅       |    partial     |     ✅      |  ❌ Apple  |      ✅      |   ✅ Docker  |       ✅       |      ⭐ ✅ **Win+Linux**      |
| **TOOLS & MCP**              |                |               |                |             |            |              |              |                |                               |
| Built-in tools               |      ~10       |     🥇 38     |      ~10       |     ~15     |    ~15     |      0       |   ~8 search  |    ~5 skill    | ✅ **23 (sandbox-jailed)**     |
| MCP client                   |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |      ❌      |      ❌      |       ❌       |   ✅ **stdio JSON-RPC**        |
| MCP server                   |       ❌       |     🥇 ✅     |       ❌       |     ❌      |     ❌     |      ❌      |      ❌      |       ❌       |   ✅ **expose tools**          |
| Dynamic tools (plugins)      |       ❌       |      ❌       |    ✅ WASM     |     ✅      |     ❌     |      ❌      |   ✅ tool    |   ✅ skills   | ⭐ ✅ **plugin engine**        |
| Sandbox enforcement          |       ❌       |      ✅       |       ✅       |     ✅      |     ✅     |   partial    | ✅ Docker    |       ❌       |       ⭐ ✅ **jail**          |
| **PLUGINS**                  |                |               |                |             |            |              |              |                |                               |
| Plugin engine                |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |    ✅ ext    |  ✅ middle   |   ✅ skills   | ⭐ ✅ **manifest v2**          |
| Plugin lifecycle             |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |   partial    |      ❌      |   ✅ auto-load| ⭐ ✅ **discover→install→run** |
| Hook system                  |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      | 🥇 11-layer  |       ❌       | ⭐ ✅ **9 hook types**         |
| Plugin-contributed tools     |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |      ❌      |       ❌       | ⭐ ✅ **auto-merge**           |
| **SKILLS & KNOWLEDGE**       |                |               |                |             |            |              |              |                |                               |
| Skill format                 |  🥇 SKILL.md   |  HAND+SKILL   |       ❌       |  SKILL.md   |  SKILL.md  |   ext.json   |   Python fn  |  Python skill  |   ✅ **SKILL.md (verified)**   |
| Skill marketplace            |   ✅ ClawHub   |      ✅       |       ❌       | ✅ ZeroHub  |  ✅ Store  |      ❌      |      ❌      |   ✅ URL imp  |          🔨 building          |
| Knowledge graph              |       ❌       |     🥇 ✅     |       ❌       |     ❌      |     ❌     |      ❌      |    partial   |       ❌       |   ✅ **SQLite (verified)**     |
| **MEMORY**                   |                |               |                |             |            |              |              |                |                               |
| Conversation persist         |  ✅ MD files   |  🥇 cross-ch  |  ✅ encrypted  |   ✅ log    |     ✅     |   ✅ JSON    | ✅ checkpoint|   ✅ JSON     |   ✅ **SQLite (verified)**     |
| Key-value store              |       ❌       |   ✅ SQLite   |       ✅       |  ✅ SQLite  |     ✅     |   ✅ JSON    |      ❌      |  ✅ ReMe MD   |   ✅ **SQLite (verified)**     |
| Vector search                |       ❌       | 🥇 sqlite-vec |       ❌       |  ✅ cosine  |     ❌     |      ❌      |      ❌      | ✅ semantic   |          🔨 building          |
| **CHANNELS**                 |                |               |                |             |            |              |              |                |                               |
| NDE-OS desktop chat          |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |      ❌      |       ❌       | ⭐ ✅ **REST+UI (integrated)** |
| Telegram                     |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |      ❌      |   ✅ Lark    |   ✅ DingTalk |    ✅ **long-poll gateway**    |
| Discord                      |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |      ❌      |   ✅ Slack   |  ✅ Discord   |           📋 Phase 3          |
| Multi-channel                |     🥇 10+     |      40+      |       5        |      4      |     4      |      1       |      3       |    🥇 6+      |    ✅ **2 (REST+Telegram)**    |
| **CLI**                      |                |               |                |             |            |              |              |                |                               |
| CLI binary                   |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |      ❌      |      ❌      |    ✅ pip     | ⭐ ✅ **`nde` command**        |
| Interactive REPL             |       ✅       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |      ❌      |       ❌       | ⭐ ✅ **streaming REPL**       |
| **DESKTOP OS (NDE-OS ONLY)** |                |               |                |             |            |              |              |                |                               |
| macOS-style desktop          |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |      ❌      |  ❌ CLI only  |    ⭐ ✅ **Tauri+Svelte**     |
| Window management            |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |      ❌      |       ❌       |  ⭐ ✅ **drag/resize/min**    |
| Animated dock                |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |      ❌      |       ❌       |    ⭐ ✅ **macOS-style**      |
| App catalog + installer      |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     | ✅ ext store |      ❌      |       ❌       |    ⭐ ✅ **manifest+uv**      |
| Built-in browser             |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |      ❌      |       ❌       |      ⭐ ✅ **embedded**       |
| Built-in code editor (IDE)   |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |      ❌      |       ❌       | ⭐ ✅ **editor+terminal+AI**  |
| Per-app sandbox venv         |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |    ✅ uv     |      ❌      |       ❌       |      ⭐ ✅ **uv+jail**        |

### Score Tally

| Framework  | ✅ Done | 🔨 Building | 📋 Planned | ⭐ Unique | 🥇 Best |
| :--------- | :-----: | :---------: | :---------: | :------: | :-----: |
| **NDE-OS** | **48**  |    **5**    |   **3**     | **26**   | **4**   |
| DeerFlow   |   18    |      0      |      0      |    0     |    3    |
| CoPaw      |   16    |      0      |      0      |    0     |    1    |
| OpenFang   |   22    |      0      |      0      |    0     |    8    |
| OpenClaw   |   18    |      0      |      0      |    0     |    4    |
| IronClaw   |   15    |      0      |      0      |    0     |    3    |
| ZeroClaw   |   17    |      0      |      0      |    0     |    3    |
| PocketPaw  |    8    |      0      |      0      |    0     |    1    |

---

## New Competitor Profiles

### 🦌 DeerFlow (ByteDance) — `bytedance/deer-flow`

**Type**: SuperAgent orchestration framework (Python + LangGraph)
**Stars**: 35k+ (trended #1 on GitHub)
**Core architecture**: LangGraph workflow engine with supervisor → sub-agents (Researcher, Coder, Reporter)

**What they do best**:
- 🥇 **Multi-agent orchestration**: Supervisor decomposes tasks → delegates to specialized sub-agents → collects & synthesizes
- 🥇 **11-layer middleware chain**: Plugin architecture via middleware stack (pre/post processing, context management)
- 🥇 **Human-in-the-loop (HITL)**: Pause execution at checkpoints, surface intermediate results for user review
- ✅ **Docker/K8s sandbox**: Each agent gets an isolated filesystem + bash terminal via ByteDance's AIO Sandbox
- ✅ **Long-term memory**: LLM-powered extraction of durable facts + LangGraph checkpointing
- ✅ **Multi-output**: Generates reports, podcast scripts, PowerPoint presentations, TTS

**What they lack**:
- ❌ No desktop UI (web-only Next.js frontend)
- ❌ No MCP support
- ❌ No native binary (Python + Docker dependency)
- ❌ No per-app sandbox venv
- ❌ No credential isolation or audit trail
- ❌ No CLI binary

**NDE-OS advantage**: Native Rust binary, OS-level sandbox without Docker, MCP client+server, desktop OS experience, plugin engine with hooks

---

### 🐾 CoPaw (AgentScope) — `agentscope-ai/CoPaw`

**Type**: Personal AI assistant framework (Python)
**Stars**: 8k+
**Core architecture**: Single agent with heartbeat system + extensible skills

**What they do best**:
- 🥇 **Multi-channel champion**: DingTalk, Feishu, QQ, Discord, iMessage — 6+ chat platforms
- ✅ **Heartbeat system**: Autonomous scheduled tasks + proactive memory management + auto-loaded skills
- ✅ **ReMe memory**: Markdown-based long-term memory with semantic search recall
- ✅ **Skill import**: Load skills from URLs, GitHub repos, or local Python files
- ✅ **Self-hosted**: Fully local deployment, no cloud required
- ✅ **Privacy-first**: All data stays on device

**What they lack**:
- ❌ No desktop OS (CLI/chat interface only)
- ❌ No sandbox or security model
- ❌ No MCP support
- ❌ No plugin hook system
- ❌ No window management, dock, launchpad
- ❌ No code editor, browser, or app catalog
- ❌ No audit trail, injection defense, or compute metering

**NDE-OS advantage**: Full desktop OS with sandbox, security stack, MCP, plugin engine, native performance, built-in apps

---

## Winner Per Category (Updated)

| Category              | Winner                            | Runner-up           | NDE-OS Status            |
| :-------------------- | :-------------------------------- | :------------------ | :----------------------- |
| Agent maturity        | 🥇 **OpenFang**                   | OpenClaw            | ✅ Loop verified          |
| Multi-agent           | 🥇 **DeerFlow** (LangGraph)      | —                   | 📋 Phase 4               |
| Tool ecosystem        | 🥇 **OpenFang** (38+MCP)          | NDE-OS (23)         | ✅ 23 + MCP done          |
| Security layers       | 🥇 **IronClaw**                   | NDE-OS              | ⭐ ✅ sandbox+env+audit   |
| Lightweight / edge    | 🥇 **ZeroClaw** (3.4MB)           | OpenFang            | 🔨 target <30MB          |
| Channels              | 🥇 **CoPaw** (6+)                 | OpenClaw (10+)      | ✅ 2 active, 6+ target   |
| Memory & knowledge    | 🥇 **DeerFlow** (LTM+checkpoint) | OpenFang            | ✅ SQLite verified        |
| Personal AI           | 🥇 **CoPaw** (heartbeat+ReMe)    | —                   | 🔨 building              |
| Skills ecosystem      | 🥇 **OpenClaw** (SKILL.md origin) | CoPaw               | ✅ Verified               |
| **Plugins**           | 🥇 **NDE-OS**                     | DeerFlow (middle)   | ⭐ ✅ manifest v2 engine  |
| **CLI**               | 🥇 **NDE-OS**                     | OpenClaw            | ⭐ ✅ `nde` binary        |
| HITL (human-in-loop)  | 🥇 **DeerFlow**                   | —                   | 📋 Phase 4               |
| **Desktop UX**        | 🥇 **NDE-OS**                     | ❌ nobody           | ⭐ ✅ done + IDE          |
| **App ecosystem**     | 🥇 **NDE-OS**                     | PocketPaw (partial) | ⭐ ✅ done                |
| **Sandbox (no WASM)** | 🥇 **NDE-OS**                     | DeerFlow (Docker)   | ⭐ ✅ done                |

---

## NDE-OS Vision: What We're Building

> **Your Personal AI Assistant with a Virtual Sandbox OS**
> macOS-style desktop · 24/7 autonomous agent · multi-modal · automation platform

### The Five Pillars

| Pillar | Description | Status |
|--------|-------------|--------|
| 🖥️ **Desktop OS** | macOS Ventura shell with dock, windows, launchpad, top bar | ⭐ ✅ Done |
| 🤖 **24/7 Agent** | Persistent agent runtime with heartbeat, task lifecycle, crash recovery | 🔨 Building |
| 🧠 **Think (Deep Work)** | Multi-step reasoning, tool chaining, research workflows | 🔨 Building |
| 🌐 **MMO (Multi-Modal Orchestration)** | Multi-agent pipelines, vision, audio, code gen | 📋 Phase 4 |
| ⚡ **Automate** | Scheduled tasks, autonomous skills, proactive assistant | 📋 Phase 4 |

### What separates NDE-OS from DeerFlow & CoPaw

| Capability | DeerFlow | CoPaw | **NDE-OS** |
|---|---|---|---|
| Desktop OS experience | ❌ web UI | ❌ CLI | ⭐ ✅ Tauri native |
| Sandbox security | Docker (heavy) | ❌ none | ⭐ OS filesystem jail (light) |
| Language | Python (slow) | Python (slow) | Rust (fast) |
| Native binary | ❌ pip install | ❌ pip install | ⭐ single binary |
| MCP ecosystem | ❌ | ❌ | ✅ client + server |
| Plugin architecture | middleware | skills | ⭐ manifest v2 + 9 hooks |
| Built-in apps | ❌ | ❌ | ⭐ 12 apps (browser, editor, etc.) |
| 24/7 task persistence | checkpoint | heartbeat | ⭐ SQLite + checkpoint + heartbeat |
| Multi-agent | 🥇 LangGraph | ❌ | 📋 Phase 4 |
| HITL | 🥇 yes | ❌ | 📋 Phase 4 |

### 🏆 The NDE-OS Unique Combination

> **App Launcher + Agent Runtime + Desktop OS + Plugin Engine + CLI + Personal AI + Sandbox = AI Operating System**
>
> DeerFlow is a research tool. CoPaw is a chat assistant. NDE-OS is an **operating system**.

| Product Type   | OpenClaw | OpenFang | DeerFlow | CoPaw | PocketPaw | **NDE-OS** |
| :------------- | :------: | :------: | :------: | :---: | :-------: | :--------: |
| App launcher   |    ❌    |    ❌    |    ❌    |  ❌   |    ✅     |     ✅     |
| Agent runtime  |    ✅    |    ✅    |    ✅    |  ✅   |    ❌     |     ✅     |
| Desktop OS     |    ❌    |    ❌    |    ❌    |  ❌   |    ❌     |     ✅     |
| Plugin engine  |    ❌    |    ❌    |  partial |  ❌   |    ✅     |     ✅     |
| CLI tool       |    ✅    |    ✅    |    ❌    |  ❌   |    ❌     |     ✅     |
| Personal AI    |    ❌    |    ❌    |    ❌    |  ✅   |    ❌     |     ✅     |
| Sandbox        |    ❌    |    ✅    |  Docker  |  ❌   |  partial  |     ✅     |
| **All seven**  |    ❌    |    ❌    |    ❌    |  ❌   |    ❌     |   **✅**   |

---

## Phase 3 Changelog (2026-03-26)

### New Competitors Added
- **DeerFlow** (ByteDance) — 35k+ stars, LangGraph multi-agent, Docker sandbox, 11-layer middleware, HITL
- **CoPaw** (AgentScope) — 8k+ stars, personal AI assistant, heartbeat system, ReMe memory, 6+ channels

### New Vision Categories
- **Multi-agent orchestration** — DeerFlow leads, NDE-OS planned for Phase 4
- **Human-in-the-loop (HITL)** — DeerFlow leads, NDE-OS planned for Phase 4
- **Personal AI** — CoPaw leads with heartbeat+ReMe, NDE-OS building

### Key Insights
1. DeerFlow validates the "multi-agent + sandbox + persistence" pattern — but relies on Python + Docker (heavy)
2. CoPaw validates the "personal AI + heartbeat + multi-channel" pattern — but has zero security
3. NDE-OS is the only one combining both patterns in a native binary with a desktop OS
