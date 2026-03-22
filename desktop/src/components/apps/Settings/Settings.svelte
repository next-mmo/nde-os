<svelte:options runes={true} />

<script lang="ts">
  import { healthStatus, lastRefreshAt, refreshAll, systemInfo } from "$lib/stores/state";

  let refreshing = $state(false);

  async function handleRefresh() {
    refreshing = true;
    await refreshAll();
    refreshing = false;
  }
</script>

<section class="settings-app">
  <div class="header">
    <div>
      <p class="eyebrow">System</p>
      <h2>Server and runtime</h2>
    </div>
    <button class="refresh" onclick={handleRefresh} disabled={refreshing}>
      {refreshing ? "Refreshing..." : "Refresh"}
    </button>
  </div>

  {#if $systemInfo}
    <div class="grid">
      <article class="card stat">
        <span>Status</span>
        <strong>{$healthStatus}</strong>
      </article>
      <article class="card stat">
        <span>Installed apps</span>
        <strong>{$systemInfo.total_apps}</strong>
      </article>
      <article class="card stat">
        <span>Running apps</span>
        <strong>{$systemInfo.running_apps}</strong>
      </article>
      <article class="card stat">
        <span>Last refresh</span>
        <strong>{$lastRefreshAt ? new Date($lastRefreshAt).toLocaleTimeString() : "Never"}</strong>
      </article>
    </div>

    <div class="details">
      <article class="card">
        <h3>Host</h3>
        <dl>
          <div><dt>OS</dt><dd>{$systemInfo.os} / {$systemInfo.arch}</dd></div>
          <div><dt>Python</dt><dd>{$systemInfo.python_version ?? "Not detected"}</dd></div>
          <div><dt>GPU</dt><dd>{$systemInfo.gpu_detected ? "Detected" : "Not detected"}</dd></div>
          <div><dt>Base dir</dt><dd>{$systemInfo.base_dir}</dd></div>
        </dl>
      </article>

      <article class="card">
        <h3>uv</h3>
        <dl>
          <div><dt>Path</dt><dd>{$systemInfo.uv.uv_path}</dd></div>
          <div><dt>Version</dt><dd>{$systemInfo.uv.uv_version}</dd></div>
        </dl>
      </article>
    </div>
  {:else}
    <div class="loading">Loading system information...</div>
  {/if}
</section>

<style>
  .settings-app {
    height: 100%;
    overflow: auto;
    padding: 1.1rem;
    display: grid;
    gap: 1rem;
    align-content: start;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
  }

  .eyebrow {
    margin: 0;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.14em;
    color: var(--system-color-text-muted);
  }

  h2,
  h3 {
    margin: 0.3rem 0 0;
  }

  .refresh {
    border-radius: 999px;
    padding: 0.65rem 1rem;
    background: linear-gradient(180deg, hsla(var(--system-color-primary-hsl) / 0.16), hsla(var(--system-color-primary-hsl) / 0.08));
    border: 1px solid hsla(var(--system-color-primary-hsl) / 0.22);
    color: var(--system-color-primary);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.85rem;
  }

  .details {
    display: grid;
    grid-template-columns: 1.2fr 1fr;
    gap: 0.85rem;
  }

  .card {
    border-radius: 1.1rem;
    padding: 1rem;
    border: 1px solid var(--system-color-border);
    background: var(--system-color-panel);
  }

  .stat span {
    display: block;
    color: var(--system-color-text-muted);
    font-size: 0.78rem;
  }

  .stat strong {
    display: block;
    margin-top: 0.45rem;
    font-size: 1.25rem;
  }

  dl {
    margin: 1rem 0 0;
    display: grid;
    gap: 0.7rem;
  }

  dl div {
    display: grid;
    gap: 0.2rem;
  }

  dt {
    font-size: 0.74rem;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--system-color-text-muted);
  }

  dd {
    margin: 0;
    font-size: 0.9rem;
    word-break: break-word;
  }

  .loading {
    color: var(--system-color-text-muted);
  }

  @media (max-width: 900px) {
    .grid,
    .details {
      grid-template-columns: 1fr;
    }
  }
</style>
