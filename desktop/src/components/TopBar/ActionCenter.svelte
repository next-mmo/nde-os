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

<div class="relative h-full" use:click_outside={hide}>
  <button
    class="h-full w-max px-2 rounded flex items-center text-black dark:text-white relative before:absolute before:inset-0 before:-z-10 before:rounded before:scale-[var(--scale)] before:origin-center before:transition-transform before:duration-100 before:bg-black/15 dark:before:bg-white/15"
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
    <div class="absolute right-0 mt-[5px] z-[9999] w-[18rem] flex flex-col gap-[0.6rem] p-[0.6rem] bg-white/35 dark:bg-black/35 backdrop-blur-[25px] rounded-[0.85rem] shadow-[0_0_14px_rgba(0,0,0,0.3)] dark:shadow-[inset_0_0_0_0.5px_rgba(255,255,255,0.1),0_0_14px_rgba(0,0,0,0.3)] select-none" use:elevation={"action-center-panel"}>
      <!-- Server Status -->
      <div class="p-2.5 rounded-[0.65rem] bg-white/50 dark:bg-black/50 shadow-[0_1px_4px_-1px_rgba(0,0,0,0.15)]">
        <div class="flex gap-2 items-center">
          <span class="w-[0.55rem] h-[0.55rem] rounded-full shrink-0 {$healthStatus === 'online' ? 'bg-green-500' : 'bg-red-500'}"></span>
          <div class="flex flex-col gap-[0.05rem]">
            <strong class="text-[0.82rem] font-semibold text-black dark:text-white">{$healthStatus === "online" ? "Server Online" : "Server Offline"}</strong>
            <span class="text-[0.72rem] text-gray-500 dark:text-gray-400">localhost:8080</span>
          </div>
        </div>
      </div>

      <!-- Settings Row: Dark Mode + Auto Hide -->
      <div class="p-2.5 rounded-[0.65rem] bg-white/50 dark:bg-black/50 shadow-[0_1px_4px_-1px_rgba(0,0,0,0.15)] flex gap-1.5">
        <button class="flex gap-2 items-center w-full rounded-lg p-1 transition-colors flex-1 hover:bg-black/5 dark:hover:bg-white/5" onclick={toggleTheme}>
          <span class="flex justify-center items-center rounded-full shrink-0 w-[1.6rem] h-[1.6rem] transition-colors {isDark ? 'bg-blue-500 text-white' : 'bg-black/10 dark:bg-white/10 text-gray-500 dark:text-gray-400'}">
            <svg class="w-[0.85rem] h-[0.85rem] {isDark ? 'text-white' : 'text-gray-500 dark:text-gray-400'}" viewBox="0 0 24 24" fill="currentColor"><path d="M21 12.79A9 9 0 1 1 11.21 3a7 7 0 0 0 9.79 9.79z"/></svg>
          </span>
          <span class="text-[0.82rem] font-semibold text-black dark:text-white">Dark Mode</span>
        </button>

        <button class="flex gap-2 items-center w-full rounded-lg p-1 transition-colors flex-1 hover:bg-black/5 dark:hover:bg-white/5" onclick={toggleDockAutoHide}>
          <span class="flex justify-center items-center rounded-full shrink-0 w-[1.6rem] h-[1.6rem] transition-colors {isDockAutoHide ? 'bg-blue-500 text-white' : 'bg-black/10 dark:bg-white/10 text-gray-500 dark:text-gray-400'}">
            <svg class="w-[0.85rem] h-[0.85rem] {isDockAutoHide ? 'text-white' : 'text-gray-500 dark:text-gray-400'}" viewBox="0 0 24 24" fill="currentColor"><path d="M20 3H4c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2Zm0 16H4V5h16v14Zm-4-4h4v2h-4v-2Zm-6 0h4v2h-4v-2ZM4 15h4v2H4v-2Z"/></svg>
          </span>
          <span class="text-[0.82rem] font-semibold text-black dark:text-white">Auto-hide</span>
        </button>
      </div>

      <!-- System Specs -->
      {#if $systemInfo || $resourceUsage}
        <div class="p-2.5 rounded-[0.65rem] bg-white/50 dark:bg-black/50 shadow-[0_1px_4px_-1px_rgba(0,0,0,0.15)]">
          <div class="text-[0.68rem] font-bold uppercase tracking-widest text-gray-500 dark:text-gray-400 mb-[0.4rem]">System</div>
          <div class="flex flex-col gap-1">
            {#if $systemInfo}
              <div class="flex items-center gap-1.5">
                <span class="w-[1.1rem] text-[0.75rem] shrink-0 text-center">💻</span>
                <span class="text-[0.75rem] font-semibold text-black dark:text-white w-11 shrink-0">OS</span>
                <span class="text-[0.72rem] text-gray-500 dark:text-gray-400 whitespace-nowrap overflow-hidden text-ellipsis">{$systemInfo.os} · {$systemInfo.arch}</span>
              </div>
              <div class="flex items-center gap-1.5">
                <span class="w-[1.1rem] text-[0.75rem] shrink-0 text-center">{$systemInfo.gpu_detected ? '🟢' : '⚪'}</span>
                <span class="text-[0.75rem] font-semibold text-black dark:text-white w-11 shrink-0">GPU</span>
                <span class="text-[0.72rem] text-gray-500 dark:text-gray-400 whitespace-nowrap overflow-hidden text-ellipsis">{$systemInfo.gpu_detected ? 'Detected' : 'Not found'}</span>
              </div>
              {#if $systemInfo.python_version}
                <div class="flex items-center gap-1.5">
                  <span class="w-[1.1rem] text-[0.75rem] shrink-0 text-center">🐍</span>
                  <span class="text-[0.75rem] font-semibold text-black dark:text-white w-11 shrink-0">Python</span>
                  <span class="text-[0.72rem] text-gray-500 dark:text-gray-400 whitespace-nowrap overflow-hidden text-ellipsis">{$systemInfo.python_version}</span>
                </div>
              {/if}
            {/if}
            {#if $resourceUsage}
              <div class="flex items-center gap-1.5">
                <span class="w-[1.1rem] text-[0.75rem] shrink-0 text-center">🧠</span>
                <span class="text-[0.75rem] font-semibold text-black dark:text-white w-11 shrink-0">RAM</span>
                <span class="text-[0.72rem] text-gray-500 dark:text-gray-400 whitespace-nowrap overflow-hidden text-ellipsis">{formatBytes($resourceUsage.memory_used_bytes)} / {formatBytes($resourceUsage.memory_total_bytes)}</span>
              </div>
              <div class="h-1 rounded-sm bg-black/10 dark:bg-white/10 ml-6 mt-[0.1rem] mb-[0.15rem] overflow-hidden">
                <div class="h-full rounded-sm transition-[width] duration-400 {$resourceUsage.memory_percent > 80 ? 'bg-red-500' : 'bg-blue-500'}" style:width="{$resourceUsage.memory_percent}%"></div>
              </div>
              <div class="flex items-center gap-1.5">
                <span class="w-[1.1rem] text-[0.75rem] shrink-0 text-center">💾</span>
                <span class="text-[0.75rem] font-semibold text-black dark:text-white w-11 shrink-0">Disk</span>
                <span class="text-[0.72rem] text-gray-500 dark:text-gray-400 whitespace-nowrap overflow-hidden text-ellipsis">{formatBytes($resourceUsage.disk_used_bytes)} / {formatBytes($resourceUsage.disk_total_bytes)}</span>
              </div>
              <div class="h-1 rounded-sm bg-black/10 dark:bg-white/10 ml-6 mt-[0.1rem] mb-[0.15rem] overflow-hidden">
                <div class="h-full rounded-sm transition-[width] duration-400 {$resourceUsage.disk_percent > 85 ? 'bg-red-500' : 'bg-blue-500'}" style:width="{$resourceUsage.disk_percent}%"></div>
              </div>
            {/if}
          </div>
        </div>
      {/if}

      <!-- Stats -->
      <div class="p-2.5 rounded-[0.65rem] bg-white/50 dark:bg-black/50 shadow-[0_1px_4px_-1px_rgba(0,0,0,0.15)]">
        <div class="grid grid-cols-2 gap-2">
          <div class="flex flex-col items-center gap-[0.1rem] py-[0.3rem]">
            <span class="text-[1.3rem] font-bold text-black dark:text-white">{$catalogCount}</span>
            <span class="text-[0.72rem] text-gray-500 dark:text-gray-400">Catalog</span>
          </div>
          <div class="flex flex-col items-center gap-[0.1rem] py-[0.3rem]">
            <span class="text-[1.3rem] font-bold text-black dark:text-white">{$runningCount}</span>
            <span class="text-[0.72rem] text-gray-500 dark:text-gray-400">Running</span>
          </div>
        </div>
      </div>

      <!-- User -->
      <div class="p-2.5 rounded-[0.65rem] bg-white/50 dark:bg-black/50 shadow-[0_1px_4px_-1px_rgba(0,0,0,0.15)]">
        <div class="flex gap-2 items-center">
          <div class="w-8 h-8 rounded-full bg-black/10 dark:bg-white/10 flex items-center justify-center text-[1.1rem]">👤</div>
          <div class="flex flex-col gap-[0.05rem]">
            <strong class="text-[0.82rem] font-semibold text-black dark:text-white">User</strong>
            <span class="text-[0.72rem] text-gray-500 dark:text-gray-400">NDE-OS Session</span>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
