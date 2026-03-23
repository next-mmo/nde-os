# Codex OAuth — Sign in with ChatGPT

Uses the installed Codex CLI (`npm i -g @openai/codex`) as the auth backend. Instead of running our own PKCE OAuth flow, we read tokens from `~/.codex/auth.json` that the Codex CLI manages.

## Architecture

```
┌──────────────┐     codex login     ┌──────────────────┐
│  Codex CLI   │ ──────────────────► │ ~/.codex/auth.json│
│  (installed) │  handles PKCE,      │  access_token     │
│              │  callback server,   │  refresh_token    │
│              │  token refresh      │  id_token         │
└──────────────┘                     │  account_id       │
                                     └────────┬─────────┘
                                              │ reads
┌──────────────┐  POST /api/codex/   ┌────────▼─────────┐
│  AI Launcher │  oauth/start        │ codex_oauth.rs    │
│  Desktop UI  │ ──────────────────► │  read_codex_auth()│
│              │                     │  get_access_token │
│              │  ◄── provider added │  CodexOAuthProvider│
└──────────────┘       & activated   └──────────────────┘
```

## Flow

1. User has `@openai/codex` installed and has run `codex login`
2. In AI Launcher → LLM Providers → Add Provider → "Codex" → "Sign in with ChatGPT"
3. Backend reads `~/.codex/auth.json` — if `access_token` found:
   - Auto-creates `codex-chatgpt` provider (codex_oauth type)
   - Sets it as active provider
   - No browser needed
4. If no auth found — instructs user to run `codex login` in terminal

## Implementation Status — ALL COMPLETE ✅

| Component | File | Status |
|-----------|------|--------|
| OAuth Module (reads Codex CLI) | `core/src/llm/codex_oauth.rs` | ✅ Complete |
| Provider Factory | `core/src/llm/mod.rs` | ✅ Complete |
| Server API Endpoints | `server/src/model_handler.rs` | ✅ Complete |
| Desktop UI — ModelSettings | `desktop/src/components/apps/ModelSettings/ModelSettings.svelte` | ✅ Complete |
| Desktop API types | `desktop/src/lib/api/backend.ts` | ✅ Complete |

## Key Design Decisions

- **No custom OAuth flow** — Delegated to Codex CLI which already handles PKCE, callback server (port 1455), token refresh
- **No port conflicts** — We don't bind any ports; Codex CLI does
- **Token store is read-only** — We never write to `~/.codex/auth.json`, only read
- **JWT email extraction** — We decode the `id_token` JWT payload to show the user's email
- **Inspired by OpenCode** — OpenCode (anomalyco/opencode) uses Device Code Flow for their own server; Codex CLI uses PKCE for OpenAI's auth

## E2E Verification ✅

| Test | Result |
|------|--------|
| `cargo build -p ai-launcher-core -p ai-launcher-server` | ✅ Compiles |
| `cargo test -p ai-launcher-core -- llm` | ✅ 4/4 tests pass |
| `GET /api/codex/oauth/status` | ✅ `authenticated: true, email: hin.sinak@gmail.com` |
| `POST /api/codex/oauth/start` | ✅ `already_authenticated: true` — provider auto-added |
| `GET /api/models` | ✅ `codex-chatgpt` active, type `codex_oauth` |
| Browser E2E: Click "Sign in with ChatGPT" | ✅ No browser tab opened (CLI auth detected) |
| Browser E2E: Provider appears in list | ✅ Shows as active & current |
| Repeated clicks | ✅ Idempotent — no issues |

## API Endpoints

### POST /api/codex/oauth/start
Checks if Codex CLI has valid auth. If yes, auto-adds the provider.

**Response (authenticated):**
```json
{
  "success": true,
  "message": "Codex CLI already authenticated — provider added",
  "data": { "auth_url": "", "already_authenticated": true }
}
```

**Response (not authenticated):**
```json
{
  "success": true,
  "message": "Run `codex login` in your terminal to authenticate",
  "data": {
    "auth_url": "https://auth.openai.com/...",
    "already_authenticated": false,
    "message": "Install Codex CLI (npm i -g @openai/codex) and run `codex login`"
  }
}
```

### GET /api/codex/oauth/status
```json
{
  "data": {
    "authenticated": true,
    "email": "user@example.com",
    "plan_type": "ChatGPT Plus/Pro"
  }
}
```
