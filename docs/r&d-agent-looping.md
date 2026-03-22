# R&D: Agent Framework Comparison

> **NDE-OS vs the entire landscape** — Rust agents, TypeScript agents, Python platforms.

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
| Agent loop                   | 🥇 ✅ original |      ✅       |       ✅       |     ✅      |     ✅     |      ❌      |          🔨 building          |
| Multi-provider LLM           |       ✅       |     🥇 26     |       ✅       |  ✅ trait   |     ✅     |      ✅      |        🔨 trait-based         |
| Local LLM (Ollama)           |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |    ✅ ext    |     ⭐ ✅ **auto-launch**     |
| Native LLM (llama.cpp)       |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |   ⭐ 🔨 **GGUF in-process**   |
| Streaming                    |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |    ✅ SSE    |          🔨 building          |
| Autonomous scheduling        |       ❌       |   🥇 Hands    |       ❌       |     ✅      |     ❌     |      ❌      |          🔨 building          |
| **SANDBOX & SECURITY**       |                |               |                |             |            |              |                               |
| Sandbox type                 |  ❌ **none**   |   WASM dual   |  🥇 WASM cap   |    WASM     | WASM+Apple |  uv+tokens   |      ⭐ ✅ **OS jail**        |
| Path traversal defense       |       ❌       |     WASM      |      WASM      |    WASM     |    WASM    |      ❌      |    ⭐ ✅ **canonicalize**     |
| Symlink defense              |       ❌       |     WASM      |      WASM      |    WASM     |    WASM    |      ❌      |  ⭐ ✅ **resolve+validate**   |
| Env-var jailing              |       ❌       |      ❌       |    partial     |     ❌      |     ❌     |      ❌      |   ⭐ ✅ **12 vars jailed**    |
| Prompt injection scan        |       ❌       |      ✅       |     🥇 ✅      |     ❌      |     ❌     |      ❌      |          🔨 building          |
| Credential isolation         |  ❌ plaintext  |      ❌       |    🥇 vault    |     ❌      |     ❌     |    ✅ JWT    |      🔨 env-per-sandbox       |
| Audit trail                  |       ❌       |   🥇 Merkle   |       ❌       |     ❌      |     ❌     |      ❌      |          🔨 building          |
| Compute metering             |       ❌       |    ✅ fuel    |       ❌       |   ✅ fuel   |     ✅     |      ❌      |          🔨 building          |
| Known CVEs                   | ⚠️ **9.4+9.6** |   ✅ clean    |    ✅ clean    |  ✅ clean   |  ✅ clean  |   ✅ clean   |           ✅ clean            |
| Cross-platform               |    ✅ Node     |      ✅       |    partial     |     ✅      |  ❌ Apple  |      ✅      |      ⭐ ✅ **Win+Linux**      |
| **TOOLS & MCP**              |                |               |                |             |            |              |                               |
| Built-in tools               |      ~10       |     🥇 38     |      ~10       |     ~15     |    ~15     |      0       |       🔨 3 → target 20+      |
| MCP client                   |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |      ❌      |          🔨 building          |
| MCP server                   |       ❌       |     🥇 ✅     |       ❌       |     ❌      |     ❌     |      ❌      |          🔨 building          |
| Dynamic tools                |       ❌       |      ❌       |    ✅ WASM     |     ✅      |     ❌     |      ❌      |          🔨 building          |
| Sandbox enforcement          |       ❌       |      ✅       |       ✅       |     ✅      |     ✅     |   partial    |       ⭐ ✅ **jail**          |
| **SKILLS & KNOWLEDGE**       |                |               |                |             |            |              |                               |
| Skill format                 |  🥇 SKILL.md   |  HAND+SKILL   |       ❌       |  SKILL.md   |  SKILL.md  |   ext.json   |          🔨 building          |
| Skill marketplace            |   ✅ ClawHub   |      ✅       |       ❌       | ✅ ZeroHub  |  ✅ Store  |      ❌      |          🔨 building          |
| Knowledge graph              |       ❌       |     🥇 ✅     |       ❌       |     ❌      |     ❌     |      ❌      |          🔨 building          |
| **MEMORY**                   |                |               |                |             |            |              |                               |
| Conversation persist         |  ✅ MD files   |  🥇 cross-ch  |  ✅ encrypted  |   ✅ log    |     ✅     |   ✅ JSON    |          🔨 building          |
| Key-value store              |       ❌       |   ✅ SQLite   |       ✅       |  ✅ SQLite  |     ✅     |   ✅ JSON    |          🔨 building          |
| Vector search                |       ❌       | 🥇 sqlite-vec |       ❌       |  ✅ cosine  |     ❌     |      ❌      |          🔨 building          |
| **CHANNELS**                 |                |               |                |             |            |              |                               |
| NDE-OS desktop chat          |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |      ⭐ 🔨 **gateway**        |
| Telegram                     |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |      ❌      |          🔨 building          |
| Discord                      |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |      ❌      |           📋 Phase 2          |
| Slack                        |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |      ❌      |           📋 Phase 2          |
| WhatsApp                     |     🥇 ✅      |      ✅       |       ❌       |     ❌      |     ❌     |      ❌      |           📋 Phase 2          |
| Web chat                     |       ✅       |      ✅       |       ✅       |     ✅      |     ✅     |      ✅      |           📋 Phase 2          |
| Total channels               |     🥇 10+     |      40+      |       5        |      4      |     4      |      1       |       🔨 2 → target 6+       |
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
| Themes + wallpaper           |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |   partial    |     ⭐ ✅ **dark/light**      |
| Keyboard shortcuts           |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |    ⭐ ✅ **Tauri global**     |
| Native notifications         |       ❌       |      ❌       |       ❌       |     ❌      |     ❌     |      ❌      |    ⭐ ✅ **Tauri plugin**     |

### Score Tally

| Framework  | ✅ Done | 🔨 Building | 📋 Phase 2 | ⭐ Unique | 🥇 Best |
| :--------- | :-----: | :---------: | :---------: | :------: | :-----: |
| **NDE-OS** | **17**  |   **22**    |   **4**     | **17**   | **4**   |
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

|                       |   **OpenClaw**   |  **OpenFang**   | **IronClaw** | **ZeroClaw** | **Moltis** | **PocketPaw** |     **NDE-OS**     |
| :-------------------- | :--------------: | :-------------: | :----------: | :----------: | :--------: | :-----------: | :----------------: |
| Agent loop            | 🥇 ✅ (original) |       ✅        |      ✅      |      ✅      |     ✅     |      ❌       |         🔨         |
| Multi-provider LLM    |        ✅        | 🥇 26 providers |      ✅      |   ✅ trait   |     ✅     |      ✅       |      🔨 trait      |
| Local LLM (Ollama)    |        ✅        |       ✅        |      ✅      |      ✅      |     ✅     |   ✅ (ext)    | ⭐ **auto-launch** |
| Streaming             |        ✅        |       ✅        |      ✅      |      ✅      |     ✅     |    ✅ SSE     |     ❌ (later)     |
| Autonomous scheduling |        ❌        |    ✅ Hands     |      ❌      |      ✅      |     ❌     |      ❌       |     ❌ (later)     |

---

### Sandbox & Security

|                        |                **OpenClaw**                 |   **OpenFang**    |    **IronClaw**    |  **ZeroClaw**  |  **Moltis**   |  **PocketPaw**   |        **NDE-OS**         |
| :--------------------- | :-----------------------------------------: | :---------------: | :----------------: | :------------: | :-----------: | :--------------: | :-----------------------: |
| Sandbox type           |                 ❌ **none**                 | WASM dual-metered | 🥇 WASM capability | WASM workspace | WASM + Apple  | uv venv + tokens | ⭐ **OS filesystem jail** |
| Path traversal defense |                     ❌                      |     via WASM      |      via WASM      |    via WASM    |   via WASM    |        ❌        |    ⭐ **canonicalize**    |
| Symlink defense        |                     ❌                      |     via WASM      |      via WASM      |    via WASM    |   via WASM    |        ❌        |  ⭐ **resolve+validate**  |
| Env-var jailing        |                     ❌                      |        ❌         |      partial       |       ❌       |      ❌       |        ❌        |   ⭐ **full (12 vars)**   |
| Prompt injection scan  |                     ❌                      |        ✅         |       🥇 ✅        |       ❌       |      ❌       |        ❌        |       🔨 (planned)        |
| Credential isolation   |                ❌ plaintext!                |        ❌         | 🥇 encrypted vault |       ❌       |      ❌       |  ✅ JWT tokens   |    🔨 env-per-sandbox     |
| Audit trail            |                     ❌                      |     🥇 Merkle     |         ❌         |       ❌       |      ❌       |        ❌        |        ❌ (later)         |
| Compute metering       |                     ❌                      |   ✅ WASM fuel    |         ❌         |    ✅ fuel     |      ✅       |        ❌        |            ❌             |
| CVEs / vulns           | ⚠️ CVE-2025-49596 (9.4) CVE-2025-6514 (9.6) |     ✅ clean      |      ✅ clean      |    ✅ clean    |   ✅ clean    |     ✅ clean     |         ✅ clean          |
| Cross-platform         |                  ✅ (Node)                  |        ✅         |      partial       |       ✅       | ❌ Apple only |        ✅        |  ⭐ **Win+Linux native**  |

---

### Tools & MCP

|                     |        **OpenClaw**         | **OpenFang** | **IronClaw** | **ZeroClaw** | **Moltis** | **PocketPaw** |       **NDE-OS**       |
| :------------------ | :-------------------------: | :----------: | :----------: | :----------: | :--------: | :-----------: | :--------------------: |
| Built-in tools      | ~10 (exec, browser, search) |    🥇 38     |     ~10      |     ~15      |    ~15     |       0       |          🔨 3          |
| MCP client          |             ✅              |      ✅      |      ✅      |      ✅      |     ✅     |      ❌       |      ❌ (Phase 2)      |
| MCP server          |             ❌              |    🥇 ✅     |      ❌      |      ❌      |     ❌     |      ❌       |      ❌ (Phase 2)      |
| Dynamic tools       |             ❌              |      ❌      |   ✅ WASM    |      ✅      |     ❌     |      ❌       |       ❌ (later)       |
| Sandbox enforcement |             ❌              |      ✅      |      ✅      |      ✅      |     ✅     |    partial    | ⭐ **filesystem jail** |

---

### Skills & Knowledge

|                   |     **OpenClaw**     |     **OpenFang**     | **IronClaw** | **ZeroClaw** | **Moltis** | **PocketPaw**  |  **NDE-OS**  |
| :---------------- | :------------------: | :------------------: | :----------: | :----------: | :--------: | :------------: | :----------: |
| Skill format      | 🥇 SKILL.md (origin) | HAND.toml + SKILL.md |      ❌      |   SKILL.md   |  SKILL.md  | extension.json | ❌ (Phase 2) |
| Skill marketplace |      ✅ ClawHub      |          ✅          |      ❌      |  ✅ ZeroHub  |  ✅ Store  |       ❌       | ❌ (future)  |
| Knowledge graph   |          ❌          |        🥇 ✅         |      ❌      |      ❌      |     ❌     |       ❌       | ❌ (future)  |

---

### Memory

|                          |   **OpenClaw**    |    **OpenFang**     | **IronClaw** |  **ZeroClaw**  | **Moltis** | **PocketPaw** |  **NDE-OS**  |
| :----------------------- | :---------------: | :-----------------: | :----------: | :------------: | :--------: | :-----------: | :----------: |
| Conversation persistence | ✅ Markdown files | 🥇 ✅ cross-channel | ✅ encrypted | ✅ append-log  |     ✅     |    ✅ JSON    | ❌ (Phase 2) |
| Key-value store          |        ❌         |      ✅ SQLite      |      ✅      |   ✅ SQLite    |     ✅     |    ✅ JSON    | ❌ (Phase 2) |
| Vector search            |        ❌         |    🥇 sqlite-vec    |      ❌      | ✅ blob+cosine |     ❌     |      ❌       | ❌ (Phase 2) |
| Image/audio memory       |        ❌         |         ❌          |      ❌      |       ❌       |     ❌     |      ❌       |      ❌      |

---

### Channels

|                | **OpenClaw** | **OpenFang** | **IronClaw** | **ZeroClaw** | **Moltis** | **PocketPaw** | **NDE-OS** |
| :------------- | :----------: | :----------: | :----------: | :----------: | :--------: | :-----------: | :--------: |
| Telegram       |      ✅      |      ✅      |      ✅      |      ✅      |     ✅     |      ❌       |  🔨 first  |
| Discord        |      ✅      |      ✅      |      ✅      |      ✅      |     ✅     |      ❌       |     ❌     |
| Slack          |      ✅      |      ✅      |      ✅      |      ✅      |     ✅     |      ❌       |     ❌     |
| WhatsApp       |    🥇 ✅     |      ✅      |      ❌      |      ❌      |     ❌     |      ❌       |     ❌     |
| Web chat       |      ✅      |      ✅      |  ✅ gateway  |      ✅      |     ✅     |   ✅ iframe   |     ❌     |
| Total channels |    🥇 10+    | 40+ adapters |      5       |      4       |     4      |       1       |    🔨 1    |

---

### 🏆 Desktop OS Experience (NDE-OS Exclusive)

|                             | **OpenClaw** | **OpenFang** | **IronClaw** | **ZeroClaw** | **Moltis** | **PocketPaw** |       **NDE-OS**       |
| :-------------------------- | :----------: | :----------: | :----------: | :----------: | :--------: | :-----------: | :--------------------: |
| **macOS-style desktop**     |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       |    ⭐ Tauri+Svelte     |
| **Window management**       |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       | ⭐ drag/resize/min/max |
| **Animated dock**           |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       |     ⭐ macOS-style     |
| **Top bar + clock**         |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       |     ⭐ system bar      |
| **Launchpad (app grid)**    |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       |      ⭐ iOS-style      |
| **App catalog + installer** |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     | ✅ ext store  |     ⭐ manifest+uv     |
| **Apps = agent tools**      |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       |    ⭐ auto-discover    |
| **1-click local AI**        |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |    partial    |    ⭐ Ollama→agent     |
| **Per-app sandbox venv**    |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |     ✅ uv     |       ⭐ uv+jail       |
| **Built-in browser**        |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       |      ⭐ embedded       |
| **Themes + wallpaper**      |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |  partial CSS  |     ⭐ dark/light      |
| **Keyboard shortcuts**      |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       |    ⭐ Tauri global     |
| **Native notifications**    |      ❌      |      ❌      |      ❌      |      ❌      |     ❌     |      ❌       |    ⭐ Tauri plugin     |

---

## Winner Per Category

| Category              | Winner                            | Runner-up           | NDE-OS Status            |
| :-------------------- | :-------------------------------- | :------------------ | :----------------------- |
| Agent maturity        | 🥇 **OpenFang**                   | OpenClaw            | 🔨 Building              |
| Tool ecosystem        | 🥇 **OpenFang** (38+MCP)          | ZeroClaw            | 🔨 3 built, 20+ target  |
| Security layers       | 🥇 **IronClaw**                   | OpenFang            | ⭐ ✅ sandbox+env done   |
| Lightweight / edge    | 🥇 **ZeroClaw** (3.4MB)           | OpenCrust           | 🔨 target <30MB          |
| Channels              | 🥇 **OpenClaw** (10+)             | OpenFang (40)       | 🔨 2 now, 6+ target     |
| Memory & knowledge    | 🥇 **OpenFang**                   | ZeroClaw            | 🔨 building             |
| Skills ecosystem      | 🥇 **OpenClaw** (SKILL.md origin) | OpenFang            | 🔨 building             |
| Python accessibility  | 🥇 **PocketPaw**                  | OpenClaw (Node)     | N/A (Rust)               |
| **Desktop UX**        | 🥇 **NDE-OS**                     | ❌ nobody           | ⭐ ✅ done               |
| **App ecosystem**     | 🥇 **NDE-OS**                     | PocketPaw (partial) | ⭐ ✅ done               |
| **Apps-as-tools**     | 🥇 **NDE-OS**                     | ❌ nobody           | ⭐ 🔨 building           |
| **Sandbox (no WASM)** | 🥇 **NDE-OS**                     | Carapace (hybrid)   | ⭐ ✅ done               |

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

> **App launcher + Agent runtime + Desktop OS = AI Operating System**
>
> No competitor combines all three. They build runtimes. We build an OS.

| Product Type  | OpenClaw | OpenFang | ZeroClaw | PocketPaw | **NDE-OS** |
| :------------ | :------: | :------: | :------: | :-------: | :--------: |
| App launcher  |    ❌    |    ❌    |    ❌    |    ✅     |     ✅     |
| Agent runtime |    ✅    |    ✅    |    ✅    |    ❌     |     🔨     |
| Desktop OS    |    ❌    |    ❌    |    ❌    |    ❌     |     ✅     |
| **All three** |    ❌    |    ❌    |    ❌    |    ❌     |  **🔨→✅** |
