<svelte:options runes={true} />

<script lang="ts">
  import * as api from "$lib/api/backend";
  import type { KnowledgeEntry } from "$lib/api/types";

  let entries = $state<KnowledgeEntry[]>([]);
  let loading = $state(true);
  let searchQuery = $state("");
  let selectedEntry = $state<KnowledgeEntry | null>(null);

  $effect(() => { refresh(); });

  async function refresh() {
    loading = true;
    try { entries = await api.listKnowledge(); }
    catch { entries = getFallback(); }
    finally { loading = false; }
  }

  async function handleSearch() {
    if (!searchQuery.trim()) { await refresh(); return; }
    loading = true;
    try { entries = await api.searchKnowledge(searchQuery); }
    catch { /* keep existing */ }
    finally { loading = false; }
  }

  function getFallback(): KnowledgeEntry[] {
    const now = new Date().toISOString();
    return [
      { id: "1", key: "project_architecture", value: "Monorepo: core/ (Rust sandbox), server/ (Rust API), desktop/ (Tauri+Svelte5). Core has ZERO deps on server/desktop.", category: "architecture", created_at: now, updated_at: now },
      { id: "2", key: "sandbox_security", value: "OS-level filesystem jail with path canonicalization, symlink defense, env-var jailing (12 vars), and SHA-256 audit chain.", category: "security", created_at: now, updated_at: now },
      { id: "3", key: "llm_providers", value: "6 providers supported: Ollama, OpenAI, Anthropic, Groq, Together, Codex. Runtime hot-swap via LlmManager.", category: "llm", created_at: now, updated_at: now },
    ];
  }

  const categories = $derived([...new Set(entries.map(e => e.category))].sort());
  const grouped = $derived.by(() => {
    const groups: Record<string, KnowledgeEntry[]> = {};
    for (const entry of entries) {
      if (!groups[entry.category]) groups[entry.category] = [];
      groups[entry.category].push(entry);
    }
    return groups;
  });

  function formatDate(iso: string) { return new Date(iso).toLocaleDateString(); }
</script>

<section class="knowledge-app">
  <div class="header">
    <div>
      <p class="eyebrow">Agent Memory</p>
      <h2>Knowledge Graph</h2>
    </div>
    <button class="action-btn" onclick={refresh} disabled={loading}>{loading ? "..." : "↻"}</button>
  </div>

  <div class="search-row">
    <input class="search" type="text" placeholder="Search knowledge..." bind:value={searchQuery} onkeydown={(e) => { if (e.key === "Enter") handleSearch(); }} />
    <button class="search-btn" onclick={handleSearch}>Search</button>
    <span class="result-count">{entries.length} entries</span>
  </div>

  <div class="main-content">
    <div class="entries-panel">
      {#if loading}
        <div class="loading">Loading knowledge graph...</div>
      {:else if entries.length === 0}
        <div class="empty-state">
          <span class="empty-icon">🧠</span>
          <h3>No Knowledge Entries</h3>
          <p>The agent will populate this as it learns during conversations.</p>
        </div>
      {:else}
        {#each categories as cat}
          <div class="category-group">
            <h3 class="category-label">{cat}</h3>
            {#each grouped[cat] ?? [] as entry (entry.id)}
              <button class="entry-item" class:selected={selectedEntry?.id === entry.id} onclick={() => selectedEntry = entry}>
                <span class="entry-key">{entry.key}</span>
                <span class="entry-date">{formatDate(entry.updated_at)}</span>
              </button>
            {/each}
          </div>
        {/each}
      {/if}
    </div>

    <div class="detail-panel">
      {#if selectedEntry}
        <div class="detail-header">
          <h3>{selectedEntry.key}</h3>
          <span class="cat-badge">{selectedEntry.category}</span>
        </div>
        <div class="detail-meta">
          <span>Created: {formatDate(selectedEntry.created_at)}</span>
          <span>Updated: {formatDate(selectedEntry.updated_at)}</span>
        </div>
        <div class="detail-value">{selectedEntry.value}</div>
      {:else}
        <div class="no-selection">
          <span>🧠</span>
          <p>Select a knowledge entry to view its contents.</p>
        </div>
      {/if}
    </div>
  </div>
</section>

<style>
  .knowledge-app { height: 100%; overflow: hidden; padding: 1.1rem; display: grid; grid-template-rows: auto auto 1fr; gap: 0.85rem; }
  .header { display: flex; justify-content: space-between; align-items: center; }
  .eyebrow { margin: 0; font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.14em; color: var(--system-color-text-muted); }
  h2 { margin: 0.3rem 0 0; }
  .action-btn {
    border-radius: 999px; padding: 0.55rem 0.8rem; font-size: 0.82rem; cursor: pointer;
    border: 1px solid var(--system-color-border); background: var(--system-color-panel); color: var(--system-color-text);
  }
  .search-row { display: flex; gap: 0.5rem; align-items: center; }
  .search {
    flex: 1; padding: 0.5rem 0.85rem; border-radius: 0.6rem; border: 1px solid var(--system-color-border);
    background: var(--system-color-panel); color: var(--system-color-text); font-size: 0.85rem;
  }
  .search:focus { border-color: var(--system-color-primary); outline: none; }
  .search-btn {
    padding: 0.5rem 1rem; border-radius: 0.6rem; border: none;
    background: var(--system-color-primary); color: white; font-size: 0.82rem; font-weight: 600; cursor: pointer;
  }
  .result-count { font-size: 0.78rem; color: var(--system-color-text-muted); white-space: nowrap; }
  .main-content { display: grid; grid-template-columns: 1fr 1.5fr; gap: 0.85rem; overflow: hidden; min-height: 0; }
  .entries-panel { overflow-y: auto; display: flex; flex-direction: column; gap: 0.5rem; padding-right: 0.3rem; }
  .category-group { display: flex; flex-direction: column; gap: 0.2rem; }
  .category-label {
    margin: 0; font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.12em;
    color: var(--system-color-text-muted); padding: 0.4rem 0 0.2rem; border-bottom: 1px solid var(--system-color-border);
  }
  .entry-item {
    display: flex; justify-content: space-between; align-items: center; padding: 0.5rem 0.6rem;
    border-radius: 0.5rem; border: none; background: transparent; color: var(--system-color-text);
    text-align: left; cursor: pointer; gap: 0.5rem; transition: background 0.1s;
  }
  .entry-item:hover { background: hsla(var(--system-color-dark-hsl) / 0.06); }
  .entry-item.selected { background: hsla(var(--system-color-primary-hsl) / 0.12); }
  .entry-key { font-size: 0.82rem; font-weight: 500; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .entry-date { font-size: 0.68rem; color: var(--system-color-text-muted); white-space: nowrap; }
  .detail-panel {
    overflow-y: auto; border-radius: 1rem; padding: 1rem;
    border: 1px solid var(--system-color-border); background: var(--system-color-panel);
  }
  .detail-header { display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; }
  .detail-header h3 { margin: 0; font-size: 1rem; }
  .cat-badge { padding: 0.15rem 0.5rem; border-radius: 999px; font-size: 0.7rem; font-weight: 600; background: hsla(var(--system-color-primary-hsl) / 0.1); color: var(--system-color-primary); }
  .detail-meta { display: flex; gap: 1rem; margin-top: 0.5rem; font-size: 0.72rem; color: var(--system-color-text-muted); }
  .detail-value {
    margin-top: 0.85rem; font-size: 0.88rem; line-height: 1.6; white-space: pre-wrap; word-break: break-word; color: var(--system-color-text);
  }
  .no-selection { display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; gap: 0.5rem; color: var(--system-color-text-muted); }
  .no-selection span { font-size: 2.5rem; }
  .no-selection p { font-size: 0.85rem; }
  .loading, .empty-state { text-align: center; padding: 2rem; color: var(--system-color-text-muted); }
  .empty-state { display: flex; flex-direction: column; align-items: center; gap: 0.5rem; }
  .empty-icon { font-size: 2.5rem; }
  .empty-state h3 { margin: 0; color: var(--system-color-text); font-size: 1rem; }
  .empty-state p { margin: 0; font-size: 0.82rem; }
</style>
