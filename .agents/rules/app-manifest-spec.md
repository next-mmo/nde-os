---
trigger: model_decision
---

# App Manifest Specification

Version: 1.0

## Overview

Every AI app installable through AI Launcher is defined by a `manifest.json` file. This document describes the full schema, required fields, optional fields, and lifecycle hooks.

## Required Fields

| Field            | Type     | Description                                                                   |
| ---------------- | -------- | ----------------------------------------------------------------------------- |
| `id`             | string   | Unique identifier. Lowercase, hyphens only. Example: `stable-diffusion-webui` |
| `name`           | string   | Human-readable display name                                                   |
| `description`    | string   | Short description (1-2 sentences)                                             |
| `author`         | string   | Author or organization name                                                   |
| `python_version` | string   | Required Python version (e.g. `"3.10"`, `"3"` for any 3.x)                    |
| `needs_gpu`      | bool     | Whether the app requires an NVIDIA GPU                                        |
| `pip_deps`       | string[] | Pip packages to install via `uv pip install`                                  |
| `launch_cmd`     | string   | Shell command to start the app                                                |
| `port`           | integer  | Port the app's web UI listens on                                              |
| `disk_size`      | string   | Estimated disk space (e.g. `"~12GB"`)                                         |

## Optional Fields

| Field               | Type               | Default              | Description                             |
| ------------------- | ------------------ | -------------------- | --------------------------------------- |
| `repo`              | string             | null                 | Git repository URL to clone             |
| `branch`            | string             | `"main"`             | Git branch to checkout                  |
| `env`               | [string, string][] | `[]`                 | Extra environment variables             |
| `tags`              | string[]           | `[]`                 | Tags for search and filtering           |
| `icon`              | string             | `"📦"`               | Emoji icon                              |
| `homepage`          | string             | null                 | Project homepage URL                    |
| `license`           | string             | null                 | License identifier                      |
| `requirements_file` | string             | `"requirements.txt"` | Path to requirements file inside repo   |
| `pre_install`       | string             | null                 | Shell command to run before pip install |
| `post_install`      | string             | null                 | Shell command to run after pip install  |
| `pre_launch`        | string             | null                 | Shell command to run before launch      |
| `health_check`      | string             | null                 | HTTP URL to poll for readiness          |
| `health_timeout`    | integer            | 30                   | Seconds to wait for health check        |
| `min_vram`          | integer            | 0                    | Minimum GPU VRAM in GB                  |
| `min_ram`           | integer            | 0                    | Minimum system RAM in GB                |
| `cuda_version`      | string             | null                 | Required CUDA version (e.g. `"12.1"`)   |
| `models`            | Model[]            | `[]`                 | Models to download (see below)          |
| `volumes`           | Volume[]           | `[]`                 | Shared volumes (see below)              |

## Model Downloads

```json
{
  "models": [
    {
      "name": "v1-5-pruned-emaonly.safetensors",
      "url": "https://huggingface.co/runwayml/stable-diffusion-v1-5/resolve/main/v1-5-pruned-emaonly.safetensors",
      "destination": "models/Stable-diffusion/",
      "size": "4.27GB",
      "checksum_sha256": "..."
    }
  ]
}
```

## Shared Volumes

Volumes let apps share data (e.g. models directory):

```json
{
  "volumes": [
    {
      "name": "shared-models",
      "mount_path": "models/",
      "shared": true
    }
  ]
}
```

When `shared: true`, the volume is stored at `~/.ai-launcher/.shared/shared-models/` and symlinked into each app's workspace, saving disk space.

## Full Example

```json
{
  "id": "stable-diffusion-webui",
  "name": "Stable Diffusion WebUI",
  "description": "AUTOMATIC1111 web interface for Stable Diffusion image generation",
  "author": "AUTOMATIC1111",
  "repo": "https://github.com/AUTOMATIC1111/stable-diffusion-webui.git",
  "branch": "master",
  "python_version": "3.10",
  "needs_gpu": true,
  "cuda_version": "12.1",
  "min_vram": 4,
  "min_ram": 8,
  "pip_deps": [],
  "requirements_file": "requirements_versions.txt",
  "pre_install": "python3 -c \"import launch; launch.prepare_environment()\"",
  "launch_cmd": "python3 launch.py --listen --port 7860 --enable-insecure-extension-access",
  "port": 7860,
  "health_check": "http://localhost:7860/sdapi/v1/progress",
  "health_timeout": 120,
  "env": [["COMMANDLINE_ARGS", "--xformers --no-half-vae"]],
  "disk_size": "~12GB",
  "tags": ["image-generation", "diffusion", "gpu", "txt2img", "img2img"],
  "icon": "🎨",
  "homepage": "https://github.com/AUTOMATIC1111/stable-diffusion-webui",
  "license": "AGPL-3.0",
  "models": [
    {
      "name": "v1-5-pruned-emaonly.safetensors",
      "url": "https://huggingface.co/runwayml/stable-diffusion-v1-5/resolve/main/v1-5-pruned-emaonly.safetensors",
      "destination": "models/Stable-diffusion/",
      "size": "4.27GB"
    }
  ],
  "volumes": [
    {
      "name": "sd-models",
      "mount_path": "models/",
      "shared": true
    }
  ]
}
```

## Lifecycle

```
1. manifest.json loaded
2. Sandbox created (workspace/ with subdirs)
3. Sandbox verified (4 security checks)
4. Git repo cloned (if `repo` set)
5. `pre_install` runs (if set)
6. uv venv created with specified Python version
7. uv pip install for `pip_deps`
8. uv pip install -r for `requirements_file` (if exists)
9. `post_install` runs (if set)
10. Models downloaded (if `models` set)
11. Status → Installed
12. On launch: `pre_launch` → `launch_cmd` → poll `health_check`
13. Status → Running
```
