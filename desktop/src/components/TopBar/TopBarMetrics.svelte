<svelte:options runes={true} />

<script lang="ts">
  import { resourceUsage } from "$lib/stores/state";
  import type { ResourceUsage } from "$lib/api/types";

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

  function memoryTitle(usage: ResourceUsage | null): string {
    if (!usage) {
      return "RAM usage unavailable";
    }

    return `RAM usage: ${formatBytes(usage.memory_used_bytes)} of ${formatBytes(usage.memory_total_bytes)} used`;
  }

  function diskTitle(usage: ResourceUsage | null): string {
    if (!usage) {
      return "Disk usage unavailable";
    }

    return `Disk usage on ${usage.disk_mount_point}: ${formatBytes(usage.disk_used_bytes)} of ${formatBytes(usage.disk_total_bytes)} used`;
  }

  const memoryLabel = $derived($resourceUsage ? `RAM ${$resourceUsage.memory_percent}%` : "RAM --%");
  const diskLabel = $derived($resourceUsage ? `Disk ${$resourceUsage.disk_percent}%` : "Disk --%");
  const memoryTone = $derived($resourceUsage ? usageTone($resourceUsage.memory_percent) : "safe");
  const diskTone = $derived($resourceUsage ? usageTone($resourceUsage.disk_percent) : "safe");
  const memoryTooltip = $derived(memoryTitle($resourceUsage));
  const diskTooltip = $derived(diskTitle($resourceUsage));
</script>

<div class="metrics" data-topbar-metrics aria-live="polite">
  <div
    class={`metric ${memoryTone}`}
    data-topbar-metric="memory"
    aria-label={memoryTooltip}
    title={memoryTooltip}
  >
    <span class="metric-dot" aria-hidden="true"></span>
    <span>{memoryLabel}</span>
  </div>

  <div
    class={`metric ${diskTone}`}
    data-topbar-metric="disk"
    aria-label={diskTooltip}
    title={diskTooltip}
  >
    <span class="metric-dot" aria-hidden="true"></span>
    <span>{diskLabel}</span>
  </div>
</div>

<style>
  .metrics {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding-right: 0.2rem;
  }

  .metric {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    height: 1.15rem;
    padding: 0 0.45rem;
    border-radius: 999px;
    white-space: nowrap;
    font-size: 0.68rem;
    font-weight: 600;
    letter-spacing: 0.02em;
    background:
      linear-gradient(180deg, hsla(var(--system-color-light-hsl) / 0.55), hsla(var(--system-color-light-hsl) / 0.36));
    box-shadow:
      inset 0 0 0 1px hsla(var(--system-color-dark-hsl) / 0.08),
      0 1px 4px hsla(var(--system-color-dark-hsl) / 0.08);
  }

  .metric-dot {
    width: 0.42rem;
    height: 0.42rem;
    border-radius: 999px;
    background-color: var(--system-color-success);
    flex-shrink: 0;
  }

  .metric.warning .metric-dot {
    background-color: var(--system-color-warning);
  }

  .metric.danger .metric-dot {
    background-color: var(--system-color-danger);
  }

  @media (max-width: 720px) {
    .metrics {
      gap: 0.25rem;
    }

    .metric {
      padding: 0 0.35rem;
      font-size: 0.64rem;
    }
  }
</style>
