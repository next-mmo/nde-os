# Current Plan: Desktop Migration

> **Status**: In Progress  
> **Target**: Tauri 2 + SvelteKit 5 + shadcn-svelte  
> **Priority**: Immediate

---

## Goal

Migrate AI Launcher from a `tiny_http` REST API into a native desktop application using **Tauri 2**. Restructure the project as a Cargo workspace so the Rust core is reusable across multiple frontends.

## Architecture

```
ai-launcher/
├── Cargo.toml              # Workspace root
├── core/                   # Shared Rust library
│   └── src/
│       ├── lib.rs          # pub mod sandbox, uv_env, app_manager, manifest
│       ├── sandbox/        # Filesystem jail (cross-platform)
│       ├── uv_env/         # uv binary bootstrap + venv management
│       ├── app_manager/    # App lifecycle (install/launch/stop/uninstall)
│       └── manifest/       # Data types + built-in catalog
├── server/                 # HTTP server (existing, moved from src/)
│   └── src/main.rs         # tiny_http — backward compatible
├── desktop/                # Tauri 2 desktop app
│   ├── src-tauri/          # Rust: #[tauri::command] wrappers around core
│   │   └── src/lib.rs      # 12 commands replacing REST routes
│   ├── src/                # SvelteKit 5 frontend
│   │   ├── routes/         # catalog, installed, running, logs, settings
│   │   └── lib/            # stores, components (shadcn-svelte), API layer
│   └── package.json
└── (future: electron/, mobile/, web/)
```

## Frontend Stack

| Layer | Choice |
|-------|--------|
| Framework | SvelteKit 5 (static adapter, SSR disabled) |
| Components | shadcn-svelte (Card, Button, Badge, Input, Sidebar, Tooltip) |
| Styling | Tailwind CSS v4 (`@import "tailwindcss"`, no config file) |
| IPC | `@tauri-apps/api/core` → `invoke()` |

### shadcn-svelte Setup

```bash
npx shadcn-svelte@latest init        # scaffolds $lib/components/ui/, utils, theme
npx shadcn-svelte@latest add \
  card button badge input tooltip \
  sidebar scroll-area separator \
  alert-dialog dropdown-menu sonner
```

| Component | Used For |
|-----------|----------|
| `Card` | App cards in catalog/installed views |
| `Button` | Install, Launch, Stop, Open, Uninstall actions |
| `Badge` | Status (Running/Installed/Error), GPU tag, app tags |
| `Input` | Search filter, settings fields |
| `Sidebar` | Main navigation (Catalog/Installed/Running/Logs/Settings) |
| `ScrollArea` | Log terminal, long app lists |
| `Separator` | Section dividers |
| `Tooltip` | Action button hints, sandbox status info |
| `AlertDialog` | Confirm uninstall/stop destructive actions |
| `DropdownMenu` | App context menu (verify sandbox, disk usage) |
| `Sonner` | Toast notifications (install success, errors) |

Dependencies: `bits-ui`, `clsx`, `tailwind-merge`, `tailwind-variants`

## Tauri Commands (replacing HTTP routes)

| Command | Old Route |
|---------|-----------|
| `health_check` | `GET /api/health` |
| `get_system_info` | `GET /api/system` |
| `get_catalog` | `GET /api/catalog` |
| `list_apps` | `GET /api/apps` |
| `install_app` | `POST /api/apps` |
| `get_app` | `GET /api/apps/:id` |
| `uninstall_app` | `DELETE /api/apps/:id` |
| `launch_app` | `POST /api/apps/:id/launch` |
| `stop_app` | `POST /api/apps/:id/stop` |
| `verify_sandbox` | `GET /api/sandbox/:id/verify` |
| `get_disk_usage` | `GET /api/sandbox/:id/disk` |
| `open_app_browser` | *(new — opens localhost URL)* |

## Key Design Decisions

- **Cargo workspace**: `core/` is a standalone crate. Any frontend (Tauri, Electron, CLI) just depends on `ai-launcher-core`.
- **Zero module changes**: sandbox, uv_env, app_manager, manifest are copied verbatim. Only the transport layer changes (HTTP → Tauri IPC).
- **Dark theme by default**: shadcn-svelte dark mode with custom accent colors.
- **Existing server preserved**: `cargo run -p ai-launcher-server` still works for headless/API use.

## Milestones

- [ ] Phase 1: Cargo workspace restructure (`core/` + `server/`)
- [ ] Phase 2: Tauri desktop (`desktop/src-tauri/` + `desktop/src/`)
- [ ] Phase 3: Verify builds + dev launch
