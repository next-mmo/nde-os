<svelte:options runes={true} />

<script lang="ts">
  import * as api from "$lib/api/backend";
  import type { ProviderStatus, PluginStatus, ChannelStatus } from "$lib/api/types";
  import { healthStatus, runningCount, catalogCount, resourceUsage, systemInfo } from "$lib/stores/state";
  import { openStaticApp } from "🍎/state/desktop.svelte";

  let providers = $state<ProviderStatus[]>([]);
  let plugins = $state<PluginStatus[]>([]);
  let channels = $state<ChannelStatus[]>([]);
  let activeModel = $state("");
  let agentTools = $state<string[]>([]);
  let loading = $state(true);

  $effect(() => { refresh(); });

  async function refresh() {
    loading = true;
    try {
      const [provs, active, plugs, config] = await Promise.all([
        api.listModels().catch(() => []),
        api.activeModel().catch(() => "unknown"),
        api.listPlugins().catch(() => []),
        api.agentConfig().catch(() => ({ tools: [], name: "", provider: "", model: "", max_iterations: 0, workspace: "" })),
      ]);
      providers = provs as ProviderStatus[];
      activeModel = active as string;
      plugins = plugs as PluginStatus[];
      agentTools = config.tools;
      channels = await api.listChannels().catch(() => [
        { name: "rest-api", channel_type: "rest" as const, is_running: true, messages_received: 0, messages_sent: 0 },
      ]);
    } catch {}
    finally { loading = false; }
  }

  const runningPlugins = $derived(plugins.filter(p => p.state === "running").length);
  const activeChannels = $derived(channels.filter(c => c.is_running).length);

  function usageTone(pct: number) { return pct >= 85 ? "danger" : pct >= 70 ? "warning" : "safe"; }
  function formatBytes(b: number) {
    if (b >= 1073741824) return `${(b / 1073741824).toFixed(1)} GB`;
    if (b >= 1048576) return `${(b / 1048576).toFixed(1)} MB`;
    return `${Math.round(b / 1024)} KB`;
  }
</script>

<section class="cc-app">
  <div class="cc-header">
    <div>
      <p class="eyebrow">Overview</p>
      <h2>Command Center</h2>
    </div>
    <button class="refresh-btn" onclick={refresh} disabled={loading}>{loading ? "..." : "↻ Refresh"}</button>
  </div>

  <!-- Status Cards Row -->
  <div class="status-row">
    <div class="status-card" class:online={$healthStatus === "online"}>
      <span class="s-dot" class:active={$healthStatus === "online"}></span>
      <div>
        <strong>{$healthStatus === "online" ? "Server Online" : "Offline"}</strong>
        <span>localhost:8080</span>
      </div>
    </div>
    <button class="status-card clickable" onclick={() => openStaticApp("model-settings")}>
      <span class="s-icon">🤖</span>
      <div>
        <strong>{activeModel || "No Model"}</strong>
        <span>{providers.length} provider{providers.length !== 1 ? "s" : ""}</span>
      </div>
    </button>
    <button class="status-card clickable" onclick={() => openStaticApp("plugins")}>
      <span class="s-icon">🧩</span>
      <div>
        <strong>{runningPlugins} Running</strong>
        <span>{plugins.length} plugin{plugins.length !== 1 ? "s" : ""}</span>
      </div>
    </button>
    <button class="status-card clickable" onclick={() => openStaticApp("channels")}>
      <span class="s-icon">📡</span>
      <div>
        <strong>{activeChannels} Active</strong>
        <span>{channels.length} channel{channels.length !== 1 ? "s" : ""}</span>
      </div>
    </button>
  </div>

  <!-- Metrics Row -->
  <div class="metrics-row">
    <div class="metric-card">
      <span class="metric-value">{$catalogCount}</span>
      <span class="metric-label">Catalog Apps</span>
    </div>
    <div class="metric-card">
      <span class="metric-value">{$runningCount}</span>
      <span class="metric-label">Running Apps</span>
    </div>
    <div class="metric-card">
      <span class="metric-value">{agentTools.length}</span>
      <span class="metric-label">Agent Tools</span>
    </div>
    <div class="metric-card">
      <span class="metric-value">{providers.length}</span>
      <span class="metric-label">LLM Providers</span>
    </div>
  </div>

  <!-- Resources -->
  {#if $resourceUsage}
    <div class="resources-row">
      <div class="resource-card">
        <div class="res-header">
          <span>Memory</span>
          <strong>{$resourceUsage.memory_percent}%</strong>
        </div>
        <div class={`res-bar ${usageTone($resourceUsage.memory_percent)}`} style:--fill={`${$resourceUsage.memory_percent}%`}><span></span></div>
        <span class="res-detail">{formatBytes($resourceUsage.memory_used_bytes)} / {formatBytes($resourceUsage.memory_total_bytes)}</span>
      </div>
      <div class="resource-card">
        <div class="res-header">
          <span>Disk</span>
          <strong>{$resourceUsage.disk_percent}%</strong>
        </div>
        <div class={`res-bar ${usageTone($resourceUsage.disk_percent)}`} style:--fill={`${$resourceUsage.disk_percent}%`}><span></span></div>
        <span class="res-detail">{formatBytes($resourceUsage.disk_used_bytes)} / {formatBytes($resourceUsage.disk_total_bytes)}</span>
      </div>
    </div>
  {/if}

  <!-- Quick Actions -->
  <div class="actions-section">
    <h3>Quick Actions</h3>
    <div class="actions-grid">
      <button class="action-card" onclick={() => openStaticApp("chat")}>
        <span>💬</span><strong>Chat</strong><span class="act-desc">Talk to the agent</span>
      </button>
      <button class="action-card" onclick={() => openStaticApp("model-settings")}>
        <span>🤖</span><strong>Models</strong><span class="act-desc">Configure LLM</span>
      </button>
      <button class="action-card" onclick={() => openStaticApp("plugins")}>
        <span>🧩</span><strong>Plugins</strong><span class="act-desc">Manage extensions</span>
      </button>
      <button class="action-card" onclick={() => openStaticApp("channels")}>
        <span>📡</span><strong>Channels</strong><span class="act-desc">Gateway status</span>
      </button>
      <button class="action-card" onclick={() => openStaticApp("mcp-tools")}>
        <span>🔧</span><strong>MCP Tools</strong><span class="act-desc">Browse tools</span>
      </button>
      <button class="action-card" onclick={() => openStaticApp("skills")}>
        <span>📘</span><strong>Skills</strong><span class="act-desc">Skill library</span>
      </button>
      <button class="action-card" onclick={() => openStaticApp("knowledge")}>
        <span>🧠</span><strong>Knowledge</strong><span class="act-desc">Agent memory</span>
      </button>
      <button class="action-card" onclick={() => openStaticApp("code-editor")}>
        <span>💻</span><strong>IDE</strong><span class="act-desc">Code editor</span>
      </button>
    </div>
  </div>

  <!-- System Info -->
  {#if $systemInfo}
    <div class="system-footer">
      <span>NDE-OS v0.2.0</span>
      <span>·</span>
      <span>{$systemInfo.os}/{$systemInfo.arch}</span>
      <span>·</span>
      <span>GPU: {$systemInfo.gpu_detected ? "✓" : "✗"}</span>
      <span>·</span>
      <span>Python: {$systemInfo.python_version ?? "N/A"}</span>
    </div>
  {/if}
</section>

<style>
  .cc-app { height: 100%; overflow: auto; padding: 1.1rem; display: grid; gap: 1rem; align-content: start; }
  .cc-header { display: flex; justify-content: space-between; align-items: center; }
  .eyebrow { margin: 0; font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.14em; color: var(--system-color-text-muted); }
  h2, h3 { margin: 0.3rem 0 0; }
  .refresh-btn {
    border-radius: 999px; padding: 0.5rem 0.9rem; font-size: 0.82rem; cursor: pointer;
    border: 1px solid var(--system-color-border); background: var(--system-color-panel); color: var(--system-color-text);
  }
  .status-row { display: grid; grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr)); gap: 0.6rem; }
  .status-card {
    display: flex; align-items: center; gap: 0.55rem; padding: 0.75rem 0.85rem;
    border-radius: 1rem; border: 1px solid var(--system-color-border); background: var(--system-color-panel); text-align: left;
  }
  .status-card.clickable { cursor: pointer; transition: all 0.15s; }
  .status-card.clickable:hover { border-color: hsla(var(--system-color-primary-hsl) / 0.3); transform: translateY(-1px); }
  .status-card div { display: flex; flex-direction: column; }
  .status-card strong { font-size: 0.82rem; }
  .status-card span:not(.s-dot):not(.s-icon) { font-size: 0.7rem; color: var(--system-color-text-muted); }
  .s-dot { width: 0.55rem; height: 0.55rem; border-radius: 50%; background: var(--system-color-danger); flex-shrink: 0; }
  .s-dot.active { background: var(--system-color-success); box-shadow: 0 0 6px hsla(160 60% 50% / 0.5); }
  .s-icon { font-size: 1.3rem; }
  .metrics-row { display: grid; grid-template-columns: repeat(auto-fit, minmax(8rem, 1fr)); gap: 0.6rem; }
  .metric-card { text-align: center; padding: 0.7rem; border-radius: 1rem; border: 1px solid var(--system-color-border); background: var(--system-color-panel); }
  .metric-value { display: block; font-size: 1.5rem; font-weight: 700; color: var(--system-color-text); }
  .metric-label { display: block; font-size: 0.68rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--system-color-text-muted); margin-top: 0.15rem; }
  .resources-row { display: grid; grid-template-columns: 1fr 1fr; gap: 0.6rem; }
  .resource-card { padding: 0.85rem; border-radius: 1rem; border: 1px solid var(--system-color-border); background: var(--system-color-panel); display: grid; gap: 0.35rem; }
  .res-header { display: flex; justify-content: space-between; align-items: center; }
  .res-header span { font-size: 0.82rem; font-weight: 600; }
  .res-header strong { font-size: 1.1rem; }
  .res-bar { --fill: 0%; height: 0.45rem; border-radius: 999px; background: hsla(var(--system-color-dark-hsl) / 0.08); overflow: hidden; }
  .res-bar span { display: block; width: var(--fill); height: 100%; border-radius: inherit; background: linear-gradient(90deg, var(--system-color-success), var(--system-color-primary)); }
  .res-bar.warning span { background: linear-gradient(90deg, var(--system-color-warning), var(--system-color-primary)); }
  .res-bar.danger span { background: linear-gradient(90deg, var(--system-color-warning), var(--system-color-danger)); }
  .res-detail { font-size: 0.72rem; color: var(--system-color-text-muted); }
  .actions-section { display: grid; gap: 0.6rem; }
  .actions-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(8rem, 1fr)); gap: 0.5rem; }
  .action-card {
    display: flex; flex-direction: column; align-items: center; gap: 0.25rem;
    padding: 0.85rem 0.5rem; border-radius: 1rem; border: 1px solid var(--system-color-border);
    background: var(--system-color-panel); cursor: pointer; transition: all 0.15s; text-align: center;
    color: var(--system-color-text);
  }
  .action-card:hover { border-color: hsla(var(--system-color-primary-hsl) / 0.35); transform: translateY(-2px); box-shadow: 0 4px 12px hsla(0 0% 0% / 0.1); }
  .action-card span:first-child { font-size: 1.5rem; }
  .action-card strong { font-size: 0.78rem; }
  .act-desc { font-size: 0.65rem; color: var(--system-color-text-muted); }
  .system-footer {
    display: flex; gap: 0.5rem; align-items: center; justify-content: center;
    font-size: 0.72rem; color: var(--system-color-text-muted); padding-top: 0.5rem;
    border-top: 1px solid var(--system-color-border);
  }
</style>
