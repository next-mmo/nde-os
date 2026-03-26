<svelte:options runes={true} />

<script lang="ts">
  import {
    healthStatus,
    lastRefreshAt,
    refreshAll,
    resourceUsage,
    systemInfo,
  } from "$lib/stores/state";
  import { desktop, toggleDockAutoHide, toggleTheme, resetIconPositions, setWallpaper } from "🍎/state/desktop.svelte";

  const wallpapers = [
    { id: 'ventura', label: 'Ventura', value: 'url("/wallpapers/ventura-1.webp")' },
    { id: 'solid-dark', label: 'Deep Space', value: '#0f172a' },
    { id: 'blue-gradient', label: 'Blue Aura', value: 'linear-gradient(135deg, #0284c7 0%, #4338ca 100%)' },
    { id: 'warm-gradient', label: 'Warm Sunset', value: 'linear-gradient(135deg, #f59e0b 0%, #ef4444 100%)' },
  ];

  let refreshing = $state(false);
  let activeTab = $state("general");
  let searchQuery = $state("");

  const displayVersion = (value: string | null | undefined) => value?.trim() || "Not detected";

  function formatBytes(bytes: number): string {
    if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GB`;
    if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
    return `${Math.round(bytes / 1024)} KB`;
  }

  function getProgressColor(percent: number): string {
    if (percent >= 85) return 'bg-red-500';
    if (percent >= 70) return 'bg-orange-500';
    return 'bg-blue-500';
  }

  async function handleRefresh() {
    refreshing = true;
    await refreshAll();
    refreshing = false;
  }

  const tabs = [
    { id: "general", label: "General", icon: "⚙️" },
    { id: "appearance", label: "Appearance", icon: "🌗" },
    { id: "desktop-dock", label: "Desktop & Dock", icon: "🖥️" },
    { id: "control-center", label: "Control Center", icon: "🎛️" },
  ];
</script>

<div class="h-full w-full flex bg-white/70 dark:bg-black/40 backdrop-blur-2xl text-gray-900 dark:text-gray-100 overflow-hidden font-sans">
  <!-- Sidebar -->
  <aside class="w-[240px] shrink-0 flex flex-col border-r border-black/10 dark:border-white/10 bg-white/30 dark:bg-white/5 py-4 px-3">
    <!-- Search Bar -->
    <div class="relative flex items-center bg-black/5 dark:bg-white/10 rounded-md px-2 py-1.5 mb-4 shadow-sm border border-black/5 dark:border-white/5">
      <svg class="w-4 h-4 text-gray-400 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path d="M21 21l-4.35-4.35m1.35-5.65a7 7 0 11-14 0 7 7 0 0114 0z"/></svg>
      <input 
        type="text" 
        placeholder="Search" 
        bind:value={searchQuery}
        class="bg-transparent border-none outline-none w-full text-[13px] text-black dark:text-white placeholder:text-gray-500" 
      />
    </div>
    
    <!-- Navigation Tabs -->
    <nav class="flex flex-col gap-1 overflow-y-auto">
      {#each tabs as tab}
        {#if !searchQuery || tab.label.toLowerCase().includes(searchQuery.toLowerCase())}
          <button
            class="flex items-center gap-2.5 px-2.5 py-1.5 rounded-lg text-[13px] font-medium transition-all {activeTab === tab.id ? 'bg-blue-500 text-white shadow-sm' : 'hover:bg-black/5 dark:hover:bg-white/10 text-gray-800 dark:text-gray-200'}"
            onclick={() => activeTab = tab.id}
          >
            <div class="w-6 h-6 flex items-center justify-center rounded shadow-sm {activeTab === tab.id ? 'bg-white/20' : 'bg-white dark:bg-white/10 border border-black/5 dark:border-white/5'}">
              {tab.icon}
            </div>
            {tab.label}
          </button>
        {/if}
      {/each}
    </nav>
  </aside>

  <!-- Content Pane -->
  <main class="flex-1 overflow-y-auto p-8 relative scroll-smooth">
    
    <!-- GENERAL PANE -->
    {#if activeTab === "general"}
      <div class="max-w-2xl mx-auto animate-in fade-in slide-in-from-bottom-2 duration-300">
        <header class="flex items-center justify-between mb-8">
          <div class="flex items-center gap-4">
            <div class="w-14 h-14 bg-linear-to-b from-gray-200 to-gray-300 dark:from-gray-700 dark:to-gray-800 rounded-2xl shadow border border-white/20 dark:border-white/10 flex items-center justify-center text-3xl">⚙️</div>
            <h1 class="text-[28px] font-semibold tracking-tight text-black dark:text-white">General</h1>
          </div>
          <button 
            class="px-4 py-1.5 rounded-full bg-white dark:bg-white/10 border border-black/10 dark:border-white/10 shadow-sm text-[13px] font-medium hover:bg-gray-50 dark:hover:bg-white/20 transition disabled:opacity-50" 
            onclick={handleRefresh} 
            disabled={refreshing}
          >
            {refreshing ? "Refreshing..." : "Refresh Status"}
          </button>
        </header>

        {#if $systemInfo}
          <div class="flex flex-col gap-6">
            <!-- Stats overview -->
            <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
              <div class="p-4 rounded-xl bg-white/50 dark:bg-black/30 backdrop-blur-md border border-black/5 dark:border-white/5 shadow-sm">
                <span class="text-[11px] uppercase tracking-wider text-gray-500 font-semibold block mb-1">Status</span>
                <strong class="text-xl font-medium text-black dark:text-white">{$healthStatus}</strong>
              </div>
              <div class="p-4 rounded-xl bg-white/50 dark:bg-black/30 backdrop-blur-md border border-black/5 dark:border-white/5 shadow-sm">
                <span class="text-[11px] uppercase tracking-wider text-gray-500 font-semibold block mb-1">Apps</span>
                <strong class="text-xl font-medium text-black dark:text-white">{$systemInfo.total_apps}</strong>
              </div>
              <div class="p-4 rounded-xl bg-white/50 dark:bg-black/30 backdrop-blur-md border border-black/5 dark:border-white/5 shadow-sm">
                <span class="text-[11px] uppercase tracking-wider text-gray-500 font-semibold block mb-1">Running</span>
                <strong class="text-xl font-medium text-black dark:text-white">{$systemInfo.running_apps}</strong>
              </div>
              <div class="p-4 rounded-xl bg-white/50 dark:bg-black/30 backdrop-blur-md border border-black/5 dark:border-white/5 shadow-sm">
                <span class="text-[11px] uppercase tracking-wider text-gray-500 font-semibold block mb-1">Last Sync</span>
                <strong class="text-sm font-medium text-black dark:text-white block mt-1.5">{$lastRefreshAt ? new Date($lastRefreshAt).toLocaleTimeString() : "Never"}</strong>
              </div>
            </div>

            <!-- Host specific details inside a macOS like list group -->
            <div class="bg-white/60 dark:bg-black/30 backdrop-blur-xl border border-black/5 dark:border-white/10 rounded-xl overflow-hidden shadow-sm">
              <div class="px-5 py-3 border-b border-black/5 dark:border-white/5 flex justify-between items-center bg-black/5 dark:bg-white/5">
                <span class="text-[13px] font-semibold text-black dark:text-white">Host Information</span>
              </div>
              <div class="p-5 grid grid-cols-1 md:grid-cols-2 gap-y-4 gap-x-8 text-[13px]">
                <div class="flex justify-between border-b border-black/5 dark:border-white/5 pb-2">
                  <span class="text-gray-500">OS / Arch</span>
                  <span class="font-medium text-black dark:text-white">{$systemInfo.os} / {$systemInfo.arch}</span>
                </div>
                <div class="flex justify-between border-b border-black/5 dark:border-white/5 pb-2">
                  <span class="text-gray-500">Python</span>
                  <span class="font-medium text-black dark:text-white">{$systemInfo.python_version ?? "Not detected"}</span>
                </div>
                <div class="flex justify-between border-b border-black/5 dark:border-white/5 pb-2">
                  <span class="text-gray-500">GPU</span>
                  <span class="font-medium text-black dark:text-white">{$systemInfo.gpu_detected ? "Detected" : "Not detected"}</span>
                </div>
                <div class="flex justify-between border-b border-black/5 dark:border-white/5 pb-2">
                  <span class="text-gray-500">Base Dir</span>
                  <span class="font-medium text-black dark:text-white truncate max-w-[150px]" title={$systemInfo.base_dir}>...{$systemInfo.base_dir.slice(-15)}</span>
                </div>
              </div>
            </div>

            <!-- Resources list group -->
            <div class="bg-white/60 dark:bg-black/30 backdrop-blur-xl border border-black/5 dark:border-white/10 rounded-xl overflow-hidden shadow-sm">
              <div class="px-5 py-3 border-b border-black/5 dark:border-white/5 flex justify-between items-center bg-black/5 dark:bg-white/5">
                <span class="text-[13px] font-semibold text-black dark:text-white">Live Resources</span>
              </div>
              <div class="p-5 flex flex-col gap-6">
                {#if $resourceUsage}
                  <div>
                    <div class="flex justify-between mb-1.5 text-[13px]">
                      <span class="text-black dark:text-white font-medium">Memory Usage</span>
                      <strong class="text-gray-500">{$resourceUsage.memory_percent}%</strong>
                    </div>
                    <div class="h-1.5 w-full bg-black/10 dark:bg-white/10 rounded-full overflow-hidden mb-1.5">
                      <div class="h-full {getProgressColor($resourceUsage.memory_percent)} transition-all duration-500" style="width: {$resourceUsage.memory_percent}%"></div>
                    </div>
                    <span class="text-[11px] text-gray-500 block">{formatBytes($resourceUsage.memory_used_bytes)} of {formatBytes($resourceUsage.memory_total_bytes)} used</span>
                  </div>

                  <div>
                    <div class="flex justify-between mb-1.5 text-[13px]">
                      <span class="text-black dark:text-white font-medium">Disk ({$resourceUsage.disk_mount_point})</span>
                      <strong class="text-gray-500">{$resourceUsage.disk_percent}%</strong>
                    </div>
                    <div class="h-1.5 w-full bg-black/10 dark:bg-white/10 rounded-full overflow-hidden mb-1.5">
                      <div class="h-full {getProgressColor($resourceUsage.disk_percent)} transition-all duration-500" style="width: {$resourceUsage.disk_percent}%"></div>
                    </div>
                    <span class="text-[11px] text-gray-500 block">{formatBytes($resourceUsage.disk_used_bytes)} of {formatBytes($resourceUsage.disk_total_bytes)} used</span>
                  </div>
                {:else}
                  <div class="text-[13px] text-gray-500">Loading resources...</div>
                {/if}
              </div>
            </div>

          </div>
        {:else}
          <div class="flex justify-center mt-20">
            <span class="text-sm text-gray-500 animate-pulse">Loading system info...</span>
          </div>
        {/if}
      </div>
    {/if}

    <!-- APPEARANCE PANE -->
    {#if activeTab === "appearance"}
      <div class="max-w-2xl mx-auto animate-in fade-in slide-in-from-bottom-2 duration-300">
        <header class="flex items-center gap-4 mb-8">
          <div class="w-14 h-14 bg-linear-to-br from-indigo-500 to-purple-600 rounded-lg shadow-md border border-white/20 flex items-center justify-center text-3xl">🌗</div>
          <h1 class="text-[28px] font-semibold tracking-tight text-black dark:text-white">Appearance</h1>
        </header>

        <div class="bg-white/60 dark:bg-black/30 backdrop-blur-xl border border-black/5 dark:border-white/10 rounded-xl overflow-hidden shadow-sm p-6 mb-6">
          <div class="flex items-center justify-between">
            <div>
              <h2 class="text-[14px] font-medium text-black dark:text-white">Dark Mode</h2>
              <p class="text-[12px] text-gray-500 mt-1">Adjust the appearance of the OS to reduce eye strain in low-light environments.</p>
            </div>
            
            <button 
              role="switch" 
              aria-label="Toggle Dark Mode"
              aria-checked={desktop.theme === 'dark'}
              class="relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 {desktop.theme === 'dark' ? 'bg-blue-500' : 'bg-gray-300 dark:bg-gray-600'}"
              onclick={toggleTheme}
            >
              <span class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {desktop.theme === 'dark' ? 'translate-x-5' : 'translate-x-0'}"></span>
            </button>
          </div>
        </div>

        <div class="bg-white/60 dark:bg-black/30 backdrop-blur-xl border border-black/5 dark:border-white/10 rounded-xl overflow-hidden shadow-sm p-6 mb-6">
          <div class="mb-4">
            <h2 class="text-[14px] font-medium text-black dark:text-white">Wallpaper</h2>
            <p class="text-[12px] text-gray-500 mt-1">Select a background for your desktop.</p>
          </div>
          
          <div class="grid grid-cols-2 gap-4 mt-4">
            {#each wallpapers as wp}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div 
                class="relative h-24 rounded-lg overflow-hidden cursor-pointer border-2 transition-all {desktop.wallpaper === wp.value ? 'border-blue-500 scale-[1.02] shadow-md' : 'border-transparent hover:border-black/20 dark:hover:border-white/20'}"
                style="background: {wp.value.includes('url') ? `center / cover no-repeat ${wp.value}` : wp.value};"
                onclick={() => setWallpaper(wp.value)}
              >
                <div class="absolute bottom-0 left-0 right-0 bg-black/50 backdrop-blur-sm p-1.5 text-center">
                  <span class="text-white text-[11px] font-medium drop-shadow-sm">{wp.label}</span>
                </div>
                {#if desktop.wallpaper === wp.value}
                  <div class="absolute top-2 right-2 w-5 h-5 bg-blue-500 rounded-full flex items-center justify-center text-white shadow-sm">
                    <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><path d="M5 13l4 4L19 7"/></svg>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      </div>
    {/if}

    <!-- DESKTOP & DOCK PANE -->
    {#if activeTab === "desktop-dock"}
      <div class="max-w-2xl mx-auto animate-in fade-in slide-in-from-bottom-2 duration-300">
        <header class="flex items-center gap-4 mb-8">
          <div class="w-14 h-14 bg-linear-to-br from-cyan-400 to-blue-500 rounded-lg shadow-md border border-white/20 flex items-center justify-center text-3xl">🖥️</div>
          <h1 class="text-[28px] font-semibold tracking-tight text-black dark:text-white">Desktop & Dock</h1>
        </header>

        <div class="bg-white/60 dark:bg-black/30 backdrop-blur-xl border border-black/5 dark:border-white/10 rounded-xl overflow-hidden shadow-sm mb-6 flex flex-col divide-y divide-black/5 dark:divide-white/5">
          <div class="flex items-center justify-between p-6">
            <div>
              <h2 class="text-[14px] font-medium text-black dark:text-white">Automatically hide and show the Dock</h2>
              <p class="text-[12px] text-gray-500 mt-1">Retracts the dock to give you more screen space when not in use.</p>
            </div>
            
            <button 
              role="switch" 
              aria-label="Toggle Dock Auto-Hide"
              aria-checked={desktop.dock_auto_hide}
              class="relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 {desktop.dock_auto_hide ? 'bg-blue-500' : 'bg-gray-300 dark:bg-gray-600'}"
              onclick={toggleDockAutoHide}
            >
              <span class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {desktop.dock_auto_hide ? 'translate-x-5' : 'translate-x-0'}"></span>
            </button>
          </div>
          
          <div class="flex items-center justify-between p-6">
            <div>
              <h2 class="text-[14px] font-medium text-black dark:text-white">Reset Desktop App Icons</h2>
              <p class="text-[12px] text-gray-500 mt-1">Restores all icons on the desktop to their default positions.</p>
            </div>
            <button 
              class="px-4 py-1.5 rounded bg-gray-200 hover:bg-gray-300 dark:bg-white/10 dark:hover:bg-white/20 text-[13px] font-medium text-black dark:text-white transition shadow-sm border border-black/5 dark:border-white/5"
              onclick={resetIconPositions}
            >
              Reset Layout
            </button>
          </div>
        </div>
      </div>
    {/if}

    <!-- CONTROL CENTER PANE -->
    {#if activeTab === "control-center"}
      <div class="max-w-2xl mx-auto animate-in fade-in slide-in-from-bottom-2 duration-300">
        <header class="flex items-center gap-4 mb-8">
          <div class="w-14 h-14 bg-linear-to-br from-gray-700 to-black rounded-lg shadow-md border border-white/20 flex items-center justify-center text-3xl">🎛️</div>
          <h1 class="text-[28px] font-semibold tracking-tight text-black dark:text-white">Control Center</h1>
        </header>

        <div class="bg-white/60 dark:bg-black/30 backdrop-blur-xl border border-black/5 dark:border-white/10 rounded-xl overflow-hidden shadow-sm p-6 mb-6">
          <div class="flex flex-col items-center justify-center py-10 opacity-70">
            <span class="text-4xl mb-4">🚧</span>
            <h2 class="text-sm font-medium text-black dark:text-white">Control Center configuration is coming soon.</h2>
            <p class="text-[12px] text-gray-500 mt-2 text-center max-w-sm">Future updates will allow you to manage which modules appear in the menu bar and Control Center dropdown.</p>
          </div>
        </div>
      </div>
    {/if}
  </main>
</div>
