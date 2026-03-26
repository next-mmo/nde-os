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

  let wifiEnabled = $state(true);
  let bluetoothEnabled = $state(false);
  let focusEnabled = $state(false);
  let volume = $state(50);

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

      <!-- Control Center Quick Actions (Ventura Style) -->
      <div class="grid grid-cols-2 gap-2">
        <!-- Connectivity Block -->
        <div class="flex flex-col gap-2 p-3 rounded-[0.85rem] bg-white/50 dark:bg-black/50 shadow-[0_1px_4px_-1px_rgba(0,0,0,0.15)] justify-center">
          <button class="flex items-center gap-3 w-full text-left" onclick={() => (wifiEnabled = !wifiEnabled)}>
            <span class="flex justify-center items-center rounded-full shrink-0 w-[1.8rem] h-[1.8rem] transition-colors {wifiEnabled ? 'bg-blue-500 text-white' : 'bg-black/10 dark:bg-white/10 text-gray-400'}">
              <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M5 12.55a11 11 0 0114.08 0 M1.42 9a16 16 0 0121.16 0 M8.53 16.11a6 6 0 016.95 0 M12 20h.01"/></svg>
            </span>
            <div class="flex flex-col">
              <span class="text-[0.82rem] font-semibold text-black dark:text-white leading-tight">Wi-Fi</span>
              <span class="text-[0.68rem] text-gray-500 dark:text-gray-400 leading-tight">{wifiEnabled ? 'NDE-Net' : 'Off'}</span>
            </div>
          </button>
          
          <div class="h-[1px] w-full bg-black/5 dark:bg-white/5 ml-9"></div>

          <button class="flex items-center gap-3 w-full text-left" onclick={() => (bluetoothEnabled = !bluetoothEnabled)}>
            <span class="flex justify-center items-center rounded-full shrink-0 w-[1.8rem] h-[1.8rem] transition-colors {bluetoothEnabled ? 'bg-blue-500 text-white' : 'bg-black/10 dark:bg-white/10 text-gray-400'}">
              <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6.5 6.5l11 11L12 23V1l5.5 5.5-11 11"/></svg>
            </span>
            <div class="flex flex-col">
              <span class="text-[0.82rem] font-semibold text-black dark:text-white leading-tight">Bluetooth</span>
              <span class="text-[0.68rem] text-gray-500 dark:text-gray-400 leading-tight">{bluetoothEnabled ? 'On' : 'Off'}</span>
            </div>
          </button>
        </div>

        <!-- Toggles Block -->
        <div class="grid grid-rows-2 gap-2">
          <!-- Focus -->
          <button class="flex items-center gap-2 px-3 rounded-[0.85rem] bg-white/50 dark:bg-black/50 shadow-[0_1px_4px_-1px_rgba(0,0,0,0.15)] transition-colors hover:bg-white/70 dark:hover:bg-black/70" onclick={() => (focusEnabled = !focusEnabled)}>
            <span class="flex justify-center items-center rounded-full shrink-0 w-[1.6rem] h-[1.6rem] transition-colors {focusEnabled ? 'bg-[#5856d6] text-white' : 'bg-black/10 dark:bg-white/10 text-gray-500'}">
              <svg class="w-4 h-4" viewBox="0 0 24 24" fill="currentColor"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 14.5v-9l6 4.5-6 4.5z"/></svg>
            </span>
            <span class="text-[0.82rem] font-semibold text-black dark:text-white">Focus</span>
          </button>

          <!-- Dark Mode -->
          <button class="flex items-center gap-2 px-3 rounded-[0.85rem] bg-white/50 dark:bg-black/50 shadow-[0_1px_4px_-1px_rgba(0,0,0,0.15)] transition-colors hover:bg-white/70 dark:hover:bg-black/70" onclick={toggleTheme}>
            <span class="flex justify-center items-center rounded-full shrink-0 w-[1.6rem] h-[1.6rem] transition-colors {isDark ? 'bg-blue-500 text-white' : 'bg-black/10 dark:bg-white/10 text-gray-500'}">
              <svg class="w-[0.85rem] h-[0.85rem] text-current" viewBox="0 0 24 24" fill="currentColor"><path d="M21 12.79A9 9 0 1 1 11.21 3a7 7 0 0 0 9.79 9.79z"/></svg>
            </span>
            <span class="text-[0.82rem] font-semibold text-black dark:text-white">Dark Mode</span>
          </button>
        </div>
      </div>

      <!-- Volume & Dock -->
      <div class="p-3 rounded-[0.85rem] bg-white/50 dark:bg-black/50 shadow-[0_1px_4px_-1px_rgba(0,0,0,0.15)] flex flex-col gap-3">
        <!-- Volume Slider -->
        <div class="flex items-center gap-3">
          <svg class="w-4 h-4 text-gray-500 dark:text-gray-400 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M11 5L6 9H2v6h4l5 4V5z"/><path d="M19.07 4.93a10 10 0 010 14.14M15.54 8.46a5 5 0 010 7.07"/></svg>
          <input type="range" class="w-full h-1.5 bg-black/10 dark:bg-white/10 rounded-full appearance-none outline-none accent-black dark:accent-white cursor-pointer" min="0" max="100" bind:value={volume}>
        </div>

        <!-- Dock Auto Hide Toggle -->
        <button class="flex gap-2 items-center w-full transition-colors group" onclick={toggleDockAutoHide}>
          <span class="flex justify-center items-center rounded-full shrink-0 w-[1.6rem] h-[1.6rem] transition-colors {isDockAutoHide ? 'bg-blue-500 text-white' : 'bg-black/10 dark:bg-white/10 text-gray-500 dark:text-gray-400'}">
            <svg class="w-[0.85rem] h-[0.85rem] text-current" viewBox="0 0 24 24" fill="currentColor"><path d="M20 3H4c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2Zm0 16H4V5h16v14Zm-4-4h4v2h-4v-2Zm-6 0h4v2h-4v-2ZM4 15h4v2H4v-2Z"/></svg>
          </span>
          <span class="text-[0.82rem] font-semibold text-black dark:text-white">Auto-hide Dock</span>
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
