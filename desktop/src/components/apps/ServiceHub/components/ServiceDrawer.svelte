<svelte:options runes={true} />

<script lang="ts">
  import { X, CircleCheck, CircleAlert, Download, Settings, Folder, Tag, Info, ExternalLink } from "@lucide/svelte";
  import type { ServiceStatus } from "../types";

  interface Props {
    svc: ServiceStatus | null;
    onClose: () => void;
    onInstall?: (customId?: string) => void;
    onConfigOpen?: () => void;
  }

  let { svc, onClose, onInstall, onConfigOpen }: Props = $props();

  // Official download URLs for externally installed services
  const downloadUrls: Record<string, { url: string; label: string }> = {
    ffmpeg: { url: "https://ffmpeg.org/download.html", label: "ffmpeg.org" },
    python: { url: "https://www.python.org/downloads/", label: "python.org" },
  };

  let downloadInfo = $derived(svc ? (downloadUrls[svc.id] ?? null) : null);

  function openDownloadUrl() {
    if (!downloadInfo) return;
    import("🍎/state/desktop.svelte").then(({ openGenericBrowserWindow }) => {
      openGenericBrowserWindow(downloadInfo!.url, `Download ${svc?.name ?? "Service"}`);
    });
  }

</script>

<!-- Backdrop overlay -->
{#if svc}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-40 bg-black/40 backdrop-blur-sm transition-opacity duration-200"
    onclick={onClose}
  ></div>

  <!-- Drawer panel — slide in from right -->
  <div
    class="fixed top-0 right-0 bottom-0 z-50 w-[380px] max-w-[85vw] bg-[#14142a]/95 backdrop-blur-2xl border-l border-white/8 shadow-2xl flex flex-col animate-in slide-in-from-right duration-250 ease-out"
  >
    <!-- Header -->
    <div class="flex items-center justify-between px-5 py-4 border-b border-white/5">
      <div class="flex items-center gap-3 min-w-0">
        <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-violet-500/30 to-blue-500/30 border border-white/10 flex items-center justify-center shrink-0">
          <Info class="w-4 h-4 text-violet-300" />
        </div>
        <div class="min-w-0">
          <p class="text-[13px] font-semibold text-white truncate">{svc.name}</p>
          <p class="text-[10px] text-white/40 capitalize">{svc.group} service</p>
        </div>
      </div>
      <button
        class="rounded-lg p-1.5 text-white/40 transition-colors hover:bg-white/5 hover:text-white/80 active:scale-90"
        onclick={onClose}
        title="Close drawer"
      >
        <X class="w-4 h-4" />
      </button>
    </div>

    <!-- Body -->
    <div class="flex-1 overflow-y-auto px-5 py-4 space-y-5">
      <!-- Status card -->
      <div class="rounded-xl border p-3.5 {svc.installed ? 'border-emerald-500/20 bg-emerald-500/5' : 'border-amber-500/20 bg-amber-500/5'}">
        <div class="flex items-center gap-2">
          {#if svc.installed}
            <CircleCheck class="w-4 h-4 text-emerald-400" />
            <span class="text-[12px] font-semibold text-emerald-300">Installed</span>
          {:else}
            <CircleAlert class="w-4 h-4 text-amber-400" />
            <span class="text-[12px] font-semibold text-amber-300">Not Installed</span>
          {/if}
        </div>
        {#if svc.version}
          <p class="text-[10px] text-white/40 mt-1.5">Version <span class="text-white/70 font-mono">v{svc.version}</span></p>
        {/if}
      </div>

      <!-- Description -->
      <div class="space-y-1.5">
        <p class="text-[10px] font-semibold text-white/50 uppercase tracking-wider">Description</p>
        <p class="text-[11px] text-white/60 leading-relaxed">{svc.description}</p>
      </div>

      <!-- Official download link for external services -->
      {#if downloadInfo && !svc.installed}
        <div class="rounded-xl border border-blue-500/20 bg-blue-500/5 p-3 space-y-2">
          <p class="text-[10px] font-semibold text-blue-300 uppercase tracking-wider flex items-center gap-1.5">
            <ExternalLink class="w-3 h-3" /> Official Download
          </p>
          <p class="text-[10px] text-white/50 leading-relaxed">
            This service requires manual installation. Download from the official website, install it, then click Refresh to re-detect.
          </p>
          <button
            class="flex items-center gap-1.5 rounded-lg bg-blue-600 px-3 py-1.5 text-[10px] font-medium text-white transition-all hover:bg-blue-500 active:scale-95"
            onclick={openDownloadUrl}
          >
            <ExternalLink class="w-3 h-3" />
            Open {downloadInfo.label}
          </button>
        </div>
      {/if}

      <!-- Details -->
      {#if svc.details}
        <div class="space-y-1.5">
          <p class="text-[10px] font-semibold text-white/50 uppercase tracking-wider">Details</p>
          <p class="text-[11px] text-white/60 leading-relaxed">{svc.details}</p>
        </div>
      {/if}

      <!-- Path -->
      {#if svc.path}
        <div class="space-y-1.5">
          <p class="text-[10px] font-semibold text-white/50 uppercase tracking-wider flex items-center gap-1.5">
            <Folder class="w-3 h-3" /> Install Path
          </p>
          <p class="text-[10px] text-white/50 font-mono bg-black/20 rounded-lg px-3 py-2 border border-white/5 break-all">{svc.path}</p>
        </div>
      {/if}

      <!-- Whisper Models (only for voice-runtime) -->
      {#if svc.id === "voice-runtime" && svc.installed}
        <div class="space-y-1.5">
          <p class="text-[10px] font-semibold text-white/50 uppercase tracking-wider flex items-center gap-1.5">
            <Download class="w-3 h-3" /> Whisper Models
          </p>
          <div class="grid gap-2">
            <!-- Base model -->
            <div class="rounded-lg bg-black/20 border border-white/5 p-3 flex items-center justify-between">
              <div>
                <p class="text-[11px] text-white/80 font-medium">Base Model</p>
                <p class="text-[9px] text-white/40">~140MB - Fast, good for English</p>
              </div>
              <button
                class="flex items-center gap-1.5 rounded-lg bg-blue-600/20 text-blue-400 border border-blue-500/20 px-2.5 py-1 text-[9px] font-medium transition-colors hover:bg-blue-600 hover:text-white"
                onclick={() => { onInstall?.("whisper-model-base"); onClose(); }}
              >
                <Download class="w-3 h-3" /> Download
              </button>
            </div>
            <!-- Large V3 model -->
            <div class="rounded-lg bg-black/20 border border-white/5 p-3 flex items-center justify-between">
              <div>
                <p class="text-[11px] text-white/80 font-medium">Large V3 Model</p>
                <p class="text-[9px] text-white/40">~1.5GB - Most accurate, multilingual</p>
              </div>
              <button
                class="flex items-center gap-1.5 rounded-lg bg-blue-600/20 text-blue-400 border border-blue-500/20 px-2.5 py-1 text-[9px] font-medium transition-colors hover:bg-blue-600 hover:text-white"
                onclick={() => { onInstall?.("whisper-model-large-v3"); onClose(); }}
              >
                <Download class="w-3 h-3" /> Download
              </button>
            </div>
          </div>
        </div>
      {/if}

      <!-- Used By -->
      {#if svc.usedBy.length > 0}
        <div class="space-y-1.5">
          <p class="text-[10px] font-semibold text-white/50 uppercase tracking-wider flex items-center gap-1.5">
            <Tag class="w-3 h-3" /> Used By
          </p>
          <div class="flex flex-wrap gap-1.5">
            {#each svc.usedBy as app}
              <span class="rounded-full bg-white/5 border border-white/5 px-2.5 py-1 text-[10px] text-white/50">{app}</span>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Metadata -->
      <div class="space-y-1.5">
        <p class="text-[10px] font-semibold text-white/50 uppercase tracking-wider">Metadata</p>
        <div class="grid grid-cols-2 gap-2 text-[10px]">
          <div class="rounded-lg bg-black/20 border border-white/5 px-3 py-2">
            <p class="text-white/30 mb-0.5">Group</p>
            <p class="text-white/70 capitalize">{svc.group}</p>
          </div>
          <div class="rounded-lg bg-black/20 border border-white/5 px-3 py-2">
            <p class="text-white/30 mb-0.5">Type</p>
            <p class="text-white/70">{svc.optional ? "Optional" : "Essential"}</p>
          </div>
          <div class="rounded-lg bg-black/20 border border-white/5 px-3 py-2">
            <p class="text-white/30 mb-0.5">ID</p>
            <p class="text-white/70 font-mono">{svc.id}</p>
          </div>
          <div class="rounded-lg bg-black/20 border border-white/5 px-3 py-2">
            <p class="text-white/30 mb-0.5">Version</p>
            <p class="text-white/70 font-mono">{svc.version ?? "—"}</p>
          </div>
        </div>
      </div>
    </div>

    <!-- Footer actions -->
    <div class="px-5 py-3.5 border-t border-white/5 bg-black/20 flex items-center gap-2">
      {#if downloadInfo && !svc.installed}
        <button
          class="flex items-center gap-1.5 rounded-lg bg-blue-600 px-3.5 py-1.5 text-[10px] font-medium text-white transition-all hover:bg-blue-500 active:scale-95"
          onclick={openDownloadUrl}
        >
          <ExternalLink class="w-3.5 h-3.5" />
          Download from {downloadInfo.label}
        </button>
      {/if}
      {#if onInstall}
        <button
          class="flex items-center gap-1.5 rounded-lg bg-violet-600 px-3.5 py-1.5 text-[10px] font-medium text-white transition-all hover:bg-violet-500 active:scale-95"
          onclick={() => onInstall()}
        >
          <Download class="w-3.5 h-3.5" />
          {svc.installed ? "Re-install" : "Install"}
        </button>
      {/if}
      {#if onConfigOpen}
        <button
          class="flex items-center gap-1.5 rounded-lg bg-white/5 border border-white/8 px-3.5 py-1.5 text-[10px] font-medium text-white/60 transition-all hover:bg-white/10 hover:text-white active:scale-95"
          onclick={() => { onConfigOpen?.(); onClose(); }}
        >
          <Settings class="w-3.5 h-3.5" />
          Settings
        </button>
      {/if}
      <div class="flex-1"></div>
      <button
        class="flex items-center gap-1.5 rounded-lg bg-white/5 border border-white/8 px-3.5 py-1.5 text-[10px] font-medium text-white/40 transition-all hover:bg-white/10 hover:text-white/60 active:scale-95"
        onclick={onClose}
      >
        Close
      </button>
    </div>
  </div>
{/if}
