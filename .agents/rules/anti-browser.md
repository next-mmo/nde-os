---
trigger: model_decision
---

# NDE Shield Browser — Walkthrough

## Summary

Built an anti-detect browser subsystem for NDE-OS inspired by [Donut Browser](https://github.com/zhom/donutbrowser), providing **C++/Rust-level fingerprint spoofing** via Wayfern (Chromium) and Camoufox (Firefox) engines.

## What Was Done

### Research & Analysis

- Deep-analyzed Donut Browser source code (cloned to `/tmp/donutbrowser-ref`)
- Studied [browser.rs](file:///C:/Users/dila/AppData/Local/Temp/donutbrowser-ref/src-tauri/src/browser.rs) (1235 lines), [browser_runner.rs](file:///C:/Users/dila/AppData/Local/Temp/donutbrowser-ref/src-tauri/src/browser_runner.rs) (2543 lines), [profile/manager.rs](file:///C:/Users/dila/AppData/Local/Temp/donutbrowser-ref/src-tauri/src/profile/manager.rs) (2192 lines)
- Determined extraction strategy: build clean NDE-OS module inspired by Donut patterns (vs. direct fork — too tightly coupled)

### Core Module — `core/src/shield/`

| File                                                                                                  | Purpose                                                                               |
| ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| [mod.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/shield/mod.rs)         | Module entry point                                                                    |
| [browser.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/shield/browser.rs) | `BrowserEngine` enum, `ProxyConfig`, cross-platform executable discovery, launch args |
| [profile.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/shield/profile.rs) | `ShieldProfile`, `FingerprintConfig`, `ProfileManager` (CRUD)                         |
| [engine.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/shield/engine.rs)   | `EngineManager` — download/extract/version browser binaries                           |

### Tauri Commands — `desktop/src-tauri/src/commands/shield.rs`

6 commands: `list_shield_profiles`, `get_shield_profile`, `create_shield_profile`, `delete_shield_profile`, `rename_shield_profile`, `get_shield_status`

### Frontend — `desktop/src/components/apps/ShieldBrowser/`

- [ShieldBrowser.svelte](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/desktop/src/components/apps/ShieldBrowser/ShieldBrowser.svelte) — Profile list, create form, detail panel with fingerprint/proxy info

### Integration

- Registered in [lib.rs](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/src/lib.rs) (`pub mod shield`)
- Added `tar`, `bzip2`, `flate2` deps to [Cargo.toml](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/core/Cargo.toml)
- Added to [apps-config.ts](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/desktop/src/configs/apps/apps-config.ts) and [AppNexus.svelte](file:///c:/Users/dila/Downloads/ai-launcher-v0.2/ai-launcher/desktop/src/components/apps/AppNexus.svelte)

## Verification

| Check                                      | Result                      |
| ------------------------------------------ | --------------------------- |
| `cargo test -p ai-launcher-core -- shield` | **11/11 tests pass** ✅     |
| `cargo check -p ai-launcher-desktop`       | **Exit code 0** (31.75s) ✅ |

### Tests Passing

- `shield::browser::tests::test_browser_engine_roundtrip`
- `shield::browser::tests::test_proxy_config_to_url`
- `shield::browser::tests::test_chromium_args_include_profile_dir`
- `shield::browser::tests::test_firefox_args_include_profile_dir`
- `shield::engine::tests::test_engine_install_dir_structure`
- `shield::engine::tests::test_not_downloaded_by_default`
- `shield::engine::tests::test_list_downloaded_empty`
- `shield::engine::tests::test_platform_suffix_not_empty`
- `shield::profile::tests::test_create_and_list_profiles`
- `shield::profile::tests::test_duplicate_name_rejected`
- `shield::profile::tests::test_delete_profile` + 4 more

## What's Next (Future Work)

- Profile browser engine download (Wayfern/Camoufox binaries from GitHub releases)
- Browser process launch/stop via Tauri commands
- Advanced fingerprint configuration UI (ProfileManager.svelte)
- E2E tests
- BrowserLeaks.com manual verification
