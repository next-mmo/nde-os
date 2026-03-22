<svelte:options runes={true} />

<script lang="ts">
  import { catalogCount, healthStatus, runningCount } from "$lib/stores/state";
  import TopBarTime from "🍎/components/TopBar/TopBarTime.svelte";
  import { activeWindow, toggleLaunchpad, toggleTheme } from "🍎/state/desktop.svelte";

  const title = $derived(activeWindow()?.title ?? "AI Launcher");
</script>

<header class="topbar">
  <div class="cluster left">
    <button class="apple" aria-label="Open Launchpad" onclick={() => toggleLaunchpad(true)}>LP</button>
    <strong>{title}</strong>
  </div>

  <div class="cluster right">
    <span class="pill" class:online={$healthStatus === "online"}>
      {$healthStatus === "online" ? "Server online" : $healthStatus === "offline" ? "Server offline" : "Checking server"}
    </span>
    <span class="meta">{$catalogCount} apps</span>
    <span class="meta">{$runningCount} running</span>
    <button class="icon-button" aria-label="Toggle theme" onclick={toggleTheme}>Theme</button>
    <button class="icon-button" aria-label="Open Launchpad" onclick={() => toggleLaunchpad(true)}>Apps</button>
    <button class="time-button" aria-label="Current time"><TopBarTime /></button>
  </div>
</header>

<style>
  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    padding: 0.45rem 1rem;
    background: hsla(var(--system-color-light-hsl) / 0.35);
    color: var(--system-color-text);
    backdrop-filter: blur(14px);
    border-bottom: 1px solid hsla(0 0% 100% / 0.14);
  }

  .cluster {
    display: flex;
    align-items: center;
    gap: 0.7rem;
    min-width: 0;
  }

  .left strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.9rem;
  }

  .pill,
  .meta,
  .icon-button,
  .time-button {
    border-radius: 999px;
    padding: 0.45rem 0.75rem;
    background: hsla(var(--system-color-light-hsl) / 0.45);
    border: 1px solid var(--system-color-border);
    font-size: 0.78rem;
  }

  .pill.online {
    color: var(--system-color-success);
  }

  .apple {
    font-size: 0.72rem;
    font-weight: 700;
  }

  .icon-button {
    min-width: 3.2rem;
  }

  .time-button {
    min-width: 6rem;
  }
</style>
