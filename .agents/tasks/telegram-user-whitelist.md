---
status: 🟢 done by AI
priority: high
tags: [security, telegram, channels]
---

# Telegram User Whitelist

## Goal
Restrict the Telegram bot to specific allowed user IDs, preventing unauthorized strangers from accessing the agent and executing tools on the host machine.

## Why
Currently, anyone who discovers the bot's username can send commands and interact with the AI agent. This is a major security risk since the agent has file I/O, shell, and other powerful tools.

## Proposed Implementation

### Backend (`core/src/channels/telegram.rs` or `server/src/telegram_gateway.rs`)
- [x] Add `allowed_user_ids: Vec<i64>` to `TelegramGatewayConfig`
- [x] Store in `channels.json` under `telegram.allowed_users`
- [x] Check `msg.from.id` against whitelist before processing
- [x] If not whitelisted, reply with "⛔ Unauthorized" and log the attempt
- [x] Empty whitelist = allow all (backward compat, but warn in logs)

### Frontend (`desktop/src/components/apps/Channels/Channels.svelte`)
- [x] Add "Allowed Users" input field (comma-separated Telegram user IDs)
- [x] Show current user's ID when they first connect (via getMe + getUpdates)
- [x] Save to channels.json via configure_channel API

### Reference
- OpenFang uses allowlisted user IDs in their channel config
- IronClaw has per-user permissions in their WASM channel system
- Telegram user ID can be found via @userinfobot or from the `from.id` field in messages
