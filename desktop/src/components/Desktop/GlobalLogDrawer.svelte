<svelte:options runes={true} />

<script lang="ts">
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { click_outside } from "🍎/actions";
  import { desktop, toggleLogDrawer } from "🍎/state/desktop.svelte";
  import { logStore, type ExtendedLogEntry } from "$lib/stores/logs";

  const LEVEL_COLORS: Record<string, string> = {
    info: "text-gray-300",
    success: "text-emerald-400",
    warning: "text-amber-400",
    error: "text-red-400",
  };

  const LEVEL_DOTS: Record<string, string> = {
    info: "bg-gray-400",
    success: "bg-emerald-400",
    warning: "bg-amber-400",
    error: "bg-red-400",
  };

  const SOURCE_COLORS: Record<string, string> = {
    telegram: "text-sky-400 bg-sky-500/10",
    gateway: "text-violet-400 bg-violet-500/10",
    system: "text-gray-400 bg-gray-500/10",
    chat: "text-emerald-400 bg-emerald-500/10",
    frontend: "text-amber-300 bg-amber-500/10",
  };

  const FILTERS = ["all", "telegram", "gateway", "system", "frontend"] as const;
  type FilterType = (typeof FILTERS)[number];
  let activeFilter = $state<FilterType>("all");

  function formatTime(iso: string) {
    return new Date(iso).toLocaleTimeString("en-US", {
      hour12: false,
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  }

  function hide() {
    toggleLogDrawer(false);
  }

  let filteredLogs = $derived(
    activeFilter === "all"
      ? $logStore
      : $logStore.filter((e: ExtendedLogEntry) => e.source === activeFilter)
  );
  let errorCount = $derived($logStore.filter((e: ExtendedLogEntry) => e.level === "error").length);
  let warnCount = $derived($logStore.filter((e: ExtendedLogEntry) => e.level === "warning").length);
  let totalCount = $derived(filteredLogs.length);

  // Count by source for filter badges
  let sourceCounts = $derived(
    $logStore.reduce((acc: Record<string, number>, e: ExtendedLogEntry) => {
      const src = e.source ?? "frontend";
      acc[src] = (acc[src] ?? 0) + 1;
      return acc;
    }, {} as Record<string, number>)
  );
</script>

<!-- FAB: always visible in bottom-right -->
{#if !desktop.collapsed && !desktop.is_locked}
  <button
    class="fixed right-0 top-1/2 z-[8999] w-5 h-10 translate-y-[28px] bg-linear-to-b from-[#5856d6] to-[#af52de] rounded-l-lg grid place-items-center cursor-pointer border-none opacity-40 hover:opacity-100 transition-opacity pointer-events-auto
      {desktop.log_drawer_open ? 'opacity-100' : ''}"
    onclick={() => toggleLogDrawer()}
    aria-label="Toggle log drawer"
    data-testid="log-drawer-fab"
  >
    <span class="text-white text-xs leading-none select-none flex items-center justify-center pointer-events-none">
      {#if desktop.log_drawer_open}
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
      {:else}
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M4 17l6-6-6-6"/><path d="M12 19h8"/></svg>
      {/if}
    </span>
    {#if !desktop.log_drawer_open && errorCount > 0}
      <span class="absolute -top-1 -left-1.5 min-w-[14px] h-[14px] rounded-full bg-red-500 text-white text-[9px] font-bold flex items-center justify-center px-0.5 shadow">{errorCount}</span>
    {:else if !desktop.log_drawer_open && $logStore.length > 0}
      <span class="absolute top-0 -left-1 w-2 h-2 rounded-full bg-emerald-400 shadow"></span>
    {/if}
  </button>
{/if}

<!-- Drawer panel -->
{#if desktop.log_drawer_open && !desktop.collapsed && !desktop.is_locked}
  <div
    class="fixed bottom-0 right-0 z-[8998] w-[420px] max-w-[100vw] pointer-events-auto"
    use:click_outside={hide}
    transition:fly={{ y: 300, duration: 250, easing: cubicOut }}
  >
    <div class="h-[50vh] max-h-[500px] min-h-[240px] flex flex-col rounded-tl-2xl overflow-hidden bg-gray-950/90 backdrop-blur-2xl border-t border-l border-white/8 shadow-[-8px_-8px_24px_rgba(0,0,0,0.3)]">
      <!-- Header -->
      <header class="flex items-center justify-between gap-3 px-4 py-2.5 border-b border-white/6 shrink-0">
        <div class="flex items-center gap-2.5">
          <span class="text-xs font-semibold text-gray-400 uppercase tracking-wider">Activity Log</span>
          {#if errorCount > 0}
            <span class="text-[10px] font-bold px-1.5 py-0.5 rounded-full bg-red-500/20 text-red-400">{errorCount} err</span>
          {/if}
          {#if warnCount > 0}
            <span class="text-[10px] font-bold px-1.5 py-0.5 rounded-full bg-amber-500/20 text-amber-400">{warnCount} warn</span>
          {/if}
        </div>
        <div class="flex items-center gap-1">
          <button
            class="w-6 h-6 rounded-md grid place-items-center text-gray-500 hover:text-gray-300 hover:bg-white/8 transition-colors text-xs"
            onclick={() => logStore.clear()}
            aria-label="Clear logs"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/></svg>
          </button>
          <button
            class="w-6 h-6 rounded-md grid place-items-center text-gray-500 hover:text-gray-300 hover:bg-white/8 transition-colors text-xs"
            onclick={hide}
            aria-label="Close log drawer"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
          </button>
        </div>
      </header>

      <!-- Filter tabs -->
      <div class="flex items-center gap-1 px-3 py-1.5 border-b border-white/4 shrink-0 overflow-x-auto">
        {#each FILTERS as filter}
          <button
            class="px-2 py-0.5 rounded-md text-[10px] font-medium uppercase tracking-wider transition-colors whitespace-nowrap
              {activeFilter === filter
                ? 'bg-white/10 text-white'
                : 'text-gray-500 hover:text-gray-300 hover:bg-white/5'}"
            onclick={() => (activeFilter = filter)}
          >
            {filter}
            {#if filter !== "all" && (sourceCounts[filter] ?? 0) > 0}
              <span class="ml-0.5 opacity-60">{sourceCounts[filter]}</span>
            {/if}
          </button>
        {/each}
      </div>

      <!-- Log entries -->
      <div class="flex-1 min-h-0 overflow-y-auto overflow-x-hidden px-1 py-1 font-mono text-[12px] leading-relaxed">
        {#each filteredLogs as entry (entry.id)}
          <div class="flex items-start gap-2 px-3 py-1 hover:bg-white/3 rounded transition-colors">
            <span class="w-1.5 h-1.5 rounded-full mt-1.5 shrink-0 {LEVEL_DOTS[entry.level] ?? 'bg-gray-500'}"></span>
            <span class="text-gray-600 shrink-0">{formatTime(entry.timestamp)}</span>
            {#if entry.source && entry.source !== "frontend"}
              <span class="shrink-0 px-1.5 py-0 rounded text-[10px] font-medium {SOURCE_COLORS[entry.source] ?? 'text-gray-400 bg-white/5'}">{entry.source}</span>
            {/if}
            {#if entry.app_id}
              <span class="shrink-0 px-1.5 py-0 rounded text-[10px] font-medium bg-white/5 text-sky-400">{entry.app_id}</span>
            {/if}
            <span class="{LEVEL_COLORS[entry.level] ?? 'text-gray-300'} break-all">{entry.message}</span>
          </div>
        {:else}
          <div class="flex items-center justify-center h-full text-gray-600 text-[11px]">
            {activeFilter === "all" ? "No activity yet." : `No ${activeFilter} logs.`}
          </div>
        {/each}
      </div>

      <!-- Footer -->
      <footer class="flex items-center justify-between px-4 py-1.5 border-t border-white/6 text-[10px] text-gray-600 shrink-0">
        <span>{totalCount} entries{activeFilter !== "all" ? ` (${activeFilter})` : ""}</span>
        <span>polling backend every 3s</span>
      </footer>
    </div>
  </div>
{/if}
