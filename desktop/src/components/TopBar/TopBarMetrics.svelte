<svelte:options runes={true} />

<script lang="ts">
  import { resourceUsage, systemInfo } from "$lib/stores/state";
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
  const gpuDetected = $derived($systemInfo?.gpu_detected ?? false);
  const gpuLabel = $derived(gpuDetected ? "GPU ✓" : "GPU ✗");
  const gpuTone = $derived(gpuDetected ? "safe" : "off");
  const gpuTooltip = $derived(gpuDetected ? "NVIDIA GPU detected" : "No GPU detected");
</script>

<div class="metrics" data-topbar-metrics aria-live="polite">
  <span class="metric" title={memoryTooltip}>{memoryLabel}</span>
  <span class="metric" title={diskTooltip}>{diskLabel}</span>
  <span class="metric" title={gpuTooltip}>{gpuLabel}</span>
</div>

<style>
  .metrics {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0 0.3rem;
    position: relative;
    z-index: 1;
  }

  .metric {
    font-size: 0.72rem;
    font-weight: 500;
    color: var(--system-color-text);
    white-space: nowrap;
    letter-spacing: 0.01em;
  }

  @media (max-width: 720px) {
    .metrics {
      gap: 0.3rem;
    }

    .metric {
      font-size: 0.66rem;
    }
  }
</style>
