<svelte:options runes={true} />

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { Download, CheckCircle2, AlertCircle, ArrowLeft, Loader2, ExternalLink, Wrench, Mic, Film, Code2, Cpu } from "@lucide/svelte";

  // ─── Types ───────────────────────────────────────────────────────────────
  type ServiceStatus = {
    id: string;
    name: string;
    description: string;
    group: "voice" | "media" | "ai" | "tooling";
    installed: boolean;
    version?: string | null;
    path?: string | null;
    usedBy: string[];
    optional: boolean;
    details?: string | null;
  };

  // ─── Props (receives window from AppNexus) ────────────────────────────
  import type { DesktopWindow } from "🍎/state/desktop.svelte";

  interface Props {
    window: DesktopWindow;
  }

  let { window: win }: Props = $props();

  // Deep-link context passed via openServiceHub({ require, returnTo })
  let require = $derived<string[]>(win.data?.require ?? []);
  let returnTo = $derived<string | null>(win.data?.returnTo ?? null);

  // ─── State ───────────────────────────────────────────────────────────────
  let services = $state<ServiceStatus[]>([]);
  let loading = $state(true);
  let installing = $state<string | null>(null);
  let installError = $state<string | null>(null);
  let installSuccess = $state<string | null>(null);

  // Group config
  const groupMeta: Record<string, { label: string; icon: typeof Mic; color: string }> = {
    voice: { label: "Voice", icon: Mic, color: "text-violet-400" },
    media: { label: "Media", icon: Film, color: "text-blue-400" },
    ai: { label: "AI", icon: Cpu, color: "text-emerald-400" },
    tooling: { label: "Tooling", icon: Code2, color: "text-amber-400" },
  };

  // ─── Lifecycle ──────────────────────────────────────────────────────────
  onMount(() => {
    refreshStatus();
  });

  async function refreshStatus() {
    try {
      loading = true;
      services = await invoke<ServiceStatus[]>("service_hub_status");
    } catch (e) {
      console.error("Failed to load service status", e);
    } finally {
      loading = false;
    }
  }

  async function installService(serviceId: string) {
    try {
      installing = serviceId;
      installError = null;
      installSuccess = null;
      const message = await invoke<string>("service_hub_install", { serviceId });
      installSuccess = message;
      await refreshStatus();
    } catch (e: any) {
      installError = e?.toString?.() ?? "Install failed";
    } finally {
      installing = null;
    }
  }

  function goBack() {
    if (returnTo) {
      import("🍎/state/desktop.svelte").then(({ openStaticApp }) => {
        openStaticApp(returnTo as any);
      });
    }
  }

  // Derived
  let grouped = $derived.by(() => {
    const groups: Record<string, ServiceStatus[]> = {};
    for (const svc of services) {
      if (!groups[svc.group]) groups[svc.group] = [];
      groups[svc.group]!.push(svc);
    }
    return groups;
  });

  let requiredServices = $derived(
    require.length > 0
      ? services.filter(s => require.includes(s.id))
      : []
  );

  let allRequiredInstalled = $derived(
    requiredServices.length > 0 && requiredServices.every(s => s.installed)
  );
</script>

<div class="flex flex-col h-full bg-[#1a1a2e] text-white overflow-hidden">
  <!-- Header -->
  <div class="flex items-center justify-between px-5 py-4 border-b border-white/5 bg-black/20 backdrop-blur-xl">
    <div class="flex items-center gap-3">
      <Wrench class="w-5 h-5 text-violet-400" />
      <div>
        <h1 class="text-sm font-semibold tracking-tight">Service Hub</h1>
        <p class="text-[10px] text-white/40 mt-0.5">Manage NDE-OS service dependencies</p>
      </div>
    </div>
    <button
      class="rounded-lg bg-white/5 px-3 py-1.5 text-[11px] text-white/60 transition-colors hover:bg-white/10 hover:text-white/80"
      onclick={refreshStatus}
      disabled={loading}
    >
      {loading ? "Refreshing..." : "↻ Refresh"}
    </button>
  </div>

  <!-- Required banner (deep-link context) -->
  {#if require.length > 0}
    <div class="mx-4 mt-4 rounded-xl border border-violet-400/20 bg-violet-400/8 p-3">
      <div class="flex items-center justify-between gap-3">
        <div class="min-w-0">
          <p class="text-xs font-semibold text-violet-200">Setup Required</p>
          <p class="text-[10px] text-violet-200/70 mt-0.5">
            {returnTo ? `${returnTo} needs` : "An app needs"} the following services to be installed before you can use it.
          </p>
        </div>
        {#if allRequiredInstalled && returnTo}
          <button
            class="shrink-0 flex items-center gap-1.5 rounded-lg bg-emerald-600 px-3 py-1.5 text-[11px] font-medium text-white transition-colors hover:bg-emerald-500"
            onclick={goBack}
          >
            <ArrowLeft class="w-3.5 h-3.5" />
            Return to {returnTo}
          </button>
        {/if}
      </div>

      <!-- Required services quick status -->
      <div class="mt-2 flex flex-wrap gap-2">
        {#each requiredServices as svc}
          <div class="flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[10px] {svc.installed ? 'bg-emerald-500/15 text-emerald-300' : 'bg-amber-500/15 text-amber-300'}">
            {#if svc.installed}
              <CheckCircle2 class="w-3 h-3" />
            {:else}
              <AlertCircle class="w-3 h-3" />
            {/if}
            {svc.name}
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Scroll body -->
  <div class="flex-1 overflow-y-auto px-4 py-4 space-y-5">
    {#if loading}
      <div class="flex flex-col items-center justify-center py-20">
        <Loader2 class="w-6 h-6 text-violet-400 animate-spin" />
        <p class="text-[11px] text-white/40 mt-2">Detecting services...</p>
      </div>
    {:else}
      {#each Object.entries(grouped) as [group, groupServices]}
        {@const meta = groupMeta[group] ?? groupMeta.tooling}
        <div>
          <div class="flex items-center gap-2 mb-2.5">
            {#if group === "voice"}
              <Mic class="w-3.5 h-3.5 text-violet-400" />
            {:else if group === "media"}
              <Film class="w-3.5 h-3.5 text-blue-400" />
            {:else if group === "ai"}
              <Cpu class="w-3.5 h-3.5 text-emerald-400" />
            {:else}
              <Code2 class="w-3.5 h-3.5 text-amber-400" />
            {/if}
            <h2 class="text-[11px] font-semibold uppercase tracking-wider {meta.color}">{meta.label}</h2>
          </div>

          <div class="space-y-2">
            {#each groupServices as svc}
              {@const isRequired = require.includes(svc.id)}
              {@const isInstalling = installing === svc.id}
              <div class="rounded-xl border {isRequired ? 'border-violet-400/20 bg-violet-400/5' : 'border-white/6 bg-white/2'} p-3 transition-colors hover:bg-white/4">
                <div class="flex items-start justify-between gap-3">
                  <div class="min-w-0 flex-1">
                    <div class="flex items-center gap-2">
                      <p class="text-[12px] font-semibold text-white/90">{svc.name}</p>
                      {#if isRequired}
                        <span class="rounded-full bg-violet-500/20 px-1.5 py-px text-[8px] uppercase tracking-wider text-violet-300">required</span>
                      {:else if !svc.optional}
                        <span class="rounded-full bg-amber-500/20 px-1.5 py-px text-[8px] uppercase tracking-wider text-amber-300">essential</span>
                      {/if}
                    </div>
                    <p class="text-[10px] text-white/45 mt-0.5 leading-relaxed">{svc.description}</p>

                    <!-- Details / version -->
                    {#if svc.version || svc.details}
                      <p class="text-[9px] text-white/30 mt-1">
                        {#if svc.version}v{svc.version}{/if}
                        {#if svc.version && svc.details} · {/if}
                        {#if svc.details}{svc.details}{/if}
                      </p>
                    {/if}

                    <!-- Used by -->
                    {#if svc.usedBy.length > 0}
                      <div class="flex items-center gap-1 mt-1.5">
                        <span class="text-[9px] text-white/25">Used by:</span>
                        {#each svc.usedBy as app}
                          <span class="rounded-full bg-white/5 px-1.5 py-px text-[9px] text-white/40">{app}</span>
                        {/each}
                      </div>
                    {/if}
                  </div>

                  <!-- Status + Install button -->
                  <div class="shrink-0 flex items-center gap-2">
                    {#if svc.installed}
                      <div class="flex items-center gap-1 rounded-full bg-emerald-500/15 px-2.5 py-1 text-[10px] text-emerald-300">
                        <CheckCircle2 class="w-3 h-3" />
                        Installed
                      </div>
                    {:else if svc.id === "voice-runtime" || svc.id === "uv"}
                      <button
                        class="flex items-center gap-1.5 rounded-lg bg-violet-600 px-3 py-1.5 text-[10px] font-medium text-white transition-colors hover:bg-violet-500 disabled:opacity-50"
                        onclick={() => installService(svc.id)}
                        disabled={isInstalling || installing !== null}
                      >
                        {#if isInstalling}
                          <Loader2 class="w-3 h-3 animate-spin" />
                          Installing...
                        {:else}
                          <Download class="w-3 h-3" />
                          Install
                        {/if}
                      </button>
                    {:else}
                      <div class="flex items-center gap-1 rounded-full bg-amber-500/15 px-2.5 py-1 text-[10px] text-amber-300">
                        <AlertCircle class="w-3 h-3" />
                        Manual
                      </div>
                    {/if}
                  </div>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/each}
    {/if}

    <!-- Install feedback -->
    {#if installError}
      <div class="rounded-xl border border-red-400/20 bg-red-400/8 p-3">
        <p class="text-[11px] text-red-300">{installError}</p>
      </div>
    {/if}
    {#if installSuccess}
      <div class="rounded-xl border border-emerald-400/20 bg-emerald-400/8 p-3">
        <p class="text-[11px] text-emerald-300">{installSuccess}</p>
      </div>
    {/if}
  </div>

  <!-- Footer: return-to CTA when all required services are ready -->
  {#if allRequiredInstalled && returnTo}
    <div class="px-4 py-3 border-t border-white/5 bg-black/20 backdrop-blur-xl">
      <button
        class="w-full flex items-center justify-center gap-2 rounded-xl bg-emerald-600 py-2.5 text-[12px] font-semibold text-white transition-colors hover:bg-emerald-500"
        onclick={goBack}
      >
        <CheckCircle2 class="w-4 h-4" />
        All Ready — Return to {returnTo}
      </button>
    </div>
  {/if}
</div>
