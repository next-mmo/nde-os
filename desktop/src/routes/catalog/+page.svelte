<script lang="ts">
  import AppCard from "$lib/components/AppCard.svelte";
  import { catalog, installedMap, searchFilter } from "$lib/stores/state";

  let filter = $state("");

  const filtered = $derived(
    $catalog
      .filter((a) =>
        !filter ||
        a.name.toLowerCase().includes(filter.toLowerCase()) ||
        a.tags?.some((t) => t.toLowerCase().includes(filter.toLowerCase()))
      )
  );
</script>

<div class="page">
  <div class="page-header">
    <div>
      <h1>App Catalog</h1>
      <p class="subtitle">Browse and install AI applications</p>
    </div>
    <div class="search-box">
      <span class="search-icon">🔍</span>
      <input
        type="text"
        bind:value={filter}
        placeholder="Search apps, tags..."
        class="search-input"
      />
    </div>
  </div>

  <div class="grid stagger-children">
    {#each filtered as app (app.id)}
      <AppCard {app} installed={$installedMap[app.id] || null} />
    {/each}
  </div>

  {#if filtered.length === 0}
    <div class="empty">
      <div class="empty-icon">🔍</div>
      <div class="empty-text">No apps match "{filter}"</div>
    </div>
  {/if}
</div>

<style>
  .page { padding: 24px 28px; max-width: 900px; }

  .page-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 20px; gap: 16px; }
  h1 { font-size: 18px; font-weight: 700; color: var(--color-text); margin: 0; }
  .subtitle { font-size: 12px; color: var(--color-text-dim); margin-top: 2px; }

  .search-box {
    display: flex; align-items: center; gap: 8px;
    background: var(--color-surface); border: 1px solid var(--color-border);
    border-radius: 8px; padding: 6px 12px; min-width: 240px;
    transition: border-color 0.2s;
  }
  .search-box:focus-within { border-color: var(--color-accent); }
  .search-icon { font-size: 14px; opacity: 0.5; }
  .search-input {
    background: none; border: none; outline: none; color: var(--color-text);
    font-size: 13px; width: 100%; font-family: inherit;
  }
  .search-input::placeholder { color: var(--color-text-muted); }

  .grid { display: flex; flex-direction: column; gap: 10px; }

  .empty { text-align: center; padding: 60px 0; }
  .empty-icon { font-size: 32px; margin-bottom: 8px; }
  .empty-text { font-size: 12px; color: var(--color-text-muted); font-family: var(--font-mono); }
</style>
