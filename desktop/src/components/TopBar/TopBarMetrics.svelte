<svelte:options runes={true} />

<script lang="ts">
  import { resourceUsage, systemInfo, healthStatus, llmActiveModel, llmProviderCount } from "$lib/stores/state";
  import { click_outside, elevation } from "🍎/actions";
  import { desktop, openStaticApp } from "🍎/state/desktop.svelte";
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

  const isOnline = $derived($healthStatus === "online");
  const serverLabel = $derived(isOnline ? "Online" : "Offline");
  const llmLabel = $derived($llmActiveModel || "No LLM");
  const providerCountLabel = $derived(`${$llmProviderCount} provider${$llmProviderCount !== 1 ? "s" : ""}`);

  function openModelSettings(e: Event) {
    e.stopPropagation();
    openStaticApp("model-settings");
  }
</script>

<div class="relative h-full z-10" use:click_outside={close}>
  <div
    class="flex items-center gap-[0.3rem] sm:gap-[0.5rem] px-[0.45rem] h-full rounded-[0.25rem] cursor-pointer hover:bg-black/10 dark:hover:bg-white/10"
    data-topbar-metrics
    role="button"
    tabindex="0"
    aria-label="System metrics"
    onclick={toggle}
    onkeydown={(e) => e.key === 'Enter' && toggle()}
  >
    <span class="w-[0.42rem] h-[0.42rem] rounded-full shrink-0 transition-colors duration-300 {isOnline ? 'bg-green-500 shadow-[0_0_4px_rgba(52,211,153,0.5)]' : 'bg-red-500'}" title={serverLabel}></span>
    <span class="text-[0.72rem] font-semibold text-black dark:text-white whitespace-nowrap cursor-pointer rounded-[0.2rem] px-[0.15rem] transition-colors duration-150 max-w-32 overflow-hidden text-ellipsis hover:text-blue-500" role="button" tabindex="0" title="Active LLM: {llmLabel}" onclick={openModelSettings} onkeydown={(e) => e.key === 'Enter' && openModelSettings(e)}>
      {llmLabel}
    </span>
    <span class="text-[0.72rem] text-gray-500 dark:text-gray-400 opacity-50">·</span>
    <span class="text-[0.66rem] sm:text-[0.72rem] font-medium text-black dark:text-white whitespace-nowrap tracking-[0.01em]">{memoryLabel}</span>
    <span class="text-[0.66rem] sm:text-[0.72rem] font-medium text-black dark:text-white whitespace-nowrap tracking-[0.01em]">{diskLabel}</span>
    <span class="text-[0.66rem] sm:text-[0.72rem] font-medium text-black dark:text-white whitespace-nowrap tracking-[0.01em]">{gpuLabel}</span>
  </div>

  {#if expanded}
    <div class="absolute right-0 top-[calc(100%+5px)] z-9999 w-80 p-[0.7rem] flex flex-col gap-[0.1rem] bg-white/40 dark:bg-black/40 backdrop-blur-[25px] rounded-[0.85rem] shadow-[0_0_14px_rgba(0,0,0,0.3)] dark:shadow-[inset_0_0_0_0.5px_rgba(255,255,255,0.1),0_0_14px_rgba(0,0,0,0.3)] select-none" use:elevation={"system-info-panel"}>
      <div class="text-[0.72rem] font-bold uppercase tracking-[0.04em] text-gray-500 dark:text-gray-400 px-[0.15rem] pb-[0.35rem]">System Info</div>

      {#if $systemInfo}
        <div class="flex flex-col gap-[0.3rem] py-[0.35rem] px-2 rounded-[0.6rem] bg-white/50 dark:bg-black/50 shadow-[0_1px_3px_-1px_rgba(0,0,0,0.12)]">
          <div class="flex items-center gap-[0.4rem]">
            <span class="w-[1.1rem] text-[0.72rem] shrink-0 text-center">💻</span>
            <span class="text-[0.72rem] font-semibold text-black dark:text-white w-[3.2rem] shrink-0">Platform</span>
            <span class="text-[0.7rem] text-gray-500 dark:text-gray-400 whitespace-nowrap overflow-hidden text-ellipsis min-w-0">{$systemInfo.os} · {$systemInfo.arch}</span>
          </div>
          {#if $systemInfo.python_version}
            <div class="flex items-center gap-[0.4rem]">
              <span class="w-[1.1rem] text-[0.72rem] shrink-0 text-center">🐍</span>
              <span class="text-[0.72rem] font-semibold text-black dark:text-white w-[3.2rem] shrink-0">Python</span>
              <span class="text-[0.7rem] text-gray-500 dark:text-gray-400 whitespace-nowrap overflow-hidden text-ellipsis min-w-0">{$systemInfo.python_version}</span>
            </div>
          {/if}
          {#if $systemInfo.uv}
            <div class="flex items-center gap-[0.4rem]">
              <span class="w-[1.1rem] text-[0.72rem] shrink-0 text-center">📦</span>
              <span class="text-[0.72rem] font-semibold text-black dark:text-white w-[3.2rem] shrink-0">uv</span>
              <span class="text-[0.7rem] text-gray-500 dark:text-gray-400 whitespace-nowrap overflow-hidden text-ellipsis min-w-0">{$systemInfo.uv.uv_version}</span>
            </div>
          {/if}
          <div class="flex items-center gap-[0.4rem]">
            <span class="w-[1.1rem] text-[0.72rem] shrink-0 text-center">{gpuDetected ? '🟢' : '⚪'}</span>
            <span class="text-[0.72rem] font-semibold text-black dark:text-white w-[3.2rem] shrink-0">GPU</span>
            <span class="text-[0.7rem] text-gray-500 dark:text-gray-400 whitespace-nowrap overflow-hidden text-ellipsis min-w-0">{gpuDetected ? 'NVIDIA Detected' : 'Not detected'}</span>
          </div>
          <div class="flex items-center gap-[0.4rem]">
            <span class="w-[1.1rem] text-[0.72rem] shrink-0 text-center">📁</span>
            <span class="text-[0.72rem] font-semibold text-black dark:text-white w-[3.2rem] shrink-0">Base Dir</span>
            <span class="text-[0.7rem] text-gray-500 dark:text-gray-400 whitespace-nowrap overflow-hidden text-ellipsis min-w-0" style="direction: rtl; text-align: left;" title={$systemInfo.base_dir}>{$systemInfo.base_dir}</span>
          </div>
        </div>
      {/if}

      {#if $resourceUsage}
        <div class="h-0 my-1"></div>
        <div class="flex flex-col gap-[0.3rem] py-[0.35rem] px-2 rounded-[0.6rem] bg-white/50 dark:bg-black/50 shadow-[0_1px_3px_-1px_rgba(0,0,0,0.12)]">
          <div class="flex items-center gap-[0.4rem]">
            <span class="w-[1.1rem] text-[0.72rem] shrink-0 text-center">🧠</span>
            <span class="text-[0.72rem] font-semibold text-black dark:text-white w-[3.2rem] shrink-0">RAM</span>
            <span class="text-[0.7rem] text-gray-500 dark:text-gray-400 whitespace-nowrap overflow-hidden text-ellipsis min-w-0">{formatBytes($resourceUsage.memory_used_bytes)} / {formatBytes($resourceUsage.memory_total_bytes)} ({$resourceUsage.memory_percent}%)</span>
          </div>
          <div class="h-1 rounded-sm bg-black/10 dark:bg-white/10 ml-6 mb-[0.1rem] overflow-hidden">
            <div
              class="h-full rounded-sm transition-[width] duration-400 {$resourceUsage.memory_percent >= 85 ? 'bg-red-500' : $resourceUsage.memory_percent >= 70 ? 'bg-yellow-500' : 'bg-green-500'}"
              style:width="{$resourceUsage.memory_percent}%"
            ></div>
          </div>

          <div class="flex items-center gap-[0.4rem]">
            <span class="w-[1.1rem] text-[0.72rem] shrink-0 text-center">💾</span>
            <span class="text-[0.72rem] font-semibold text-black dark:text-white w-[3.2rem] shrink-0">Disk</span>
            <span class="text-[0.7rem] text-gray-500 dark:text-gray-400 whitespace-nowrap overflow-hidden text-ellipsis min-w-0">{formatBytes($resourceUsage.disk_used_bytes)} / {formatBytes($resourceUsage.disk_total_bytes)} ({$resourceUsage.disk_percent}%)</span>
          </div>
          <div class="h-1 rounded-sm bg-black/10 dark:bg-white/10 ml-6 mb-[0.1rem] overflow-hidden">
            <div
              class="h-full rounded-sm transition-[width] duration-400 {$resourceUsage.disk_percent >= 85 ? 'bg-red-500' : $resourceUsage.disk_percent >= 70 ? 'bg-yellow-500' : 'bg-green-500'}"
              style:width="{$resourceUsage.disk_percent}%"
            ></div>
          </div>

          <div class="flex items-center gap-[0.4rem]">
            <span class="w-[1.1rem] text-[0.72rem] shrink-0 text-center">📂</span>
            <span class="text-[0.72rem] font-semibold text-black dark:text-white w-[3.2rem] shrink-0">Mount</span>
            <span class="text-[0.7rem] text-gray-500 dark:text-gray-400 whitespace-nowrap overflow-hidden text-ellipsis min-w-0">{$resourceUsage.disk_mount_point}</span>
          </div>
        </div>
      {/if}

      {#if $systemInfo}
        <div class="h-0 my-1"></div>
        <div class="flex flex-col gap-[0.3rem] py-[0.35rem] px-2 rounded-[0.6rem] bg-white/50 dark:bg-black/50 shadow-[0_1px_3px_-1px_rgba(0,0,0,0.12)]">
          <div class="flex items-center gap-[0.4rem]">
            <span class="w-[1.1rem] text-[0.72rem] shrink-0 text-center">📊</span>
            <span class="text-[0.72rem] font-semibold text-black dark:text-white w-[3.2rem] shrink-0">Apps</span>
            <span class="text-[0.7rem] text-gray-500 dark:text-gray-400 whitespace-nowrap overflow-hidden text-ellipsis min-w-0">{$systemInfo.total_apps} installed · {$systemInfo.running_apps} running</span>
          </div>
        </div>
      {/if}

      <div class="h-0 my-1"></div>
      <div class="flex flex-col gap-[0.3rem] py-[0.35rem] px-2 rounded-[0.6rem] bg-white/50 dark:bg-black/50 shadow-[0_1px_3px_-1px_rgba(0,0,0,0.12)]">
        <div class="flex items-center gap-[0.4rem]">
          <span class="w-[1.1rem] text-[0.72rem] shrink-0 text-center">🤖</span>
          <span class="text-[0.72rem] font-semibold text-black dark:text-white w-[3.2rem] shrink-0">Model</span>
          <span class="text-[0.7rem] text-gray-500 dark:text-gray-400 whitespace-nowrap overflow-hidden text-ellipsis min-w-0">{llmLabel}</span>
        </div>
        <div class="flex items-center gap-[0.4rem]">
          <span class="w-[1.1rem] text-[0.72rem] shrink-0 text-center">🔌</span>
          <span class="text-[0.72rem] font-semibold text-black dark:text-white w-[3.2rem] shrink-0">LLM</span>
          <span class="text-[0.7rem] text-gray-500 dark:text-gray-400 whitespace-nowrap overflow-hidden text-ellipsis min-w-0">{providerCountLabel}</span>
        </div>
        <button class="text-[0.7rem] font-semibold text-blue-500 cursor-pointer mt-[0.15rem] text-left py-[0.15rem] rounded transition-opacity duration-150 hover:opacity-75" onclick={openModelSettings}>Configure →</button>
      </div>
    </div>
  {/if}
</div>
