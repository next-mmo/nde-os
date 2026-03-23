# Codex OAuth — Sign in with ChatGPT

Add browser-based OAuth authentication for OpenAI Codex, letting users sign in with their ChatGPT Plus/Pro subscription instead of needing an API key.

## User Review Required

> [!IMPORTANT]
> This adds 3 new server endpoints and a new Rust module. The OAuth flow opens the user's browser to `auth.openai.com` and runs a temporary local HTTP listener on port `14551` for the callback.

> [!WARNING]
> The OAuth client ID and auth endpoints are based on the public Codex CLI implementation. If OpenAI changes these, the flow may break.

## Proposed Changes

### OAuth Module (Rust Core)

#### [NEW] [codex_oauth.rs](file:///c:/Users/MT-Staff/Documents/GitHub/nde-os/core/src/llm/codex_oauth.rs)

New module implementing the full OAuth PKCE flow:

1. **`CodexOAuthFlow`** struct:
   - [start()](file:///c:/Users/MT-Staff/Documents/GitHub/nde-os/desktop/src/lib/api/backend.ts#242-245) → generates PKCE challenge, state, builds auth URL (`https://auth.openai.com/oauth/authorize`), starts local callback server on port `14551`
   - `exchange_code(code, verifier)` → POST to `https://auth.openai.com/oauth/token` to exchange auth code for access + refresh tokens
   - [refresh(refresh_token)](file:///c:/Users/MT-Staff/Documents/GitHub/nde-os/desktop/src/lib/stores/state.ts#87-98) → auto-refresh expired access tokens

2. **`CodexTokenStore`** struct:
   - Persists tokens to `{data_dir}/codex_tokens.json`
   - Fields: `access_token`, `refresh_token`, `expires_at`, `account_email`
   - [load()](file:///c:/Users/MT-Staff/Documents/GitHub/nde-os/desktop/src/lib/api/backend.ts#180-183) / `save()` / `is_expired()` / `get_valid_token()` (auto-refreshes if expired)

3. **`CodexOAuthProvider`** — implements [LlmProvider](file:///c:/Users/MT-Staff/Documents/GitHub/nde-os/core/src/llm/mod.rs#85-117) trait:
   - Wraps [OpenAiCompatProvider](file:///c:/Users/MT-Staff/Documents/GitHub/nde-os/core/src/llm/openai_compat.rs#6-12) but injects OAuth token as bearer auth
   - Auto-refreshes token before each request if expired
   - Uses `https://api.openai.com/v1` as base URL

---

### Provider Factory Update

#### [MODIFY] [mod.rs](file:///c:/Users/MT-Staff/Documents/GitHub/nde-os/core/src/llm/mod.rs)

- Add `pub mod codex_oauth;`
- Add `"codex_oauth"` match arm in [create_provider()](file:///c:/Users/MT-Staff/Documents/GitHub/nde-os/core/src/llm/mod.rs#120-202) — creates `CodexOAuthProvider` loading tokens from disk

---

### Provider Config Update

#### [MODIFY] [manager.rs](file:///c:/Users/MT-Staff/Documents/GitHub/nde-os/core/src/llm/manager.rs)

- Add `oauth_token: Option<String>` to [ProviderConfig](file:///c:/Users/MT-Staff/Documents/GitHub/nde-os/desktop/src/lib/api/types.ts#153-162) for storing OAuth access tokens at runtime

---

### Server API Endpoints

#### [MODIFY] [model_handler.rs](file:///c:/Users/MT-Staff/Documents/GitHub/nde-os/server/src/model_handler.rs)

Add 5 new handlers:

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/models/providers` | Add a new provider from config JSON |
| `DELETE` | `/api/models/providers/{name}` | Remove a provider |
| `POST` | `/api/codex/oauth/start` | Start OAuth flow → returns `{ auth_url }` |
| `GET` | `/api/codex/oauth/callback` | Handles browser redirect with auth code |
| `GET` | `/api/codex/oauth/status` | Returns `{ authenticated, email, plan }` |

The `/start` endpoint spawns a temporary HTTP listener on port `14551`, generates the PKCE auth URL, and opens the user's default browser. The `/callback` endpoint is the redirect target — it receives the auth code, exchanges it for tokens, persists them, and auto-adds a `codex_oauth` provider to the LLM manager.

#### [MODIFY] [main.rs](file:///c:/Users/MT-Staff/Documents/GitHub/nde-os/server/src/main.rs)

- Add routes for the 5 new endpoints in the [route()](file:///c:/Users/MT-Staff/Documents/GitHub/nde-os/server/src/main.rs#34-163) function
- Pass `data_dir` to model handlers that need it for token storage

---

### Desktop UI

#### [MODIFY] [ModelSettings.svelte](file:///c:/Users/MT-Staff/Documents/GitHub/nde-os/desktop/src/components/apps/ModelSettings/ModelSettings.svelte)

- Add **"Sign in with ChatGPT"** button in the Add Provider form when `formType === "codex"`
- Clicking it calls `POST /api/codex/oauth/start`, which opens browser
- Poll `GET /api/codex/oauth/status` until authenticated
- Show auth status badge (email, plan type) on Codex provider cards

#### [MODIFY] [backend.ts](file:///c:/Users/MT-Staff/Documents/GitHub/nde-os/desktop/src/lib/api/backend.ts)

- Add `codexOAuthStart()`, `codexOAuthStatus()` API functions

#### [MODIFY] [types.ts](file:///c:/Users/MT-Staff/Documents/GitHub/nde-os/desktop/src/lib/api/types.ts)

- Add `CodexOAuthStatus` interface: `{ authenticated: boolean, email?: string, plan_type?: string }`

---

## Verification Plan

### Manual Verification
1. Start dev server (`bash dev.sh`)
2. Open Model Settings → select Codex provider type
3. Click "Sign in with ChatGPT" — verify browser opens to `auth.openai.com`
4. Complete login → verify callback auto-adds Codex provider
5. Check provider card shows auth status (email, plan)
6. Send a chat message → verify it uses the Codex OAuth token
7. Restart server → verify tokens persist and auto-load
