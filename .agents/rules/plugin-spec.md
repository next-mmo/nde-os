---
trigger: model_decision
---

# Plugin Specification

Version: 1.0

## Overview

Plugins extend AI Launcher with custom functionality: monitoring, model management, custom UI, integrations, and lifecycle hooks. Plugins run inside the launcher's process space (for Rust plugins) or as sandboxed child processes (for Python/shell plugins).

## Plugin Types

### `hook` — Lifecycle Events

Runs code at specific app lifecycle events.

```json
{
  "id": "slack-notifier",
  "type": "hook",
  "hooks": ["post_install", "post_launch", "on_error"],
  "entry": "notify.py",
  "language": "python",
  "config_schema": {
    "webhook_url": { "type": "string", "required": true },
    "channel": { "type": "string", "default": "#ai-launcher" }
  }
}
```

**Available hooks:**

| Hook             | Trigger                  | Payload                            |
| ---------------- | ------------------------ | ---------------------------------- |
| `pre_install`    | Before sandbox creation  | `{ app_id, manifest }`             |
| `post_install`   | After successful install | `{ app_id, manifest, workspace }`  |
| `pre_launch`     | Before process spawn     | `{ app_id, launch_cmd, env_vars }` |
| `post_launch`    | After process started    | `{ app_id, pid, port }`            |
| `pre_stop`       | Before kill signal       | `{ app_id, pid }`                  |
| `post_stop`      | After process stopped    | `{ app_id }`                       |
| `pre_uninstall`  | Before workspace removal | `{ app_id, workspace }`            |
| `post_uninstall` | After cleanup            | `{ app_id }`                       |
| `on_error`       | On any error             | `{ app_id, error, phase }`         |
| `on_health_fail` | Health check failed      | `{ app_id, url, status }`          |

### `monitor` — Background Tasks

Continuous polling tasks that provide system data.

```json
{
  "id": "gpu-monitor",
  "type": "monitor",
  "entry": "monitor.py",
  "language": "python",
  "interval_seconds": 5,
  "api_routes": [
    {
      "method": "GET",
      "path": "/api/plugins/gpu-monitor/stats",
      "description": "Current GPU stats"
    },
    {
      "method": "GET",
      "path": "/api/plugins/gpu-monitor/history",
      "description": "GPU usage history (last 1h)"
    }
  ],
  "deps": ["pynvml"],
  "config_schema": {
    "refresh_rate": { "type": "integer", "default": 5, "min": 1, "max": 60 },
    "alert_temp_celsius": { "type": "integer", "default": 85 },
    "history_minutes": { "type": "integer", "default": 60 }
  }
}
```

### `provider` — App Sources

Adds new sources for discovering and installing apps.

```json
{
  "id": "huggingface-spaces",
  "type": "provider",
  "entry": "provider.py",
  "language": "python",
  "api_routes": [
    {
      "method": "GET",
      "path": "/api/plugins/huggingface-spaces/search",
      "description": "Search HuggingFace Spaces",
      "params": [
        { "name": "q", "type": "string", "description": "Search query" },
        {
          "name": "sdk",
          "type": "string",
          "enum": ["gradio", "streamlit", "docker"]
        }
      ]
    },
    {
      "method": "POST",
      "path": "/api/plugins/huggingface-spaces/import",
      "description": "Import a Space as an installable app"
    }
  ],
  "deps": ["huggingface-hub"],
  "config_schema": {
    "hf_token": {
      "type": "string",
      "secret": true,
      "description": "HuggingFace API token"
    }
  }
}
```

### `middleware` — Request Interceptors

Intercepts API requests for auth, logging, rate limiting.

```json
{
  "id": "api-auth",
  "type": "middleware",
  "entry": "auth.rs",
  "language": "rust",
  "intercept": ["all"],
  "priority": 100,
  "config_schema": {
    "api_key": { "type": "string", "secret": true },
    "exclude_paths": { "type": "string[]", "default": ["/api/health"] }
  }
}
```

### `ui` — Dashboard Panels

Adds custom panels to the web dashboard.

```json
{
  "id": "system-stats",
  "type": "ui",
  "entry": "panel.html",
  "language": "html",
  "ui_slot": "sidebar",
  "api_routes": [
    {
      "method": "GET",
      "path": "/api/plugins/system-stats/data",
      "description": "System CPU, RAM, disk data"
    }
  ]
}
```

**UI Slots:**

| Slot       | Location                    |
| ---------- | --------------------------- |
| `sidebar`  | Right sidebar panel         |
| `tab`      | New tab in main navigation  |
| `header`   | Header bar widget           |
| `app_card` | Injected into each app card |
| `modal`    | Popup dialog                |

---

## Plugin Schema (Full)

```jsonc
{
  // Required
  "id": "my-plugin", // Unique identifier
  "name": "My Plugin", // Display name
  "version": "1.0.0", // Semver
  "type": "hook|monitor|provider|middleware|ui",
  "description": "What this plugin does",
  "entry": "main.py", // Entry point file

  // Required for execution
  "language": "python|rust|shell|html", // Runtime language

  // Optional
  "author": "username",
  "homepage": "https://...",
  "license": "MIT",
  "deps": ["pynvml", "requests"], // Python dependencies
  "rust_deps": { "tokio": "1" }, // Rust dependencies (for rust plugins)
  "min_launcher_version": "0.2.0", // Minimum AI Launcher version

  // Type-specific
  "hooks": ["post_install", "on_error"], // For hook type
  "interval_seconds": 5, // For monitor type
  "intercept": ["all"], // For middleware type
  "priority": 100, // For middleware type (lower = first)
  "ui_slot": "sidebar", // For ui type

  // API extensions
  "api_routes": [
    {
      "method": "GET",
      "path": "/api/plugins/{plugin_id}/...",
      "description": "What this endpoint does",
      "params": [{ "name": "q", "type": "string" }],
    },
  ],

  // User configuration
  "config_schema": {
    "option_name": {
      "type": "string|integer|boolean|string[]",
      "default": "value",
      "required": false,
      "secret": false,
      "description": "What this option does",
      "min": 0,
      "max": 100,
      "enum": ["a", "b", "c"],
    },
  },
}
```

---

## Plugin Lifecycle

```
1. Plugin discovered in plugins/ directory
2. plugin.json parsed and validated
3. Dependencies installed (uv pip install for Python plugins)
4. Plugin registered with the launcher
5. API routes mounted
6. Hooks connected to lifecycle events
7. Monitor loop started (if monitor type)
8. Plugin config loaded from user preferences
```

### Enable/Disable

Plugins can be toggled without removal:

```bash
# Enable
curl -X POST http://localhost:8080/api/plugins/gpu-monitor/enable

# Disable
curl -X POST http://localhost:8080/api/plugins/gpu-monitor/disable

# Configure
curl -X PUT http://localhost:8080/api/plugins/gpu-monitor/config \
  -H "Content-Type: application/json" \
  -d '{"refresh_rate": 10, "alert_temp_celsius": 90}'
```

---

## Built-in Plugins

### `gpu-monitor`

Real-time NVIDIA GPU monitoring. Shows utilization, temperature, VRAM, power draw.

### `model-downloader`

Background model downloading with progress tracking. Supports HuggingFace, CivitAI, direct URLs. Shared model storage across apps.

### `disk-cleaner`

Identifies and cleans uv caches, unused venvs, old logs, and orphaned workspaces.

### `log-viewer`

Aggregates stdout/stderr from all running apps into a searchable, filterable log stream.

### `port-manager`

Prevents port conflicts between apps. Auto-assigns available ports. Shows port usage map.

### `backup-restore`

Exports/imports app configs, models, and workspace snapshots. Supports local and S3.

---

## Example: Writing a Hook Plugin

`plugins/my-hook/plugin.json`:

```json
{
  "id": "my-hook",
  "name": "My Hook",
  "version": "1.0.0",
  "type": "hook",
  "description": "Logs all app launches to a file",
  "entry": "hook.py",
  "language": "python",
  "hooks": ["post_launch", "post_stop"]
}
```

`plugins/my-hook/hook.py`:

```python
import json
import sys
from datetime import datetime

def handle(event_type: str, payload: dict):
    """Called by AI Launcher on each hooked event."""
    with open("launch_log.txt", "a") as f:
        f.write(f"{datetime.now().isoformat()} [{event_type}] {json.dumps(payload)}\n")

if __name__ == "__main__":
    event = json.loads(sys.argv[1])
    handle(event["type"], event["payload"])
```

## Example: Writing a Monitor Plugin

`plugins/gpu-monitor/plugin.json`:

```json
{
  "id": "gpu-monitor",
  "name": "GPU Monitor",
  "version": "1.0.0",
  "type": "monitor",
  "entry": "monitor.py",
  "language": "python",
  "interval_seconds": 5,
  "deps": ["pynvml"],
  "api_routes": [{ "method": "GET", "path": "/api/plugins/gpu-monitor/stats" }]
}
```

`plugins/gpu-monitor/monitor.py`:

```python
import json
import pynvml

pynvml.nvmlInit()

def get_stats():
    handle = pynvml.nvmlDeviceGetHandleByIndex(0)
    util = pynvml.nvmlDeviceGetUtilizationRates(handle)
    mem = pynvml.nvmlDeviceGetMemoryInfo(handle)
    temp = pynvml.nvmlDeviceGetTemperature(handle, pynvml.NVML_TEMPERATURE_GPU)
    return {
        "gpu_util_percent": util.gpu,
        "mem_used_mb": mem.used // 1048576,
        "mem_total_mb": mem.total // 1048576,
        "temperature_c": temp,
        "name": pynvml.nvmlDeviceGetName(handle),
    }

# Called by AI Launcher on each tick
if __name__ == "__main__":
    print(json.dumps(get_stats()))
```
