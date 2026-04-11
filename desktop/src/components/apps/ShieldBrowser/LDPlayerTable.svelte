<svelte:options runes={true} />

<script lang="ts">
  import { useStore } from "../../../lib/use-store.svelte";
  import { shieldBrowserStore } from "../../../state/shield";
  import { onMount, onDestroy } from "svelte";
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Badge } from "$lib/components/ui/badge";
  import * as Table from "$lib/components/ui/table";
  import type { LdPlayerInstance } from "./types";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { Download } from "@lucide/svelte";

  let dlStage = $state("");
  let dlPercent = $state(0);
  let dlMessage = $state("");
  let ulistenDl: UnlistenFn | null = null;

  const store = useStore(shieldBrowserStore);
  let pollInterval: ReturnType<typeof setInterval>;

  onMount(async () => {
    store.detectLdPlayer();
    store.loadLdInstances();
    pollInterval = setInterval(() => {
      if (!store.ldActionBusy && !store.ldLoading) {
        store.loadLdInstances();
      }
    }, 5000);

    ulistenDl = await listen("ldplayer-download-progress", (event: any) => {
      dlStage = event.payload.stage;
      dlPercent = event.payload.percent;
      dlMessage = event.payload.message;
      if (dlStage === "done" || dlStage === "installing") {
        setTimeout(() => store.detectLdPlayer(), 5000);
      }
    });
  });

  onDestroy(() => {
    clearInterval(pollInterval);
    ulistenDl?.();
  });

  async function startDownload() {
    dlStage = "connecting";
    dlMessage = "Starting download...";
    try {
      await invoke("shield_download_ldplayer");
    } catch (e: any) {
      dlStage = "error";
      dlMessage = e.toString();
    }
  }

  // ── Local sort state ───────────────────────────────────────────
  let sortKey = $state<"name" | "index" | "is_running">("index");
  let sortAsc = $state(true);

  const sortedInstances = $derived.by(() => {
    return [...store.ldInstances].sort((a, b) => {
      const av = a[sortKey];
      const bv = b[sortKey];
      const cmp = av < bv ? -1 : av > bv ? 1 : 0;
      return sortAsc ? cmp : -cmp;
    });
  });

  function toggleSort(key: typeof sortKey) {
    if (sortKey === key) {
      sortAsc = !sortAsc;
    } else {
      sortKey = key;
      sortAsc = true;
    }
  }

  function sortIcon(key: string) {
    if (sortKey !== key) return "⇅";
    return sortAsc ? "▲" : "▼";
  }

  function formatMemory(mb: number | null): string {
    if (!mb) return "—";
    return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb} MB`;
  }

  function clickOutside(node: Node, callback: () => void) {
    const handleClick = (e: MouseEvent) => {
      if (node && !node.contains(e.target as Node)) callback();
    };
    document.addEventListener("mousedown", handleClick);
    return { destroy() { document.removeEventListener("mousedown", handleClick); } };
  }
</script>

<div class="relative flex flex-col gap-5 h-full text-foreground w-full">
  <!-- Header -->
  <div class="flex justify-between items-start">
    <div class="flex flex-col gap-1">
      <h3 class="text-2xl font-bold m-0 flex items-center gap-2">🎮 LDPlayer Manager</h3>
      <p class="text-muted-foreground text-sm m-0">
        {#if store.ldDetection?.available}
          Connected to <span class="font-mono text-xs text-primary">{store.ldDetection.version_dir ?? "LDPlayer"}</span>
        {:else}
          LDPlayer not detected on this system.
        {/if}
      </p>
    </div>
    <div class="flex gap-2">
      <Button variant="outline" onclick={() => store.loadLdInstances()} disabled={store.ldLoading}>
        🔄 {store.ldLoading ? "Scanning..." : "Refresh"}
      </Button>
      <Button variant="outline" onclick={() => { store.setLdShowCreateDialog(true); }}>
        ➕ New Instance
      </Button>
      {#if store.ldInstances.some(i => i.is_running)}
        <Button variant="destructive" class="bg-destructive/10 text-destructive hover:bg-destructive/20 border-destructive/20" onclick={() => { if (confirm("Quit ALL running LDPlayer instances?")) store.quitAllLdPlayer(); }} disabled={store.ldActionBusy}>
          ⏹ Quit All
        </Button>
      {/if}
    </div>
  </div>

  {#if store.ldError}
    <div class="flex items-center gap-2 p-3 bg-destructive/10 text-destructive border border-destructive/20 rounded-lg text-sm">
      <span>⚠️</span>
      <p class="m-0">{store.ldError}</p>
    </div>
  {/if}

  {#if !store.ldDetection?.available}
    <!-- Not Installed State -->
    <div class="flex-1 flex flex-col items-center justify-center gap-5 text-muted-foreground">
      <span class="text-6xl opacity-50">🎮</span>
      <div class="flex flex-col items-center gap-1.5 max-w-md text-center">
        <p class="text-base font-semibold text-foreground m-0">LDPlayer Not Detected</p>
        <p class="text-sm text-muted-foreground m-0">
          <code class="text-xs bg-secondary px-1.5 py-0.5 rounded">ldconsole.exe</code> was not found on this system.
          Download and install LDPlayer, then click Re-detect.
        </p>
      </div>
      
      <!-- Auto Download link -->
      {#if dlStage && dlStage !== "error" && dlStage !== "done"}
        <div class="w-full max-w-md mt-2 flex flex-col gap-2 rounded-xl border border-primary/20 bg-primary/5 p-4">
          <div class="flex items-center justify-between text-xs font-medium">
            <span class="text-primary">{dlMessage}</span>
            <span class="text-muted-foreground">{dlPercent}%</span>
          </div>
          <div class="h-2 w-full overflow-hidden rounded-full bg-secondary/30">
            <div 
              class="h-full bg-primary transition-all duration-300"
              style="width: {dlPercent}%"
            ></div>
          </div>
          {#if dlStage === "installing"}
            <p class="text-[10px] text-muted-foreground text-center mt-1">Please finish the LDPlayer setup window that appears.</p>
            <div class="flex items-center justify-center mt-2">
              <Button variant="outline" size="sm" onclick={() => store.detectLdPlayer()}>🔍 Check Installation</Button>
            </div>
          {/if}
        </div>
      {:else}
        <div class="flex items-center gap-3 px-4 py-3 rounded-xl border border-primary/20 bg-primary/5">
          <div class="flex flex-col gap-0.5">
            <span class="text-xs font-semibold text-foreground">Automatic Installation</span>
            <span class="text-[11px] text-muted-foreground">Download & launch the LDPlayer 9 installer</span>
          </div>
          <Button variant="default" size="sm" onclick={startDownload}>
            <Download class="w-4 h-4 mr-1.5" />
            Install
          </Button>
        </div>

        {#if dlStage === "error"}
          <div class="text-xs text-destructive bg-destructive/10 px-3 py-2 rounded">Format error: {dlMessage}</div>
        {/if}

        <div class="flex items-center gap-2">
          <Button variant="outline" onclick={() => store.detectLdPlayer()}>🔍 Re-detect</Button>
          <Button variant="secondary" onclick={() => {
            import("🍎/state/desktop.svelte").then(({ openServiceHub }) => {
              openServiceHub({ require: ["ldplayer"], returnTo: "shield-browser", returnData: { returnView: "emulators" } });
            });
          }}>🛠️ Service Hub</Button>
        </div>
      {/if}
    </div>
  {:else if store.ldInstances.length === 0 && !store.ldLoading}
    <div class="flex-1 flex flex-col items-center justify-center gap-3 text-muted-foreground border border-dashed border-border/60 rounded-xl bg-secondary/10">
      <span class="text-5xl opacity-80">🎮</span>
      <p class="text-sm">No LDPlayer instances found. Create one to get started.</p>
    </div>
  {:else}
    <!-- Instances Table -->
    <div class="flex-1 overflow-auto rounded-xl border border-border bg-card shadow-sm">
      <Table.Root>
        <Table.Header class="bg-secondary/20 sticky top-0 z-10">
          <Table.Row class="hover:bg-transparent border-border/60">
            {#each [
              { key: "index", label: "#" },
              { key: "name", label: "Instance Name" },
              { key: "is_running", label: "Status" },
            ] as col}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <Table.Head
                class="h-10 px-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground cursor-pointer hover:bg-secondary/40 transition-colors select-none"
                onclick={() => toggleSort(col.key as typeof sortKey)}
              >
                <div class="flex items-center gap-1.5">
                  {col.label}
                  <span class="text-[9px] {sortKey === col.key ? 'text-primary/70' : 'text-muted-foreground/30'}">
                    {sortIcon(col.key)}
                  </span>
                </div>
              </Table.Head>
            {/each}
            <Table.Head class="h-10 px-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground">CPU</Table.Head>
            <Table.Head class="h-10 px-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground">RAM</Table.Head>
            <Table.Head class="h-10 px-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground">Resolution</Table.Head>
            <Table.Head class="h-10 px-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground">Notes</Table.Head>
            <Table.Head class="h-10 px-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground">Tags</Table.Head>
            <Table.Head class="w-[140px] h-10 px-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground">Actions</Table.Head>
          </Table.Row>
        </Table.Header>
        <Table.Body>
          {#each sortedInstances as instance (instance.index)}
            <Table.Row
              class="cursor-pointer transition-colors border-border/40 group border-l-2 {store.ldSelectedInstance?.index === instance.index && store.ldDrawerOpen ? 'bg-primary/5 hover:bg-primary/10 border-l-primary' : 'hover:bg-secondary/30 border-l-transparent'} {instance.is_running ? '!border-l-emerald-500' : ''}"
              onclick={() => { store.setLdSelectedInstance(instance); store.setLdDrawerOpen(true); }}
            >
              <Table.Cell class="py-2.5 px-4 text-sm text-muted-foreground font-mono">{instance.index}</Table.Cell>
              <Table.Cell class="py-2.5 px-4 text-sm font-semibold">{instance.name}</Table.Cell>
              <Table.Cell class="py-2.5 px-4 text-sm">
                {#if instance.is_running}
                  <Badge variant="default" class="bg-emerald-500/15 text-emerald-500 hover:bg-emerald-500/20 text-[10px]">🟢 Running</Badge>
                {:else}
                  <Badge variant="secondary" class="text-[10px]">⚪ Stopped</Badge>
                {/if}
              </Table.Cell>
              <Table.Cell class="py-2.5 px-4 text-sm text-muted-foreground">{instance.cpu ? `${instance.cpu} cores` : "—"}</Table.Cell>
              <Table.Cell class="py-2.5 px-4 text-sm text-muted-foreground">{formatMemory(instance.memory)}</Table.Cell>
              <Table.Cell class="py-2.5 px-4 text-sm text-muted-foreground font-mono text-xs">{instance.resolution || "—"}</Table.Cell>
              <Table.Cell class="py-2.5 px-4 text-sm text-muted-foreground truncate max-w-[120px]">{instance.notes || "—"}</Table.Cell>
              <Table.Cell class="py-2.5 px-4">
                {#if instance.tags.length > 0}
                  <div class="flex flex-wrap gap-1">
                    {#each instance.tags.slice(0, 2) as tag}
                      <Badge variant="outline" class="text-[9px] px-1.5 py-0">{tag}</Badge>
                    {/each}
                    {#if instance.tags.length > 2}
                      <Badge variant="outline" class="text-[9px] px-1.5 py-0">+{instance.tags.length - 2}</Badge>
                    {/if}
                  </div>
                {:else}
                  <span class="text-muted-foreground text-sm">—</span>
                {/if}
              </Table.Cell>
              <Table.Cell class="py-1 px-4">
                <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                  {#if instance.is_running}
                    <Button variant="ghost" size="icon" class="h-7 w-7 text-xs text-emerald-500 hover:bg-emerald-500/20 border-0" title="Stop"
                      onclick={(e) => { e.stopPropagation(); store.quitLdPlayer(instance.name); }}>⏹</Button>
                  {:else}
                    <Button variant="ghost" size="icon" class="h-7 w-7 text-xs hover:bg-primary/20 hover:text-primary border-0" title="Launch"
                      onclick={(e) => { e.stopPropagation(); store.launchLdPlayer(instance.name); }} disabled={store.ldActionBusy}>▶</Button>
                  {/if}
                  <Button variant="ghost" size="icon" class="h-7 w-7 text-xs hover:bg-secondary border-0" title="Clone"
                    onclick={(e) => { e.stopPropagation(); store.setLdCloneSourceName(instance.name); store.setLdCloneNewName(`${instance.name}-copy`); store.setLdShowCloneDialog(true); }}>📋</Button>
                  <Button variant="ghost" size="icon" class="h-7 w-7 text-xs text-destructive hover:bg-destructive/20 border-0" title="Delete"
                    onclick={(e) => { e.stopPropagation(); if (confirm(`Delete instance "${instance.name}"? This cannot be undone.`)) store.removeLdPlayer(instance.name); }} disabled={instance.is_running}>🗑</Button>
                </div>
              </Table.Cell>
            </Table.Row>
          {/each}
        </Table.Body>
      </Table.Root>
    </div>
  {/if}
</div>

<!-- ═══ Detail Drawer ═══ -->
{#if store.ldDrawerOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="absolute inset-0 bg-background/30 backdrop-blur-sm z-40"
    transition:fly={{ duration: 200, opacity: 0 }}
    onclick={() => store.setLdDrawerOpen(false)}
  ></div>

  <div
    class="absolute top-0 right-0 bottom-0 w-[420px] bg-card border-l border-border/60 shadow-2xl z-50 flex flex-col"
    transition:fly={{ x: 420, duration: 350, easing: cubicOut }}
    use:clickOutside={() => store.setLdDrawerOpen(false)}
  >
    {#if store.ldSelectedInstance}
      {@const inst = store.ldSelectedInstance}
      <!-- Drawer Header -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-border/40 bg-secondary/10 shrink-0">
        <div class="flex items-center gap-3">
          <div class="text-2xl bg-secondary/50 w-10 h-10 rounded-lg flex items-center justify-center border border-border/50">🎮</div>
          <div class="flex flex-col">
            <h3 class="m-0 text-lg font-bold tracking-tight">{inst.name}</h3>
            <span class="text-[11px] text-muted-foreground font-mono">Index #{inst.index}</span>
          </div>
        </div>
        <Button variant="ghost" size="icon" class="h-8 w-8 text-muted-foreground" onclick={() => store.setLdDrawerOpen(false)}>✕</Button>
      </div>

      <!-- Drawer Body -->
      <div class="flex-1 overflow-y-auto p-6 flex flex-col gap-5">
        <!-- Status Section -->
        <div class="flex flex-col gap-0">
          {#each [
            { label: "Status", value: inst.is_running ? "Running" : "Stopped", badge: true },
            { label: "PID", value: inst.pid > 0 ? inst.pid.toString() : "—", mono: true },
            { label: "CPU", value: inst.cpu ? `${inst.cpu} cores` : "Default" },
            { label: "Memory", value: formatMemory(inst.memory) },
            { label: "Resolution", value: inst.resolution || "Default", mono: true },
            { label: "Proxy", value: inst.proxy_host ? `${inst.proxy_host}:${inst.proxy_port ?? 8080}` : "None", mono: true },
            { label: "Shield Profile", value: inst.linked_shield_profile_id || "Not linked" },
          ] as row}
            <div class="flex justify-between items-center py-2.5 border-b border-border/40 last:border-0">
              <span class="text-xs uppercase tracking-widest font-semibold text-muted-foreground">{row.label}</span>
              {#if row.badge}
                <Badge variant={inst.is_running ? "default" : "secondary"}
                  class={inst.is_running ? "bg-emerald-500/15 text-emerald-500 hover:bg-emerald-500/20" : ""}>
                  {row.value}
                </Badge>
              {:else}
                <span class="text-sm font-medium {row.mono ? 'font-mono text-xs' : ''}">{row.value}</span>
              {/if}
            </div>
          {/each}
        </div>

        <!-- Tags -->
        {#if inst.tags.length > 0}
          <div class="flex flex-wrap gap-1.5 pt-1">
            {#each inst.tags as tag}
              <Badge variant="outline" class="bg-primary/5 text-primary border-primary/20 text-[10px] uppercase tracking-wider">{tag}</Badge>
            {/each}
          </div>
        {/if}

        <!-- Notes -->
        {#if inst.notes}
          <div class="p-3 bg-amber-500/10 text-amber-600 border border-amber-500/20 rounded-lg text-sm leading-relaxed">
            <strong>Note:</strong> {inst.notes}
          </div>
        {/if}
      </div>

      <!-- Drawer Footer -->
      <div class="p-5 border-t border-border/40 bg-secondary/10 flex items-center gap-2 shrink-0">
        {#if inst.is_running}
          <Button class="flex-1 bg-emerald-500 hover:bg-emerald-600 text-white font-semibold" onclick={() => store.quitLdPlayer(inst.name)} disabled={store.ldActionBusy}>
            ⏹ Stop Instance
          </Button>
        {:else}
          <Button class="flex-1 font-semibold" onclick={() => store.launchLdPlayer(inst.name)} disabled={store.ldActionBusy}>
            ▶ Launch Instance
          </Button>
        {/if}
        <Button variant="outline" size="icon" title="Clone" onclick={() => { store.setLdCloneSourceName(inst.name); store.setLdCloneNewName(`${inst.name}-copy`); store.setLdShowCloneDialog(true); }}>📋</Button>
        <Button variant="ghost" size="icon" class="text-destructive hover:bg-destructive/10" title="Delete"
          onclick={() => { if (confirm(`Delete "${inst.name}"?`)) { store.removeLdPlayer(inst.name); store.setLdDrawerOpen(false); } }} disabled={inst.is_running}>🗑</Button>
      </div>
    {/if}
  </div>
{/if}

<!-- ═══ Create Instance Dialog ═══ -->
{#if store.ldShowCreateDialog}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="fixed inset-0 bg-background/80 backdrop-blur-sm z-50 flex items-center justify-center" onclick={() => store.setLdShowCreateDialog(false)}>
    <div class="bg-card w-full max-w-sm rounded-xl border border-border/60 shadow-xl p-6" onclick={e => e.stopPropagation()}>
      <div class="flex flex-col gap-1.5 mb-5">
        <h3 class="text-lg font-bold m-0 text-foreground tracking-tight">Create LDPlayer Instance</h3>
        <p class="text-sm text-muted-foreground m-0">A new Android emulator will be created via ldconsole.</p>
      </div>
      <Input type="text" placeholder="Instance name (e.g. Farm-01)" value={store.ldNewInstanceName}
        oninput={e => store.setLdNewInstanceName(e.currentTarget.value)}
        onkeydown={e => { if (e.key === "Enter") store.createLdPlayer(); }}
        class="mb-6 text-sm h-10" />
      <div class="flex justify-end gap-2">
        <Button variant="ghost" onclick={() => store.setLdShowCreateDialog(false)}>Cancel</Button>
        <Button onclick={() => store.createLdPlayer()} disabled={!store.ldNewInstanceName.trim() || store.ldActionBusy}>
          {store.ldActionBusy ? "Creating..." : "Create"}
        </Button>
      </div>
    </div>
  </div>
{/if}

<!-- ═══ Clone Instance Dialog ═══ -->
{#if store.ldShowCloneDialog}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="fixed inset-0 bg-background/80 backdrop-blur-sm z-50 flex items-center justify-center" onclick={() => store.setLdShowCloneDialog(false)}>
    <div class="bg-card w-full max-w-sm rounded-xl border border-border/60 shadow-xl p-6" onclick={e => e.stopPropagation()}>
      <div class="flex flex-col gap-1.5 mb-5">
        <h3 class="text-lg font-bold m-0 text-foreground tracking-tight">Clone Instance</h3>
        <p class="text-sm text-muted-foreground m-0">
          Create a copy of <span class="font-semibold text-foreground">"{store.ldCloneSourceName}"</span>.
        </p>
      </div>
      <Input type="text" placeholder="New instance name" value={store.ldCloneNewName}
        oninput={e => store.setLdCloneNewName(e.currentTarget.value)}
        onkeydown={e => { if (e.key === "Enter") store.cloneLdPlayer(); }}
        class="mb-6 text-sm h-10" />
      <div class="flex justify-end gap-2">
        <Button variant="ghost" onclick={() => store.setLdShowCloneDialog(false)}>Cancel</Button>
        <Button onclick={() => store.cloneLdPlayer()} disabled={!store.ldCloneNewName.trim() || store.ldActionBusy}>
          {store.ldActionBusy ? "Cloning..." : "Clone"}
        </Button>
      </div>
    </div>
  </div>
{/if}
