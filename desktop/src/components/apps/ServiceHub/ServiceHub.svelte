<svelte:options runes={true} />

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import { Download, CircleCheck, CircleAlert, ArrowLeft, Loader2, ExternalLink, Wrench, Mic, Film, Code2, Cpu, Play, Square, RefreshCw, Settings, X, Save, List as ListIcon, LayoutGrid, Search, ArrowUpCircle } from "@lucide/svelte";
  import { getServiceConfig, setServiceConfig } from "$lib/api/backend";
  import type { ServiceConfig, ConfigField } from "$lib/api/types";
  import { desktop, openGenericBrowserWindow } from "🍎/state/desktop.svelte";
  import { logStore } from "$lib/stores/logs";
  import type { ServiceStatus } from "./types";
  import ServiceCard from "./components/ServiceCard.svelte";
  import ServiceDrawer from "./components/ServiceDrawer.svelte";
  // ─── Props (receives window from AppNexus) ────────────────────────────
  import type { DesktopWindow } from "🍎/state/desktop.svelte";

  interface Props {
    window: DesktopWindow;
  }

  let { window: win }: Props = $props();

  // Deep-link context passed via openServiceHub({ require, returnTo, autoInstall })
  let require = $derived<string[]>(win.data?.require ?? []);
  let returnTo = $derived<string | null>(win.data?.returnTo ?? null);
  let autoInstall = $derived<boolean>(win.data?.autoInstall === true);
  let autoInstallTriggered = $state(new Set<string>());

  // ─── State ───────────────────────────────────────────────────────────────
  let services = $state<ServiceStatus[]>([]);
  let loading = $state(true);
  let installing = $state<string | null>(null);
  let installError = $state<string | null>(null);
  let installSuccess = $state<string | null>(null);
  let dlStage = $state("");
  let dlPercent = $state(0);
  let dlMessage = $state("");
  let ulistenDl: UnlistenFn | null = null;
  let activeTab = $state<string>("All");
  let viewMode = $state<"list" | "grid">("list");
  let drawerService = $state<ServiceStatus | null>(null);

  // Update check state
  let updateChecking = $state(false);
  let updateResult = $state<{
    currentVersion: string;
    latestVersion: string | null;
    updateAvailable: boolean;
    releaseName: string | null;
    releaseUrl: string | null;
    releaseBody: string | null;
    publishedAt: string | null;
    error: string | null;
  } | null>(null);
  let searchQuery = $state("");


  // Group config
  const groupMeta: Record<string, { label: string; icon: typeof Mic; color: string }> = {
    voice: { label: "Voice", icon: Mic, color: "text-violet-400" },
    media: { label: "Media", icon: Film, color: "text-blue-400" },
    ai: { label: "AI", icon: Cpu, color: "text-emerald-400" },
    tooling: { label: "Tooling", icon: Code2, color: "text-amber-400" },
  };

  // ─── Lifecycle ──────────────────────────────────────────────────────────
  onMount(async () => {
    refreshStatus();
    // Deep-link: pre-fill search to navigate directly to the required service
    if (require.length > 0) {
      searchQuery = require[0];
    }

    ulistenDl = await listen("ldplayer-download-progress", (event: any) => {
      dlStage = event.payload.stage;
      dlPercent = event.payload.percent;
      dlMessage = event.payload.message;
    });
  });

  onDestroy(() => {
    ulistenDl?.();
  });

  // Reactive: update search when deep-link context changes on an already-open window
  $effect(() => {
    if (require.length > 0) {
      searchQuery = require[0];
    }
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
    maybeAutoInstall();
  }

  function maybeAutoInstall() {
    if (!autoInstall || require.length === 0 || installing) return;
    for (const id of require) {
      if (autoInstallTriggered.has(id)) continue;
      const svc = services.find((s) => s.id === id);
      if (!svc || svc.installed) continue;
      autoInstallTriggered.add(id);
      installService(id);
      break; // install one at a time; next one kicks off after refreshStatus()
    }
  }

  async function checkForUpdates() {
    try {
      updateChecking = true;
      logStore.info("Checking for updates...", "service-hub");
      updateResult = await invoke("check_for_updates");
      if (updateResult?.error) {
        logStore.error(`Update check failed: ${updateResult.error}`, "service-hub");
      } else if (updateResult?.updateAvailable) {
        logStore.success(`Update available: v${updateResult.latestVersion}`, "service-hub");
      } else {
        logStore.info(`You're on the latest version (v${updateResult?.currentVersion})`, "service-hub");
      }
    } catch (e: any) {
      logStore.error(`Update check failed: ${e?.toString?.() ?? "Unknown error"}`, "service-hub");
    } finally {
      updateChecking = false;
    }
  }

  async function installService(serviceId: string) {
    try {
      installing = serviceId;
      installError = null;
      installSuccess = null;
      logStore.info(`Installing ${serviceId}...`, "service-hub");
      
      let message = "";
      if (serviceId === "ldplayer") {
        message = await invoke<string>("shield_download_ldplayer");
      } else {
        message = await invoke<string>("service_hub_install", { serviceId });
      }
      
      installSuccess = message;
      logStore.success(`Installed ${serviceId}`, "service-hub");
      await refreshStatus();
    } catch (e: any) {
      installError = e?.toString?.() ?? "Install failed";
      logStore.error(`Failed to install ${serviceId}: ${installError}`, "service-hub");
    } finally {
      installing = null;
    }
  }

  function goBack() {
    if (returnTo) {
      import("🍎/state/desktop.svelte").then(({ openStaticApp }) => {
        // Pass deep-link context back to the originating app
        // For shield-browser, this switches to the emulators tab on return
        const returnData = win.data?.returnData ?? undefined;
        openStaticApp(returnTo as any, returnData);
      });
    }
  }

  // Derived
  let grouped = $derived.by(() => {
    const groups: Record<string, ServiceStatus[]> = {};
    const query = searchQuery.toLowerCase().trim();
    
    for (const svc of services) {
      let keep = false;
      if (activeTab === "All") keep = true;
      else if (activeTab === "Installed") keep = svc.installed;
      else if (activeTab === "Installing") keep = installing === svc.id;
      else keep = activeTab === svc.group;

      if (!keep) continue;
      
      if (query && !svc.name.toLowerCase().includes(query) && !(svc.description || "").toLowerCase().includes(query) && !svc.id.toLowerCase().includes(query)) {
        continue;
      }

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


  // ─── Service Config State ──────────────────────────────────────────────
  let configOpen = $state<string | null>(null);
  let configData = $state<ServiceConfig | null>(null);
  let configLoading = $state(false);
  let configSaving = $state(false);
  let configError = $state<string | null>(null);
  let configSuccess = $state<string | null>(null);
  let editValues = $state<Record<string, unknown>>({});

  async function openConfig(serviceId: string) {
    if (configOpen === serviceId) { configOpen = null; return; }
    configOpen = serviceId;
    configLoading = true;
    configError = null;
    configSuccess = null;
    try {
      configData = await getServiceConfig(serviceId);
      editValues = { ...configData.values };
    } catch (e: any) {
      configError = e?.toString?.() ?? "Failed to load config";
    }
    configLoading = false;
  }

  async function saveConfig() {
    if (!configOpen) return;
    configSaving = true;
    configError = null;
    configSuccess = null;
    logStore.info(`Saving settings for ${configOpen}...`, "service-hub");
    try {
      await setServiceConfig(configOpen, editValues);
      configSuccess = "Settings saved";
      logStore.success(`Saved settings for ${configOpen}`, "service-hub");
      if (configData) configData.values = { ...editValues };
    } catch (e: any) {
      configError = e?.toString?.() ?? "Failed to save";
      logStore.error(`Failed to save settings for ${configOpen}: ${configError}`, "service-hub");
    }
    configSaving = false;
  }
</script>

{#snippet configPanel()}
  <div class="mt-3 rounded-lg border border-white/10 bg-black/30 p-3 space-y-3">
    {#if configLoading}
      <div class="flex items-center gap-2 py-2">
        <Loader2 class="w-3.5 h-3.5 animate-spin text-white/40" />
        <span class="text-[10px] text-white/40">Loading settings...</span>
      </div>
    {:else if configData && configData.fields.length > 0}
      <div class="flex items-center justify-between">
        <p class="text-[10px] font-semibold text-white/60 uppercase tracking-wider">Configuration</p>
        <button onclick={() => { configOpen = null; }} class="text-white/30 hover:text-white/60 transition-colors">
          <X class="w-3.5 h-3.5" />
        </button>
      </div>

      {#each configData.fields as field}
        <div class="space-y-1">
          <label class="flex items-center gap-1 text-[10px] text-white/50">
            {field.label}
            {#if field.required}<span class="text-red-400">*</span>{/if}
          </label>
          <p class="text-[8px] text-white/25 leading-relaxed">{field.description}</p>

          {#if field.fieldType === "select"}
            <select
              class="w-full rounded-md border border-white/10 bg-white/5 px-2 py-1.5 text-[10px] text-white/80 outline-none focus:border-violet-500/50"
              value={editValues[field.key] as string ?? ""}
              onchange={(e) => { editValues[field.key] = (e.target as HTMLSelectElement).value; }}
            >
              {#each field.options ?? [] as opt}
                <option value={opt} class="bg-[#1a1a2e]">{opt}</option>
              {/each}
            </select>
          {:else if field.fieldType === "number"}
            <input
              type="number"
              class="w-full rounded-md border border-white/10 bg-white/5 px-2 py-1.5 text-[10px] text-white/80 outline-none focus:border-violet-500/50"
              value={editValues[field.key] as number ?? 0}
              oninput={(e) => { editValues[field.key] = Number((e.target as HTMLInputElement).value); }}
            />
          {:else if field.fieldType === "password"}
            <input
              type="password"
              class="w-full rounded-md border border-white/10 bg-white/5 px-2 py-1.5 text-[10px] text-white/80 outline-none focus:border-violet-500/50"
              value={editValues[field.key] as string ?? ""}
              oninput={(e) => { editValues[field.key] = (e.target as HTMLInputElement).value; }}
              placeholder="Leave empty if not required"
            />
          {:else if field.fieldType === "toggle"}
            <button
              class="relative w-8 h-4 rounded-full transition-colors {editValues[field.key] ? 'bg-violet-500' : 'bg-white/10'}"
              onclick={() => { editValues[field.key] = !editValues[field.key]; }}
            >
              <span class="absolute top-0.5 left-0.5 w-3 h-3 rounded-full bg-white transition-transform {editValues[field.key] ? 'translate-x-4' : ''}"></span>
            </button>
          {:else}
            <input
              type="text"
              class="w-full rounded-md border border-white/10 bg-white/5 px-2 py-1.5 text-[10px] text-white/80 outline-none focus:border-violet-500/50"
              value={editValues[field.key] as string ?? ""}
              oninput={(e) => { editValues[field.key] = (e.target as HTMLInputElement).value; }}
            />
          {/if}
        </div>
      {/each}

      <!-- Save / feedback -->
      <div class="flex items-center gap-2 pt-1">
        <button
          class="flex items-center gap-1.5 rounded-lg bg-violet-600 px-3 py-1.5 text-[10px] font-medium text-white transition-colors hover:bg-violet-500 disabled:opacity-50"
          onclick={saveConfig}
          disabled={configSaving}
        >
          {#if configSaving}
            <Loader2 class="w-3 h-3 animate-spin" /> Saving...
          {:else}
            <Save class="w-3 h-3" /> Save
          {/if}
        </button>
        {#if configSuccess}
          <span class="text-[9px] text-emerald-400">{configSuccess}</span>
        {/if}
        {#if configError}
          <span class="text-[9px] text-red-400">{configError}</span>
        {/if}
      </div>
    {:else}
      <p class="text-[10px] text-white/30 py-1">No configurable settings for this service.</p>
    {/if}
  </div>
{/snippet}

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
    <div class="flex items-center gap-2">
      <button
        onclick={checkForUpdates}
        disabled={updateChecking}
        title="Check for updates"
        class="flex items-center gap-1.5 rounded-lg {updateResult?.updateAvailable ? 'bg-emerald-600/20 border border-emerald-500/30 text-emerald-300 hover:bg-emerald-600/30' : 'bg-white/5 border border-white/10 text-white/70 hover:bg-white/10 hover:text-white'} px-3 py-1.5 text-[11px] font-medium transition-colors disabled:opacity-50"
      >
        {#if updateChecking}
          <Loader2 class="w-3.5 h-3.5 animate-spin" />
          Checking...
        {:else if updateResult?.updateAvailable}
          <ArrowUpCircle class="w-3.5 h-3.5" />
          Update Available
        {:else}
          <ArrowUpCircle class="w-3.5 h-3.5" />
          Check Updates
        {/if}
      </button>
      <button
        onclick={() => openGenericBrowserWindow("http://localhost:8080/swagger-ui/", "NDE-OS API Swagger")}
        title="NDE-OS API Swagger"
        class="flex items-center gap-1.5 rounded-lg bg-white/5 border border-white/10 px-3 py-1.5 text-[11px] font-medium text-white/70 transition-colors hover:bg-white/10 hover:text-white"
      >
        <ExternalLink class="w-3.5 h-3.5" /> API Swagger
      </button>
      <button
        class="flex items-center gap-1.5 rounded-lg bg-white/5 px-3 py-1.5 text-[11px] text-white/60 transition-colors hover:bg-white/10 hover:text-white/80"
        onclick={() => { refreshStatus(); }}
        disabled={loading}
      >
        <RefreshCw class="w-3.5 h-3.5 {loading ? 'animate-spin' : ''}" />
        {loading ? "Refreshing..." : "Refresh"}
      </button>
    </div>
  </div>

  <!-- Update available banner -->
  {#if updateResult?.updateAvailable}
    <div class="mx-4 mt-4 rounded-xl border border-emerald-400/20 bg-gradient-to-r from-emerald-500/8 to-teal-500/8 p-3.5">
      <div class="flex items-start justify-between gap-3">
        <div class="flex items-center gap-3 min-w-0">
          <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-emerald-500 to-teal-600 flex items-center justify-center shadow-lg shrink-0">
            <ArrowUpCircle class="w-5 h-5 text-white" />
          </div>
          <div class="min-w-0">
            <p class="text-xs font-semibold text-emerald-200">Update Available — v{updateResult.latestVersion}</p>
            <p class="text-[10px] text-emerald-200/60 mt-0.5">
              {updateResult.releaseName ?? "New release"} · Current: v{updateResult.currentVersion}
            </p>
          </div>
        </div>
        <div class="shrink-0 flex items-center gap-2">
          {#if updateResult.releaseUrl}
            <button
              class="flex items-center gap-1.5 rounded-lg bg-emerald-600 px-3 py-1.5 text-[11px] font-medium text-white transition-colors hover:bg-emerald-500"
              onclick={() => openGenericBrowserWindow(updateResult!.releaseUrl!, `NDE-OS ${updateResult!.latestVersion}`)}
            >
              <ExternalLink class="w-3.5 h-3.5" /> Download
            </button>
          {/if}
          <button
            class="rounded-lg p-1.5 text-emerald-300/40 transition-colors hover:bg-white/5 hover:text-emerald-300"
            onclick={() => updateResult = null}
            title="Dismiss"
          >
            <X class="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </div>
  {:else if updateResult && !updateResult.updateAvailable && !updateResult.error}
    <div class="mx-4 mt-4 rounded-xl border border-white/8 bg-white/2 p-3 flex items-center justify-between">
      <div class="flex items-center gap-2">
        <CircleCheck class="w-4 h-4 text-emerald-400" />
        <span class="text-[11px] text-white/60">You're running the latest version <span class="font-mono text-white/80">v{updateResult.currentVersion}</span></span>
      </div>
      <button
        class="rounded-lg p-1 text-white/20 transition-colors hover:bg-white/5 hover:text-white/50"
        onclick={() => updateResult = null}
        title="Dismiss"
      >
        <X class="w-3 h-3" />
      </button>
    </div>
  {:else if updateResult?.error}
    <div class="mx-4 mt-4 rounded-xl border border-red-400/20 bg-red-400/5 p-3 flex items-center justify-between">
      <div class="flex items-center gap-2 min-w-0">
        <CircleAlert class="w-4 h-4 text-red-400 shrink-0" />
        <span class="text-[11px] text-red-300/80 truncate">{updateResult.error}</span>
      </div>
      <button
        class="rounded-lg p-1 text-red-300/20 transition-colors hover:bg-white/5 hover:text-red-300/50 shrink-0"
        onclick={() => updateResult = null}
        title="Dismiss"
      >
        <X class="w-3 h-3" />
      </button>
    </div>
  {/if}

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
              <CircleCheck class="w-3 h-3" />
            {:else}
              <CircleAlert class="w-3 h-3" />
            {/if}
            {svc.name}
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Tabs -->
  <div class="px-5 pt-3 border-b border-white/5 bg-[#1a1a2e] flex items-center justify-between">
    <div class="flex-1 overflow-x-auto scrollbar-hide mr-4">
      <div class="flex items-center gap-4 min-w-max">
        <button
          class="pb-3 text-[11px] font-medium transition-colors border-b-2 {activeTab === 'All' ? 'border-violet-500 text-white' : 'border-transparent text-white/50 hover:text-white/80'}"
          onclick={() => activeTab = 'All'}
        >
          All
        </button>
        <button
          class="pb-3 text-[11px] font-medium transition-colors border-b-2 {activeTab === 'Installed' ? 'border-emerald-500 text-white' : 'border-transparent text-white/50 hover:text-white/80'}"
          onclick={() => activeTab = 'Installed'}
        >
          Installed
        </button>
        <button
          class="pb-3 text-[11px] font-medium transition-colors border-b-2 {activeTab === 'Installing' ? 'border-blue-500 text-white' : 'border-transparent text-white/50 hover:text-white/80'}"
          onclick={() => activeTab = 'Installing'}
        >
          Installing
        </button>
        
        <div class="w-px h-4 bg-white/10 mx-1"></div>
        
        {#each Object.keys(groupMeta) as tabKey}
          <button
            class="pb-3 text-[11px] font-medium transition-colors border-b-2 {activeTab === tabKey ? 'border-violet-500 text-white' : 'border-transparent text-white/50 hover:text-white/80'}"
            onclick={() => activeTab = tabKey}
          >
            {groupMeta[tabKey].label}
          </button>
        {/each}
      </div>
    </div>
    
    <!-- Search & View Toggles -->
    <div class="shrink-0 flex items-center pb-3">
      <div class="relative w-40 lg:w-56 mr-3">
        <Search class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-white/40" />
        <input
          type="text"
          placeholder="Search services..."
          bind:value={searchQuery}
          class="w-full bg-white/5 border border-white/10 rounded-md pl-8 pr-3 py-1.5 text-[11px] text-white placeholder:text-white/30 outline-none focus:border-violet-500/50 focus:bg-white/10 transition-colors"
        />
      </div>

      <div class="flex items-center gap-1 border-l border-white/10 pl-3">
        <button 
          class="p-1.5 rounded-md transition-colors {viewMode === 'list' ? 'bg-white/10 text-white' : 'text-white/40 hover:text-white/80 hover:bg-white/5'}"
          onclick={() => viewMode = 'list'}
          title="List View"
        >
          <ListIcon class="w-3.5 h-3.5" />
        </button>
        <button 
          class="p-1.5 rounded-md transition-colors {viewMode === 'grid' ? 'bg-white/10 text-white' : 'text-white/40 hover:text-white/80 hover:bg-white/5'}"
          onclick={() => viewMode = 'grid'}
          title="Grid View"
        >
          <LayoutGrid class="w-3.5 h-3.5" />
        </button>
      </div>
    </div>
  </div>

  <!-- Scroll body -->
  <div class="flex-1 overflow-y-auto px-4 py-4 space-y-5">
    {#if loading}
      <div class="flex flex-col items-center justify-center py-20">
        <Loader2 class="w-6 h-6 text-violet-400 animate-spin" />
        <p class="text-[11px] text-white/40 mt-2">Detecting services...</p>
      </div>
    {:else}


      <!-- ═══ Other Services Grid ═══ -->
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

          <div class="{viewMode === 'grid' ? 'grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3' : 'space-y-2'}">
            {#each groupServices as svc}
              <ServiceCard 
                {svc}
                isRequired={require.includes(svc.id)}
                isInstalling={installing === svc.id}
                installingAny={installing !== null}
                {viewMode}
                isConfigOpen={configOpen === svc.id}
                onConfigOpen={() => openConfig(svc.id)}
                onInstall={() => installService(svc.id)}
                onRefresh={() => refreshStatus()}
                onDrawerOpen={() => drawerService = svc}
              >
                {#if configOpen === svc.id}
                  {@render configPanel()}
                {/if}
                {#if installing === "ldplayer" && svc.id === "ldplayer" && dlStage && dlStage !== "done"}
                  <div class="mt-3 p-3 rounded-xl border border-violet-500/20 bg-violet-500/5">
                    <div class="flex items-center justify-between text-[10px] font-medium text-violet-300 mb-1.5">
                      <span>{dlMessage}</span>
                      <span>{dlPercent}%</span>
                    </div>
                    <div class="h-1.5 w-full overflow-hidden rounded-full bg-violet-500/20">
                      <div 
                        class="h-full bg-violet-500 transition-all duration-300"
                        style="width: {dlPercent}%"
                      ></div>
                    </div>
                    {#if dlStage === "installing"}
                      <p class="text-[9px] text-white/50 text-center mt-2">Follow the setup wizard, then click Refresh.</p>
                    {/if}
                  </div>
                {/if}
              </ServiceCard>
            {/each}
          </div>
        </div>
      {/each}
      
      {#if Object.keys(grouped).length === 0}
        <div class="flex flex-col items-center justify-center py-16 text-center">
          <Wrench class="w-8 h-8 text-white/10 mb-3" />
          <p class="text-[12px] font-medium text-white/40">No services found</p>
          <p class="text-[10px] text-white/20 mt-1">There are no services matching this category filter.</p>
        </div>
      {/if}
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
        <CircleCheck class="w-4 h-4" />
        All Ready — Return to {returnTo}
      </button>
    </div>
  {/if}
</div>

<!-- Right-side detail drawer -->
<ServiceDrawer
  svc={drawerService}
  onClose={() => drawerService = null}
  onInstall={drawerService ? (customId) => { installService(typeof customId === 'string' ? customId : drawerService!.id); drawerService = null; } : undefined}
  onConfigOpen={drawerService ? () => { openConfig(drawerService!.id); drawerService = null; } : undefined}
/>
