<p align="center">
  <em>🤖 Built with ❤️ by <strong>Opus 4.6</strong> · <strong>Codex 4.2</strong> · <strong>Gemini Pro 3.1</strong></em>
</p>

---

# 🖥️ NDE-OS

**A sandboxed OS environment for AI applications — in your browser.**

NDE-OS combines a **macOS-style web desktop** with a **Rust-powered sandbox backend** to give users a familiar, secure environment for installing and running AI apps. Each app lives in its own filesystem jail with an isolated Python virtual environment managed by [uv](https://docs.astral.sh/uv/).

> **Cross-platform. Mac, Linux + Windows native. No WSL required.**

---

## ✨ Features

- 🖥️ **Web Desktop** — macOS Ventura–inspired desktop UI (Svelte + TypeScript)
- 🔒 **Filesystem Sandbox** — per-app jailed workspaces with path traversal, symlink, and env-var defense
- ⚡ **uv-powered Environments** — 10–100× faster than pip, one venv per app, auto-bootstrapped
- 📦 **App Catalog** — install AI apps (Stable Diffusion, Ollama, ComfyUI, Gradio…) from manifests
- 🔌 **Plugin System** — extend with monitors, hooks, providers, middleware, and UI panels
- 🌐 **REST API + Swagger** — full OpenAPI 3.0.3 spec, headless-friendly
- 🧩 **Modular Rust Core** — reusable across desktop (Tauri), server, CLI frontends

---

## Quick Start

### Prerequisites

- **Rust 1.70+** — [Install Rust](https://rustup.rs)
- **Node.js 18+** — for the web desktop frontend
- **pnpm** — `npm i -g pnpm`

### Clone & Run

```bash
git clone https://github.com/next-mmo/nde-os.git
cd nde-os
```

#### Start the backend (API server)

```bash
# Linux
chmod +x run.sh && ./run.sh

# Windows
.\run.bat
```

#### Start the desktop frontend

```bash
cd desktop
npm install
pnpm dev
```

### Endpoints

| URL                                           | Description          |
| --------------------------------------------- | -------------------- |
| `http://localhost:5174`                       | Desktop UI           |
| `http://localhost:8080/swagger-ui/`           | Swagger API Explorer |
| `http://localhost:8080/api-docs/openapi.json` | OpenAPI 3.0.3 Spec   |

---

## Architecture

```
nde-os/
├── src/                        # Rust backend (sandbox + app manager)
│   ├── main.rs                 # REST API server (tiny_http) + Swagger UI
│   ├── sandbox/mod.rs          # Filesystem jail — path validation, symlink defense
│   ├── uv_env/mod.rs           # uv bootstrap, venv creation, pip deps
│   ├── app_manager/mod.rs      # App lifecycle (install → launch → stop → uninstall)
│   └── manifest/mod.rs         # App manifest types + built-in catalog
│
├── desktop/                    # Desktop frontend (Svelte 5 + Vite + Tauri)
│   ├── src/
│   │   ├── components/         # Desktop shell — dock, top bar, windows, apps
│   │   ├── state/              # Svelte 5 desktop/session state
│   │   └── configs/            # App configs, themes, wallpapers
│   ├── src-tauri/              # Tauri commands and desktop bridge
│   └── public/                 # Static assets (app icons, wallpapers)
│
├── apps/                       # App manifest repository
│   ├── gradio-demo/manifest.json
│   ├── stable-diffusion/manifest.json
│   └── ...
│
├── plugins/                    # Plugin extensions
│   ├── gpu-monitor/plugin.json
│   ├── model-downloader/plugin.json
│   └── ...
│
└── docs/
    ├── current-plan.md         # Desktop migration roadmap (Tauri 2)
    └── future-plan.md          # Agent OS & licensing roadmap
```

---

## Sandbox Security

Every installed app runs inside a **jailed workspace** with strict security enforcement:

| Attack Vector                          | Defense                              | Result     |
| -------------------------------------- | ------------------------------------ | ---------- |
| Path traversal (`../../../etc/passwd`) | Canonicalize + containment check     | ✅ Blocked |
| Absolute escape (`/etc/shadow`)        | Reject paths outside workspace       | ✅ Blocked |
| Symlink escape (`ln -s /etc escape`)   | Resolve symlinks before validation   | ✅ Blocked |
| Env variable leak                      | Override HOME, TMPDIR, APPDATA, etc. | ✅ Jailed  |

### Environment Jailing

| Variable                      | Jailed To                           |
| ----------------------------- | ----------------------------------- |
| `HOME` / `USERPROFILE`        | `workspace/`                        |
| `TMPDIR` / `TEMP` / `TMP`     | `workspace/tmp/`                    |
| `XDG_CONFIG_HOME` / `APPDATA` | `workspace/config/`                 |
| `VIRTUAL_ENV`                 | `workspace/.venv/`                  |
| `PATH`                        | `workspace/.venv/bin/:$SYSTEM_PATH` |
| `UV_CACHE_DIR`                | `workspace/.uv_cache/`              |

### Verify any app's sandbox

```bash
curl http://localhost:8080/api/sandbox/gradio-demo/verify
# → { "path_traversal_blocked": true, "absolute_escape_blocked": true, ... }
```

---

## REST API

Base URL: `http://localhost:8080`

| Method   | Path                       | Description              |
| -------- | -------------------------- | ------------------------ |
| `GET`    | `/api/health`              | Health check             |
| `GET`    | `/api/system`              | OS, Python, GPU, uv info |
| `GET`    | `/api/catalog`             | Available apps           |
| `GET`    | `/api/apps`                | Installed apps           |
| `POST`   | `/api/apps`                | Install app              |
| `GET`    | `/api/apps/{id}`           | App details              |
| `DELETE` | `/api/apps/{id}`           | Uninstall app            |
| `POST`   | `/api/apps/{id}/launch`    | Launch app               |
| `POST`   | `/api/apps/{id}/stop`      | Stop app                 |
| `GET`    | `/api/sandbox/{id}/verify` | Security tests           |
| `GET`    | `/api/sandbox/{id}/disk`   | Disk usage               |

Interactive docs: [Swagger UI](http://localhost:8080/swagger-ui/)

---

## App Manifests

Apps are defined by JSON manifests in the `apps/` directory:

```jsonc
{
  "id": "my-app",
  "name": "My AI App",
  "description": "What this app does",
  "author": "username",
  "python_version": "3.10",
  "needs_gpu": false,
  "pip_deps": ["gradio", "torch"],
  "launch_cmd": "python3 app.py",
  "port": 7860,
  "disk_size": "~200MB",
  "tags": ["image-gen", "gpu"],
}
```

See [`docs/app-manifest-spec.md`](docs/app-manifest-spec.md) for the full spec.

---

## Plugin System

| Type         | Description                                 | Example             |
| ------------ | ------------------------------------------- | ------------------- |
| `hook`       | Lifecycle events (pre-install, post-launch) | Notifications       |
| `monitor`    | Background tasks                            | GPU monitor         |
| `provider`   | New app sources                             | HuggingFace browser |
| `middleware` | API request interceptors                    | Auth, rate limiting |
| `ui`         | Dashboard panels                            | System stats        |

See [`docs/plugin-spec.md`](docs/plugin-spec.md) for the full spec.

---

## Roadmap

| Phase | Status     | Description                                            |
| ----- | ---------- | ------------------------------------------------------ |
| 0     | ✅ Done    | Sandbox + uv + REST API + plugins                      |
| 0.5   | 🔄 partial | Desktop migration (Tauri 2 + Svelte 5 + shadcn-svelte) |
| 0.6   | 📋 Planned | License server (Axum + Ed25519)                        |
| 1     | 📋 current | Agent runtime (loop, LLM drivers, tools)               |
| 2     | 📋 current | Memory & tools (SQLite, MCP, 20+ built-in)             |
| 3     | 📋 current | Workflows & orchestration (DAG, sub-agents)            |
| 4     | 📋 current | Channels & autonomy (Telegram, Discord, cron)          |

See [`docs/current-plan.md`](docs/current-plan.md) and [`docs/future-plan.md`](docs/future-plan.md) for details.

---

## Cross-Platform

| Feature        | Linux              | Windows                      |
| -------------- | ------------------ | ---------------------------- |
| Process spawn  | `sh -c`            | `cmd.exe /C`                 |
| Permissions    | `chmod 700`        | `icacls` ACLs                |
| Python binary  | `python3`          | `python`                     |
| uv bootstrap   | `curl \| sh`       | `PowerShell irm \| iex`      |
| Data directory | `~/.ai-launcher`   | `%LOCALAPPDATA%\ai-launcher` |
| Venv python    | `.venv/bin/python` | `.venv\Scripts\python.exe`   |

---

## Contributing

See [AGENTS.md](AGENTS.md) for coding guidelines and architecture decisions.

```bash
cargo test       # Run Rust tests
cargo build      # Build the backend
```

---

## Tech Stack

| Component   | Technology                               |
| ----------- | ---------------------------------------- |
| Backend     | Rust (tiny_http, serde, anyhow)          |
| Web Desktop | Svelte 5, TypeScript, Vite, SCSS         |
| Sandboxing  | Custom filesystem jail (no Docker)       |
| Python envs | uv (Rust-based, 10–100× faster than pip) |
| API spec    | OpenAPI 3.0.3                            |
| Desktop     | Tauri 2                                  |

---

## License

MIT
