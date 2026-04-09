# Shield Android Device Profiles

**Status:** 🟢 done by AI

## Goal

Extend Shield Browser to support Android emulator/device profiles alongside desktop browser profiles. Users can manage LDPlayer, Android Studio AVDs, real USB devices, and other ADB-connected Android devices — all from the same Shield Browser UI.

## Architecture

The Rust `core/src/shield/emulator.rs` already has a solid `EmulatorManager` with:
- ADB device discovery (`list_devices`, `list_emulators`)
- AVD listing (`list_avds`)
- AVD launch/stop lifecycle
- ADB TCP connect (LDPlayer, Nox, etc.)
- Proxy push/clear via ADB shell
- Screenshot capture via ADB
- Boot readiness polling

## Checklist

### Phase 1: Tauri Commands (wire up EmulatorManager)

- [x] Add `shield_adb_status` — checks if ADB/emulator binaries are available
- [x] Add `shield_list_android_devices` — returns all ADB devices with type classification
- [x] Add `shield_list_avds` — returns available AVDs for launch dropdown
- [x] Add `shield_launch_avd` — launches an Android Studio AVD
- [x] Add `shield_stop_device` — stops emulator or disconnects network device
- [x] Add `shield_adb_connect` — connects to TCP device (LDPlayer, Nox, etc.)
- [x] Add `shield_configure_proxy` — pushes proxy to device via ADB
- [x] Add `shield_clear_proxy` — clears proxy on device
- [x] Add `shield_device_screenshot` — takes screenshot and returns path
- [x] Add `shield_open_url_on_device` — opens URL in device browser
- [x] Register all new commands in `lib.rs`
- [x] `cargo check` passes, all emulator tests pass (8/8)

### Phase 2: Frontend UI — Devices Tab

- [x] "📱 Devices" button in header, opens devices view
- [x] Device list with status badges (online/offline/booting/unauthorized)
- [x] Device type indicators: 📱 USB, 🖥️ AVD, 🎮 LDPlayer/Nox, 🌐 TCP
- [x] Device detail panel with serial, type, status, AVD/model info
- [x] Quick actions: 📸 Screenshot, 🌐 Open URL (with URL input)
- [x] Proxy management: set host:port or clear
- [x] "🔌 Connect TCP" dialog with presets (LDPlayer #1-3, NoxPlayer)
- [x] "🚀 Launch AVD" dropdown (only when AVDs detected)
- [x] ADB not found state with installation instructions
- [x] Refresh button for device scanning

### Phase 3: Profile ↔ Device Linking

- [x] "Link to Profile" section in device detail (shows profiles with proxy configured)
- [x] Push Proxy button for each profile → device
- [x] Stop/Disconnect button with smart labeling per device type
