<svelte:options runes={true} />

<script lang="ts">
  import { useStore } from "../../../lib/use-store.svelte";
  import { shieldBrowserStore } from "../../../state/shield";
  import { onMount, onDestroy } from "svelte";
  import { Button } from "$lib/components/ui/button";
  
  import ShieldSetup from "./ShieldSetup.svelte";
  import ShieldSettings from "./ShieldSettings.svelte";
  import ShieldCreateProfile from "./ShieldCreateProfile.svelte";
  import ShieldDevices from "./ShieldDevices.svelte";
  import ProfileTable from "./ProfileTable.svelte";

  const store = useStore(shieldBrowserStore);

  onMount(() => {
    store.init();
  });

  onDestroy(() => {
    store.stopListenerTracker();
  });
</script>

<div class="flex flex-col w-full h-full bg-background text-foreground overflow-hidden">
  {#if store.loading}
    <div class="flex-1 flex flex-col items-center justify-center gap-4">
      <div class="w-8 h-8 border-4 border-white/20 border-t-white rounded-full animate-spin"></div>
      <p class="text-white/70 text-sm">Initializing Shield Browser...</p>
    </div>
  {:else if store.view === "setup"}
    <ShieldSetup />
  {:else}
    <!-- ═══ Main App Layout ═══ -->
    <header class="flex items-center justify-between px-6 py-4 border-b border-border bg-card shrink-0 shadow-sm z-10">
      <div class="flex items-center gap-3">
        <div class="flex items-center justify-center w-10 h-10 bg-secondary rounded-lg border border-border/50 text-2xl shadow-inner">
          🛡️
        </div>
        <div class="flex flex-col">
          <h2 class="m-0 text-lg font-semibold tracking-tight leading-none">Shield Browser</h2>
          <span class="text-[11px] text-muted-foreground uppercase tracking-wider font-medium mt-1">Anti-Detect Engine</span>
        </div>
      </div>
      
      <div class="flex items-center p-1 bg-secondary/50 rounded-lg border border-border/50 gap-1 shadow-inner">
        <Button 
          variant={store.view === "profiles" ? "secondary" : "ghost"} 
          size="sm" 
          class="h-8 shadow-none" 
          onclick={() => store.setView("profiles")}
        >
          Profiles
        </Button>
        <Button 
          variant={store.view === "devices" ? "secondary" : "ghost"} 
          size="sm" 
          class="h-8 shadow-none" 
          onclick={() => store.setView("devices")}
        >
          Devices
        </Button>
        <Button 
          variant={store.view === "create" ? "secondary" : "ghost"} 
          size="sm" 
          class="h-8 shadow-none gap-1" 
          onclick={() => { store.setView("create"); store.resolveCreateVersion(); }}
        >
          <span class="text-base font-light leading-none">+</span> Create
        </Button>
        <div class="w-[1px] h-4 bg-border mx-1"></div>
        <Button 
          variant={store.view === "settings" ? "secondary" : "ghost"} 
          size="icon" 
          class="h-8 w-8 shadow-none text-base" 
          title="Settings" 
          onclick={() => store.openSettings()}
        >
          ⚙️
        </Button>
      </div>

      <div class="flex items-center gap-4 px-4 py-2 rounded-lg border border-border bg-background shadow-sm">
        {#if store.status}
          <div class="flex items-baseline gap-1.5">
            <span class="text-base font-bold text-foreground">{store.status.total_profiles}</span>
            <span class="text-[11px] text-muted-foreground uppercase tracking-widest font-medium">Profiles</span>
          </div>
          <div class="flex items-baseline gap-1.5">
            <span class="text-base font-bold {store.status.running_profiles > 0 ? 'text-emerald-500' : 'text-foreground'}">
              {store.status.running_profiles}
            </span>
            <span class="text-[11px] text-muted-foreground uppercase tracking-widest font-medium">Active</span>
          </div>
        {/if}
      </div>
    </header>

    <main class="flex-1 min-h-0 p-6 overflow-hidden flex flex-col bg-muted/20">
      {#if store.view === "settings"}
        <ShieldSettings />
      {:else if store.view === "create"}
        <ShieldCreateProfile />
      {:else if store.view === "devices"}
        <ShieldDevices />
      {:else if store.view === "profiles"}
        <ProfileTable />
      {/if}
    </main>
  {/if}
</div>
