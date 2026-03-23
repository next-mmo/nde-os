<svelte:options runes={true} />

<script lang="ts">
  import * as api from "$lib/api/backend";
  import type { PluginStatus } from "$lib/api/types";

  let plugins = $state<PluginStatus[]>([]);
  let loading = $state(true);
  let discovering = $state(false);
  let actionLoading = $state<Record<string, boolean>>({});

  const TYPE_ICONS: Record<string, string> = {
    monitor: "📊", hook: "🪝", provider: "🔌", tool: "🔧", ui_panel: "🖼️", daemon: "👻",
  };
  const STATE_COLORS: Record<string, string> = {
    discovered: "var(--system-color-text-muted)", installed: "var(--system-color-warning)",
    running: "var(--system-color-success)", stopped: "var(--system-color-text-muted)", error: "var(--system-color-danger)",
  };

  $effect(() => { refresh(); });

  async function refresh() {
    loading = true;
    try { plugins = await api.listPlugins(); } catch { plugins = []; }
    finally { loading = false; }
  }

  async function handleDiscover() {
    discovering = true;
    try { await api.discoverPlugins(); await refresh(); } catch (e: any) { console.error(e); }
    finally { discovering = false; }
  }

  async function handleAction(id: string, action: "install" | "start" | "stop") {
    actionLoading = { ...actionLoading, [id]: true };
    try {
      if (action === "install") await api.installPlugin(id);
      else if (action === "start") await api.startPlugin(id);
      else await api.stopPlugin(id);
      await refresh();
    } catch (e: any) { console.error(e); }
    finally { actionLoading = { ...actionLoading, [id]: false }; }
  }
</script>

<section class="plugins-app">
  <div class="header">
    <div>
      <p class="eyebrow">Extensions</p>
      <h2>Plugin Manager</h2>
    </div>
    <div class="header-actions">
      <button class="action-btn" onclick={refresh} disabled={loading}>{loading ? "Loading..." : "↻ Refresh"}</button>
      <button class="action-btn discover-btn" onclick={handleDiscover} disabled={discovering}>
        {discovering ? "Scanning..." : "🔍 Discover"}
      </button>
    </div>
  </div>

  {#if loading}
    <div class="loading">Scanning for plugins...</div>
  {:else if plugins.length === 0}
    <div class="empty-state">
      <div class="empty-icon">🧩</div>
      <h3>No Plugins Found</h3>
      <p>Place plugin folders in the <code>plugins/</code> directory, then click Discover.</p>
      <button class="action-btn discover-btn" onclick={handleDiscover}>🔍 Discover Plugins</button>
    </div>
  {:else}
    <div class="plugins-grid">
      {#each plugins as plugin (plugin.id)}
        <article class="plugin-card" class:running={plugin.state === "running"} class:error={plugin.state === "error"}>
          <div class="plugin-header">
            <span class="plugin-icon">{TYPE_ICONS[plugin.plugin_type] ?? "🧩"}</span>
            <div class="plugin-info">
              <strong>{plugin.name}</strong>
              <span class="plugin-meta">v{plugin.version} · {plugin.plugin_type}</span>
            </div>
            <span class="state-dot" style="background: {STATE_COLORS[plugin.state] ?? 'gray'}"></span>
          </div>

          <div class="plugin-details">
            <span class="state-label">{plugin.state}</span>
            {#if plugin.pid}
              <span class="detail">PID: {plugin.pid}</span>
            {/if}
            {#if plugin.port}
              <span class="detail">Port: {plugin.port}</span>
            {/if}
          </div>

          {#if plugin.hooks.length > 0}
            <div class="hooks">
              {#each plugin.hooks as hook}
                <span class="hook-tag">{hook}</span>
              {/each}
            </div>
          {/if}

          <div class="plugin-actions">
            {#if plugin.state === "discovered"}
              <button class="act-btn install" onclick={() => handleAction(plugin.id, "install")} disabled={actionLoading[plugin.id]}>
                {actionLoading[plugin.id] ? "..." : "Install"}
              </button>
            {:else if plugin.state === "installed" || plugin.state === "stopped"}
              <button class="act-btn start" onclick={() => handleAction(plugin.id, "start")} disabled={actionLoading[plugin.id]}>
                {actionLoading[plugin.id] ? "..." : "▶ Start"}
              </button>
            {:else if plugin.state === "running"}
              <button class="act-btn stop" onclick={() => handleAction(plugin.id, "stop")} disabled={actionLoading[plugin.id]}>
                {actionLoading[plugin.id] ? "..." : "⏹ Stop"}
              </button>
            {/if}
          </div>
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .plugins-app { height: 100%; overflow: auto; padding: 1.1rem; display: grid; gap: 1rem; align-content: start; }
  .header { display: flex; justify-content: space-between; align-items: center; gap: 1rem; flex-wrap: wrap; }
  .eyebrow { margin: 0; font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.14em; color: var(--system-color-text-muted); }
  h2 { margin: 0.3rem 0 0; }
  .header-actions { display: flex; gap: 0.5rem; }
  .action-btn {
    border-radius: 999px; padding: 0.55rem 1rem; font-size: 0.82rem; cursor: pointer;
    border: 1px solid var(--system-color-border); background: var(--system-color-panel); color: var(--system-color-text);
  }
  .discover-btn { background: hsla(270 60% 55% / 0.12); border-color: hsla(270 60% 55% / 0.25); color: hsl(270 60% 65%); }
  .plugins-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(18rem, 1fr)); gap: 0.85rem; }
  .plugin-card {
    border-radius: 1.1rem; padding: 1rem; border: 1px solid var(--system-color-border);
    background: var(--system-color-panel); display: flex; flex-direction: column; gap: 0.6rem; transition: all 0.2s;
  }
  .plugin-card:hover { border-color: hsla(270 60% 55% / 0.3); }
  .plugin-card.running { border-color: var(--system-color-success); background: hsla(160 60% 50% / 0.04); }
  .plugin-card.error { border-color: var(--system-color-danger); }
  .plugin-header { display: flex; align-items: center; gap: 0.6rem; }
  .plugin-icon { font-size: 1.4rem; }
  .plugin-info { display: flex; flex-direction: column; flex: 1; }
  .plugin-info strong { font-size: 0.92rem; }
  .plugin-meta { font-size: 0.72rem; color: var(--system-color-text-muted); }
  .state-dot { width: 0.55rem; height: 0.55rem; border-radius: 50%; flex-shrink: 0; }
  .plugin-details { display: flex; gap: 0.6rem; align-items: center; }
  .state-label { font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.08em; color: var(--system-color-text-muted); }
  .detail { font-size: 0.72rem; color: var(--system-color-text-muted); padding: 0.1rem 0.4rem; border-radius: 999px; background: hsla(var(--system-color-dark-hsl) / 0.06); }
  .hooks { display: flex; flex-wrap: wrap; gap: 0.3rem; }
  .hook-tag { font-size: 0.68rem; padding: 0.15rem 0.45rem; border-radius: 999px; background: hsla(270 60% 55% / 0.1); color: hsl(270 60% 65%); }
  .plugin-actions { display: flex; gap: 0.4rem; }
  .act-btn {
    padding: 0.4rem 0.85rem; border-radius: 999px; border: none; font-size: 0.78rem; font-weight: 600; cursor: pointer; transition: all 0.15s;
  }
  .act-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .act-btn.install { background: hsla(var(--system-color-primary-hsl) / 0.15); color: var(--system-color-primary); }
  .act-btn.start { background: hsla(160 60% 50% / 0.15); color: hsl(160 60% 50%); }
  .act-btn.stop { background: hsla(0 70% 55% / 0.15); color: hsl(0 70% 60%); }
  .loading, .empty-state { text-align: center; padding: 2rem 1rem; color: var(--system-color-text-muted); }
  .empty-state { display: flex; flex-direction: column; align-items: center; gap: 0.5rem; }
  .empty-icon { font-size: 3rem; }
  .empty-state h3 { margin: 0; color: var(--system-color-text); }
  .empty-state p { margin: 0; max-width: 320px; font-size: 0.85rem; }
  .empty-state code { padding: 0.15rem 0.4rem; border-radius: 0.3rem; background: hsla(var(--system-color-dark-hsl) / 0.1); font-size: 0.82rem; }
</style>
