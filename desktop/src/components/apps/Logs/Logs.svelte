<svelte:options runes={true} />

<script lang="ts">
  import { logStore } from "$lib/stores/logs";

  const LEVEL_COLORS: Record<string, string> = {
    info: "var(--system-color-text)",
    success: "var(--system-color-success)",
    warning: "var(--system-color-warning)",
    error: "var(--system-color-danger)",
  };

  const LEVEL_ICONS: Record<string, string> = {
    info: "i",
    success: "OK",
    warning: "!",
    error: "X",
  };

  function formatTime(iso: string) {
    return new Date(iso).toLocaleTimeString("en-US", {
      hour12: false,
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  }
</script>

<section class="logs-app">
  <div class="header">
    <div>
      <p class="eyebrow">Activity</p>
      <h2>Runtime logs</h2>
    </div>
    <button class="clear" onclick={() => logStore.clear()}>Clear</button>
  </div>

  <div class="terminal" role="log">
    {#each $logStore as entry (entry.id)}
      <div class="line">
        <span class="time">{formatTime(entry.timestamp)}</span>
        <span class="icon">{LEVEL_ICONS[entry.level]}</span>
        {#if entry.app_id}
          <span class="app">{entry.app_id}</span>
        {/if}
        <span class="message" style={`color:${LEVEL_COLORS[entry.level]}`}>{entry.message}</span>
      </div>
    {:else}
      <div class="empty">No activity yet.</div>
    {/each}
  </div>
</section>

<style>
  .logs-app {
    height: 100%;
    display: grid;
    grid-template-rows: auto 1fr;
    gap: 1rem;
    padding: 1.1rem;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .eyebrow {
    margin: 0;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.14em;
    color: var(--system-color-text-muted);
  }

  h2 {
    margin: 0.3rem 0 0;
    font-size: 1.2rem;
  }

  .clear {
    border-radius: 999px;
    padding: 0.6rem 0.95rem;
    border: 1px solid var(--system-color-border);
    background: var(--system-color-panel);
  }

  .terminal {
    overflow: auto;
    border-radius: 1rem;
    padding: 0.95rem;
    background: hsl(218 30% 12%);
    color: hsl(210 28% 94%);
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    border: 1px solid hsla(0 0% 100% / 0.06);
  }

  .line {
    display: flex;
    gap: 0.7rem;
    align-items: baseline;
    padding: 0.2rem 0;
    font-size: 0.83rem;
  }

  .time {
    color: hsla(210 28% 94% / 0.55);
  }

  .icon {
    width: 1rem;
    color: hsla(210 28% 94% / 0.55);
  }

  .app {
    padding: 0.1rem 0.4rem;
    border-radius: 999px;
    background: hsla(210 28% 94% / 0.08);
    color: hsl(198 86% 70%);
  }

  .empty {
    color: hsla(210 28% 94% / 0.55);
  }
</style>
