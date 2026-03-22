<script lang="ts">
  import { logStore } from "$lib/stores/logs";

  const LEVEL_COLORS: Record<string, string> = {
    info: "var(--color-text-dim)",
    success: "var(--color-success)",
    warning: "var(--color-warning)",
    error: "var(--color-danger)",
  };

  const LEVEL_ICONS: Record<string, string> = {
    info: "ℹ️", success: "✅", warning: "⚠️", error: "❌",
  };

  function formatTime(iso: string) {
    const d = new Date(iso);
    return d.toLocaleTimeString("en-US", { hour12: false, hour: "2-digit", minute: "2-digit", second: "2-digit" });
  }
</script>

<div class="page">
  <div class="page-header">
    <div>
      <h1>Activity Logs</h1>
      <p class="subtitle">Real-time activity feed</p>
    </div>
    <button class="btn-clear" onclick={() => logStore.clear()}>🗑 Clear</button>
  </div>

  <div class="terminal">
    {#each $logStore as entry (entry.id)}
      <div class="log-line animate-fade-in">
        <span class="log-time">{formatTime(entry.timestamp)}</span>
        <span class="log-icon">{LEVEL_ICONS[entry.level]}</span>
        {#if entry.app_id}
          <span class="log-app">[{entry.app_id}]</span>
        {/if}
        <span class="log-msg" style="color: {LEVEL_COLORS[entry.level]}">{entry.message}</span>
      </div>
    {:else}
      <div class="log-empty">
        <span style="opacity: 0.3; font-size: 18px">📋</span>
        <span style="color: var(--color-text-muted); font-size: 12px">No activity yet</span>
      </div>
    {/each}
  </div>
</div>

<style>
  .page { padding: 24px 28px; height: calc(100vh - 48px); display: flex; flex-direction: column; }

  .page-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 16px; }
  h1 { font-size: 18px; font-weight: 700; color: var(--color-text); margin: 0; }
  .subtitle { font-size: 12px; color: var(--color-text-dim); margin-top: 2px; }

  .btn-clear {
    font-size: 11px; padding: 5px 12px; border-radius: 6px;
    background: var(--color-surface); border: 1px solid var(--color-border);
    color: var(--color-text-dim); cursor: pointer; transition: all 0.15s;
  }
  .btn-clear:hover { border-color: var(--color-danger); color: var(--color-danger); }

  .terminal {
    flex: 1; overflow-y: auto;
    background: var(--color-surface); border: 1px solid var(--color-border);
    border-radius: var(--radius); padding: 12px;
    font-family: var(--font-mono); font-size: 12px;
  }

  .log-line { display: flex; align-items: baseline; gap: 8px; padding: 3px 0; line-height: 1.5; }
  .log-time { color: var(--color-text-muted); font-size: 10px; flex-shrink: 0; }
  .log-icon { font-size: 12px; flex-shrink: 0; }
  .log-app { color: var(--color-accent); font-size: 11px; flex-shrink: 0; }
  .log-msg { word-break: break-word; }

  .log-empty { display: flex; flex-direction: column; align-items: center; justify-content: center; height: 200px; gap: 8px; }
</style>
