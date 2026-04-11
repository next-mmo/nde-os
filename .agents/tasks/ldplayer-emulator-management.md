# LDPlayer Emulator Full Management

- **ID:** NDE-10
- **Status:** 🟢 `done by AI`

## Feature
Comprehensive LDPlayer emulator management: auto-detect installations, manage instances (create/clone/launch/quit/remove/configure), persist profiles in SQLite DB, and display everything in a rich profiles table in the Shield Browser UI.

## Purpose
Users running LDPlayer instances need full lifecycle management directly from the NDE-OS Shield Browser — no manual CLI needed. LDPlayer's `ldconsole.exe` CLI provides `list2`, `launch`, `quit`, `add`, `copy`, `remove`, `modify`, and `isrunning`. We wrap all of these in a proper Rust module with SQLite-backed metadata (notes, tags, proxy config, linked shield profile), and expose them via Tauri IPC commands + a Svelte 5 table UI tab.

## Architecture

### Backend (Rust core)
1. **`core/src/shield/ldplayer.rs`** — LDPlayer manager module:
   - Auto-detect `ldconsole.exe` from registry/common paths (LDPlayer9, LDPlayer4)
   - Parse `ldconsole list2` output (index, name, ... ) into typed structs
   - Wrap all commands: `launch`, `quit`, `quitall`, `add`, `copy`, `remove`, `modify`, `isrunning`
   - Each method returns `anyhow::Result`

2. **`core/src/shield/ldplayer_db.rs`** — SQLite persistence for LDPlayer profiles:
   - Table: `ldplayer_profiles` (id, ld_index, name, status, cpu, memory, resolution, notes, tags, linked_shield_profile_id, proxy_host, proxy_port, created_at, updated_at)
   - CRUD operations: insert, update, delete, list, get_by_index
   - Sync function: reconcile DB rows with live `ldconsole list2` output

3. **Tauri commands** in `desktop/src-tauri/src/commands/shield.rs`:
   - `shield_detect_ldplayer` → check if ldconsole.exe is available
   - `shield_list_ldplayer_instances` → merged live + DB data
   - `shield_launch_ldplayer` → launch by index/name
   - `shield_quit_ldplayer` → quit by index/name
   - `shield_create_ldplayer` → create new instance
   - `shield_clone_ldplayer` → copy existing instance
   - `shield_remove_ldplayer` → delete instance
   - `shield_modify_ldplayer` → change CPU/memory/resolution
   - `shield_update_ldplayer_meta` → update notes/tags/proxy in DB

### Frontend (Svelte 5)
4. **New view "emulators"** in ShieldBrowser nav
5. **`LDPlayerTable.svelte`** — shadcn Table with:
   - Columns: Name, Index, Status (🟢/⚪), CPU, RAM, Resolution, Notes, Tags, Actions
   - Row actions: Launch/Stop, Edit, Clone, Delete
   - Detail drawer with metadata editing
6. **Store additions** in `desktop/src/state/shield.ts`

## Inputs & Outputs
- **Input**: LDPlayer installation on Windows (path auto-detected)
- **Output**: Full CRUD management of LDPlayer instances from Shield Browser UI, with metadata persisted in SQLite

## Edge Cases & Security
- LDPlayer not installed → graceful fallback, show install guidance
- ldconsole.exe not found at typical paths → allow manual path configuration
- Instance running while deletion attempted → prevent with error
- DB ↔ live state drift → sync on every list call
- Cross-platform: LDPlayer is Windows-only; wrap in `cfg!(windows)` guards

## Task Checklist

### Backend — Core
- [x] 1. Create `core/src/shield/ldplayer.rs` — LDPlayer manager with auto-detect + all CLI wrappers
- [x] 2. Create `core/src/shield/ldplayer_db.rs` — SQLite DB for profile metadata
- [x] 3. Register both modules in `core/src/shield/mod.rs`
- [x] 4. Add unit tests for output parsing and DB CRUD

### Backend — Tauri Commands
- [x] 5. Add LDPlayer Tauri commands to `desktop/src-tauri/src/commands/shield.rs`
- [x] 6. Register commands in `desktop/src-tauri/src/lib.rs`

### Frontend — Store & Types
- [x] 7. Add LDPlayer types to `desktop/src/components/apps/ShieldBrowser/types.ts`
- [x] 8. Add LDPlayer state + actions to `desktop/src/state/shield.ts`

### Frontend — UI
- [x] 9. Create `LDPlayerTable.svelte` with full management table
- [x] 10. Add "Emulators" tab to `ShieldBrowser.svelte` nav
- [x] 11. Wire drawer for detail/edit view on row click

## Definition of Done
- [x] ldconsole auto-detection works on Windows with LDPlayer9
- [x] All CRUD operations work end-to-end (create, clone, launch, stop, remove, modify)
- [x] SQLite DB persists metadata (notes, tags, proxy) across sessions
- [x] Table UI shows live status + persisted metadata merged
- [x] DB ↔ live state sync handles drift gracefully
- [x] Graceful fallback when LDPlayer is not installed
- [x] No panics, no TODOs, no hardcoded paths (PathBuf::join only)
- [x] `cargo check -p ai-launcher-desktop` passes
