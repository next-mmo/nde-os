# AI Launcher vs Pinokio — Feature Parity Analysis

> Based on reading every file in `core/src/` (6 modules, ~1630 lines of Rust)

## Scorecard

| Pinokio Feature | AI Launcher Status | Details |
|---|---|---|
| **Pip cache / shared packages** | ⚠️ **Partial** | `uv` caches globally by default, but `UV_CACHE_DIR` is **per-app** (`workspace/.uv_cache`) — defeats the purpose. No cross-app dedup. |
| **Symlink-shared AI models** | ❌ **Missing** | No shared model directory. Each app has its own `models/` inside its sandbox. No symlink or reflink sharing. |
| **Conda support** | ❌ **Missing** | Only `uv venv` (pip-compatible). No conda/mamba/micromamba. |
| **CUDA / GPU detection** | ⚠️ **Manifest-only** | `needs_gpu: bool` exists in manifest but the core does **zero** CUDA detection, version checking, or `--extra-index-url` for PyTorch GPU wheels. |
| **Multiple Python versions** | ✅ **Has it** | `uv python install <version>` per app. Manifest declares `python_version`. Fallback to system Python. |
| **Per-app venv isolation** | ✅ **Has it** | One `.venv` per workspace via `uv venv --python <ver>`. |
| **Filesystem sandbox** | ✅ **Has it** | Full jail with path traversal defense, symlink escape blocking, `icacls` on Windows. Stronger than Pinokio. |
| **App install from Git** | ✅ **Has it** | `git clone --depth 1` + manifest.json parsing. |
| **App install from Zip** | ✅ **Has it** | Cross-platform extraction (PowerShell / unzip). |
| **App install from folder** | ✅ **Has it** | Direct copy with validation. |
| **Node.js support** | ⚠️ **Basic** | Detects `package.json` → runs `npm install`. No `nvm`, no version pinning, no pnpm/yarn. |
| **Disk usage tracking** | ✅ **Has it** | Per-app disk usage + system metrics (memory, disk). |
| **Cross-platform** | ✅ **Has it** | Proper `cfg!(windows)` everywhere. No WSL dependency. |
| **uv bootstrap** | ✅ **Has it** | Auto-downloads uv if missing (PowerShell on Win, curl on Unix). |

---

## Critical Gaps (What Pinokio Does That You Don't)

### 1. 🔴 No Shared Pip Cache
**Pinokio**: Global pip cache so packages like `torch` (~2.5 GB) download once.  
**AI Launcher**: Line 147 of `uv_env/mod.rs` sets `UV_CACHE_DIR` to `workspace/.uv_cache` — **per workspace**. This means every app re-downloads the same packages.

**Fix**: Point `UV_CACHE_DIR` to a single global directory (e.g., `base_dir/.uv_cache`). `uv` already deduplicates at the cache level via content-addressed storage.

### 2. 🔴 No Shared Model Directory (Symlinks)
**Pinokio**: Shared `models/` directory, symlinked into each app workspace. A 4GB SDXL checkpoint is stored once.  
**AI Launcher**: Sandbox creates `models/` per workspace (line 136 of `sandbox/mod.rs`) but there is no global model store or symlink/hardlink mechanism.

**Fix**: Add a global `base_dir/shared_models/` directory, create symlinks or hardlinks from `workspace/models/<name>` → global store. The sandbox already has `allow_readonly()` for this exact pattern.

### 3. 🔴 No Conda / Mamba Support  
**Pinokio**: Supports both pip and conda environments. Some AI apps (e.g., certain ComfyUI forks, some biology tools) require conda.  
**AI Launcher**: Only `uv venv` + `uv pip install`. No conda at all.

**Fix**: Add a `CondaEnv` module parallel to `UvEnv`. Use `micromamba` (single binary, fast) for apps that declare `"env_type": "conda"` in the manifest.

### 4. 🔴 No CUDA/GPU Detection
**Pinokio**: Detects GPU vendor (NVIDIA/AMD/Intel), CUDA version, and auto-selects the right PyTorch index URL (`--extra-index-url https://download.pytorch.org/whl/cu121`).  
**AI Launcher**: `needs_gpu: bool` is a passive flag. The core never checks if CUDA is installed, what version, or adjusts pip install URLs.

**Fix**: Add a `gpu_detect` module that:
- Runs `nvidia-smi` to detect NVIDIA GPU + CUDA version
- Checks for ROCm (`rocm-smi`) on AMD
- Maps CUDA version → correct PyTorch `--extra-index-url`
- Injects the URL into `uv pip install` arguments

### 5. 🟡 No Node.js Version Management
**Pinokio**: Can pin Node.js versions per app.  
**AI Launcher**: Just runs system `npm install`. No `nvm`, no version pinning.

**Fix**: Could use `uv`-style approach but for Node: use `fnm` or `volta` to install a specific Node version per workspace.

---

## What AI Launcher Does Better Than Pinokio

| Feature | Why it's better |
|---|---|
| **Security sandbox** | Pinokio has no filesystem jail. AI Launcher has path canonicalization, symlink defense, blocked filenames, platform-specific ACLs. |
| **uv instead of pip** | 10-100× faster installs. Pinokio uses raw pip. |
| **Rust core** | No Python runtime dependency for the launcher itself. Single binary potential. |
| **System metrics** | Built-in memory/disk monitoring with sysinfo crate. |
| **Store upload API** | Validation + trial install pipeline for user-submitted apps. Pinokio relies on community scripts. |

---

## Summary Priority

If the goal is Pinokio parity, these are the **4 things to build** (in priority order):

1. **Global pip/uv cache** — 1 line change (`UV_CACHE_DIR` → global path). Huge disk savings.
2. **Shared model store with symlinks** — Add `shared_models/` + symlink logic. ~100 lines.
3. **GPU/CUDA detection** — New `gpu_detect` module. ~150 lines.  
4. **Conda/micromamba support** — New `conda_env` module parallel to `uv_env`. ~300 lines.
