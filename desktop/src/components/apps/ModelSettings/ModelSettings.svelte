<svelte:options runes={true} />

<script lang="ts">
  import * as api from "$lib/api/backend";
  import type { ProviderStatus, ProviderConfig } from "$lib/api/types";
  import type { LocalGgufModel, VerifyResult } from "$lib/api/backend";

  let providers = $state<ProviderStatus[]>([]);
  let activeProvider = $state("");
  let loading = $state(true);
  let switching = $state(false);
  let showAddForm = $state(false);
  let addError = $state("");
  let addLoading = $state(false);
  let testLoading = $state(false);
  let testResult = $state<{ ok: boolean; message: string } | null>(null);
  let connectionVerified = $state(false);
  let codexOAuthLoading = $state(false);
  let codexOAuthStatus = $state<api.CodexOAuthStatus | null>(null);

  // GGUF Recommendations
  let recommendations = $state<api.GgufModelRecommendation[]>([]);
  let localModels = $state<LocalGgufModel[]>([]);
  let recLoading = $state(false);
  let verifyResult = $state<VerifyResult | null>(null);
  let verifying = $state(false);

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
    { value: "openai_compat", label: "OpenAI Compatible", icon: "🔌" },
    { value: "anthropic", label: "Anthropic", icon: "🧠" },
    { value: "groq", label: "Groq", icon: "⚡" },
    { value: "together", label: "Together AI", icon: "🤝" },
    { value: "openrouter", label: "OpenRouter", icon: "🌐" },
    { value: "codex", label: "Codex", icon: "💻" },
    { value: "codex_oauth", label: "Codex (ChatGPT)", icon: "💻" },
    { value: "omx", label: "oh-my-codex (OMX)", icon: "🚀" },
  ];

  const PROVIDER_DEFAULTS: Record<string, { model: string; baseUrl: string }> = {
    gguf: { model: "", baseUrl: "" },
    ollama: { model: "llama3.2", baseUrl: "http://localhost:11434" },
    openai: { model: "gpt-4o", baseUrl: "https://api.openai.com" },
    openai_compat: { model: "", baseUrl: "" },
    anthropic: { model: "claude-sonnet-4-20250514", baseUrl: "https://api.anthropic.com" },
    groq: { model: "llama-3.3-70b-versatile", baseUrl: "https://api.groq.com" },
    together: { model: "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo", baseUrl: "https://api.together.xyz" },
    openrouter: { model: "openai/gpt-4o", baseUrl: "https://openrouter.ai/api" },
    codex: { model: "gpt-4o-mini", baseUrl: "https://api.openai.com" },
    omx: { model: "gpt-4o", baseUrl: "" },
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

  async function handleTypeChange() {
    const defs = PROVIDER_DEFAULTS[formType];
    if (defs) {
      formModel = defs.model;
      formBaseUrl = defs.baseUrl;
    }
    if (formType === "codex") {
      try {
        codexOAuthStatus = await api.codexOAuthStatus();
      } catch { /* ignore */ }
    } else if (formType === "omx") {
      // OMX uses same Codex OAuth auth
      try {
        codexOAuthStatus = await api.codexOAuthStatus();
      } catch { /* ignore */ }
    } else if (formType === "gguf" && recommendations.length === 0) {
      recLoading = true;
      try {
        const [recs, locals] = await Promise.all([
          api.recommendModels(),
          api.localModels(),
        ]);
        recommendations = recs;
        localModels = locals;
      } catch (e: any) {
        console.error("Failed to load recommendations:", e);
      } finally {
        recLoading = false;
      }
    }
  }

  function useRecommendation(rec: api.GgufModelRecommendation) {
    formName = `local-${rec.id.split("/").pop() || rec.id}`;
    formModel = rec.id;
    verifyResult = null;
  }

  function useLocalModel(model: LocalGgufModel) {
    const stem = model.filename.replace(/\.gguf$/i, "");
    formName = `local-${stem}`;
    formModel = stem;
    formBaseUrl = model.path;
    formMaxTokens = 4096;
    verifyResult = null;
  }

  function buildConfig(): ProviderConfig {
    return {
      name: formName.trim(),
      provider_type: formType,
      model: formModel.trim(),
      base_url: formBaseUrl.trim() || undefined,
      api_key: formApiKey.trim() || undefined,
      api_key_env: formApiKeyEnv.trim() || undefined,
      max_tokens: formMaxTokens,
    };
  }

  async function handleTestConnection() {
    if (!formModel.trim()) {
      addError = "Model is required to test connection";
      return;
    }
    testLoading = true;
    addError = "";
    testResult = null;
    connectionVerified = false;
    try {
      const config = buildConfig();
      const result = await api.verifyProvider(config);
      if (result.ok) {
        testResult = { ok: true, message: "Connection successful — provider is reachable and model is available" };
        connectionVerified = true;
      } else {
        testResult = { ok: false, message: result.error || "Verification failed" };
        connectionVerified = false;
      }
    } catch (e: any) {
      const msg = e.message || "Connection test failed";
      testResult = { ok: false, message: msg };
      connectionVerified = false;
    } finally {
      testLoading = false;
    }
  }

  async function handleAdd() {
    if (!formName.trim() || !formModel.trim()) {
      addError = "Name and model are required";
      return;
    }
    if (!connectionVerified) {
      addError = "Please test the connection first";
      return;
    }
    addLoading = true;
    addError = "";
    testResult = null;
    verifyResult = null;
    try {
      const config = buildConfig();
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
    verifyResult = null;
    verifying = false;
    testResult = null;
    testLoading = false;
    connectionVerified = false;
  }

  function getProviderIcon(type: string): string {
    return PROVIDER_TYPES.find(p => p.value === type)?.icon ?? "🔌";
  }

  async function handleCodexLogin() {
    codexOAuthLoading = true;
    addError = "";
    try {
      const result = await api.codexOAuthStart(formModel);

      if (result.already_authenticated) {
        codexOAuthLoading = false;
        codexOAuthStatus = await api.codexOAuthStatus();
        showAddForm = false;
        await refresh();
        return;
      }

      addError = result.message || "Complete the login flow in your browser to authenticate.";

      let attempts = 0;
      const poll = setInterval(async () => {
        attempts++;
        try {
          const status = await api.codexOAuthStatus();
          if (status.authenticated) {
            clearInterval(poll);
            codexOAuthLoading = false;
            codexOAuthStatus = status;
            addError = "";
            showAddForm = false;
            await api.codexOAuthStart(formModel);
            await refresh();
          }
        } catch { /* keep polling */ }
        if (attempts > 120) {
          clearInterval(poll);
          codexOAuthLoading = false;
          addError = "OAuth timed out — run `codex login` in your terminal";
        }
      }, 2000);
    } catch (e: any) {
      addError = e.message || "Failed to start OAuth flow";
      codexOAuthLoading = false;
    }
  }
</script>

<section class="h-full overflow-auto p-4 flex flex-col gap-4">
  <!-- Header -->
  <div class="flex justify-between items-center gap-4 flex-wrap">
    <div>
      <p class="m-0 text-[0.72rem] uppercase tracking-[0.14em] text-gray-500 dark:text-gray-400">AI Configuration</p>
      <h2 class="m-0 mt-1 text-lg font-bold text-black dark:text-white">LLM Providers</h2>
    </div>
    <div class="flex gap-2">
      <button
        class="rounded-full px-4 py-2 text-[0.82rem] cursor-pointer border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white transition-all duration-150 hover:bg-blue-500/10 disabled:opacity-50 disabled:cursor-not-allowed"
        onclick={refresh}
        disabled={loading}
      >
        {loading ? "Loading..." : "↻ Refresh"}
      </button>
      <button
        class="rounded-full px-4 py-2 text-[0.82rem] cursor-pointer border border-blue-500/25 bg-blue-500/12 text-blue-500 font-medium transition-all duration-150 hover:bg-blue-500/20"
        onclick={() => { showAddForm = !showAddForm; resetForm(); }}
      >
        {showAddForm ? "✕ Cancel" : "+ Add Provider"}
      </button>
    </div>
  </div>

  <!-- Add Provider Form -->
  {#if showAddForm}
    <div class="p-5 rounded-2xl border border-blue-500/20 bg-blue-500/4">
      <h3 class="m-0 mb-3 text-sm font-semibold text-black dark:text-white">Add New Provider</h3>

      <!-- Provider type selector -->
      <div class="grid grid-cols-1 gap-3 mb-3">
        <label class="flex flex-col gap-1">
          <span class="text-[0.75rem] uppercase tracking-widest text-gray-500 dark:text-gray-400">Provider Type</span>
          <select
            bind:value={formType}
            onchange={handleTypeChange}
            class="px-3 py-2.5 rounded-lg border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white text-[0.88rem] font-inherit focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/15"
          >
            {#each PROVIDER_TYPES as pt}
              <option value={pt.value}>{pt.icon} {pt.label}</option>
            {/each}
          </select>
        </label>
      </div>

      {#if formType === "codex"}
        <!-- Codex panel -->
        <div class="flex flex-col gap-3">
          <div class="flex gap-3 items-start p-3.5 rounded-xl bg-blue-500/6 border border-blue-500/15">
            <div class="text-3xl shrink-0">💻</div>
            <div>
              <strong class="block text-[0.92rem] mb-0.5 text-black dark:text-white">Codex (ChatGPT Plus/Pro)</strong>
              <p class="m-0 text-[0.8rem] text-gray-500 dark:text-gray-400">Use your ChatGPT Plus or Pro subscription — no API key needed.</p>
            </div>
          </div>

          <!-- OAuth status -->
          {#if codexOAuthStatus?.authenticated}
            <div class="flex gap-2.5 items-center px-3.5 py-2.5 rounded-xl bg-emerald-500/8 border border-emerald-500/20">
              <span class="text-xl text-emerald-500">✓</span>
              <div>
                <strong class="block text-[0.85rem] text-black dark:text-white">{codexOAuthStatus.email || "Authenticated"}</strong>
                {#if codexOAuthStatus.plan_type}
                  <span class="text-[0.72rem] text-emerald-500 font-semibold uppercase tracking-[0.05em]">{codexOAuthStatus.plan_type}</span>
                {/if}
              </div>
            </div>
          {:else}
            <div class="flex gap-2.5 items-center px-3.5 py-2.5 rounded-xl bg-amber-500/8 border border-amber-500/20">
              <span class="text-xl">🔒</span>
              <div>
                <strong class="block text-[0.85rem] text-black dark:text-white">Not authenticated</strong>
                <span class="block text-[0.78rem] text-gray-500 dark:text-gray-400 mt-0.5">Sign in with your ChatGPT account first</span>
              </div>
              <button
                class="ml-auto px-4 py-1.5 rounded-full border border-blue-500/30 bg-blue-500/12 text-blue-500 text-[0.78rem] font-semibold cursor-pointer transition-colors duration-150 hover:bg-blue-500/20 disabled:opacity-50 disabled:cursor-not-allowed"
                onclick={handleCodexLogin}
                disabled={codexOAuthLoading}
              >
                {codexOAuthLoading ? "⏳ Waiting..." : "🔑 Sign in"}
              </button>
            </div>
          {/if}

          <!-- Form fields (same pattern as others) -->
          <div class="grid grid-cols-[repeat(auto-fit,minmax(14rem,1fr))] gap-3">
            <label class="flex flex-col gap-1">
              <span class="text-[0.75rem] uppercase tracking-widest text-gray-500 dark:text-gray-400">Name</span>
              <input type="text" bind:value={formName} placeholder="e.g. my-codex"
                class="px-3 py-2.5 rounded-lg border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white text-[0.88rem] focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/15" />
            </label>
            <label class="flex flex-col gap-1">
              <span class="text-[0.75rem] uppercase tracking-widest text-gray-500 dark:text-gray-400">Model</span>
              <input type="text" bind:value={formModel} placeholder="e.g. gpt-4o-mini"
                class="px-3 py-2.5 rounded-lg border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white text-[0.88rem] focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/15" />
            </label>
            <label class="flex flex-col gap-1">
              <span class="text-[0.75rem] uppercase tracking-widest text-gray-500 dark:text-gray-400">Max Tokens</span>
              <input type="number" bind:value={formMaxTokens} min="256" max="128000"
                class="px-3 py-2.5 rounded-lg border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white text-[0.88rem] focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/15" />
            </label>
          </div>

          {#if addError}
            <div class="flex gap-2 items-start px-3 py-2.5 rounded-xl mt-2 bg-red-500/10 border border-red-500/25">
              <span class="text-base shrink-0 leading-snug">⚠</span>
              <p class="m-0 text-red-400 text-[0.82rem] leading-snug">{addError}</p>
            </div>
          {/if}
          {#if testResult}
            <div class="flex gap-2.5 items-start px-3 py-2.5 rounded-xl mt-2 {testResult.ok ? 'bg-emerald-500/8 border border-emerald-500/20' : 'bg-red-500/8 border border-red-500/20'}">
              <span class="text-lg shrink-0 leading-none {testResult.ok ? 'text-emerald-500' : 'text-red-500'}">{testResult.ok ? '✓' : '✕'}</span>
              <div>
                <strong class="block text-[0.85rem] mb-0.5 text-black dark:text-white">{testResult.ok ? 'Connected' : 'Connection Failed'}</strong>
                <span class="text-[0.78rem] text-gray-500 dark:text-gray-400">{testResult.message}</span>
              </div>
            </div>
          {/if}

          <!-- Same buttons as all other providers -->
          <div class="flex items-center gap-3 flex-wrap mt-3">
            <button
              class="px-5 py-2.5 rounded-full border border-blue-500/30 bg-blue-500/12 text-blue-500 text-[0.88rem] font-semibold cursor-pointer transition-colors duration-150 hover:bg-blue-500/20 disabled:opacity-50 disabled:cursor-not-allowed"
              onclick={handleTestConnection}
              disabled={testLoading || addLoading}
            >
              {testLoading ? "⏳ Testing..." : connectionVerified ? "✓ Re-test Connection" : "🔌 Test Connection"}
            </button>
            <button
              class="px-5 py-2.5 rounded-full border-none bg-blue-500 text-white text-[0.88rem] font-semibold cursor-pointer transition-colors duration-150 hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed"
              onclick={handleCodexLogin}
              disabled={addLoading || codexOAuthLoading || !connectionVerified}
            >
              {codexOAuthLoading ? "⏳ Adding..." : "Add Provider"}
            </button>
          </div>
        </div>
      {:else if formType === "omx"}
        <!-- oh-my-codex (OMX) panel -->
        <div class="flex flex-col gap-3">
          <div class="flex gap-3 items-start p-3.5 rounded-xl bg-violet-500/6 border border-violet-500/15">
            <div class="text-3xl shrink-0">🚀</div>
            <div>
              <strong class="block text-[0.92rem] mb-0.5 text-black dark:text-white">oh-my-codex (OMX)</strong>
              <p class="m-0 text-[0.8rem] text-gray-500 dark:text-gray-400">Multi-agent orchestration for Codex CLI — adds enhanced prompts, skills ($plan, $architect, $team), and workflow runtime.</p>
              <p class="m-0 mt-1 text-[0.72rem] text-gray-400 dark:text-gray-500">Requires: <code class="px-1 py-0.5 rounded bg-black/8 dark:bg-white/8 text-[0.7rem]">npm install -g @openai/codex oh-my-codex</code></p>
            </div>
          </div>

          <!-- Codex OAuth status (OMX reuses the same auth) -->
          {#if codexOAuthStatus?.authenticated}
            <div class="flex gap-2.5 items-center px-3.5 py-2.5 rounded-xl bg-emerald-500/8 border border-emerald-500/20">
              <span class="text-xl text-emerald-500">✓</span>
              <div>
                <strong class="block text-[0.85rem] text-black dark:text-white">{codexOAuthStatus.email || "Authenticated"}</strong>
                {#if codexOAuthStatus.plan_type}
                  <span class="text-[0.72rem] text-emerald-500 font-semibold uppercase tracking-[0.05em]">{codexOAuthStatus.plan_type}</span>
                {/if}
              </div>
            </div>
          {:else}
            <div class="flex gap-2.5 items-center px-3.5 py-2.5 rounded-xl bg-amber-500/8 border border-amber-500/20">
              <span class="text-xl">🔒</span>
              <div>
                <strong class="block text-[0.85rem] text-black dark:text-white">Not authenticated</strong>
                <span class="block text-[0.78rem] text-gray-500 dark:text-gray-400 mt-0.5">OMX uses Codex auth — sign in with your ChatGPT account or run <code class="px-1 py-0.5 rounded bg-black/8 dark:bg-white/8 text-[0.7rem]">codex login</code></span>
              </div>
              <button
                class="ml-auto px-4 py-1.5 rounded-full border border-violet-500/30 bg-violet-500/12 text-violet-500 text-[0.78rem] font-semibold cursor-pointer transition-colors duration-150 hover:bg-violet-500/20 disabled:opacity-50 disabled:cursor-not-allowed"
                onclick={handleCodexLogin}
                disabled={codexOAuthLoading}
              >
                {codexOAuthLoading ? "⏳ Waiting..." : "🔑 Sign in"}
              </button>
            </div>
          {/if}

          <div class="grid grid-cols-[repeat(auto-fit,minmax(14rem,1fr))] gap-3">
            <label class="flex flex-col gap-1">
              <span class="text-[0.75rem] uppercase tracking-widest text-gray-500 dark:text-gray-400">Name</span>
              <input type="text" bind:value={formName} placeholder="e.g. my-omx"
                class="px-3 py-2.5 rounded-lg border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white text-[0.88rem] focus:border-violet-500 focus:outline-none focus:ring-2 focus:ring-violet-500/15" />
            </label>
            <label class="flex flex-col gap-1">
              <span class="text-[0.75rem] uppercase tracking-widest text-gray-500 dark:text-gray-400">Model</span>
              <input type="text" bind:value={formModel} placeholder="e.g. gpt-4o"
                class="px-3 py-2.5 rounded-lg border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white text-[0.88rem] focus:border-violet-500 focus:outline-none focus:ring-2 focus:ring-violet-500/15" />
            </label>
            <label class="flex flex-col gap-1">
              <span class="text-[0.75rem] uppercase tracking-widest text-gray-500 dark:text-gray-400">OMX Binary Path</span>
              <input type="text" bind:value={formBaseUrl} placeholder="Optional (defaults to omx on PATH)"
                class="px-3 py-2.5 rounded-lg border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white text-[0.88rem] focus:border-violet-500 focus:outline-none focus:ring-2 focus:ring-violet-500/15" />
            </label>
            <label class="flex flex-col gap-1">
              <span class="text-[0.75rem] uppercase tracking-widest text-gray-500 dark:text-gray-400">Max Tokens</span>
              <input type="number" bind:value={formMaxTokens} min="256" max="128000"
                class="px-3 py-2.5 rounded-lg border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white text-[0.88rem] focus:border-violet-500 focus:outline-none focus:ring-2 focus:ring-violet-500/15" />
            </label>
          </div>

          {#if addError}
            <div class="flex gap-2 items-start px-3 py-2.5 rounded-xl mt-2 bg-red-500/10 border border-red-500/25">
              <span class="text-base shrink-0 leading-snug">⚠</span>
              <p class="m-0 text-red-400 text-[0.82rem] leading-snug">{addError}</p>
            </div>
          {/if}
          {#if testResult}
            <div class="flex gap-2.5 items-start px-3 py-2.5 rounded-xl mt-2 {testResult.ok ? 'bg-emerald-500/8 border border-emerald-500/20' : 'bg-red-500/8 border border-red-500/20'}">
              <span class="text-lg shrink-0 leading-none {testResult.ok ? 'text-emerald-500' : 'text-red-500'}">{testResult.ok ? '✓' : '✕'}</span>
              <div>
                <strong class="block text-[0.85rem] mb-0.5 text-black dark:text-white">{testResult.ok ? 'Connected' : 'Connection Failed'}</strong>
                <span class="text-[0.78rem] text-gray-500 dark:text-gray-400">{testResult.message}</span>
              </div>
            </div>
          {/if}

          <div class="flex items-center gap-3 flex-wrap mt-3">
            <button
              class="px-5 py-2.5 rounded-full border border-violet-500/30 bg-violet-500/12 text-violet-500 text-[0.88rem] font-semibold cursor-pointer transition-colors duration-150 hover:bg-violet-500/20 disabled:opacity-50 disabled:cursor-not-allowed"
              onclick={handleTestConnection}
              disabled={testLoading || addLoading}
            >
              {testLoading ? "⏳ Testing..." : connectionVerified ? "✓ Re-test Connection" : "🔌 Test Connection"}
            </button>
            <button
              class="px-5 py-2.5 rounded-full border-none bg-violet-500 text-white text-[0.88rem] font-semibold cursor-pointer transition-colors duration-150 hover:bg-violet-600 disabled:opacity-50 disabled:cursor-not-allowed"
              onclick={handleAdd}
              disabled={addLoading || !connectionVerified}
            >
              {addLoading ? "Adding..." : "Add Provider"}
            </button>
          </div>
        </div>
      {:else if formType === "gguf"}
        <!-- GGUF panel -->
        <div class="flex flex-col gap-3">
          <div class="flex gap-3 items-start p-3.5 rounded-xl bg-indigo-500/6 border border-indigo-500/15">
            <div class="text-3xl shrink-0">📦</div>
            <div>
              <strong class="block text-[0.92rem] mb-0.5 text-black dark:text-white">GGUF (Local Inference)</strong>
              <p class="m-0 text-[0.8rem] text-gray-500 dark:text-gray-400">Runs directly on your machine. We recommend these models based on your {recommendations.length > 0 ? `${recommendations[0]?.recommended_ram_gb}GB+ RAM` : 'system RAM'}.</p>
            </div>
          </div>

          {#if recLoading}
            <div class="text-[0.85rem] text-gray-500 dark:text-gray-400 py-4 text-center">Analyzing system and finding models...</div>
          {:else if recommendations.length > 0}
            <div class="grid grid-cols-[repeat(auto-fill,minmax(13rem,1fr))] gap-3 mt-2">
              {#each recommendations as rec}
                <button
                  type="button"
                  class="text-left bg-white/50 dark:bg-black/50 border border-black/10 dark:border-white/10 p-3 rounded-xl cursor-pointer transition-all duration-150 font-inherit text-inherit hover:border-blue-500/40 hover:bg-blue-500/3 {formModel === rec.id ? 'border-blue-500! bg-blue-500/10! shadow-[inset_0_0_0_1px_rgb(59,130,246)]' : ''}"
                  onclick={() => useRecommendation(rec)}
                >
                  <div class="flex justify-between items-start mb-1.5">
                    <strong class="text-[0.85rem] block max-w-[70%] whitespace-nowrap overflow-hidden text-ellipsis text-black dark:text-white">{rec.name}</strong>
                    <span class="text-[0.7rem] px-1.5 py-0.5 bg-black/10 dark:bg-white/10 rounded-full">{rec.size_gb.toFixed(1)} GB</span>
                  </div>
                  <p class="text-[0.75rem] text-gray-500 dark:text-gray-400 m-0 mb-2 leading-snug line-clamp-2">{rec.description}</p>
                  <div class="text-[0.7rem] font-medium text-gray-400 dark:text-gray-500">
                    <span>RAM: {rec.recommended_ram_gb} GB+</span>
                  </div>
                </button>
              {/each}
            </div>
          {:else}
            <div class="text-[0.85rem] text-gray-500 dark:text-gray-400 py-4 text-center">No recommendations found or failed to load.</div>
          {/if}

          {#if localModels.length > 0}
            <div class="mt-4">
              <h4 class="m-0 mb-2 text-[0.88rem] font-semibold text-black dark:text-white">📁 Already Downloaded ({localModels.length})</h4>
              <div class="grid grid-cols-1 gap-1.5">
                {#each localModels as model}
                  <button
                    type="button"
                    class="flex items-center justify-between text-left bg-white/50 dark:bg-black/50 border border-black/10 dark:border-white/10 px-3 py-2 rounded-lg cursor-pointer transition-all duration-150 font-inherit text-inherit hover:border-emerald-500/40 hover:bg-emerald-500/4 {formBaseUrl === model.path ? 'border-emerald-500! bg-emerald-500/10! shadow-[inset_0_0_0_1px_hsl(160,60%,45%)]' : ''}"
                    onclick={() => useLocalModel(model)}
                  >
                    <div class="flex items-center gap-2 min-w-0">
                      <span class="text-[0.9rem] shrink-0 text-emerald-500">{formBaseUrl === model.path ? '✓' : '○'}</span>
                      <strong class="text-[0.82rem] whitespace-nowrap overflow-hidden text-ellipsis text-black dark:text-white">{model.filename}</strong>
                    </div>
                    <span class="text-[0.72rem] px-2 py-0.5 bg-black/8 dark:bg-white/8 rounded-full shrink-0">{model.size_display}</span>
                  </button>
                {/each}
              </div>
            </div>
          {/if}

          <div class="grid grid-cols-[repeat(auto-fit,minmax(14rem,1fr))] gap-3 mt-4">
            <label class="flex flex-col gap-1">
              <span class="text-[0.75rem] uppercase tracking-widest text-gray-500 dark:text-gray-400">Name</span>
              <input type="text" bind:value={formName} placeholder="e.g. local-llama3"
                class="px-3 py-2.5 rounded-lg border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white text-[0.88rem] focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/15" />
            </label>
            <label class="flex flex-col gap-1">
              <span class="text-[0.75rem] uppercase tracking-widest text-gray-500 dark:text-gray-400">Model ID</span>
              <input type="text" bind:value={formModel} placeholder="e.g. TheBloke/Llama-2-7b-Chat-GGUF"
                class="px-3 py-2.5 rounded-lg border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white text-[0.88rem] focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/15" />
            </label>
            <label class="flex flex-col gap-1">
              <span class="text-[0.75rem] uppercase tracking-widest text-gray-500 dark:text-gray-400">Max Tokens</span>
              <input type="number" bind:value={formMaxTokens} min="256" max="128000"
                class="px-3 py-2.5 rounded-lg border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white text-[0.88rem] focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/15" />
            </label>
          </div>

          {#if addError}
            <div class="flex gap-2 items-start px-3 py-2.5 rounded-xl mt-2 bg-red-500/10 border border-red-500/25">
              <span class="text-base shrink-0 leading-snug">⚠</span>
              <p class="m-0 text-red-400 text-[0.82rem] leading-snug">{addError}</p>
            </div>
          {/if}
          {#if testResult}
            <div class="flex gap-2.5 items-start px-3 py-2.5 rounded-xl mt-2 {testResult.ok ? 'bg-emerald-500/8 border border-emerald-500/20' : 'bg-red-500/8 border border-red-500/20'}">
              <span class="text-lg shrink-0 leading-none {testResult.ok ? 'text-emerald-500' : 'text-red-500'}">{testResult.ok ? '✓' : '✕'}</span>
              <div>
                <strong class="block text-[0.85rem] mb-0.5 text-black dark:text-white">{testResult.ok ? 'Connected' : 'Connection Failed'}</strong>
                <span class="text-[0.78rem] text-gray-500 dark:text-gray-400">{testResult.message}</span>
              </div>
            </div>
          {:else if verifyResult}
            <div class="flex gap-2.5 items-start px-3 py-2.5 rounded-xl mt-2 {verifyResult.ok ? 'bg-emerald-500/8 border border-emerald-500/20' : 'bg-red-500/8 border border-red-500/20'}">
              <span class="text-lg shrink-0 leading-none {verifyResult.ok ? 'text-emerald-500' : 'text-red-500'}">{verifyResult.ok ? '✓' : '✕'}</span>
              <div>
                {#if verifyResult.ok}
                  <strong class="block text-[0.85rem] mb-0.5 text-black dark:text-white">Ready</strong>
                  {#if verifyResult.model_exists}
                    <span class="text-[0.78rem] text-gray-500 dark:text-gray-400">Model file found locally</span>
                  {:else}
                    <span class="text-[0.78rem] text-gray-500 dark:text-gray-400">Model will be auto-downloaded on first use</span>
                  {/if}
                {:else}
                  <strong class="block text-[0.85rem] mb-0.5 text-black dark:text-white">Not Ready</strong>
                  <span class="text-[0.78rem] text-gray-500 dark:text-gray-400">{verifyResult.error}</span>
                {/if}
              </div>
            </div>
          {/if}

          <div class="flex items-center gap-3 flex-wrap mt-3">
            <button
              class="px-5 py-2.5 rounded-full border border-blue-500/30 bg-blue-500/12 text-blue-500 text-[0.88rem] font-semibold cursor-pointer transition-colors duration-150 hover:bg-blue-500/20 disabled:opacity-50 disabled:cursor-not-allowed"
              onclick={handleTestConnection}
              disabled={testLoading || addLoading}
            >
              {testLoading ? "⏳ Testing..." : connectionVerified ? "✓ Re-test Connection" : "🔌 Test Connection"}
            </button>
            <button
              class="px-5 py-2.5 rounded-full border-none bg-blue-500 text-white text-[0.88rem] font-semibold cursor-pointer transition-colors duration-150 hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed"
              onclick={handleAdd}
              disabled={addLoading || !connectionVerified}
            >
              {addLoading ? "Adding..." : "Add Provider"}
            </button>
          </div>
        </div>
      {:else}
        <!-- Standard provider form (Ollama, OpenAI, Anthropic, etc.) -->
        <div class="grid grid-cols-[repeat(auto-fit,minmax(14rem,1fr))] gap-3">
          <label class="flex flex-col gap-1">
            <span class="text-[0.75rem] uppercase tracking-widest text-gray-500 dark:text-gray-400">Name</span>
            <input type="text" bind:value={formName} placeholder="e.g. my-openai"
              class="px-3 py-2.5 rounded-lg border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white text-[0.88rem] focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/15" />
          </label>
          <label class="flex flex-col gap-1">
            <span class="text-[0.75rem] uppercase tracking-widest text-gray-500 dark:text-gray-400">Model</span>
            <input type="text" bind:value={formModel} placeholder="e.g. gpt-4o"
              class="px-3 py-2.5 rounded-lg border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white text-[0.88rem] focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/15" />
          </label>
          <label class="flex flex-col gap-1">
            <span class="text-[0.75rem] uppercase tracking-widest text-gray-500 dark:text-gray-400">Base URL</span>
            <input type="text" bind:value={formBaseUrl} placeholder="Optional"
              class="px-3 py-2.5 rounded-lg border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white text-[0.88rem] focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/15" />
          </label>
          <label class="flex flex-col gap-1">
            <span class="text-[0.75rem] uppercase tracking-widest text-gray-500 dark:text-gray-400">API Key</span>
            <input type="password" bind:value={formApiKey} placeholder="Direct key (optional)"
              class="px-3 py-2.5 rounded-lg border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white text-[0.88rem] focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/15" />
          </label>
          <label class="flex flex-col gap-1">
            <span class="text-[0.75rem] uppercase tracking-widest text-gray-500 dark:text-gray-400">API Key Env Var</span>
            <input type="text" bind:value={formApiKeyEnv} placeholder="e.g. OPENAI_API_KEY"
              class="px-3 py-2.5 rounded-lg border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white text-[0.88rem] focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/15" />
          </label>
          <label class="flex flex-col gap-1">
            <span class="text-[0.75rem] uppercase tracking-widest text-gray-500 dark:text-gray-400">Max Tokens</span>
            <input type="number" bind:value={formMaxTokens} min="256" max="128000"
              class="px-3 py-2.5 rounded-lg border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 text-black dark:text-white text-[0.88rem] focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/15" />
          </label>
        </div>

        {#if addError}
          <div class="flex gap-2 items-start px-3 py-2.5 rounded-xl mt-2 bg-red-500/10 border border-red-500/25">
            <span class="text-base shrink-0 leading-snug">⚠</span>
            <p class="m-0 text-red-400 text-[0.82rem] leading-snug">{addError}</p>
          </div>
        {/if}
        {#if testResult}
          <div class="flex gap-2.5 items-start px-3 py-2.5 rounded-xl mt-2 {testResult.ok ? 'bg-emerald-500/8 border border-emerald-500/20' : 'bg-red-500/8 border border-red-500/20'}">
            <span class="text-lg shrink-0 leading-none {testResult.ok ? 'text-emerald-500' : 'text-red-500'}">{testResult.ok ? '✓' : '✕'}</span>
            <div>
              <strong class="block text-[0.85rem] mb-0.5 text-black dark:text-white">{testResult.ok ? 'Connected' : 'Connection Failed'}</strong>
              <span class="text-[0.78rem] text-gray-500 dark:text-gray-400">{testResult.message}</span>
            </div>
          </div>
        {/if}

        <div class="flex items-center gap-3 flex-wrap mt-3">
          <button
            class="px-5 py-2.5 rounded-full border border-blue-500/30 bg-blue-500/12 text-blue-500 text-[0.88rem] font-semibold cursor-pointer transition-colors duration-150 hover:bg-blue-500/20 disabled:opacity-50 disabled:cursor-not-allowed"
            onclick={handleTestConnection}
            disabled={testLoading || addLoading}
          >
            {testLoading ? "⏳ Testing..." : connectionVerified ? "✓ Re-test Connection" : "🔌 Test Connection"}
          </button>
          <button
            class="px-5 py-2.5 rounded-full border-none bg-blue-500 text-white text-[0.88rem] font-semibold cursor-pointer transition-colors duration-150 hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed"
            onclick={handleAdd}
            disabled={addLoading || !connectionVerified}
          >
            {addLoading ? "Adding..." : "Add Provider"}
          </button>
        </div>
      {/if}
    </div>
  {/if}

  <!-- Provider List -->
  {#if loading}
    <div class="text-center py-8 text-gray-500 dark:text-gray-400">Loading providers...</div>
  {:else if providers.length === 0}
    <div class="flex flex-col items-center gap-2 text-center py-8 text-gray-500 dark:text-gray-400">
      <div class="text-5xl">🤖</div>
      <h3 class="m-0 text-black dark:text-white">No Providers Configured</h3>
      <p class="m-0 max-w-xs text-[0.85rem]">Add an LLM provider to power the agent. Choose from Ollama, OpenAI, Anthropic, and more.</p>
      <button
        class="mt-2 rounded-full px-4 py-2 text-[0.82rem] cursor-pointer border border-blue-500/25 bg-blue-500/12 text-blue-500 font-medium transition-all duration-150 hover:bg-blue-500/20"
        onclick={() => showAddForm = true}
      >+ Add Provider</button>
    </div>
  {:else}
    <div class="grid grid-cols-[repeat(auto-fill,minmax(16rem,1fr))] gap-3">
      {#each providers as provider (provider.name)}
        <article class="rounded-2xl p-4 border border-black/10 dark:border-white/10 bg-white/50 dark:bg-black/50 transition-all duration-200 flex flex-col gap-3 hover:border-blue-500/30 {provider.is_active ? 'border-blue-500! bg-blue-500/6!' : ''} {switching && provider.name !== activeProvider ? 'opacity-50' : ''}">
          <div class="flex items-center gap-2.5">
            <span class="text-2xl">{getProviderIcon(provider.provider_type)}</span>
            <div class="flex flex-col flex-1">
              <strong class="text-[0.92rem] text-black dark:text-white">{provider.name}</strong>
              <span class="text-[0.72rem] text-gray-500 dark:text-gray-400 uppercase tracking-[0.08em]">{provider.provider_type}</span>
            </div>
            {#if provider.is_active}
              <span class="px-2.5 py-1 rounded-full text-[0.7rem] font-semibold bg-emerald-500/15 text-emerald-500">Active</span>
            {/if}
          </div>
          <div class="flex items-center gap-2">
            {#if !provider.is_active}
              <button
                class="px-3 py-1.5 rounded-full border border-blue-500/30 bg-blue-500/10 text-blue-500 text-[0.78rem] cursor-pointer transition-all duration-150 hover:bg-blue-500/20 disabled:opacity-40 disabled:cursor-not-allowed"
                onclick={() => handleSwitch(provider.name)}
                disabled={switching}
              >
                {switching ? "..." : "Switch to"}
              </button>
            {:else}
              <span class="text-[0.78rem] text-emerald-500 font-semibold">Current</span>
            {/if}
            <button
              class="ml-auto w-7 h-7 rounded-full border-none bg-red-500/10 text-red-500 cursor-pointer text-[0.8rem] flex items-center justify-center transition-all duration-150 hover:bg-red-500/25"
              onclick={() => handleRemove(provider.name)}
              title="Remove provider"
            >
              ✕
            </button>
          </div>
        </article>
      {/each}
    </div>
  {/if}
</section>
