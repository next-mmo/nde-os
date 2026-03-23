<svelte:options runes={true} />

<script lang="ts">
  import { catalogCount, healthStatus, runningCount, systemInfo, resourceUsage } from "$lib/stores/state";
  import { desktop, toggleTheme, toggleDockAutoHide } from "🍎/state/desktop.svelte";
  import { click_outside, elevation } from "🍎/actions";

  let visible = $state(false);

  const isDark = $derived(desktop.theme === "dark");
  const isDockAutoHide = $derived(desktop.dock_auto_hide);

  function formatBytes(bytes: number): string {
    if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(1)} GB`;
    if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(0)} MB`;
    return `${(bytes / 1024).toFixed(0)} KB`;
  }

  function show() { visible = true; }
  function hide() { visible = false; }
</script>

<div class="ac-container" use:click_outside={hide}>
  <button
    class="ac-toggle"
    style:--scale={visible ? 1 : 0}
    onclick={show}
    onfocus={show}
    aria-label="Toggle Control Center"
  >
    <svg width="14" height="12" viewBox="0 0 14 12" fill="currentColor">
      <rect x="0" y="0" width="6" height="5" rx="1.2" />
      <rect x="8" y="0" width="6" height="5" rx="1.2" />
      <rect x="0" y="7" width="6" height="5" rx="1.2" />
      <rect x="8" y="7" width="6" height="5" rx="1.2" />
    </svg>
  </button>

  {#if visible}
    <div class="ac-panel" use:elevation={"action-center-panel"} class:dark={isDark}>
      <!-- Server Status -->
      <div class="ac-surface">
        <div class="ac-tile" class:online={$healthStatus === "online"}>
          <span class="status-dot"></span>
          <div class="tile-label">
            <strong>{$healthStatus === "online" ? "Server Online" : "Server Offline"}</strong>
            <span>localhost:8080</span>
          </div>
        </div>
      </div>

      <!-- Settings Row: Dark Mode + Auto Hide -->
      <div class="ac-surface row">
        <button class="ac-tile clickable flex-half" onclick={toggleTheme}>
          <span class="toggle-icon" class:filled={isDark}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M21 12.79A9 9 0 1 1 11.21 3a7 7 0 0 0 9.79 9.79z"/></svg>
          </span>
          <span class="tile-text">Dark Mode</span>
        </button>

        <button class="ac-tile clickable flex-half" onclick={toggleDockAutoHide}>
          <span class="toggle-icon" class:filled={isDockAutoHide}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M20 3H4c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2Zm0 16H4V5h16v14Zm-4-4h4v2h-4v-2Zm-6 0h4v2h-4v-2ZM4 15h4v2H4v-2Z"/></svg>
          </span>
          <span class="tile-text">Auto-hide</span>
        </button>
      </div>

      <!-- System Specs -->
      {#if $systemInfo || $resourceUsage}
        <div class="ac-surface">
          <div class="spec-header">System</div>
          <div class="spec-grid">
            {#if $systemInfo}
              <div class="spec-row">
                <span class="spec-icon">💻</span>
                <span class="spec-key">OS</span>
                <span class="spec-val">{$systemInfo.os} · {$systemInfo.arch}</span>
              </div>
              <div class="spec-row">
                <span class="spec-icon">{$systemInfo.gpu_detected ? '🟢' : '⚪'}</span>
                <span class="spec-key">GPU</span>
                <span class="spec-val">{$systemInfo.gpu_detected ? 'Detected' : 'Not found'}</span>
              </div>
              {#if $systemInfo.python_version}
                <div class="spec-row">
                  <span class="spec-icon">🐍</span>
                  <span class="spec-key">Python</span>
                  <span class="spec-val">{$systemInfo.python_version}</span>
                </div>
              {/if}
            {/if}
            {#if $resourceUsage}
              <div class="spec-row">
                <span class="spec-icon">🧠</span>
                <span class="spec-key">RAM</span>
                <span class="spec-val">{formatBytes($resourceUsage.memory_used_bytes)} / {formatBytes($resourceUsage.memory_total_bytes)}</span>
              </div>
              <div class="meter-track">
                <div class="meter-fill" class:warn={$resourceUsage.memory_percent > 80} style:width="{$resourceUsage.memory_percent}%"></div>
              </div>
              <div class="spec-row">
                <span class="spec-icon">💾</span>
                <span class="spec-key">Disk</span>
                <span class="spec-val">{formatBytes($resourceUsage.disk_used_bytes)} / {formatBytes($resourceUsage.disk_total_bytes)}</span>
              </div>
              <div class="meter-track">
                <div class="meter-fill" class:warn={$resourceUsage.disk_percent > 85} style:width="{$resourceUsage.disk_percent}%"></div>
              </div>
            {/if}
          </div>
        </div>
      {/if}

      <!-- Stats -->
      <div class="ac-surface">
        <div class="ac-stat-row">
          <div class="stat">
            <span class="stat-value">{$catalogCount}</span>
            <span class="stat-label">Catalog</span>
          </div>
          <div class="stat">
            <span class="stat-value">{$runningCount}</span>
            <span class="stat-label">Running</span>
          </div>
        </div>
      </div>

      <!-- User -->
      <div class="ac-surface">
        <div class="ac-tile user-tile">
          <div class="user-avatar">👤</div>
          <div class="tile-label">
            <strong>User</strong>
            <span>NDE-OS Session</span>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .ac-container {
    position: relative;
    height: 100%;
  }

  .ac-toggle {
    height: 100%;
    width: max-content;
    padding: 0 0.5rem;
    border-radius: 0.25rem;
    position: relative;
    display: flex;
    align-items: center;
    color: var(--system-color-text);
  }

  .ac-toggle::before {
    content: "";
    position: absolute;
    inset: 0;
    z-index: -1;
    border-radius: inherit;
    transform: scale(var(--scale));
    transform-origin: center center;
    transition: transform 100ms ease;
    background-color: hsla(var(--system-color-dark-hsl) / 0.15);
  }

  .ac-panel {
    --border-size: 0;

    position: absolute;
    right: 0;
    margin-top: 5px;
    z-index: 9999;
    width: 18rem;

    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 0.6rem;

    background-color: hsla(var(--system-color-light-hsl) / 0.35);
    backdrop-filter: blur(25px);
    border-radius: 0.85rem;

    box-shadow:
      hsla(0, 0%, 0%, 0.3) 0px 0px 14px 0px,
      inset 0 0 0 var(--border-size) hsla(var(--system-color-dark-hsl) / 0.3),
      0 0 0 var(--border-size) hsla(var(--system-color-light-hsl) / 0.3);

    user-select: none;
  }

  .ac-panel.dark {
    --border-size: 0.5px;
  }

  .ac-surface {
    padding: 0.55rem 0.65rem;
    border-radius: 0.65rem;
    background-color: hsla(var(--system-color-light-hsl) / 0.5);
    box-shadow: hsla(0, 0%, 0%, 0.15) 0px 1px 4px -1px;
  }

  .ac-tile {
    display: flex;
    gap: 0.55rem;
    align-items: center;
  }

  .tile-label {
    display: flex;
    flex-direction: column;
    gap: 0.05rem;
  }

  .tile-label strong {
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--system-color-text);
  }

  .tile-label span,
  .stat-label {
    font-size: 0.72rem;
    color: var(--system-color-text-muted);
  }

  .status-dot {
    width: 0.55rem;
    height: 0.55rem;
    border-radius: 50%;
    background-color: var(--system-color-danger);
    flex-shrink: 0;
  }

  .ac-tile.online .status-dot {
    background-color: var(--system-color-success);
  }

  .row {
    display: flex;
    gap: 0.4rem;
  }

  .clickable {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    width: 100%;
    border-radius: 0.5rem;
    padding: 0.3rem;
    transition: background-color 120ms ease;
  }

  .flex-half {
    flex: 1 1 50%;
  }

  .clickable:hover {
    background-color: hsla(var(--system-color-dark-hsl) / 0.08);
  }

  .tile-text {
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--system-color-text);
  }

  .toggle-icon {
    --size: 1.6rem;
    height: var(--size);
    width: var(--size);
    display: flex;
    justify-content: center;
    align-items: center;
    border-radius: 50%;
    background-color: hsla(var(--system-color-dark-hsl) / 0.1);
    transition: background-color 150ms ease;
    flex-shrink: 0;
  }

  .toggle-icon :global(svg) {
    width: 0.85rem;
    height: 0.85rem;
    color: var(--system-color-text-muted);
  }

  .toggle-icon.filled {
    background-color: var(--system-color-primary);
  }

  .toggle-icon.filled :global(svg) {
    color: white;
  }

  .ac-stat-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }

  .stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.1rem;
    padding: 0.3rem 0;
  }

  .stat-value {
    font-size: 1.3rem;
    font-weight: 700;
    color: var(--system-color-text);
  }

  .user-tile .user-avatar {
    width: 2rem;
    height: 2rem;
    border-radius: 50%;
    background-color: hsla(var(--system-color-dark-hsl) / 0.1);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.1rem;
  }

  .spec-header {
    font-size: 0.68rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--system-color-text-muted);
    margin-bottom: 0.4rem;
  }

  .spec-grid {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .spec-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .spec-icon {
    width: 1.1rem;
    font-size: 0.75rem;
    flex-shrink: 0;
    text-align: center;
  }

  .spec-key {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--system-color-text);
    width: 2.8rem;
    flex-shrink: 0;
  }

  .spec-val {
    font-size: 0.72rem;
    color: var(--system-color-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .meter-track {
    height: 4px;
    border-radius: 2px;
    background-color: hsla(var(--system-color-dark-hsl) / 0.08);
    margin: 0.1rem 0 0.15rem 1.5rem;
    overflow: hidden;
  }

  .meter-fill {
    height: 100%;
    border-radius: 2px;
    background-color: var(--system-color-primary);
    transition: width 0.4s ease;
  }

  .meter-fill.warn {
    background-color: var(--system-color-danger);
  }
</style>
