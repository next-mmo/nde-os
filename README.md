# 🚀 AI Launcher

**Cross-platform, sandboxed AI application manager powered by uv.**

Install, run, and manage AI applications in isolated filesystem jails with per-app Python virtual environments. Think Pinokio — but built in Rust with real OS-level security.

> **Linux + Windows native. No WSL required.**

---

## Table of Contents

- [Quick Start](#quick-start)
- [How It Works](#how-it-works)
- [Architecture](#architecture)
- [App Manifests](#app-manifests)
- [Plugin System](#plugin-system)
- [Sandbox Security](#sandbox-security)
- [REST API](#rest-api)
- [Cross-Platform Details](#cross-platform-details)
- [Configuration](#configuration)
- [Contributing](#contributing)

---

## Quick Start

### Prerequisites

- **Rust 1.70+** — [Install Rust](https://rustup.rs)
- **uv** — auto-downloaded on first run, or [install manually](https://docs.astral.sh/uv/)
- **Python 3.10+** — uv can install this for you

### Linux

```bash
git clone https://github.com/ai-launcher/ai-launcher
cd ai-launcher
chmod +x run.sh
./run.sh
```

### Windows

```powershell
git clone https://github.com/ai-launcher/ai-launcher
cd ai-launcher
.\run.bat
```

### Then open

- **Swagger UI**: http://localhost:8080/swagger-ui/
- **API docs**: http://localhost:8080/api-docs/openapi.json

### One-liner install + run an app

```bash
# Install Gradio demo
curl -X POST http://localhost:8080/api/apps \
  -H "Content-Type: application/json" \
  -d @apps/gradio-demo/manifest.json

# Launch it
curl -X POST http://localhost:8080/api/apps/gradio-demo/launch

# Open in browser
open http://localhost:7860
```

---

## How It Works

### Install Flow

```
User clicks "Install Gradio Demo"
  │
  ├─ 1. CREATE SANDBOX
  │     mkdir ~/.ai-launcher/gradio-demo/workspace/
  │     mkdir data/ models/ outputs/ logs/ tmp/ config/
  │     chmod 700 workspace/  (Linux)
  │     icacls workspace/ /inheritance:r /grant:r %USERNAME%:(OI)(CI)F  (Windows)
  │
  ├─ 2. VERIFY SECURITY
  │     ✓ Path traversal blocked (../../../etc/passwd)
  │     ✓ Absolute escape blocked (/etc/shadow)
  │     ✓ Symlink escape blocked
  │     ✓ Valid sandbox paths resolve correctly
  │
  ├─ 3. CREATE UV VENV
  │     uv python install 3.10
  │     uv venv workspace/.venv --python 3.10     (< 1 second)
  │
  └─ 4. INSTALL DEPENDENCIES
        uv pip install gradio --python .venv      (10-100x faster than pip)
```

### Launch Flow

```
User clicks "Launch"
  │
  ├─ Rewrite command:  python3 app.py → .venv/bin/python app.py
  │
  ├─ Set jailed environment:
  │     HOME / USERPROFILE   → workspace/
  │     TMPDIR / TEMP / TMP  → workspace/tmp/
  │     PYTHONPATH           → workspace/.venv/lib/
  │     VIRTUAL_ENV          → workspace/.venv/
  │     PATH                 → workspace/.venv/bin/:$PATH
  │     XDG_CONFIG_HOME      → workspace/config/      (Linux)
  │     APPDATA              → workspace/config/       (Windows)
  │
  ├─ Spawn process:
  │     sh -c "..."          (Linux)
  │     cmd.exe /C "..."     (Windows)
  │
  └─ Track PID, port → mark as Running
```

---

## Architecture

```
ai-launcher/
├── Cargo.toml                  # Rust project config
├── run.sh                      # Linux launcher
├── run.bat                     # Windows launcher
├── README.md                   # This file
├── AGENTS.md                   # AI coding agent instructions
│
├── src/
│   ├── main.rs                 # REST API server + Swagger UI + OpenAPI spec
│   ├── sandbox/mod.rs          # Filesystem jail (path validation, symlink defense)
│   ├── uv_env/mod.rs           # UV environment manager (bootstrap, venv, deps)
│   ├── app_manager/mod.rs      # App lifecycle (install, launch, stop, uninstall)
│   └── manifest/mod.rs         # App manifest types + built-in catalog
│
├── apps/                       # App manifest repository
│   ├── gradio-demo/
│   │   └── manifest.json
│   ├── stable-diffusion/
│   │   └── manifest.json
│   ├── ollama/
│   │   └── manifest.json
│   ├── comfyui/
│   │   └── manifest.json
│   └── ...
│
├── plugins/                    # Plugin extensions
│   ├── gpu-monitor/
│   │   └── plugin.json
│   ├── model-downloader/
│   │   └── plugin.json
│   └── ...
│
└── docs/
    ├── app-manifest-spec.md    # How to write app manifests
    ├── plugin-spec.md          # How to write plugins
    └── sandbox-internals.md    # Sandbox implementation details
```

### Data Directory at Runtime

```
~/.ai-launcher/                         # Linux
%LOCALAPPDATA%\ai-launcher\             # Windows
│
├── .uv/                                # Bundled uv binary
│   └── uv (or uv.exe)
├── registry.json                       # Installed apps database
│
├── gradio-demo/
│   └── workspace/                      # ← SANDBOXED (chmod 700)
│       ├── .venv/                      # uv-managed Python venv
│       │   ├── bin/python              # (Linux)
│       │   ├── Scripts/python.exe      # (Windows)
│       │   └── lib/site-packages/
│       ├── .uv_cache/                  # uv cache
│       ├── .sandbox_info               # Sandbox metadata
│       ├── data/
│       ├── models/
│       ├── outputs/
│       ├── logs/
│       ├── tmp/
│       ├── config/
│       └── app.py
│
└── stable-diffusion/
    └── workspace/
        ├── .venv/
        └── ...
```

---

## App Manifests

Apps are defined by JSON manifest files. See `apps/` directory for examples.

### Manifest Schema

```jsonc
{
  // Required
  "id": "my-app",                        // Unique identifier (lowercase, hyphens)
  "name": "My AI App",                   // Display name
  "description": "What this app does",   // Short description
  "author": "username",                  // Author or org
  "python_version": "3.10",             // Required Python version
  "needs_gpu": false,                    // Whether GPU is required
  "pip_deps": ["gradio", "torch"],       // Pip packages to install via uv
  "launch_cmd": "python3 app.py",        // Command to start the app
  "port": 7860,                          // Port the app listens on
  "disk_size": "~200MB",                 // Estimated disk space

  // Optional
  "repo": "https://github.com/...",      // Git repo to clone
  "env": [["KEY", "VALUE"]],             // Extra environment variables
  "tags": ["image-gen", "gpu"],          // Tags for search/filtering
  "requirements_file": "requirements.txt", // Alt requirements file
  "pre_install": "bash setup.sh",        // Script to run before pip install
  "post_install": "bash post.sh",        // Script to run after pip install
  "health_check": "http://localhost:7860/api/health", // Health check URL
  "min_vram": 4,                         // Minimum GPU VRAM in GB
  "min_ram": 8,                          // Minimum system RAM in GB
  "icon": "🎨",                          // Emoji icon
  "homepage": "https://...",             // Project homepage
  "license": "MIT"                       // License
}
```

### Creating a New App

1. Create `apps/my-app/manifest.json`
2. POST it to the API or place it in the apps directory
3. The launcher handles sandbox creation, venv, deps, and launching

See `docs/app-manifest-spec.md` for the full specification.

---

## Plugin System

Plugins extend the launcher with custom functionality. See `plugins/` directory.

### Plugin Types

| Type | Description | Example |
|------|-------------|---------|
| `hook` | Runs at lifecycle events (pre-install, post-launch, etc.) | Notifications, logging |
| `monitor` | Continuous background task | GPU monitor, disk watcher |
| `provider` | Adds new app sources | HuggingFace browser, CivitAI models |
| `middleware` | Intercepts API requests | Auth, rate limiting |
| `ui` | Adds UI panels to the dashboard | System stats, model browser |

### Plugin Schema

```jsonc
{
  "id": "gpu-monitor",
  "name": "GPU Monitor",
  "version": "1.0.0",
  "type": "monitor",
  "description": "Real-time GPU usage, temperature, and VRAM tracking",
  "author": "ai-launcher",
  "entry": "monitor.py",                // Plugin entry point
  "language": "python",                  // python | rust | shell
  "hooks": ["post_launch", "on_tick"],   // Lifecycle hooks
  "interval_seconds": 5,                 // For monitors: poll interval
  "api_routes": [                        // Custom API endpoints
    { "method": "GET", "path": "/api/plugins/gpu-monitor/stats" }
  ],
  "deps": ["pynvml"],                   // Plugin dependencies
  "config_schema": {                     // User-configurable options
    "refresh_rate": { "type": "integer", "default": 5 },
    "alert_temp": { "type": "integer", "default": 85 }
  }
}
```

See `docs/plugin-spec.md` for the full specification.

---

## Sandbox Security

### What's Blocked

| Attack | Linux Test | Windows Test | Result |
|--------|-----------|--------------|--------|
| Path traversal | `../../../etc/passwd` | `..\..\..\Windows\System32` | BLOCKED |
| Absolute escape | `/etc/shadow` | `C:\Windows\System32` | BLOCKED |
| Symlink escape | `ln -s /etc escape` | `mklink /D escape C:\Windows` | BLOCKED |
| Env variable leak | `HOME`, `TMPDIR` | `USERPROFILE`, `TEMP` | JAILED |

### Environment Jailing

Every process launched by AI Launcher has these env vars overridden:

| Variable | Jailed To |
|----------|-----------|
| `HOME` / `USERPROFILE` | `workspace/` |
| `TMPDIR` / `TEMP` / `TMP` | `workspace/tmp/` |
| `XDG_CONFIG_HOME` / `APPDATA` | `workspace/config/` |
| `XDG_DATA_HOME` / `LOCALAPPDATA` | `workspace/data/` |
| `PYTHONPATH` | `workspace/.venv/lib/` |
| `VIRTUAL_ENV` | `workspace/.venv/` |
| `PATH` | `workspace/.venv/bin/:$SYSTEM_PATH` |
| `SANDBOX_ROOT` | `workspace/` |
| `PIP_TARGET` | `workspace/pip_packages/` |
| `UV_CACHE_DIR` | `workspace/.uv_cache/` |

### Verify Any App's Sandbox

```bash
curl http://localhost:8080/api/sandbox/gradio-demo/verify
```

Returns:
```json
{
  "path_traversal_blocked": true,
  "absolute_escape_blocked": true,
  "symlink_escape_blocked": true,
  "valid_path_works": true,
  "platform": "linux",
  "sandbox_root": "/home/user/.ai-launcher/gradio-demo/workspace"
}
```

---

## REST API

Base URL: `http://localhost:8080`

### System

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/health` | Health check |
| `GET` | `/api/system` | OS, Python, GPU, uv version |

### Catalog

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/catalog` | List available apps |

### Apps

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/apps` | List installed apps |
| `POST` | `/api/apps` | Install app (sandbox + uv venv + deps) |
| `GET` | `/api/apps/{id}` | Get app details |
| `DELETE` | `/api/apps/{id}` | Uninstall app + remove workspace |
| `POST` | `/api/apps/{id}/launch` | Launch inside sandbox |
| `POST` | `/api/apps/{id}/stop` | Stop running app |

### Sandbox

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/sandbox/{id}/verify` | Run security tests |
| `GET` | `/api/sandbox/{id}/disk` | Workspace disk usage |

### Swagger UI

Interactive API explorer: http://localhost:8080/swagger-ui/

OpenAPI 3.0.3 JSON: http://localhost:8080/api-docs/openapi.json

---

## Cross-Platform Details

| Feature | Linux | Windows |
|---------|-------|---------|
| Process spawn | `sh -c` | `cmd.exe /C` |
| Permissions | `chmod 700` | `icacls` ACLs |
| Python binary | `python3` | `python` |
| Pip binary | `pip3` | `pip` |
| uv bootstrap | `curl \| sh` | `PowerShell irm \| iex` |
| HOME jail | `HOME=` | `USERPROFILE=` |
| Temp jail | `TMPDIR=` | `TEMP=`, `TMP=` |
| Config jail | `XDG_CONFIG_HOME=` | `APPDATA=` |
| Data directory | `~/.ai-launcher` | `%LOCALAPPDATA%\ai-launcher` |
| Symlink test | `std::os::unix::fs::symlink` | `std::os::windows::fs::symlink_dir` |
| Venv python | `.venv/bin/python` | `.venv\Scripts\python.exe` |
| Venv activate | `source .venv/bin/activate` | `.venv\Scripts\activate.bat` |

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `AI_LAUNCHER_DIR` | `~/.ai-launcher` | Override data directory |
| `AI_LAUNCHER_PORT` | `8080` | API server port |
| `AI_LAUNCHER_HOST` | `0.0.0.0` | API server bind address |
| `UV_INSTALL_DIR` | `$AI_LAUNCHER_DIR/.uv` | Where to put uv binary |

### Why uv Over conda/pip?

| | uv | pip | conda |
|---|---|---|---|
| Install speed | 10-100x faster | Baseline | 2-5x slower |
| Installs Python | Yes | No | Yes |
| Venv creation | <1 second | ~3 seconds | 10-30 seconds |
| Binary size | ~25MB | Comes with Python | ~400MB (Miniforge) |
| Written in | Rust | Python | Python/C |
| Disk per env | Small (hardlinks) | Medium | Large (copies) |
| Lock files | Yes (uv.lock) | No native | Yes (environment.yml) |

---

## Contributing

See [AGENTS.md](AGENTS.md) for coding guidelines and architecture decisions.

### Running Tests

```bash
cargo test
```

### Project Structure

```
src/main.rs          → REST API, Swagger, OpenAPI spec
src/sandbox/         → Filesystem jail implementation
src/uv_env/          → uv integration (bootstrap, venv, deps)
src/app_manager/     → App lifecycle management
src/manifest/        → Type definitions and built-in catalog
```

---

## License

MIT
