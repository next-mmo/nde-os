<svelte:options runes={true} />

<script lang="ts">
  import { Settings, CircleCheck, Download, CircleAlert, Loader2, RefreshCw, Info, PanelRight, ExternalLink } from "@lucide/svelte";
  import type { ServiceStatus } from "../types";

  interface Props {
    svc: ServiceStatus;
    isRequired: boolean;
    isInstalling: boolean;
    installingAny: boolean;
    viewMode: "list" | "grid";
    isConfigOpen: boolean;
    onConfigOpen: () => void;
    onInstall: () => void;
    onRefresh?: () => void;
    onDrawerOpen?: () => void;
    children?: import("svelte").Snippet;
  }

  let {
    svc,
    isRequired,
    isInstalling,
    installingAny,
    viewMode,
    isConfigOpen,
    onConfigOpen,
    onInstall,
    onRefresh,
    onDrawerOpen,
    children
  }: Props = $props();

  let showDetails = $state(false);

  // Official download URLs for externally installed services
  const downloadUrls: Record<string, { url: string; label: string }> = {
    ffmpeg: { url: "https://ffmpeg.org/download.html", label: "ffmpeg.org" },
    python: { url: "https://www.python.org/downloads/", label: "python.org" },
  };

  let downloadInfo = $derived(downloadUrls[svc.id] ?? null);

  function openDownloadUrl() {
    if (!downloadInfo) return;
    import("🍎/state/desktop.svelte").then(({ openGenericBrowserWindow }) => {
      openGenericBrowserWindow(downloadInfo!.url, `Download ${svc.name}`);
    });
  }
</script>

<div class="rounded-xl border {isRequired ? 'border-violet-400/20 bg-violet-400/5' : 'border-white/6 bg-white/2'} p-3 transition-colors hover:bg-white/4 {viewMode === 'grid' ? 'flex flex-col h-full' : ''}">
  <div class="flex {viewMode === 'grid' ? 'flex-col flex-1 gap-2' : 'items-start justify-between gap-3'}">
    <div class="min-w-0 flex-1 w-full">
      <div class="flex items-center gap-2">
        <p class="text-[12px] font-semibold text-white/90">{svc.name}</p>
        {#if isRequired}
          <span class="rounded-full bg-violet-500/20 px-1.5 py-px text-[8px] uppercase tracking-wider text-violet-300">required</span>
        {:else if !svc.optional}
          <span class="rounded-full bg-amber-500/20 px-1.5 py-px text-[8px] uppercase tracking-wider text-amber-300">essential</span>
        {/if}
        <!-- Status badge inline -->
        {#if svc.installed}
          <span class="ml-auto flex items-center gap-1 rounded-full bg-emerald-500/15 px-2 py-0.5 text-[9px] text-emerald-300">
            <CircleCheck class="w-2.5 h-2.5" />
            Installed
          </span>
        {:else}
          <span class="ml-auto flex items-center gap-1 rounded-full bg-amber-500/15 px-2 py-0.5 text-[9px] text-amber-300">
            <CircleAlert class="w-2.5 h-2.5" />
            {svc.id === "voice-runtime" || svc.id === "whisperx" || svc.id === "uv" || svc.id === "ai-vision-runtime" || svc.id === "demucs" || svc.id === "ldplayer" ? "Not Installed" : "Manual"}
          </span>
        {/if}
      </div>
      <p class="text-[10px] text-white/45 mt-0.5 leading-relaxed">{svc.description}</p>

      <!-- Official download link for external services -->
      {#if downloadInfo && !svc.installed}
        <div class="mt-2 flex items-center gap-2 rounded-lg bg-blue-500/8 border border-blue-500/15 px-2.5 py-1.5">
          <ExternalLink class="w-3 h-3 text-blue-400 shrink-0" />
          <span class="text-[10px] text-blue-300/80">Download from</span>
          <button
            class="text-[10px] font-semibold text-blue-300 hover:text-blue-200 underline underline-offset-2 transition-colors"
            onclick={openDownloadUrl}
          >
            {downloadInfo.label}
          </button>
          <span class="text-[9px] text-blue-300/50">→ install → re-detect</span>
        </div>
      {/if}

      <!-- Expandable details -->
      {#if showDetails}
        <div class="mt-2 rounded-lg bg-black/20 border border-white/5 p-2 space-y-1 animate-in fade-in slide-in-from-top-1 duration-200">
          {#if svc.version}
            <p class="text-[9px] text-white/40"><span class="text-white/25">Version:</span> v{svc.version}</p>
          {/if}
          {#if svc.details}
            <p class="text-[9px] text-white/40"><span class="text-white/25">Details:</span> {svc.details}</p>
          {/if}
          {#if svc.path}
            <p class="text-[9px] text-white/40 font-mono truncate"><span class="text-white/25 font-sans">Path:</span> {svc.path}</p>
          {/if}
          {#if svc.usedBy.length > 0}
            <div class="flex items-center gap-1 flex-wrap">
              <span class="text-[9px] text-white/25">Used by:</span>
              {#each svc.usedBy as app}
                <span class="rounded-full bg-white/5 px-1.5 py-px text-[8px] text-white/40">{app}</span>
              {/each}
            </div>
          {/if}
          {#if !svc.version && !svc.details && !svc.path && svc.usedBy.length === 0}
            <p class="text-[9px] text-white/30 italic">No additional details available.</p>
          {/if}
        </div>
      {:else}
        <!-- Compact used-by when details collapsed -->
        {#if svc.usedBy.length > 0}
          <div class="flex items-center gap-1 mt-1.5">
            <span class="text-[9px] text-white/25">Used by:</span>
            {#each svc.usedBy as app}
              <span class="rounded-full bg-white/5 px-1.5 py-px text-[9px] text-white/40">{app}</span>
            {/each}
          </div>
        {/if}
      {/if}
    </div>
  </div>

  <!-- ─── 4 Action Buttons (+ 5th drawer when installed) ─── -->
  <div class="{viewMode === 'grid' ? 'flex w-full items-center justify-between pt-2 mt-auto border-t border-white/5' : 'flex items-center gap-1 mt-2 pt-2 border-t border-white/5'}">
    <!-- Action 1: Install / Re-install -->
    {#if !svc.installed && (svc.id === "voice-runtime" || svc.id === "whisperx" || svc.id === "demucs" || svc.id === "uv" || svc.id === "ai-vision-runtime" || svc.id === "ldplayer")}
      <button
        class="flex items-center gap-1.5 rounded-lg bg-violet-600 px-2.5 py-1 text-[10px] font-medium text-white transition-all hover:bg-violet-500 disabled:opacity-50 active:scale-95"
        onclick={onInstall}
        disabled={installingAny}
        title="Install"
      >
        {#if isInstalling}
          <Loader2 class="w-3 h-3 animate-spin" />
          Installing...
        {:else}
          <Download class="w-3 h-3" />
          Install
        {/if}
      </button>
    {:else if svc.installed}
      <button
        class="flex items-center gap-1.5 rounded-lg bg-white/5 border border-white/8 px-2.5 py-1 text-[10px] font-medium text-white/60 transition-all hover:bg-violet-600/20 hover:text-violet-300 hover:border-violet-500/30 disabled:opacity-50 active:scale-95"
        onclick={onInstall}
        disabled={installingAny}
        title="Re-install"
      >
        {#if isInstalling}
          <Loader2 class="w-3 h-3 animate-spin" />
          Re-installing...
        {:else}
          <Download class="w-3 h-3" />
          Re-install
        {/if}
      </button>
    {:else if downloadInfo}
      <!-- Manual-install services: show download button instead -->
      <button
        class="flex items-center gap-1.5 rounded-lg bg-blue-600 px-2.5 py-1 text-[10px] font-medium text-white transition-all hover:bg-blue-500 active:scale-95"
        onclick={openDownloadUrl}
        title="Open official download page"
      >
        <ExternalLink class="w-3 h-3" />
        Download
      </button>
    {/if}

    <div class="flex-1"></div>

    <!-- Action 2: Info / Details toggle -->
    <button
      class="rounded-md p-1.5 transition-all active:scale-90 {showDetails ? 'bg-blue-500/15 text-blue-400' : 'text-white/30 hover:bg-white/5 hover:text-white/60'}"
      onclick={() => showDetails = !showDetails}
      title={showDetails ? "Hide Details" : "Show Details"}
    >
      <Info class="w-3.5 h-3.5" />
    </button>

    <!-- Action 3: Refresh status -->
    {#if onRefresh}
      <button
        class="rounded-md p-1.5 text-white/30 transition-all hover:bg-white/5 hover:text-white/60 active:scale-90"
        onclick={onRefresh}
        title="Refresh Status"
      >
        <RefreshCw class="w-3.5 h-3.5" />
      </button>
    {/if}

    <!-- Action 4: Settings -->
    <button
      class="rounded-md p-1.5 transition-all active:scale-90 {isConfigOpen ? 'bg-white/5 text-white/60' : 'text-white/30 hover:bg-white/5 hover:text-white/60'}"
      onclick={onConfigOpen}
      title="Settings"
    >
      <Settings class="w-3.5 h-3.5" />
    </button>

    <!-- Action 5: Open Full Drawer (only when installed) -->
    {#if svc.installed && onDrawerOpen}
      <button
        class="rounded-md p-1.5 text-white/30 transition-all hover:bg-emerald-500/15 hover:text-emerald-400 active:scale-90"
        onclick={onDrawerOpen}
        title="Open Full Details"
      >
        <PanelRight class="w-3.5 h-3.5" />
      </button>
    {/if}
  </div>

  {#if children}{@render children()}{/if}
</div>
