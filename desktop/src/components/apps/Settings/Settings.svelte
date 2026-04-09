<svelte:options runes={true} />

<script lang="ts">
  import {
    healthStatus,
    lastRefreshAt,
    refreshAll,
    resourceUsage,
    systemInfo,
  } from "$lib/stores/state";
  import { onDestroy } from "svelte";
  import { desktop, toggleDockAutoHide, toggleTheme, resetIconPositions, setWallpaper, toggleStartExpanded } from "🍎/state/desktop.svelte";
  import {
    wallpaperSettings,
    fetchWallpaper,
    startRotation,
    stopRotation,
    restartRotation,
    setWallpaperEnabled,
    setWallpaperCategory,
    setWallpaperInterval,
    WALLPAPER_CATEGORIES,
    type WallpaperCategory,
    type RotationUnit,
  } from "🍎/state/wallpaper.svelte";

  const wallpapers = [
    { id: 'ventura', label: 'Ventura', value: 'url("/wallpapers/ventura-1.webp")' },
    { id: 'solid-dark', label: 'Deep Space', value: '#0f172a' },
    { id: 'blue-gradient', label: 'Blue Aura', value: 'linear-gradient(135deg, #0284c7 0%, #4338ca 100%)' },
    { id: 'warm-gradient', label: 'Warm Sunset', value: 'linear-gradient(135deg, #f59e0b 0%, #ef4444 100%)' },
  ];

  let refreshing = $state(false);
  
  interface Props {
    window?: any;
  }
  let { window }: Props = $props();

  let activeTab = $state("general");

  $effect(() => {
    if (window?.data?.tab && window.data.tab !== activeTab) {
      activeTab = window.data.tab;
    }
  });

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

  // ── Online Wallpaper Handlers ─────────────────────────────────────────
  const wpSetSettings = wallpaperSettings;
  const categories = WALLPAPER_CATEGORIES;
  const unitOptions: { id: RotationUnit; label: string }[] = [
    { id: "seconds", label: "Sec" },
    { id: "minutes", label: "Min" },
    { id: "hours", label: "Hr" },
  ];

  let wpFetching = $state(false);

  // ── Preview State ─────────────────────────────────────────────────────
  // previewUrl: raw image URL of the fetched image (null = no preview pending)
  // previewBg: the CSS background value for display in the monitor preview
  // displayWallpaper: derived from the desktop state for the active wallpaper
  let previewUrl = $state<string | null>(null);
  let previewBg = $state<string | null>(null);
  const displayWallpaper = $derived(desktop.wallpaper);

  function applyPreview() {
    if (!previewUrl) return;
    const cssValue = `url("${previewUrl}")`;
    setWallpaper(cssValue);
    previewUrl = null;
    previewBg = null;
    // Restart rotation from now
    if (wpSetSettings.enabled) {
      restartRotation(applyWallpaperCallback);
    }
  }

  function dismissPreview() {
    previewUrl = null;
    previewBg = null;
  }

  function applyWallpaperCallback(cssValue: string) {
    setWallpaper(cssValue);
  }

  async function handleToggleOnline() {
    const next = !wpSetSettings.enabled;
    setWallpaperEnabled(next);
    if (next) {
      // Fetch first wallpaper and apply it immediately to the desktop
      wpFetching = true;
      try {
        const url = await fetchWallpaper();
        if (url) {
          const cssValue = `url("${url}")`;
          setWallpaper(cssValue);
          // Clear preview — wallpaper is already live on the desktop
          previewUrl = null;
          previewBg = null;
        }
      } finally {
        wpFetching = false;
      }
      startRotation(applyWallpaperCallback);
    } else {
      stopRotation();
      previewUrl = null;
      previewBg = null;
    }
  }

  /** Fetch next wallpaper and show in preview (not applied yet) */
  async function handlePreviewNext() {
    if (wpFetching) return;
    wpFetching = true;
    try {
      const url = await fetchWallpaper();
      if (url) {
        previewUrl = url;
        previewBg = `url("${url}")`;
      }
    } finally {
      wpFetching = false;
    }
  }

  /** Fetch random-category wallpaper and show in preview */
  async function handlePreviewRandom() {
    if (wpFetching) return;
    wpFetching = true;
    try {
      const randomCategories: WallpaperCategory[] = [
        "nature", "space", "abstract", "landscape", "ocean",
        "mountains", "forest", "city", "sunset", "animals", "flowers", "winter",
      ];
      const randomCat = randomCategories[Math.floor(Math.random() * randomCategories.length)];
      const url = await fetchWallpaper(randomCat);
      if (url) {
        previewUrl = url;
        previewBg = `url("${url}")`;
      }
    } finally {
      wpFetching = false;
    }
  }

  function handleCategoryChange(cat: WallpaperCategory) {
    setWallpaperCategory(cat);
    if (wpSetSettings.enabled) {
      restartRotation(applyWallpaperCallback);
    }
  }

  function handleIntervalChange() {
    setWallpaperInterval(wpSetSettings.interval, wpSetSettings.unit);
    if (wpSetSettings.enabled) {
      restartRotation(applyWallpaperCallback);
    }
  }

  function handleUnitChange(unit: RotationUnit) {
    setWallpaperInterval(wpSetSettings.interval, unit);
    if (wpSetSettings.enabled) {
      restartRotation(applyWallpaperCallback);
    }
  }

  function handleClearHistory() {
    wpSetSettings.history = [];
    wpSetSettings.historyIndex = -1;
    persistWallpaperSettings();
  }
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

        <!-- ═══ LIVE WALLPAPER PREVIEW (macOS monitor mockup) ═══ -->
        <div class="bg-white/60 dark:bg-black/30 backdrop-blur-xl border border-black/5 dark:border-white/10 rounded-xl overflow-hidden shadow-sm p-6 mb-6">
          <div class="flex items-center justify-between mb-4">
            <div>
              <h2 class="text-[14px] font-medium text-black dark:text-white">Wallpaper Preview</h2>
              <p class="text-[12px] text-gray-500 mt-1">Live preview of your current desktop wallpaper.</p>
            </div>
            {#if previewUrl}
              <div class="flex gap-2">
                <button
                  class="px-3 py-1.5 rounded-lg bg-emerald-500 hover:bg-emerald-600 text-white text-[12px] font-medium transition shadow-sm"
                  onclick={applyPreview}
                >
                  ✓ Apply
                </button>
                <button
                  class="px-3 py-1.5 rounded-lg bg-gray-200 hover:bg-gray-300 dark:bg-white/10 dark:hover:bg-white/20 text-gray-700 dark:text-gray-300 text-[12px] font-medium transition shadow-sm"
                  onclick={dismissPreview}
                >
                  ✕ Skip
                </button>
              </div>
            {/if}
          </div>

          <!-- Monitor frame -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <div
            class="relative w-full aspect-[16/10] rounded-xl overflow-hidden border-2 border-black/10 dark:border-white/10 shadow-lg cursor-pointer group"
            onclick={() => { if (previewUrl) applyPreview(); }}
          >
            <!-- Wallpaper background -->
            <div
              class="absolute inset-0 transition-all duration-700 ease-out"
              style="background: {previewBg
                ? (previewBg.includes('url') ? `center / cover no-repeat ${previewBg}` : previewBg)
                : (displayWallpaper.includes('url') ? `center / cover no-repeat ${displayWallpaper}` : displayWallpaper)};"
            ></div>

            <!-- Simulated top bar -->
            <div class="absolute top-0 left-0 right-0 h-5 bg-black/20 backdrop-blur-md flex items-center px-2.5 gap-1.5 z-10">
              <div class="flex gap-1">
                <div class="w-2 h-2 rounded-full bg-[#ff5f57]"></div>
                <div class="w-2 h-2 rounded-full bg-[#febc2e]"></div>
                <div class="w-2 h-2 rounded-full bg-[#28c840]"></div>
              </div>
              <span class="text-[7px] text-white/70 font-medium ml-auto">NDE-OS</span>
            </div>

            <!-- Simulated dock -->
            <div class="absolute bottom-2 left-1/2 -translate-x-1/2 flex gap-1 px-2 py-1 rounded-xl bg-white/20 backdrop-blur-xl border border-white/30 z-10">
              {#each ["📱", "🌐", "⚙️", "📂", "💬"] as emoji}
                <div class="w-5 h-5 rounded-md bg-white/30 flex items-center justify-center text-[9px]">{emoji}</div>
              {/each}
            </div>

            <!-- Preview badge -->
            {#if previewUrl}
              <div class="absolute top-7 left-1/2 -translate-x-1/2 px-3 py-1 rounded-full bg-blue-500/90 backdrop-blur-sm text-white text-[10px] font-medium shadow-lg z-10 animate-pulse">
                ✨ Preview — Click to Apply
              </div>
            {/if}

            <!-- Loading overlay -->
            {#if wpFetching}
              <div class="absolute inset-0 bg-black/40 backdrop-blur-sm flex items-center justify-center z-20">
                <div class="flex flex-col items-center gap-2">
                  <span class="inline-block w-8 h-8 border-3 border-white/30 border-t-white rounded-full animate-spin"></span>
                  <span class="text-white text-[11px] font-medium">Fetching wallpaper…</span>
                </div>
              </div>
            {/if}

            <!-- Hover overlay for expand -->
            <div class="absolute inset-0 bg-black/0 group-hover:bg-black/10 transition-colors duration-200 z-[5]"></div>
          </div>

          <!-- Monitor stand -->
          <div class="flex justify-center mt-0">
            <div class="w-16 h-3 bg-gradient-to-b from-gray-300 to-gray-400 dark:from-gray-600 dark:to-gray-700 rounded-b-lg"></div>
          </div>
          <div class="flex justify-center -mt-[1px]">
            <div class="w-24 h-1.5 bg-gradient-to-b from-gray-400 to-gray-300 dark:from-gray-700 dark:to-gray-600 rounded-b-md"></div>
          </div>
        </div>

        <!-- Dark Mode toggle -->
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

        <!-- Static Wallpapers -->
        <div class="bg-white/60 dark:bg-black/30 backdrop-blur-xl border border-black/5 dark:border-white/10 rounded-xl overflow-hidden shadow-sm p-6 mb-6">
          <div class="mb-4">
            <h2 class="text-[14px] font-medium text-black dark:text-white">Local Wallpapers</h2>
            <p class="text-[12px] text-gray-500 mt-1">Select a built-in background for your desktop.</p>
          </div>
          
          <div class="grid grid-cols-4 gap-3 mt-4">
            {#each wallpapers as wp}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <div 
                class="relative h-20 rounded-lg overflow-hidden cursor-pointer border-2 transition-all {desktop.wallpaper === wp.value && !previewUrl ? 'border-blue-500 scale-[1.04] shadow-md' : 'border-transparent hover:border-black/20 dark:hover:border-white/20 hover:scale-[1.02]'}"
                style="background: {wp.value.includes('url') ? `center / cover no-repeat ${wp.value}` : wp.value};"
                onclick={() => { previewUrl = null; previewBg = null; wpSetSettings.enabled = false; setWallpaperEnabled(false); stopRotation(); setWallpaper(wp.value); }}
              >
                <div class="absolute bottom-0 left-0 right-0 bg-black/50 backdrop-blur-sm p-1 text-center">
                  <span class="text-white text-[10px] font-medium drop-shadow-sm">{wp.label}</span>
                </div>
                {#if desktop.wallpaper === wp.value && !previewUrl}
                  <div class="absolute top-1.5 right-1.5 w-4 h-4 bg-blue-500 rounded-full flex items-center justify-center text-white shadow-sm">
                    <svg class="w-2.5 h-2.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><path d="M5 13l4 4L19 7"/></svg>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        </div>

        <!-- Online Wallpaper System -->
        <div class="bg-white/60 dark:bg-black/30 backdrop-blur-xl border border-black/5 dark:border-white/10 rounded-xl overflow-hidden shadow-sm p-6 mb-6">
          <div class="flex items-center justify-between mb-4">
            <div>
              <h2 class="text-[14px] font-medium text-black dark:text-white">🌐 Online Wallpapers</h2>
              <p class="text-[12px] text-gray-500 mt-1">Fetch stunning HD wallpapers from the internet. Preview before applying.</p>
            </div>
            <button 
              role="switch" 
              aria-label="Toggle Online Wallpapers"
              aria-checked={wpSetSettings.enabled}
              class="relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 {wpSetSettings.enabled ? 'bg-blue-500' : 'bg-gray-300 dark:bg-gray-600'}"
              onclick={handleToggleOnline}
            >
              <span class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {wpSetSettings.enabled ? 'translate-x-5' : 'translate-x-0'}"></span>
            </button>
          </div>

          {#if wpSetSettings.enabled}
            <!-- Quick Actions -->
            <div class="flex gap-3 mb-5">
              <button
                class="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg bg-blue-500 hover:bg-blue-600 text-white text-[13px] font-medium transition shadow-sm disabled:opacity-50 disabled:pointer-events-none"
                onclick={handlePreviewNext}
                disabled={wpFetching}
              >
                {#if wpFetching}
                  <span class="inline-block w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin"></span>
                  Loading…
                {:else}
                  ⏭️ Preview Next
                {/if}
              </button>
              <button
                class="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg bg-purple-500 hover:bg-purple-600 text-white text-[13px] font-medium transition shadow-sm disabled:opacity-50 disabled:pointer-events-none"
                onclick={handlePreviewRandom}
                disabled={wpFetching}
              >
                {#if wpFetching}
                  <span class="inline-block w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin"></span>
                  Loading…
                {:else}
                  🎲 Preview Random
                {/if}
              </button>
            </div>

            <!-- Category Selector -->
            <div class="mb-5">
              <h3 class="text-[13px] font-medium text-black dark:text-white mb-3">Category</h3>
              <div class="grid grid-cols-4 gap-2">
                {#each categories as cat}
                  <button
                    class="flex items-center gap-1.5 px-2.5 py-2 rounded-lg text-[12px] font-medium transition-all border {wpSetSettings.category === cat.id ? 'bg-blue-500 text-white border-blue-600 shadow-sm' : 'bg-black/5 dark:bg-white/5 text-gray-700 dark:text-gray-300 border-transparent hover:bg-black/10 dark:hover:bg-white/10'}"
                    onclick={() => handleCategoryChange(cat.id)}
                  >
                    <span class="text-sm">{cat.icon}</span>
                    <span class="truncate">{cat.label}</span>
                  </button>
                {/each}
              </div>
            </div>

            <!-- Auto-Rotation Interval -->
            <div class="bg-black/5 dark:bg-white/5 rounded-xl p-4">
              <h3 class="text-[13px] font-medium text-black dark:text-white mb-3">⏱️ Auto-Rotate Interval</h3>
              <div class="flex items-center gap-3">
                <input
                  type="number"
                  min="1"
                  max="999"
                  bind:value={wpSetSettings.interval}
                  onchange={handleIntervalChange}
                  class="w-20 px-3 py-2 rounded-lg bg-white dark:bg-black/40 border border-black/10 dark:border-white/10 text-[13px] text-black dark:text-white text-center focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
                <div class="flex rounded-lg overflow-hidden border border-black/10 dark:border-white/10">
                  {#each unitOptions as u}
                    <button
                      class="px-3 py-2 text-[12px] font-medium transition-colors {wpSetSettings.unit === u.id ? 'bg-blue-500 text-white' : 'bg-white dark:bg-black/40 text-gray-600 dark:text-gray-400 hover:bg-black/5 dark:hover:bg-white/10'}"
                      onclick={() => handleUnitChange(u.id)}
                    >
                      {u.label}
                    </button>
                  {/each}
                </div>
                <span class="text-[12px] text-gray-500 ml-auto">
                  Every {wpSetSettings.interval} {wpSetSettings.unit}
                </span>
              </div>
            </div>

            <!-- History Info -->
            {#if wpSetSettings.history.length > 0}
              <div class="mt-4 flex items-center justify-between text-[12px] text-gray-500">
                <span>📜 {wpSetSettings.history.length} wallpapers in history</span>
                <button
                  class="text-blue-500 hover:text-blue-600 font-medium transition"
                  onclick={handleClearHistory}
                >
                  Clear History
                </button>
              </div>
            {/if}
          {/if}
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
            <div class="flex-1 min-w-0">
              <h2 class="text-[14px] font-medium text-black dark:text-white">Automatically hide and show the Dock</h2>
              <p class="text-[12px] text-gray-500 mt-1">Retracts the dock to give you more screen space when not in use.</p>
            </div>
            
            <button 
              role="switch" 
              aria-label="Toggle Dock Auto-Hide"
              aria-checked={desktop.dock_auto_hide}
              class="ml-4 relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 {desktop.dock_auto_hide ? 'bg-blue-500' : 'bg-gray-300 dark:bg-gray-600'}"
              onclick={toggleDockAutoHide}
            >
              <span class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {desktop.dock_auto_hide ? 'translate-x-5' : 'translate-x-0'}"></span>
            </button>
          </div>
          
          <div class="flex items-center justify-between p-6">
            <div class="flex-1 min-w-0">
              <h2 class="text-[14px] font-medium text-black dark:text-white">Start Expanded on Launch</h2>
              <p class="text-[12px] text-gray-500 mt-1">Always open in full desktop mode instead of the collapsed floating button.</p>
            </div>
            
            <button 
              role="switch" 
              aria-label="Toggle Start Expanded"
              aria-checked={desktop.start_expanded}
              class="ml-4 relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 {desktop.start_expanded ? 'bg-blue-500' : 'bg-gray-300 dark:bg-gray-600'}"
              onclick={toggleStartExpanded}
            >
              <span class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {desktop.start_expanded ? 'translate-x-5' : 'translate-x-0'}"></span>
            </button>
          </div>

          <div class="flex items-center justify-between p-6">
            <div class="flex-1 min-w-0">
              <h2 class="text-[14px] font-medium text-black dark:text-white">Reset Desktop App Icons</h2>
              <p class="text-[12px] text-gray-500 mt-1">Restores all icons on the desktop to their default positions.</p>
            </div>
            <button 
              class="ml-4 shrink-0 px-4 py-1.5 rounded bg-gray-200 hover:bg-gray-300 dark:bg-white/10 dark:hover:bg-white/20 text-[13px] font-medium text-black dark:text-white transition shadow-sm border border-black/5 dark:border-white/5"
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
