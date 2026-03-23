# R&D: Agent Framework Comparison

> **NDE-OS vs the entire landscape** — Rust agents, TypeScript agents, Python platforms.
> 
> *Last updated: 2026-03-23 — Phase 2 complete*

## Legend

✅ Done · 🔨 Building · 📋 Planned · ❌ Missing · ⭐ NDE-OS unique · 🥇 Best in class

---

## All Features at a Glance

| Feature                      |    OpenClaw    |   OpenFang    |    IronClaw    |  ZeroClaw   |   Moltis   |  PocketPaw   |          **NDE-OS**           |
| :--------------------------- | :------------: | :-----------: | :------------: | :---------: | :--------: | :----------: | :---------------------------: |
| **IDENTITY**                 |                |               |                |             |            |              |                               |
| Language                     |   TypeScript   |     Rust      |      Rust      |    Rust     |    Rust    |    Python    |           **Rust**            |
| Type                         |  AI assistant  |   Agent OS    | Security agent | Lightweight |  Runtime   | Ext platform |       **AI Desktop OS**       |
| Binary size                  | ~200 MB (Node) |     32 MB     |     ~20 MB     |  🥇 3.4 MB  |   44 MB    | ~50 MB (Py)  |        🔨 target <30MB        |
| RAM usage                    |    ~300 MB     |    ~50 MB     |     ~80 MB     |  🥇 <5 MB   |   ~60 MB   |   ~120 MB    |        🔨 target <50MB        |
| Boot time                    |      ~2s       |    <200ms     |       —        |  🥇 <10ms   |     —      |     ~3s      |       🔨 target <500ms        |
| **AGENT RUNTIME**            |                |               |                |             |            |              |                               |
| Agent loop                   | 🥇 ✅ original |      ✅       |       ✅       |     ✅      |     ✅     |      ❌      |        ✅ **verified**         |
| Multi-provider LLM           |       ✅       |     🥇 26     |       ✅       |  ✅ trait   |     ✅     |      ✅      | ⭐ ✅ **6 providers (tested)** |
| Local LLM (Ollama)           |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |    ✅ ext    |     ⭐ ✅ **auto-launch**     |
| Native LLM (llama.cpp)       |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |   ⭐ 🔨 **GGUF in-process**   |
| Streaming                    |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |    ✅ SSE    |     ✅ **SSE (word-level)**    |
| LLM hot-swap                 |       ❌       |     partial   |       ❌       |     ❌      |     ❌     |      ❌      | ⭐ ✅ **runtime switch**       |
| Autonomous scheduling        |       ❌       |   🥇 Hands    |       ❌       |     ✅      |     ❌     |      ❌      |          🔨 building          |
| **SANDBOX & SECURITY**       |                |               |                |             |            |              |                               |
| Sandbox type                 |  ❌ **none**   |   WASM dual   |  🥇 WASM cap   |    WASM     | WASM+Apple |  uv+tokens   |      ⭐ ✅ **OS jail**        |
| Path traversal defense       |       ❌       |     WASM      |      WASM      |    WASM     |    WASM    |      ❌      |    ⭐ ✅ **canonicalize**     |
| Symlink defense              |       ❌       |     WASM      |      WASM      |    WASM     |    WASM    |      ❌      |  ⭐ ✅ **resolve+validate**   |
| Env-var jailing              |       ❌       |      ❌       |    partial     |     ❌      |     ❌     |      ❌      |   ⭐ ✅ **12 vars jailed**    |
| Prompt injection scan        |       ❌       |      ✅       |     🥇 ✅      |     ❌      |     ❌     |      ❌      |    ✅ **verified (28 tests)** |
| Credential isolation         |  ❌ plaintext  |      ❌       |    🥇 vault    |     ❌      |     ❌     |    ✅ JWT    |      ✅ env-per-sandbox       |
| Audit trail                  |       ❌       |   🥇 Merkle   |       ❌       |     ❌      |     ❌     |      ❌      |    ✅ **SHA-256 hash chain**  |
| Compute metering             |       ❌       |    ✅ fuel    |       ❌       |   ✅ fuel   |     ✅     |      ❌      |       ✅ **token+tool+time** |
| Known CVEs                   | ⚠️ **9.4+9.6** |   ✅ clean    |    ✅ clean    |  ✅ clean   |  ✅ clean  |   ✅ clean   |           ✅ clean            |
| Cross-platform               |    ✅ Node     |      ✅       |    partial     |     ✅      |  ❌ Apple  |      ✅      |      ⭐ ✅ **Win+Linux**      |
| **TOOLS & MCP**              |                |               |                |             |            |              |                               |
| Built-in tools               |      ~10       |     🥇 38     |      ~10       |     ~15     |    ~15     |      0       | ✅ **6 (sandbox-jailed)**      |
| MCP client                   |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |      ❌      |   ✅ **stdio JSON-RPC**        |
| MCP server                   |       ❌       |     🥇 ✅     |       ❌       |     ❌      |     ❌     |      ❌      |   ✅ **expose tools**          |
| Dynamic tools (plugins)      |       ❌       |      ❌       |    ✅ WASM     |     ✅      |     ❌     |      ❌      | ⭐ ✅ **plugin engine**        |
| Sandbox enforcement          |       ❌       |      ✅       |       ✅       |     ✅      |     ✅     |   partial    |       ⭐ ✅ **jail**          |
| **PLUGINS**                  |                |               |                |             |            |              |                               |
| Plugin engine                |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |    ✅ ext    | ⭐ ✅ **manifest v2**          |
| Plugin lifecycle             |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |   partial    | ⭐ ✅ **discover→install→run** |
| Hook system                  |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      | ⭐ ✅ **9 hook types**         |
| Plugin-contributed tools     |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      | ⭐ ✅ **auto-merge**           |
| **SKILLS & KNOWLEDGE**       |                |               |                |             |            |              |                               |
| Skill format                 |  🥇 SKILL.md   |  HAND+SKILL   |       ❌       |  SKILL.md   |  SKILL.md  |   ext.json   |   ✅ **SKILL.md (verified)**   |
| Skill marketplace            |   ✅ ClawHub   |      ✅       |       ❌       | ✅ ZeroHub  |  ✅ Store  |      ❌      |          🔨 building          |
| Knowledge graph              |       ❌       |     🥇 ✅     |       ❌       |     ❌      |     ❌     |      ❌      |   ✅ **SQLite (verified)**     |
| **MEMORY**                   |                |               |                |             |            |              |                               |
| Conversation persist         |  ✅ MD files   |  🥇 cross-ch  |  ✅ encrypted  |   ✅ log    |     ✅     |   ✅ JSON    |   ✅ **SQLite (verified)**     |
| Key-value store              |       ❌       |   ✅ SQLite   |       ✅       |  ✅ SQLite  |     ✅     |   ✅ JSON    |   ✅ **SQLite (verified)**     |
| Vector search                |       ❌       | 🥇 sqlite-vec |       ❌       |  ✅ cosine  |     ❌     |      ❌      |          🔨 building          |
| **CHANNELS**                 |                |               |                |             |            |              |                               |
| NDE-OS desktop chat          |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      | ⭐ ✅ **REST+UI (integrated)** |
| Telegram                     |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |      ❌      |    ✅ **long-poll gateway**    |
| Discord                      |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |      ❌      |           📋 Phase 3          |
| Slack                        |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |      ❌      |           📋 Phase 3          |
| WhatsApp                     |     🥇 ✅      |      ✅       |       ❌       |     ❌      |     ❌     |      ❌      |           📋 Phase 3          |
| Web chat                     |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |      ✅      |           📋 Phase 3          |
| Total channels               |     🥇 10+     |      40+      |       5        |      4      |     4      |      1       |    ✅ **2 (REST+Telegram)**    |
| **CLI**                      |                |               |                |             |            |              |                               |
| CLI binary                   |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |      ❌      | ⭐ ✅ **`nde` command**        |
| Interactive REPL             |       ✅       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      | ⭐ ✅ **streaming REPL**       |
| App management               |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      | ⭐ ✅ **list/install/launch**  |
| MCP server mode              |       ❌       |      ✅       |       ❌       |     ❌      |     ❌     |      ❌      | ⭐ ✅ **`nde mcp`**            |
| **DESKTOP OS (NDE-OS ONLY)** |                |               |                |             |            |              |                               |
| macOS-style desktop          |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |    ⭐ ✅ **Tauri+Svelte**     |
| Window management            |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |  ⭐ ✅ **drag/resize/min**    |
| Animated dock                |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |    ⭐ ✅ **macOS-style**      |
| Top bar + clock              |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |     ⭐ ✅ **system bar**      |
| Launchpad (app grid)         |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |     ⭐ ✅ **iOS-style**       |
| App catalog + installer      |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     | ✅ ext store |    ⭐ ✅ **manifest+uv**      |
| Apps = agent tools           |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |    ⭐ 🔨 **auto-discover**    |
| 1-click local AI stack       |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |   partial    |    ⭐ 🔨 **Ollama→agent**     |
| Per-app sandbox venv         |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |    ✅ uv     |      ⭐ ✅ **uv+jail**        |
| Built-in browser             |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |      ⭐ ✅ **embedded**       |
| Built-in code editor (IDE)   |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      | ⭐ ✅ **editor+terminal+AI**  |
| Themes + wallpaper           |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |   partial    |     ⭐ ✅ **dark/light**      |
| Keyboard shortcuts           |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |    ⭐ ✅ **Tauri global**     |
| Native notifications         |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |    ⭐ ✅ **Tauri plugin**     |

### Score Tally

| Framework  | ✅ Done | 🔨 Building | 📋 Phase 3 | ⭐ Unique | 🥇 Best |
| :--------- | :-----: | :---------: | :---------: | :------: | :-----: |
| **NDE-OS** | **48**  |    **4**    |   **4**     | **26**   | **4**   |
| OpenFang   |   22    |      0      |      0      |    0     |    8    |
| OpenClaw   |   18    |      0      |      0      |    0     |    4    |
| IronClaw   |   15    |      0      |      0      |    0     |    3    |
| ZeroClaw   |   17    |      0      |      0      |    0     |    3    |
| PocketPaw  |    8    |      0      |      0      |    0     |    1    |

---

## Master Comparison Table

### Identity

|                  |  **OpenClaw**  | **OpenFang** |     **IronClaw**     |   **ZeroClaw**    |  **Moltis**   |   **PocketPaw**    |    **NDE-OS**     |
| :--------------- | :------------: | :----------: | :------------------: | :---------------: | :-----------: | :----------------: | :---------------: |
| Language         |   TypeScript   |     Rust     |         Rust         |       Rust        |     Rust      |       Python       |     **Rust**      |
| Type             |  AI assistant  |   Agent OS   | Security-first agent | Lightweight agent | Agent runtime | Extension platform | **AI Desktop OS** |
| Binary / Install | Node.js ~200MB |    32 MB     |        ~20 MB        |     🥇 3.4 MB     |     44 MB     |    Python ~50MB    |        TBD        |
| RAM usage        |    ~300 MB     |    ~50 MB    |        ~80 MB        |     🥇 <5 MB      |    ~60 MB     |      ~120 MB       |        TBD        |
| Boot time        |      ~2s       |    <200ms    |       unknown        |     🥇 <10ms      |    unknown    |        ~3s         |        TBD        |

---

### Agent Runtime

|                       |   **OpenClaw**   |  **OpenFang**   | **IronClaw** | **ZeroClaw** | **Moltis** | **PocketPaw** |       **NDE-OS**        |
| :-------------------- | :--------------: | :-------------: | :----------: | :----------: | :--------: | :-----------: | :---------------------: |
| Agent loop            | 🥇 ✅ (original) |       ✅        |      ✅      |      ✅      |     ✅     |      ❌       |     ✅ **verified**      |
| Multi-provider LLM    |        ✅        | 🥇 26 providers |      ✅      |   ✅ trait   |     ✅     |      ✅       | ⭐ ✅ **6 + hot-swap**   |
| Local LLM (Ollama)    |        ✅        |       ✅        |      ✅      |      ✅      |     ✅     |   ✅ (ext)    | ⭐ ✅ **auto-launch**    |
| Streaming             |        ✅        |       ✅        |      ✅      |      ✅      |     ✅     |    ✅ SSE     |    ✅ **SSE streaming**  |
| LLM hot-swap          |        ❌        |     partial     |      ❌      |      ❌      |     ❌     |      ❌       | ⭐ ✅ **runtime switch** |
| Autonomous scheduling |        ❌        |    ✅ Hands     |      ❌      |      ✅      |     ❌     |      ❌       |       🔨 Phase 3        |

---

### Sandbox & Security

|                        |                **OpenClaw**                 |   **OpenFang**    |    **IronClaw**    |  **ZeroClaw**  |  **Moltis**   |  **PocketPaw**   |        **NDE-OS**         |
| :--------------------- | :-----------------------------------------: | :---------------: | :----------------: | :------------: | :-----------: | :--------------: | :-----------------------: |
| Sandbox type           |                 ❌ **none**                 | WASM dual-metered | 🥇 WASM capability | WASM workspace | WASM + Apple  | uv venv + tokens | ⭐ **OS filesystem jail** |
| Path traversal defense |                     ❌                      |     via WASM      |      via WASM      |    via WASM    |   via WASM    |        ❌        |    ⭐ ✅ **canonicalize** |
| Symlink defense        |                     ❌                      |     via WASM      |      via WASM      |    via WASM    |   via WASM    |        ❌        |  ⭐ ✅ **resolve+validate**|
| Env-var jailing        |                     ❌                      |        ❌         |      partial       |       ❌       |      ❌       |        ❌        |   ⭐ ✅ **full (12 vars)**|
| Prompt injection scan  |                     ❌                      |        ✅         |       🥇 ✅        |       ❌       |      ❌       |        ❌        |  ✅ **verified (28 tests)**|
| Credential isolation   |                ❌ plaintext!                |        ❌         | 🥇 encrypted vault |       ❌       |      ❌       |  ✅ JWT tokens   |    ✅ env-per-sandbox     |
| Audit trail            |                     ❌                      |     🥇 Merkle     |         ❌         |       ❌       |      ❌       |        ❌        |   ✅ **SHA-256 chain**    |
| Compute metering       |                     ❌                      |   ✅ WASM fuel    |         ❌         |    ✅ fuel     |      ✅       |        ❌        |   ✅ **token+tool+time**  |
| CVEs / vulns           | ⚠️ CVE-2025-49596 (9.4) CVE-2025-6514 (9.6) |     ✅ clean      |      ✅ clean      |    ✅ clean    |   ✅ clean    |     ✅ clean     |         ✅ clean          |
| Cross-platform         |                  ✅ (Node)                  |        ✅         |      partial       |       ✅       | ❌ Apple only |        ✅        |  ⭐ ✅ **Win+Linux native**|

---

### Tools & MCP

|                     |        **OpenClaw**         | **OpenFang** | **IronClaw** | **ZeroClaw** | **Moltis** | **PocketPaw** |       **NDE-OS**       |
| :------------------ | :-------------------------: | :----------: | :----------: | :----------: | :--------: | :-----------: | :--------------------: |
| Built-in tools      | ~10 (exec, browser, search) |    🥇 38     |     ~10      |     ~15      |    ~15     |       0       |   ✅ **6 (jailed)**    |
| MCP client          |             ✅              |      ✅      |      ✅      |      ✅      |     ✅     |      ❌       |  ✅ **stdio JSON-RPC** |
| MCP server          |             ❌              |    🥇 ✅     |      ❌      |      ❌      |     ❌     |      ❌       |  ✅ **expose tools**   |
| Dynamic tools       |             ❌              |      ❌      |   ✅ WASM    |      ✅      |     ❌     |      ❌       | ⭐ ✅ **plugin engine** |
| Sandbox enforcement |             ❌              |      ✅      |      ✅      |      ✅      |     ✅     |    partial    | ⭐ ✅ **filesystem jail**|

---

### Plugins (Phase 2 — NEW)

|                        | **OpenClaw** | **OpenFang** | **IronClaw** | **ZeroClaw** | **Moltis** | **PocketPaw** |         **NDE-OS**         |
| :--------------------- | :----------: | :----------: | :----------: | :----------: | :--------: | :-----------: | :------------------------: |
| Plugin engine          |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |    ✅ ext     |  ⭐ ✅ **manifest v2**      |
| Plugin lifecycle       |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |   partial     | ⭐ ✅ **discover→run**      |
| Hook system            |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       |  ⭐ ✅ **9 hook types**     |
| Plugin-contributed tools|      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       | ⭐ ✅ **auto-merge**        |
| Plugin types           |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      2        | ⭐ ✅ **6** (Monitor/Hook/Provider/Tool/UiPanel/Daemon) |
| Plugin API (REST)      |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       | ⭐ ✅ **CRUD + lifecycle**  |

---

### Skills & Knowledge

|                   |     **OpenClaw**     |     **OpenFang**     | **IronClaw** | **ZeroClaw** | **Moltis** | **PocketPaw**  |       **NDE-OS**       |
| :---------------- | :------------------: | :------------------: | :----------: | :----------: | :--------: | :------------: | :--------------------: |
| Skill format      | 🥇 SKILL.md (origin) | HAND.toml + SKILL.md |      ❌      |   SKILL.md   |  SKILL.md  | extension.json | ✅ **SKILL.md verified** |
| Skill marketplace |      ✅ ClawHub      |          ✅          |      ❌      |  ✅ ZeroHub  |  ✅ Store  |       ❌       |       🔨 building      |
| Knowledge graph   |          ❌          |        🥇 ✅         |      ❌      |      ❌      |     ❌     |       ❌       | ✅ **SQLite (verified)** |

---

### Memory

|                          |   **OpenClaw**    |    **OpenFang**     | **IronClaw** |  **ZeroClaw**  | **Moltis** | **PocketPaw** |       **NDE-OS**       |
| :----------------------- | :---------------: | :-----------------: | :----------: | :------------: | :--------: | :-----------: | :--------------------: |
| Conversation persistence | ✅ Markdown files | 🥇 ✅ cross-channel | ✅ encrypted | ✅ append-log  |     ✅     |    ✅ JSON    | ✅ **SQLite (verified)** |
| Key-value store          |        ❌         |      ✅ SQLite      |      ✅      |   ✅ SQLite    |     ✅     |    ✅ JSON    | ✅ **SQLite (verified)** |
| Vector search            |        ❌         |    🥇 sqlite-vec    |      ❌      | ✅ blob+cosine |     ❌     |      ❌       |       🔨 building      |
| Image/audio memory       |        ❌         |         ❌          |      ❌      |       ❌       |     ❌     |      ❌       |           ❌           |

---

### Channels

|                | **OpenClaw** | **OpenFang** | **IronClaw** | **ZeroClaw** | **Moltis** | **PocketPaw** |       **NDE-OS**        |
| :------------- | :----------: | :----------: | :----------: | :----------: | :--------: | :-----------: | :---------------------: |
| Desktop chat   |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       | ⭐ ✅ **REST+UI**        |
| Telegram       |      ✅      |      ✅      |      ✅      |      ✅      |     ✅     |      ❌       |  ✅ **long-poll gateway** |
| Discord        |      ✅      |      ✅      |      ✅      |      ✅      |     ✅     |      ❌       |       📋 Phase 3        |
| Slack          |      ✅      |      ✅      |      ✅      |      ✅      |     ✅     |      ❌       |       📋 Phase 3        |
| WhatsApp       |    🥇 ✅     |      ✅      |      ❌      |      ❌      |     ❌     |      ❌       |       📋 Phase 3        |
| Web chat       |      ✅      |      ✅      |  ✅ gateway  |      ✅      |     ✅     |   ✅ iframe   |       📋 Phase 3        |
| Total channels |    🥇 10+    | 40+ adapters |      5       |      4       |     4      |       1       |      ✅ **2 active**     |

---

### CLI (Phase 2 — NEW)

|                  | **OpenClaw** | **OpenFang** | **IronClaw** | **ZeroClaw** | **Moltis** | **PocketPaw** |       **NDE-OS**        |
| :--------------- | :----------: | :----------: | :----------: | :----------: | :--------: | :-----------: | :---------------------: |
| CLI binary       |      ✅      |      ✅      |      ✅      |      ✅      |     ✅     |      ❌       | ⭐ ✅ **`nde` (clap)**   |
| Interactive REPL |      ✅      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       | ⭐ ✅ **streaming REPL** |
| App management   |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       | ⭐ ✅ **CRUD + launch**  |
| Plugin management|      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       | ⭐ ✅ **install/start**  |
| Model switching  |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       | ⭐ ✅ **hot-swap**       |
| Status dashboard |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       | ⭐ ✅ **colored bars**   |
| MCP server mode  |      ❌      |      ✅      |      ❌      |      ❌      |     ❌     |      ❌       | ⭐ ✅ **`nde mcp`**      |

---

### 🏆 Desktop OS Experience (NDE-OS Exclusive)

|                             | **OpenClaw** | **OpenFang** | **IronClaw** | **ZeroClaw** | **Moltis** | **PocketPaw** |        **NDE-OS**         |
| :-------------------------- | :----------: | :----------: | :----------: | :----------: | :--------: | :-----------: | :-----------------------: |
| **macOS-style desktop**     |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       |    ⭐ ✅ Tauri+Svelte     |
| **Window management**       |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       | ⭐ ✅ drag/resize/min/max |
| **Animated dock**           |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       |     ⭐ ✅ macOS-style     |
| **Top bar + clock**         |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       |     ⭐ ✅ system bar      |
| **Launchpad (app grid)**    |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       |      ⭐ ✅ iOS-style      |
| **App catalog + installer** |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     | ✅ ext store  |     ⭐ ✅ manifest+uv     |
| **Apps = agent tools**      |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       |    ⭐ 🔨 auto-discover    |
| **1-click local AI**        |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |    partial    |    ⭐ 🔨 Ollama→agent     |
| **Per-app sandbox venv**    |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |     ✅ uv     |       ⭐ ✅ uv+jail       |
| **Built-in browser**        |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       |      ⭐ ✅ embedded       |
| **Built-in Code Editor**    |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       | ⭐ ✅ **editor+term+AI**  |
| **Themes + wallpaper**      |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |  partial CSS  |     ⭐ ✅ dark/light      |
| **Keyboard shortcuts**      |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       |    ⭐ ✅ Tauri global     |
| **Native notifications**    |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       |    ⭐ ✅ Tauri plugin     |

---

## Winner Per Category

| Category              | Winner                            | Runner-up           | NDE-OS Status            |
| :-------------------- | :-------------------------------- | :------------------ | :----------------------- |
| Agent maturity        | 🥇 **OpenFang**                   | OpenClaw            | ✅ Loop verified          |
| Tool ecosystem        | 🥇 **OpenFang** (38+MCP)          | ZeroClaw            | ✅ 6 + MCP done           |
| Security layers       | 🥇 **IronClaw**                   | OpenFang            | ⭐ ✅ sandbox+env+audit   |
| Lightweight / edge    | 🥇 **ZeroClaw** (3.4MB)           | OpenCrust           | 🔨 target <30MB          |
| Channels              | 🥇 **OpenClaw** (10+)             | OpenFang (40)       | ✅ 2 active, 6+ target   |
| Memory & knowledge    | 🥇 **OpenFang**                   | ZeroClaw            | ✅ SQLite verified        |
| Skills ecosystem      | 🥇 **OpenClaw** (SKILL.md origin) | OpenFang            | ✅ Verified               |
| **Plugins**           | 🥇 **NDE-OS**                     | PocketPaw (partial) | ⭐ ✅ manifest v2 engine  |
| **CLI**               | 🥇 **NDE-OS**                     | OpenClaw            | ⭐ ✅ `nde` binary        |
| Python accessibility  | 🥇 **PocketPaw**                  | OpenClaw (Node)     | N/A (Rust)               |
| **Desktop UX**        | 🥇 **NDE-OS**                     | ❌ nobody           | ⭐ ✅ done + IDE          |
| **App ecosystem**     | 🥇 **NDE-OS**                     | PocketPaw (partial) | ⭐ ✅ done                |
| **Apps-as-tools**     | 🥇 **NDE-OS**                     | ❌ nobody           | ⭐ 🔨 building           |
| **Sandbox (no WASM)** | 🥇 **NDE-OS**                     | Carapace (hybrid)   | ⭐ ✅ done                |

---

## Phase 2 Changelog (2026-03-22 → 2026-03-23)

### New Crates
- **`cli/`** — `nde` CLI binary (clap, colored, futures, reqwest)

### New Core Modules
| Module | Files | Description |
|--------|-------|-------------|
| `core/llm/streaming.rs` | SSE types | `ChunkStream`, `StreamAccumulator`, word-level token delivery |
| `core/llm/anthropic.rs` | Provider | Anthropic Claude API with streaming + native tool use |
| `core/llm/manager.rs` | Hot-swap | `LlmManager` with runtime provider switching, env-var key resolution |
| `core/channels/` | 4 files | `Channel` trait, gateway normalization, channel manager, Telegram bot |
| `core/plugins/` | 4 files | Manifest v2 schema, 9 hook types, plugin engine with lifecycle |
| `core/mcp/` | 2 files | MCP client (stdio JSON-RPC) + MCP server (expose tools) |
| `core/tools/builtin/code_tools.rs` | 3 tools | `code_search`, `code_edit`, `code_symbols` |

### New Server Handlers
| Handler | Endpoints |
|---------|-----------|
| `stream_handler.rs` | `POST /api/agent/chat/stream` (SSE) |
| `plugin_handler.rs` | `GET/POST /api/plugins/*` (6 endpoints) |
| `model_handler.rs` | `GET/POST /api/models/*` (3 endpoints) |

### New Desktop Components
| Component | Description |
|-----------|-------------|
| `CodeEditor.svelte` | VS Code-style IDE with file explorer, editor, terminal, AI assist, status bar |

### Build
```
cargo check --workspace  →  ✅ 4 crates compile (core, server, cli, desktop)
cargo test -p ai-launcher-core  →  ✅ All tests pass
```

---

## Verdict

### They win today

- **OpenFang** — most complete agent OS (38 tools, knowledge graph, 26 LLM providers, Merkle audit)
- **OpenClaw** — most popular, most channels (10+), created SKILL.md standard, biggest community
- **IronClaw** — strongest security (encrypted vault, WASM capabilities, prompt injection defense)
- **ZeroClaw** — smallest footprint ever (3.4 MB, <5 MB RAM, <10ms boot)
- **PocketPaw** — easiest for Python developers, extension platform

### ⚠️ OpenClaw's fatal flaw

OpenClaw has **critical CVEs** (9.4+9.6 CVSS), no sandbox, plaintext credential storage, and default-open network ports. China issued national security guidelines. This is the gap that Rust-based alternatives (and NDE-OS) fill.

### 🏆 Only NDE-OS does this

> **App launcher + Agent runtime + Desktop OS + Plugin engine + CLI = AI Operating System**
>
> No competitor combines all five. They build runtimes. We build an OS.

| Product Type   | OpenClaw | OpenFang | ZeroClaw | PocketPaw | **NDE-OS** |
| :------------- | :------: | :------: | :------: | :-------: | :--------: |
| App launcher   |    ❌    |    ❌    |    ❌    |    ✅     |     ✅     |
| Agent runtime  |    ✅    |    ✅    |    ✅    |    ❌     |     ✅     |
| Desktop OS     |    ❌    |    ❌    |    ❌    |    ❌     |     ✅     |
| Plugin engine  |    ❌    |    ❌    |    ❌    |    ✅     |     ✅     |
| CLI tool       |    ✅    |    ✅    |    ✅    |    ❌     |     ✅     |
| **All five**   |    ❌    |    ❌    |    ❌    |    ❌     |   **✅**   |
