<svelte:options runes={true} />

<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { Download, CircleCheck, CircleAlert, ArrowLeft, Loader2, ExternalLink, Wrench, Mic, Film, Code2, Cpu, Play, Square, RefreshCw, Settings, X, Save, List as ListIcon, LayoutGrid, Search } from "@lucide/svelte";
  import { vikingStatus, vikingInstall, vikingStart, vikingStop, getServiceConfig, setServiceConfig } from "$lib/api/backend";
  import type { VikingStatus, ServiceConfig, ConfigField } from "$lib/api/types";
  import { desktop, setVikingInstalled, openGenericBrowserWindow } from "🍎/state/desktop.svelte";
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

  // Deep-link context passed via openServiceHub({ require, returnTo })
  let require = $derived<string[]>(win.data?.require ?? []);
  let returnTo = $derived<string | null>(win.data?.returnTo ?? null);

  // ─── State ───────────────────────────────────────────────────────────────
  let services = $state<ServiceStatus[]>([]);
  let loading = $state(true);
  let installing = $state<string | null>(null);
  let installError = $state<string | null>(null);
  let installSuccess = $state<string | null>(null);
  let activeTab = $state<string>("All");
  let viewMode = $state<"list" | "grid">("list");
  let drawerService = $state<ServiceStatus | null>(null);
  let searchQuery = $state("");

  // Viking-specific state
  let viking = $state<VikingStatus | null>(null);
  let vikingLoading = $state(false);
  let vikingAction = $state("");

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
    fetchVikingStatus();
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

  async function fetchVikingStatus() {
    vikingLoading = true;
    try {
      viking = await vikingStatus();
      if (viking.connected) setVikingInstalled();
    } catch { viking = null; }
    vikingLoading = false;
  }

  async function installService(serviceId: string) {
    try {
      installing = serviceId;
      installError = null;
      installSuccess = null;
      logStore.info(`Installing ${serviceId}...`, "service-hub");
      const message = await invoke<string>("service_hub_install", { serviceId });
      installSuccess = message;
      logStore.success(`Installed ${serviceId}`, "service-hub");
      await refreshStatus();
      // If we just installed OpenViking, refresh its live status too
      if (serviceId === "openviking") await fetchVikingStatus();
    } catch (e: any) {
      installError = e?.toString?.() ?? "Install failed";
      logStore.error(`Failed to install ${serviceId}: ${installError}`, "service-hub");
    } finally {
      installing = null;
    }
  }

  async function handleVikingStart() {
    vikingAction = "starting";
    logStore.info("Starting OpenViking...", "service-hub");
    try {
      viking = await vikingStart();
      logStore.success(`Started OpenViking (Port ${viking?.port})`, "service-hub");
      setVikingInstalled();
    } catch (e: any) {
      logStore.error(`Failed to start OpenViking: ${e?.toString()}`, "service-hub");
      await fetchVikingStatus(); 
    }
    vikingAction = "";
  }

  async function handleVikingStop() {
    vikingAction = "stopping";
    logStore.info("Stopping OpenViking...", "service-hub");
    try {
      viking = await vikingStop();
      logStore.success("Stopped OpenViking", "service-hub");
    } catch (e: any) {
      logStore.error(`Failed to stop OpenViking: ${e?.toString()}`, "service-hub");
      await fetchVikingStatus(); 
    }
    vikingAction = "";
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
    const query = searchQuery.toLowerCase().trim();
    
    for (const svc of services) {
      // Skip openviking from the general grid — it has its own featured card
      if (svc.id === "openviking") continue;
      
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

  // Viking convenience derivations
  let vikingService = $derived(services.find(s => s.id === "openviking"));
  let vikingInstalled = $derived(vikingService?.installed ?? false);
  let vikingConnected = $derived(viking?.connected ?? false);
  let isBusy = $derived(vikingAction !== "" || installing === "openviking" || desktop.viking_onboard_state?.stage === "installing" || desktop.viking_onboard_state?.stage === "starting");

  let showViking = $derived.by(() => {
    const query = searchQuery.toLowerCase().trim();
    if (query) {
      const matchText = "openviking context database core memory agent".toLowerCase();
      if (!matchText.includes(query)) return false;
    }

    if (activeTab === "All" || activeTab === "ai") return true;
    if (activeTab === "Installed" && vikingInstalled) return true;
    if (activeTab === "Installing" && (installing === "openviking" || desktop.viking_onboard_state?.stage === "installing")) return true;
    return false;
  });

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
        onclick={() => openGenericBrowserWindow("http://localhost:8080/swagger-ui/", "NDE-OS API Swagger")}
        title="NDE-OS API Swagger"
        class="flex items-center gap-1.5 rounded-lg bg-white/5 border border-white/10 px-3 py-1.5 text-[11px] font-medium text-white/70 transition-colors hover:bg-white/10 hover:text-white"
      >
        <ExternalLink class="w-3.5 h-3.5" /> API Swagger
      </button>
      <button
        class="flex items-center gap-1.5 rounded-lg bg-white/5 px-3 py-1.5 text-[11px] text-white/60 transition-colors hover:bg-white/10 hover:text-white/80"
        onclick={() => { refreshStatus(); fetchVikingStatus(); }}
        disabled={loading}
      >
        <RefreshCw class="w-3.5 h-3.5 {loading ? 'animate-spin' : ''}" />
        {loading ? "Refreshing..." : "Refresh"}
      </button>
    </div>
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

      <!-- ═══ OpenViking Featured Card ═══ -->
      {#if showViking}
      <div class="rounded-xl border border-emerald-500/20 bg-emerald-500/5 p-4">
        <div class="flex items-start justify-between gap-3">
          <div class="flex items-center gap-3 min-w-0">
            <div class="w-10 h-10 rounded-xl bg-gradient-to-br from-emerald-500 to-teal-600 flex items-center justify-center text-xl shadow-lg shrink-0">🗄️</div>
            <div class="min-w-0">
              <div class="flex items-center gap-2">
                <p class="text-[13px] font-semibold text-white">OpenViking</p>
                <span class="rounded-full bg-emerald-500/20 px-1.5 py-px text-[8px] uppercase tracking-wider text-emerald-300">core</span>
              </div>
              <p class="text-[10px] text-white/45 mt-0.5 leading-relaxed">Context database for agent memory, resources & skills — powers semantic search, virtual FS, and agent tools</p>
            </div>
          </div>

          <!-- Live status pill -->
          <div class="shrink-0">
            {#if vikingLoading}
              <div class="flex items-center gap-1.5 rounded-full bg-white/5 px-2.5 py-1 text-[10px] text-white/40">
                <Loader2 class="w-3 h-3 animate-spin" /> Checking...
              </div>
            {:else if vikingConnected}
              <div class="flex items-center gap-1.5 rounded-full bg-emerald-500/15 px-2.5 py-1 text-[10px] text-emerald-300">
                <span class="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span> Connected
              </div>
            {:else if vikingInstalled}
              <div class="flex items-center gap-1.5 rounded-full bg-amber-500/15 px-2.5 py-1 text-[10px] text-amber-300">
                <span class="w-2 h-2 rounded-full bg-amber-400"></span> Offline
              </div>
            {:else}
              <div class="flex items-center gap-1.5 rounded-full bg-red-500/15 px-2.5 py-1 text-[10px] text-red-300">
                <CircleAlert class="w-3 h-3" /> Not Installed
              </div>
            {/if}
          </div>
        </div>

        <!-- Auto-onboard banner -->
        {#if desktop.viking_onboard_state && (desktop.viking_onboard_state.stage === "installing" || desktop.viking_onboard_state.stage === "starting")}
          <div class="mt-3 flex items-center gap-2.5 rounded-lg bg-blue-500/10 border border-blue-500/15 px-3 py-2">
            <div class="w-4 h-4 border-2 border-blue-400 border-t-transparent rounded-full animate-spin shrink-0"></div>
            <span class="text-[10px] font-medium text-blue-300">{desktop.viking_onboard_state.message}</span>
          </div>
        {/if}
        {#if desktop.viking_onboard_state && desktop.viking_onboard_state.stage === "error"}
          <div class="mt-3 flex items-center gap-2 rounded-lg bg-red-500/10 border border-red-500/15 px-3 py-2">
            <span class="text-red-400 text-sm shrink-0">!</span>
            <span class="text-[10px] text-red-300">{desktop.viking_onboard_state.message}</span>
          </div>
        {/if}

        <!-- Viking details row -->
        {#if viking}
          <div class="mt-3 flex items-center gap-4 text-[9px] text-white/30">
            <span>Port {viking.port}</span>
            <span>Process: {viking.process_managed ? "Managed" : "External / None"}</span>
            {#if viking.message && !viking.connected}
              <span>{viking.message}</span>
            {/if}
          </div>
        {/if}

        <!-- Action buttons -->
        <div class="mt-3 flex items-center gap-2">
          {#if !vikingConnected}
            {#if !vikingInstalled}
              <button
                class="flex items-center gap-1.5 rounded-lg bg-violet-600 px-3 py-1.5 text-[10px] font-medium text-white transition-colors hover:bg-violet-500 disabled:opacity-50"
                onclick={() => installService("openviking")}
                disabled={isBusy}
              >
                {#if installing === "openviking" || desktop.viking_onboard_state?.stage === "installing"}
                  <Loader2 class="w-3 h-3 animate-spin" /> Installing...
                {:else}
                  <Download class="w-3 h-3" /> Install
                {/if}
              </button>
            {:else}
              <button
                class="flex items-center gap-1.5 rounded-lg bg-violet-600 px-3 py-1.5 text-[10px] font-medium text-white transition-colors hover:bg-violet-500 disabled:opacity-50"
                onclick={() => installService("openviking")}
                disabled={isBusy}
              >
                <RefreshCw class="w-3 h-3" /> Re-install
              </button>
            {/if}
          {/if}

          {#if vikingInstalled && !vikingConnected}
            <button
              class="flex items-center gap-1.5 rounded-lg bg-emerald-600 px-3 py-1.5 text-[10px] font-medium text-white transition-colors hover:bg-emerald-500 disabled:opacity-50"
              onclick={handleVikingStart}
              disabled={isBusy}
            >
              {#if vikingAction === "starting" || desktop.viking_onboard_state?.stage === "starting"}
                <Loader2 class="w-3 h-3 animate-spin" /> Starting...
              {:else}
                <Play class="w-3 h-3" /> Start
              {/if}
            </button>
          {/if}

          {#if vikingConnected}
            <button
              class="flex items-center gap-1.5 rounded-lg bg-red-600/80 px-3 py-1.5 text-[10px] font-medium text-white transition-colors hover:bg-red-500 disabled:opacity-50"
              onclick={handleVikingStop}
              disabled={isBusy}
            >
              {#if vikingAction === "stopping"}
                <Loader2 class="w-3 h-3 animate-spin" /> Stopping...
              {:else}
                <Square class="w-3 h-3" /> Stop
              {/if}
            </button>

            <!-- API Docs link -->
            <button
              onclick={() => openGenericBrowserWindow(`http://localhost:${viking?.port ?? 1933}/docs`, "OpenViking Docs")}
              class="flex items-center gap-1.5 rounded-lg bg-white/5 border border-white/10 px-3 py-1.5 text-[10px] font-medium text-white/70 transition-colors hover:bg-white/10 hover:text-white"
            >
              <ExternalLink class="w-3 h-3" /> API Docs
            </button>
          {/if}
        </div>

        <!-- Used by + API endpoint + Settings gear -->
        <div class="flex items-center gap-1 mt-2.5 flex-wrap">
          <span class="text-[9px] text-white/25">Used by:</span>
          <span class="rounded-full bg-white/5 px-1.5 py-px text-[9px] text-white/40">Agent Chat</span>
          <span class="rounded-full bg-white/5 px-1.5 py-px text-[9px] text-white/40">MCP Tools</span>
          {#if vikingConnected}
            <span class="text-[9px] text-white/20 ml-1">·</span>
            <span class="rounded-full bg-emerald-500/10 px-1.5 py-px text-[9px] text-emerald-300/60 font-mono">http://localhost:{viking?.port ?? 1933}</span>
          {/if}
          <button
            class="ml-auto flex items-center gap-1 rounded-lg px-2 py-1 text-[9px] text-white/40 transition-colors hover:bg-white/5 hover:text-white/70 {configOpen === 'openviking' ? 'bg-white/5 text-white/70' : ''}"
            onclick={() => openConfig("openviking")}
          >
            <Settings class="w-3 h-3" /> Settings
          </button>
        </div>

        <!-- OpenViking Config Panel -->
        {#if configOpen === "openviking"}
          {@render configPanel()}
        {/if}
      </div>
      {/if}

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
              </ServiceCard>
            {/each}
          </div>
        </div>
      {/each}
      
      {#if Object.keys(grouped).length === 0 && !showViking}
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
  onInstall={drawerService ? () => installService(drawerService!.id) : undefined}
  onConfigOpen={drawerService ? () => { openConfig(drawerService!.id); drawerService = null; } : undefined}
/>
