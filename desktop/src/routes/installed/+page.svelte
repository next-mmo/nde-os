<script lang="ts">
  import AppCard from "$lib/components/AppCard.svelte";
  import { installed } from "$lib/stores/state";
</script>

<div class="page">
  <div class="page-header">
    <h1>Installed Apps</h1>
    <p class="subtitle">{$installed.length} app(s) installed</p>
  </div>

  {#if $installed.length > 0}
    <div class="grid stagger-children">
      {#each $installed as app (app.manifest.id)}
        <AppCard app={app.manifest} installed={app} />
      {/each}
    </div>
  {:else}
    <div class="empty">
      <div class="empty-icon">📭</div>
      <div class="empty-title">No apps installed</div>
      <div class="empty-hint">Head to the <a href="/catalog">Catalog</a> to install your first app</div>
    </div>
  {/if}
</div>

<style>
  .page { padding: 24px 28px; max-width: 900px; }
  .page-header { margin-bottom: 20px; }
  h1 { font-size: 18px; font-weight: 700; color: var(--color-text); margin: 0; }
  .subtitle { font-size: 12px; color: var(--color-text-dim); margin-top: 2px; }
  .grid { display: flex; flex-direction: column; gap: 10px; }

  .empty { text-align: center; padding: 60px 0; }
  .empty-icon { font-size: 40px; margin-bottom: 8px; }
  .empty-title { font-size: 14px; font-weight: 600; color: var(--color-text-dim); }
  .empty-hint { font-size: 12px; color: var(--color-text-muted); margin-top: 4px; }
  .empty-hint a { color: var(--color-accent); text-decoration: none; }
  .empty-hint a:hover { text-decoration: underline; }
</style>
