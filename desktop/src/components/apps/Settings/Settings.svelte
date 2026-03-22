<svelte:options runes={true} />

<script lang="ts">
  import {
    healthStatus,
    lastRefreshAt,
    refreshAll,
    resourceUsage,
    systemInfo,
  } from "$lib/stores/state";

  let refreshing = $state(false);

  const displayVersion = (value: string | null | undefined) => value?.trim() || "Not detected";

  function formatBytes(bytes: number): string {
    if (bytes >= 1024 ** 3) {
      return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
    }

    if (bytes >= 1024 ** 2) {
      return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
    }

    return `${Math.round(bytes / 1024)} KB`;
  }

  function usageTone(percent: number): "safe" | "warning" | "danger" {
    if (percent >= 85) {
      return "danger";
    }

    if (percent >= 70) {
      return "warning";
    }

    return "safe";
  }

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
          <div><dt>Version</dt><dd>{displayVersion($systemInfo.uv.uv_version)}</dd></div>
        </dl>
      </article>

      <article class="card">
        <h3>Live resources</h3>
        {#if $resourceUsage}
          <div class="resource-list">
            <section class="resource-item" data-resource-card="memory">
              <div class="resource-heading">
                <span>Memory</span>
                <strong>{$resourceUsage.memory_percent}%</strong>
              </div>
              <div class={`resource-bar ${usageTone($resourceUsage.memory_percent)}`} style:--fill={`${$resourceUsage.memory_percent}%`}>
                <span></span>
              </div>
              <p>{formatBytes($resourceUsage.memory_used_bytes)} of {formatBytes($resourceUsage.memory_total_bytes)} used</p>
            </section>

            <section class="resource-item" data-resource-card="disk">
              <div class="resource-heading">
                <span>Disk</span>
                <strong>{$resourceUsage.disk_percent}%</strong>
              </div>
              <div class={`resource-bar ${usageTone($resourceUsage.disk_percent)}`} style:--fill={`${$resourceUsage.disk_percent}%`}>
                <span></span>
              </div>
              <p>{formatBytes($resourceUsage.disk_used_bytes)} of {formatBytes($resourceUsage.disk_total_bytes)} used on {$resourceUsage.disk_mount_point}</p>
            </section>
          </div>
        {:else}
          <div class="loading">Loading live resource usage...</div>
        {/if}
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
    grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
    gap: 0.85rem;
  }

  .details {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
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

  .resource-list {
    margin-top: 1rem;
    display: grid;
    gap: 0.85rem;
  }

  .resource-item {
    display: grid;
    gap: 0.45rem;
  }

  .resource-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .resource-heading span {
    font-size: 0.82rem;
    font-weight: 600;
  }

  .resource-heading strong {
    font-size: 1.15rem;
  }

  .resource-bar {
    --fill: 0%;

    height: 0.5rem;
    overflow: hidden;
    border-radius: 999px;
    background-color: hsla(var(--system-color-dark-hsl) / 0.08);
  }

  .resource-bar span {
    display: block;
    width: var(--fill);
    height: 100%;
    border-radius: inherit;
    background: linear-gradient(90deg, var(--system-color-success), var(--system-color-primary));
  }

  .resource-bar.warning span {
    background: linear-gradient(90deg, var(--system-color-warning), var(--system-color-primary));
  }

  .resource-bar.danger span {
    background: linear-gradient(90deg, var(--system-color-warning), var(--system-color-danger));
  }

  .resource-item p,
  .loading {
    margin: 0;
    color: var(--system-color-text-muted);
  }
</style>
