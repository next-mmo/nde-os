<svelte:options runes={true} />

<script lang="ts">
  import * as api from "$lib/api/backend";
  import type { McpTool, McpServerInfo } from "$lib/api/types";

  let tools = $state<McpTool[]>([]);
  let servers = $state<McpServerInfo[]>([]);
  let loading = $state(true);
  let searchQuery = $state("");

  $effect(() => { refresh(); });

  async function refresh() {
    loading = true;
    try {
      const [t, s] = await Promise.all([api.listMcpTools(), api.listMcpServers()]);
      tools = t; servers = s;
    } catch {
      tools = getFallbackTools();
      servers = getFallbackServers();
    }
    finally { loading = false; }
  }

  function getFallbackTools(): McpTool[] {
    return [
      { name: "read_file", description: "Read a file from the sandboxed workspace", parameters: { type: "object", properties: { path: { type: "string" } } } },
      { name: "write_file", description: "Write content to a file in the sandbox", parameters: { type: "object", properties: { path: { type: "string" }, content: { type: "string" } } } },
      { name: "list_dir", description: "List directory contents in the sandbox", parameters: { type: "object", properties: { path: { type: "string" } } } },
      { name: "run_command", description: "Execute a shell command inside the sandbox", parameters: { type: "object", properties: { command: { type: "string" } } } },
      { name: "search_knowledge", description: "Search the knowledge graph", parameters: { type: "object", properties: { query: { type: "string" } } } },
      { name: "store_memory", description: "Store a key-value pair in memory", parameters: { type: "object", properties: { key: { type: "string" }, value: { type: "string" } } } },
    ];
  }

  function getFallbackServers(): McpServerInfo[] {
    return [
      { name: "nde-os", transport: "stdio", tools_count: 6, is_connected: true },
    ];
  }

  const filteredTools = $derived(
    searchQuery.trim()
      ? tools.filter(t => t.name.toLowerCase().includes(searchQuery.toLowerCase()) || t.description.toLowerCase().includes(searchQuery.toLowerCase()))
      : tools
  );
</script>

<section class="mcp-app">
  <div class="header">
    <div>
      <p class="eyebrow">Interoperability</p>
      <h2>MCP Tools & Servers</h2>
    </div>
    <button class="action-btn" onclick={refresh} disabled={loading}>{loading ? "Loading..." : "↻ Refresh"}</button>
  </div>

  <p class="intro">Model Context Protocol enables the agent to call tools and connect to external MCP servers via stdio JSON-RPC.</p>

  <!-- Servers -->
  <div class="section">
    <h3>Connected Servers</h3>
    <div class="servers-row">
      {#each servers as s (s.name)}
        <div class="server-card" class:connected={s.is_connected}>
          <span class="server-dot" class:active={s.is_connected}></span>
          <div class="server-info">
            <strong>{s.name}</strong>
            <span>{s.transport} · {s.tools_count} tools</span>
          </div>
        </div>
      {:else}
        <p class="muted">No MCP servers connected.</p>
      {/each}
    </div>
  </div>

  <!-- Tools -->
  <div class="section">
    <div class="tools-header">
      <h3>Available Tools ({filteredTools.length})</h3>
      <input class="search" type="text" placeholder="Search tools..." bind:value={searchQuery} />
    </div>
    <div class="tools-grid">
      {#each filteredTools as tool (tool.name)}
        <article class="tool-card">
          <div class="tool-name">
            <span class="tool-icon">🔧</span>
            <code>{tool.name}</code>
          </div>
          <p class="tool-desc">{tool.description}</p>
          {#if tool.parameters && typeof tool.parameters === "object"}
            <div class="tool-params">
              {#each Object.keys((tool.parameters as any).properties ?? {}) as param}
                <span class="param-tag">{param}</span>
              {/each}
            </div>
          {/if}
        </article>
      {:else}
        <p class="muted">No tools match your search.</p>
      {/each}
    </div>
  </div>
</section>

<style>
  .mcp-app { height: 100%; overflow: auto; padding: 1.1rem; display: grid; gap: 1rem; align-content: start; }
  .header { display: flex; justify-content: space-between; align-items: center; gap: 1rem; }
  .eyebrow { margin: 0; font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.14em; color: var(--system-color-text-muted); }
  h2 { margin: 0.3rem 0 0; }
  h3 { margin: 0; font-size: 0.95rem; }
  .intro { margin: 0; font-size: 0.85rem; color: var(--system-color-text-muted); max-width: 600px; }
  .action-btn {
    border-radius: 999px; padding: 0.55rem 1rem; font-size: 0.82rem; cursor: pointer;
    border: 1px solid var(--system-color-border); background: var(--system-color-panel); color: var(--system-color-text);
  }
  .section { display: grid; gap: 0.6rem; }
  .servers-row { display: flex; gap: 0.6rem; flex-wrap: wrap; }
  .server-card {
    display: flex; align-items: center; gap: 0.5rem; padding: 0.6rem 0.9rem;
    border-radius: 0.8rem; border: 1px solid var(--system-color-border); background: var(--system-color-panel);
  }
  .server-card.connected { border-color: hsla(160 60% 50% / 0.4); }
  .server-dot { width: 0.5rem; height: 0.5rem; border-radius: 50%; background: var(--system-color-text-muted); }
  .server-dot.active { background: var(--system-color-success); box-shadow: 0 0 6px hsla(160 60% 50% / 0.5); }
  .server-info { display: flex; flex-direction: column; }
  .server-info strong { font-size: 0.85rem; }
  .server-info span { font-size: 0.72rem; color: var(--system-color-text-muted); }
  .tools-header { display: flex; justify-content: space-between; align-items: center; gap: 1rem; }
  .search {
    padding: 0.45rem 0.8rem; border-radius: 999px; border: 1px solid var(--system-color-border);
    background: var(--system-color-panel); color: var(--system-color-text); font-size: 0.82rem; width: 14rem;
  }
  .search:focus { border-color: var(--system-color-primary); outline: none; }
  .tools-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr)); gap: 0.65rem; }
  .tool-card {
    border-radius: 0.85rem; padding: 0.85rem; border: 1px solid var(--system-color-border);
    background: var(--system-color-panel); display: flex; flex-direction: column; gap: 0.4rem; transition: all 0.15s;
  }
  .tool-card:hover { border-color: hsla(var(--system-color-primary-hsl) / 0.3); }
  .tool-name { display: flex; align-items: center; gap: 0.4rem; }
  .tool-icon { font-size: 0.9rem; }
  .tool-name code { font-size: 0.82rem; font-weight: 600; color: var(--system-color-primary); }
  .tool-desc { margin: 0; font-size: 0.78rem; color: var(--system-color-text-muted); line-height: 1.4; }
  .tool-params { display: flex; flex-wrap: wrap; gap: 0.25rem; }
  .param-tag { font-size: 0.68rem; padding: 0.1rem 0.4rem; border-radius: 999px; background: hsla(var(--system-color-dark-hsl) / 0.06); color: var(--system-color-text-muted); font-family: ui-monospace, monospace; }
  .muted { color: var(--system-color-text-muted); font-size: 0.82rem; }
</style>
