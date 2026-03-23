<svelte:options runes={true} />

<script lang="ts">
  import * as api from "$lib/api/backend";
  import type { ProviderStatus, ProviderConfig } from "$lib/api/types";

  let providers = $state<ProviderStatus[]>([]);
  let activeProvider = $state("");
  let loading = $state(true);
  let switching = $state(false);
  let showAddForm = $state(false);
  let addError = $state("");
  let addLoading = $state(false);
  let codexOAuthLoading = $state(false);
  let codexOAuthStatus = $state<api.CodexOAuthStatus | null>(null);

  // Add provider form
  let formName = $state("");
  let formType = $state("ollama");
  let formModel = $state("llama3.2");
  let formBaseUrl = $state("");
  let formApiKey = $state("");
  let formApiKeyEnv = $state("");
  let formMaxTokens = $state(4096);

  const PROVIDER_TYPES = [
    { value: "gguf", label: "GGUF (Local, No Setup)", icon: "📦" },
    { value: "ollama", label: "Ollama (Local)", icon: "🦙" },
    { value: "openai", label: "OpenAI", icon: "🤖" },
    { value: "anthropic", label: "Anthropic", icon: "🧠" },
    { value: "groq", label: "Groq", icon: "⚡" },
    { value: "together", label: "Together AI", icon: "🤝" },
    { value: "codex", label: "Codex", icon: "💻" },
  ];

  const PROVIDER_DEFAULTS: Record<string, { model: string; baseUrl: string }> = {
    gguf: { model: "tinyllama-1.1b-chat-v1.0", baseUrl: "" },
    ollama: { model: "llama3.2", baseUrl: "http://localhost:11434" },
    openai: { model: "gpt-4o", baseUrl: "https://api.openai.com" },
    anthropic: { model: "claude-sonnet-4-20250514", baseUrl: "https://api.anthropic.com" },
    groq: { model: "llama-3.3-70b-versatile", baseUrl: "https://api.groq.com" },
    together: { model: "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo", baseUrl: "https://api.together.xyz" },
    codex: { model: "codex-mini-latest", baseUrl: "https://api.openai.com" },
  };

  $effect(() => {
    refresh();
  });

  async function refresh() {
    loading = true;
    try {
      const [provs, active, oauthStatus] = await Promise.all([
        api.listModels(),
        api.activeModel(),
        api.codexOAuthStatus().catch(() => null),
      ]);
      providers = provs;
      activeProvider = active;
      codexOAuthStatus = oauthStatus;
    } catch {
      providers = [];
    } finally {
      loading = false;
    }
  }

  async function handleSwitch(name: string) {
    if (name === activeProvider || switching) return;
    switching = true;
    try {
      await api.switchModel(name);
      activeProvider = name;
      await refresh();
    } catch (e: any) {
      console.error("Switch failed:", e);
    } finally {
      switching = false;
    }
  }

  function handleTypeChange() {
    const defs = PROVIDER_DEFAULTS[formType];
    if (defs) {
      formModel = defs.model;
      formBaseUrl = defs.baseUrl;
    }
  }

  async function handleAdd() {
    if (!formName.trim() || !formModel.trim()) {
      addError = "Name and model are required";
      return;
    }
    addLoading = true;
    addError = "";
    try {
      const config: ProviderConfig = {
        name: formName.trim(),
        provider_type: formType,
        model: formModel.trim(),
        base_url: formBaseUrl.trim() || undefined,
        api_key: formApiKey.trim() || undefined,
        api_key_env: formApiKeyEnv.trim() || undefined,
        max_tokens: formMaxTokens,
      };
      await api.addProvider(config);
      resetForm();
      showAddForm = false;
      await refresh();
    } catch (e: any) {
      addError = e.message || "Failed to add provider";
    } finally {
      addLoading = false;
    }
  }

  async function handleRemove(name: string) {
    try {
      await api.removeProvider(name);
      await refresh();
    } catch (e: any) {
      console.error("Remove failed:", e);
    }
  }

  function resetForm() {
    formName = "";
    formType = "ollama";
    formModel = "llama3.2";
    formBaseUrl = "";
    formApiKey = "";
    formApiKeyEnv = "";
    formMaxTokens = 4096;
    addError = "";
  }

  function getProviderIcon(type: string): string {
    return PROVIDER_TYPES.find(p => p.value === type)?.icon ?? "🔌";
  }

  async function handleCodexLogin() {
    codexOAuthLoading = true;
    addError = "";
    try {
      const result = await api.codexOAuthStart();
      // Open auth URL in a new browser tab (works in both browser and Tauri)
      window.open(result.auth_url, "_blank", "noopener,noreferrer");
      // Poll for completion
      let attempts = 0;
      const poll = setInterval(async () => {
        attempts++;
        try {
          const status = await api.codexOAuthStatus();
          if (status.authenticated) {
            clearInterval(poll);
            codexOAuthLoading = false;
            codexOAuthStatus = status;
            showAddForm = false;
            await refresh();
          }
        } catch { /* keep polling */ }
        if (attempts > 120) {
          clearInterval(poll);
          codexOAuthLoading = false;
          addError = "OAuth timed out — try again";
        }
      }, 2000);
    } catch (e: any) {
      addError = e.message || "Failed to start OAuth flow";
      codexOAuthLoading = false;
    }
  }
</script>

<section class="model-settings">
  <div class="header">
    <div>
      <p class="eyebrow">AI Configuration</p>
      <h2>LLM Providers</h2>
    </div>
    <div class="header-actions">
      <button class="action-btn refresh-btn" onclick={refresh} disabled={loading}>
        {loading ? "Loading..." : "↻ Refresh"}
      </button>
      <button class="action-btn add-btn" onclick={() => { showAddForm = !showAddForm; resetForm(); }}>
        {showAddForm ? "✕ Cancel" : "+ Add Provider"}
      </button>
    </div>
  </div>

  {#if showAddForm}
    <div class="add-form">
      <h3>Add New Provider</h3>
      <div class="form-grid">
        <label class="field">
          <span>Name</span>
          <input type="text" bind:value={formName} placeholder="e.g. my-openai" />
        </label>
        <label class="field">
          <span>Provider Type</span>
          <select bind:value={formType} onchange={handleTypeChange}>
            {#each PROVIDER_TYPES as pt}
              <option value={pt.value}>{pt.icon} {pt.label}</option>
            {/each}
          </select>
        </label>
        <label class="field">
          <span>Model</span>
          <input type="text" bind:value={formModel} placeholder="e.g. gpt-4o" />
        </label>
        <label class="field">
          <span>Base URL</span>
          <input type="text" bind:value={formBaseUrl} placeholder="Optional" />
        </label>
        <label class="field">
          <span>API Key</span>
          <input type="password" bind:value={formApiKey} placeholder="Direct key (optional)" />
        </label>
        <label class="field">
          <span>API Key Env Var</span>
          <input type="text" bind:value={formApiKeyEnv} placeholder="e.g. OPENAI_API_KEY" />
        </label>
        <label class="field">
          <span>Max Tokens</span>
          <input type="number" bind:value={formMaxTokens} min="256" max="128000" />
        </label>
      </div>
      {#if addError}
        <p class="form-error">{addError}</p>
      {/if}
      <div class="form-actions">
        <button class="submit-btn" onclick={handleAdd} disabled={addLoading}>
          {addLoading ? "Adding..." : "Add Provider"}
        </button>
        {#if formType === "codex"}
          <button
            class="submit-btn oauth-btn"
            onclick={handleCodexLogin}
            disabled={codexOAuthLoading}
          >
            {codexOAuthLoading ? "⏳ Waiting for login..." : "🔑 Sign in with ChatGPT"}
          </button>
        {/if}
        {#if codexOAuthStatus?.authenticated}
          <span class="oauth-badge">
            ✓ {codexOAuthStatus.email || "Authenticated"}
            {#if codexOAuthStatus.plan_type}
              · {codexOAuthStatus.plan_type}
            {/if}
          </span>
        {/if}
      </div>
    </div>
  {/if}

  {#if loading}
    <div class="loading">Loading providers...</div>
  {:else if providers.length === 0}
    <div class="empty-state">
      <div class="empty-icon">🤖</div>
      <h3>No Providers Configured</h3>
      <p>Add an LLM provider to power the agent. Ollama is added by default on server start.</p>
      <button class="action-btn add-btn" onclick={() => showAddForm = true}>+ Add Provider</button>
    </div>
  {:else}
    <div class="providers-grid">
      {#each providers as provider (provider.name)}
        <article class="provider-card" class:active={provider.is_active} class:switching={switching && provider.name !== activeProvider}>
          <div class="provider-header">
            <span class="provider-icon">{getProviderIcon(provider.provider_type)}</span>
            <div class="provider-info">
              <strong>{provider.name}</strong>
              <span class="provider-type">{provider.provider_type}</span>
            </div>
            {#if provider.is_active}
              <span class="active-badge">Active</span>
            {/if}
          </div>
          <div class="provider-actions">
            {#if !provider.is_active}
              <button class="switch-btn" onclick={() => handleSwitch(provider.name)} disabled={switching}>
                {switching ? "..." : "Switch to"}
              </button>
            {:else}
              <span class="current-label">Current</span>
            {/if}
            <button class="remove-btn" onclick={() => handleRemove(provider.name)} title="Remove provider">
              ✕
            </button>
          </div>
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .model-settings {
    height: 100%;
    overflow: auto;
    padding: 1.1rem;
    display: grid;
    gap: 1rem;
    align-content: start;
  }
  .header { display: flex; justify-content: space-between; align-items: center; gap: 1rem; flex-wrap: wrap; }
  .eyebrow { margin: 0; font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.14em; color: var(--system-color-text-muted); }
  h2 { margin: 0.3rem 0 0; }
  h3 { margin: 0 0 0.75rem; font-size: 1rem; }
  .header-actions { display: flex; gap: 0.5rem; }
  .action-btn {
    border-radius: 999px; padding: 0.55rem 1rem; font-size: 0.82rem; cursor: pointer;
    border: 1px solid var(--system-color-border); background: var(--system-color-panel);
    color: var(--system-color-text); transition: all 0.15s;
  }
  .action-btn:hover { background: hsla(var(--system-color-primary-hsl) / 0.1); }
  .add-btn { background: hsla(var(--system-color-primary-hsl) / 0.12); border-color: hsla(var(--system-color-primary-hsl) / 0.25); color: var(--system-color-primary); }
  .add-form {
    padding: 1.2rem; border-radius: 1rem; border: 1px solid hsla(var(--system-color-primary-hsl) / 0.2);
    background: hsla(var(--system-color-primary-hsl) / 0.04);
  }
  .form-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr)); gap: 0.75rem; }
  .field { display: flex; flex-direction: column; gap: 0.3rem; }
  .field span { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--system-color-text-muted); }
  .field input, .field select {
    padding: 0.6rem 0.8rem; border-radius: 0.6rem; border: 1px solid var(--system-color-border);
    background: var(--system-color-panel); color: var(--system-color-text); font-size: 0.88rem; font-family: inherit;
  }
  .field input:focus, .field select:focus { border-color: var(--system-color-primary); outline: none; box-shadow: 0 0 0 2px hsla(var(--system-color-primary-hsl) / 0.15); }
  .form-error { margin: 0.5rem 0 0; color: var(--system-color-danger); font-size: 0.82rem; }
  .submit-btn {
    margin-top: 0.85rem; padding: 0.65rem 1.5rem; border-radius: 999px; border: none;
    background: var(--system-color-primary); color: white; font-size: 0.88rem; font-weight: 600; cursor: pointer;
  }
  .submit-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .providers-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr)); gap: 0.85rem; }
  .provider-card {
    border-radius: 1.1rem; padding: 1rem; border: 1px solid var(--system-color-border);
    background: var(--system-color-panel); transition: all 0.2s; display: flex; flex-direction: column; gap: 0.75rem;
  }
  .provider-card:hover { border-color: hsla(var(--system-color-primary-hsl) / 0.3); }
  .provider-card.active { border-color: var(--system-color-primary); background: hsla(var(--system-color-primary-hsl) / 0.06); }
  .provider-header { display: flex; align-items: center; gap: 0.6rem; }
  .provider-icon { font-size: 1.5rem; }
  .provider-info { display: flex; flex-direction: column; flex: 1; }
  .provider-info strong { font-size: 0.92rem; }
  .provider-type { font-size: 0.72rem; color: var(--system-color-text-muted); text-transform: uppercase; letter-spacing: 0.08em; }
  .active-badge {
    padding: 0.2rem 0.6rem; border-radius: 999px; font-size: 0.7rem; font-weight: 600;
    background: hsla(160 60% 50% / 0.15); color: hsl(160 60% 50%);
  }
  .provider-actions { display: flex; align-items: center; gap: 0.5rem; }
  .switch-btn {
    padding: 0.4rem 0.8rem; border-radius: 999px; border: 1px solid hsla(var(--system-color-primary-hsl) / 0.3);
    background: hsla(var(--system-color-primary-hsl) / 0.1); color: var(--system-color-primary);
    font-size: 0.78rem; cursor: pointer; transition: all 0.15s;
  }
  .switch-btn:hover { background: hsla(var(--system-color-primary-hsl) / 0.2); }
  .switch-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .current-label { font-size: 0.78rem; color: hsl(160 60% 50%); font-weight: 600; }
  .remove-btn {
    margin-left: auto; width: 1.8rem; height: 1.8rem; border-radius: 50%; border: none;
    background: hsla(0 70% 55% / 0.1); color: hsl(0 70% 60%); cursor: pointer; font-size: 0.8rem;
    display: flex; align-items: center; justify-content: center; transition: all 0.15s;
  }
  .remove-btn:hover { background: hsla(0 70% 55% / 0.25); }
  .loading, .empty-state { text-align: center; padding: 2rem 1rem; color: var(--system-color-text-muted); }
  .empty-state { display: flex; flex-direction: column; align-items: center; gap: 0.5rem; }
  .empty-icon { font-size: 3rem; }
  .empty-state h3 { margin: 0; color: var(--system-color-text); }
  .empty-state p { margin: 0; max-width: 320px; font-size: 0.85rem; }

  .form-actions { display: flex; align-items: center; gap: 0.75rem; flex-wrap: wrap; margin-top: 0.85rem; }
  .form-actions .submit-btn { margin-top: 0; }
  .oauth-btn {
    background: hsl(160 60% 42%); color: white;
  }
  .oauth-btn:hover:not(:disabled) { background: hsl(160 60% 36%); }
  .oauth-badge {
    font-size: 0.78rem; font-weight: 600;
    color: hsl(160 60% 45%);
    padding: 0.35rem 0.75rem;
    border-radius: 999px;
    background: hsla(160 60% 50% / 0.12);
  }
</style>
