<script lang="ts">
  import { systemInfo, refreshSystemInfo } from "$lib/stores/state";

  let refreshing = $state(false);

  async function handleRefresh() {
    refreshing = true;
    await refreshSystemInfo();
    refreshing = false;
  }
</script>

<div class="page">
  <div class="page-header">
    <div>
      <h1>Settings</h1>
      <p class="subtitle">System information and configuration</p>
    </div>
    <button class="btn-refresh" onclick={handleRefresh} disabled={refreshing}>
      <span class:animate-spin={refreshing}>🔄</span> Refresh
    </button>
  </div>

  {#if $systemInfo}
    <div class="section">
      <h2>System</h2>
      <div class="info-grid">
        <div class="info-row">
          <span class="info-label">Operating System</span>
          <span class="info-value">{$systemInfo.os} / {$systemInfo.arch}</span>
        </div>
        <div class="info-row">
          <span class="info-label">Python</span>
          <span class="info-value">{$systemInfo.python_version || "Not detected"}</span>
        </div>
        <div class="info-row">
          <span class="info-label">GPU</span>
          <span class="info-value">{$systemInfo.gpu_detected ? "✅ NVIDIA detected" : "❌ Not detected"}</span>
        </div>
        <div class="info-row">
          <span class="info-label">Data Directory</span>
          <span class="info-value mono">{$systemInfo.base_dir}</span>
        </div>
      </div>
    </div>

    <div class="section">
      <h2>UV Package Manager</h2>
      <div class="info-grid">
        <div class="info-row">
          <span class="info-label">Path</span>
          <span class="info-value mono">{$systemInfo.uv.uv_path}</span>
        </div>
        <div class="info-row">
          <span class="info-label">Version</span>
          <span class="info-value">{$systemInfo.uv.uv_version}</span>
        </div>
      </div>
    </div>

    <div class="section">
      <h2>Stats</h2>
      <div class="stats">
        <div class="stat">
          <div class="stat-value">{$systemInfo.total_apps}</div>
          <div class="stat-label">Installed</div>
        </div>
        <div class="stat">
          <div class="stat-value">{$systemInfo.running_apps}</div>
          <div class="stat-label">Running</div>
        </div>
      </div>
    </div>
  {:else}
    <div class="loading">Loading system information...</div>
  {/if}
</div>

<style>
  .page { padding: 24px 28px; max-width: 700px; }

  .page-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 24px; }
  h1 { font-size: 18px; font-weight: 700; color: var(--color-text); margin: 0; }
  .subtitle { font-size: 12px; color: var(--color-text-dim); margin-top: 2px; }

  .btn-refresh {
    font-size: 12px; padding: 6px 14px; border-radius: 6px;
    background: var(--color-surface); border: 1px solid var(--color-border);
    color: var(--color-text-dim); cursor: pointer; display: flex; align-items: center; gap: 4px;
    transition: all 0.15s;
  }
  .btn-refresh:hover { border-color: var(--color-accent); color: var(--color-accent); }
  .btn-refresh:disabled { opacity: 0.5; cursor: not-allowed; }

  .section { margin-bottom: 24px; }
  h2 {
    font-size: 13px; font-weight: 600; color: var(--color-text-dim);
    text-transform: uppercase; letter-spacing: 0.05em;
    margin: 0 0 12px 0; padding-bottom: 8px; border-bottom: 1px solid var(--color-border);
  }

  .info-grid { display: flex; flex-direction: column; gap: 2px; }
  .info-row {
    display: flex; justify-content: space-between; align-items: center;
    padding: 8px 12px; border-radius: 6px;
    background: var(--color-surface);
  }
  .info-row:hover { background: var(--color-surface-hover); }
  .info-label { font-size: 13px; color: var(--color-text-dim); }
  .info-value { font-size: 13px; color: var(--color-text); }
  .mono { font-family: var(--font-mono); font-size: 11px; }

  .stats { display: flex; gap: 12px; }
  .stat {
    flex: 1; text-align: center; padding: 20px;
    background: var(--color-surface); border: 1px solid var(--color-border);
    border-radius: var(--radius);
  }
  .stat-value { font-size: 28px; font-weight: 700; color: var(--color-accent); }
  .stat-label { font-size: 12px; color: var(--color-text-muted); margin-top: 4px; }

  .loading { color: var(--color-text-muted); font-size: 13px; padding: 40px; text-align: center; }
</style>
