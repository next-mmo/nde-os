<svelte:options runes={true} />

<script lang="ts">
  import { resourceUsage, systemInfo } from "$lib/stores/state";
  import { click_outside, elevation } from "🍎/actions";
  import { desktop } from "🍎/state/desktop.svelte";
  import type { ResourceUsage } from "$lib/api/types";

  let expanded = $state(false);
  const isDark = $derived(desktop.theme === "dark");

  function formatBytes(bytes: number): string {
    if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
    if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
    return `${Math.round(bytes / 1024)} KB`;
  }

  function usageTone(percent: number): "safe" | "warning" | "danger" {
    if (percent >= 85) return "danger";
    if (percent >= 70) return "warning";
    return "safe";
  }

  function memoryTitle(usage: ResourceUsage | null): string {
    if (!usage) return "RAM usage unavailable";
    return `RAM usage: ${formatBytes(usage.memory_used_bytes)} of ${formatBytes(usage.memory_total_bytes)} used`;
  }

  function diskTitle(usage: ResourceUsage | null): string {
    if (!usage) return "Disk usage unavailable";
    return `Disk usage on ${usage.disk_mount_point}: ${formatBytes(usage.disk_used_bytes)} of ${formatBytes(usage.disk_total_bytes)} used`;
  }

  const memoryLabel = $derived($resourceUsage ? `RAM ${$resourceUsage.memory_percent}%` : "RAM --%");
  const diskLabel = $derived($resourceUsage ? `Disk ${$resourceUsage.disk_percent}%` : "Disk --%");
  const memoryTooltip = $derived(memoryTitle($resourceUsage));
  const diskTooltip = $derived(diskTitle($resourceUsage));
  const gpuDetected = $derived($systemInfo?.gpu_detected ?? false);
  const gpuLabel = $derived(gpuDetected ? "GPU ✓" : "GPU ✗");
  const gpuTooltip = $derived(gpuDetected ? "NVIDIA GPU detected" : "No GPU detected");

  function toggle() { expanded = !expanded; }
  function close() { expanded = false; }
</script>

<div class="metrics-container" use:click_outside={close}>
  <button
    class="metrics-toggle"
    data-topbar-metrics
    aria-label="System metrics"
    onclick={toggle}
  >
    <span class="metric">{memoryLabel}</span>
    <span class="metric">{diskLabel}</span>
    <span class="metric">{gpuLabel}</span>
  </button>

  {#if expanded}
    <div class="info-panel" class:dark={isDark} use:elevation={"system-info-panel"}>
      <div class="panel-title">System Info</div>

      {#if $systemInfo}
        <div class="info-section">
          <div class="info-row">
            <span class="info-icon">💻</span>
            <span class="info-key">Platform</span>
            <span class="info-val">{$systemInfo.os} · {$systemInfo.arch}</span>
          </div>
          {#if $systemInfo.python_version}
            <div class="info-row">
              <span class="info-icon">🐍</span>
              <span class="info-key">Python</span>
              <span class="info-val">{$systemInfo.python_version}</span>
            </div>
          {/if}
          {#if $systemInfo.uv}
            <div class="info-row">
              <span class="info-icon">📦</span>
              <span class="info-key">uv</span>
              <span class="info-val">{$systemInfo.uv.uv_version}</span>
            </div>
          {/if}
          <div class="info-row">
            <span class="info-icon">{gpuDetected ? '🟢' : '⚪'}</span>
            <span class="info-key">GPU</span>
            <span class="info-val">{gpuDetected ? 'NVIDIA Detected' : 'Not detected'}</span>
          </div>
          <div class="info-row">
            <span class="info-icon">📁</span>
            <span class="info-key">Base Dir</span>
            <span class="info-val truncate" title={$systemInfo.base_dir}>{$systemInfo.base_dir}</span>
          </div>
        </div>
      {/if}

      {#if $resourceUsage}
        <div class="info-divider"></div>
        <div class="info-section">
          <div class="info-row">
            <span class="info-icon">🧠</span>
            <span class="info-key">RAM</span>
            <span class="info-val">{formatBytes($resourceUsage.memory_used_bytes)} / {formatBytes($resourceUsage.memory_total_bytes)} ({$resourceUsage.memory_percent}%)</span>
          </div>
          <div class="meter-track">
            <div
              class="meter-fill"
              class:warning={$resourceUsage.memory_percent >= 70 && $resourceUsage.memory_percent < 85}
              class:danger={$resourceUsage.memory_percent >= 85}
              style:width="{$resourceUsage.memory_percent}%"
            ></div>
          </div>

          <div class="info-row">
            <span class="info-icon">💾</span>
            <span class="info-key">Disk</span>
            <span class="info-val">{formatBytes($resourceUsage.disk_used_bytes)} / {formatBytes($resourceUsage.disk_total_bytes)} ({$resourceUsage.disk_percent}%)</span>
          </div>
          <div class="meter-track">
            <div
              class="meter-fill"
              class:warning={$resourceUsage.disk_percent >= 70 && $resourceUsage.disk_percent < 85}
              class:danger={$resourceUsage.disk_percent >= 85}
              style:width="{$resourceUsage.disk_percent}%"
            ></div>
          </div>

          <div class="info-row">
            <span class="info-icon">📂</span>
            <span class="info-key">Mount</span>
            <span class="info-val">{$resourceUsage.disk_mount_point}</span>
          </div>
        </div>
      {/if}

      {#if $systemInfo}
        <div class="info-divider"></div>
        <div class="info-section">
          <div class="info-row">
            <span class="info-icon">📊</span>
            <span class="info-key">Apps</span>
            <span class="info-val">{$systemInfo.total_apps} installed · {$systemInfo.running_apps} running</span>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .metrics-container {
    position: relative;
    height: 100%;
    z-index: 1;
  }

  .metrics-toggle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0 0.45rem;
    height: 100%;
    border-radius: 0.25rem;
    cursor: pointer;
  }

  .metrics-toggle:hover {
    background-color: hsla(var(--system-color-dark-hsl) / 0.1);
  }

  .metric {
    font-size: 0.72rem;
    font-weight: 500;
    color: var(--system-color-text);
    white-space: nowrap;
    letter-spacing: 0.01em;
  }

  /* ── Popover panel ── */

  .info-panel {
    --border-size: 0;
    position: absolute;
    right: 0;
    top: calc(100% + 5px);
    z-index: 9999;
    width: 20rem;
    padding: 0.7rem;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;

    background-color: hsla(var(--system-color-light-hsl) / 0.40);
    backdrop-filter: blur(25px);
    border-radius: 0.85rem;
    box-shadow:
      hsla(0, 0%, 0%, 0.3) 0px 0px 14px 0px,
      inset 0 0 0 var(--border-size) hsla(var(--system-color-dark-hsl) / 0.3),
      0 0 0 var(--border-size) hsla(var(--system-color-light-hsl) / 0.3);
    user-select: none;
  }

  .info-panel.dark {
    --border-size: 0.5px;
  }

  .panel-title {
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--system-color-text-muted);
    padding: 0 0.15rem 0.35rem;
  }

  .info-section {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    padding: 0.35rem 0.5rem;
    border-radius: 0.6rem;
    background-color: hsla(var(--system-color-light-hsl) / 0.5);
    box-shadow: hsla(0, 0%, 0%, 0.12) 0px 1px 3px -1px;
  }

  .info-divider {
    height: 0;
    margin: 0.25rem 0;
  }

  .info-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .info-icon {
    width: 1.1rem;
    font-size: 0.72rem;
    flex-shrink: 0;
    text-align: center;
  }

  .info-key {
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--system-color-text);
    width: 3.2rem;
    flex-shrink: 0;
  }

  .info-val {
    font-size: 0.7rem;
    color: var(--system-color-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .truncate {
    direction: rtl;
    text-align: left;
  }

  .meter-track {
    height: 4px;
    border-radius: 2px;
    background-color: hsla(var(--system-color-dark-hsl) / 0.08);
    margin: 0 0 0.1rem 1.5rem;
    overflow: hidden;
  }

  .meter-fill {
    height: 100%;
    border-radius: 2px;
    background-color: var(--system-color-success);
    transition: width 0.4s ease;
  }

  .meter-fill.warning {
    background-color: var(--system-color-warning);
  }

  .meter-fill.danger {
    background-color: var(--system-color-danger);
  }

  @media (max-width: 720px) {
    .metrics-toggle {
      gap: 0.3rem;
    }

    .metric {
      font-size: 0.66rem;
    }
  }
</style>
