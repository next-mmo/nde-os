# Performance Optimization Plan — Low RAM Target (4GB PCs)

> **Phase**: Next  
> **Status**: Planned  
> **Goal**: Bring NDE-OS idle footprint from ~400–600MB to ~150–250MB

## Context

NDE-OS currently has memory hotspots that make it heavy on 4GB PCs. The WebView2 process alone consumes 300–500MB, and the Rust backend adds 50–100MB due to uncached `sysinfo` allocations and heavy dependencies. Similar tools (Pinokio/Electron, LM Studio, Jan.ai) face the same challenge — NDE-OS has the advantage of Tauri's lighter WebView2 vs Electron's bundled Chromium.

---

## 1. Rust Backend — sysinfo Caching (Biggest Win)

**File**: `core/src/system_metrics/mod.rs`

`snapshot_resource_usage()` allocates a fresh `System::new()` every call (every 5s from frontend). `sysinfo::System` enumerates all processes, CPUs, network — ~25MB per allocation.

**Fix**: Cache `System` in `static OnceLock<Mutex<System>>`, only call `refresh_memory()` on the cached instance.

## 2. Rust Backend — Dependency Trimming

| File | Change | Savings |
|------|--------|---------|
| `core/Cargo.toml` | `sysinfo` features → `["system"]` only | ~15MB/refresh |
| `core/Cargo.toml` | Remove `reqwest` `blocking` feature | ~8MB (second tokio runtime) |
| `desktop/src-tauri/Cargo.toml` | `devtools` behind `[features] dev` flag | ~2MB binary |

## 3. Release Profile

**File**: Root `Cargo.toml`

```toml
[profile.release]
panic = "abort"
codegen-units = 1
lto = true
opt-level = "s"
strip = true
```

~30–50% smaller binary, ~15–20MB less RSS.

## 4. Frontend — Polling Intervals

**File**: `desktop/src/components/Desktop/Desktop.svelte`

| Timer | Current | Target |
|-------|---------|--------|
| `refreshAll` | 20s | 60s |
| `refreshResourceUsage` | 5s | 15s |
| `QueryClient.staleTime` | 30s | 60s |

Reduces GC pressure and HTTP traffic in WebView2.

## 5. Frontend — Lazy iframe Loading

**File**: `desktop/src/components/apps/Launcher/SessionSurface.svelte`

Add `loading="lazy"` to embedded session iframes for deferred loading.

## 6. Frontend — Vite Code Splitting

**File**: `desktop/vite.config.ts`

Add `rollupOptions.output.manualChunks` to split heavy components (ShieldBrowser, CodeEditor, Chat, Knowledge) into separate chunks.

## 7. Tauri Config — Security

**File**: `desktop/src-tauri/tauri.conf.json`

- Configure CSP (currently `null`)
- Enable `freezePrototype: true`

---

## Verification Checklist

- [ ] `cargo test -p ai-launcher-core -- system_metrics`
- [ ] `cargo build --release -p ai-launcher-desktop` compiles
- [ ] `cd desktop && pnpm check` passes
- [ ] Task Manager: idle RSS < 300MB
- [ ] DevTools Network: polling intervals match targets
