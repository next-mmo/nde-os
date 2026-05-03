# NDE-OS AI Launcher Architecture (Compact)

## System Overview
NDE-OS is a cross-platform (Windows, Linux, macOS) web-desktop application and autonomous AI agent platform. 
It uses a Rust workspace allowing `core/` to power both a headless `server/` and a `desktop/` Tauri 2 application.

## Core Engine Structure
1. `core/app_manager/`: App lifecycle (install, launch, stop, uninstall).
2. `core/sandbox/`: Filesystem jail defending against path traversal/symloops.
3. `core/uv_env/`: Bootstraps `.venv` using `uv` allowing rapid sandboxed dependencies.

## Agent Runtime & Channels
- **State Machine**: Autonomous agent loop handling Thinking, Acting, and Observing.
- **Channels**: Connected via REST/Tauri-IPC. Includes Desktop Chat and Telegram bot (`teloxide`).
- **Memory**: Unified `MemorySubstrate` handling SQLite-backed structured (KV), semantic (vector embeddings), and knowledge graph stores. Supports cross-channel canonical sessions, automatic LLM-based compaction, and fast MsgPack serialization.
- **Tools**: Includes `file_read`, `file_write`, `shell_exec`. Connects to external MCP clients/servers.
- **Skills**: Discovers `.md` skill files providing extra workflows.
- **Security**: Prompt injection scanning, cryptographically secure audit trails, and strict sandbox constraints.

## LLM Providers Layer
Agents communicate directly with:
- **Llama Native**: Direct GGUF loading using the `llama_cpp` crate (no separate processes).
- **Ollama**: Automatically launched instance detection.
- **OpenAI-compat**: Supports OpenAI, Groq, Together APIs via SSE.
- **Codex OAuth**: Detects active `codex login` sessions via `~/.codex/auth.json` to bypass manual browser OAuth.

## Frontend (Desktop)
- **Framework**: Tauri 2 bridge + SvelteKit 5 + Tailwind v4 + shadcn-svelte.
- **Integration**: Commands route via `invoke()` directly into `core` modules, replacing old tiny_http endpoints.
