<script lang="ts">
  import { useStore } from "../../../lib/use-store.svelte";
  import { shieldBrowserStore } from "../../../state/shield";
  import { engineIcon, engineLabel, formatDateTime, formatDate } from "./utils";
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import * as Table from "$lib/components/ui/table";
  import { Badge } from "$lib/components/ui/badge";
  import type { ShieldProfile } from "./types";

  const store = useStore(shieldBrowserStore);

  // ── Local UI state ──────────────────────────────────────────
  let searchFilter = $state("");
  let renamingId = $state<string | null>(null);
  let renameValue = $state("");
  let sortKey = $state<keyof ShieldProfile>("name");
  let sortAsc = $state(true);

  // ── Derived filtered + sorted rows ─────────────────────────
  const filteredProfiles = $derived.by(() => {
    const q = searchFilter.toLowerCase();
    let rows = store.profiles.filter((p) =>
      !q ||
      p.name.toLowerCase().includes(q) ||
      p.engine.toLowerCase().includes(q) ||
      p.engine_version.toLowerCase().includes(q)
    );
    rows = [...rows].sort((a, b) => {
      // Coerce nulls so < / > never operate on null (avoids TS error + runtime NaN)
      const coerce = (v: unknown) => (v === null || v === undefined ? (typeof a[sortKey] === "number" ? 0 : "") : v);
      const av = coerce(a[sortKey]);
      const bv = coerce(b[sortKey]);
      const cmp = av < bv ? -1 : av > bv ? 1 : 0;
      return sortAsc ? cmp : -cmp;
    });
    return rows;
  });

  function toggleSort(key: keyof ShieldProfile) {
    if (sortKey === key) {
      sortAsc = !sortAsc;
    } else {
      sortKey = key;
      sortAsc = true;
    }
  }

  function sortIcon(key: keyof ShieldProfile) {
    if (sortKey !== key) return "⇅";
    return sortAsc ? "▲" : "▼";
  }

  async function startRename(profile: ShieldProfile) {
    renamingId = profile.id;
    renameValue = profile.name;
  }

  async function confirmRename() {
    if (!renamingId || !renameValue.trim()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("rename_shield_profile", { id: renamingId, newName: renameValue.trim() });
      renamingId = null;
      await store.refresh();
    } catch (e: unknown) {
      alert(`Rename failed: ${e}`);
    }
  }

  function clickOutside(node: Node, callback: () => void) {
    const handleClick = (e: MouseEvent) => {
      if (node && !node.contains(e.target as Node)) callback();
    };
    document.addEventListener("mousedown", handleClick);
    return { destroy() { document.removeEventListener("mousedown", handleClick); } };
  }
</script>

<div class="relative h-full flex overflow-hidden w-full">
  <!-- Main Table Layout -->
  <div class="flex flex-col flex-1 min-w-0">

    <div class="flex justify-between items-center mb-4 pb-2 border-b border-border/40">
      <div class="relative w-80">
        <span class="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground opacity-70 text-xs">🔍</span>
        <Input
          type="text"
          placeholder="Search profiles..."
          bind:value={searchFilter}
          class="pl-8 h-9 text-sm"
        />
      </div>
      <div class="text-xs text-muted-foreground font-medium uppercase tracking-wider">
        {filteredProfiles.length} of {store.profiles.length} profiles
      </div>
    </div>

    {#if store.profiles.length === 0}
      <div class="flex-1 flex flex-col items-center justify-center gap-3 text-muted-foreground border border-dashed border-border/60 rounded-xl bg-secondary/10">
        <span class="text-5xl opacity-80">🛡️</span>
        <p class="text-sm">No browser profiles created yet.</p>
      </div>
    {:else}
      <div class="flex-1 overflow-auto rounded-xl border border-border bg-card shadow-sm">
        <Table.Root>
          <Table.Header class="bg-secondary/20 sticky top-0 z-10">
            <Table.Row class="hover:bg-transparent border-border/60">
              {#each [
                { key: "name", label: "Profile" },
                { key: "engine", label: "Engine" },
                { key: "engine_version", label: "Version" },
                { key: "is_running", label: "Status" },
                { key: "has_proxy", label: "Proxy" },
                { key: "created_at", label: "Created" },
              ] as col}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                <Table.Head
                  class="h-10 px-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground cursor-pointer hover:bg-secondary/40 transition-colors select-none"
                  onclick={() => toggleSort(col.key as keyof ShieldProfile)}
                >
                  <div class="flex items-center gap-1.5">
                    {col.label}
                    <span class="text-[9px] {sortKey === col.key ? 'text-primary/70' : 'text-muted-foreground/30'}">
                      {sortIcon(col.key as keyof ShieldProfile)}
                    </span>
                  </div>
                </Table.Head>
              {/each}
              <Table.Head class="w-[110px] h-10 px-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Actions
              </Table.Head>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {#each filteredProfiles as profile (profile.id)}
              <Table.Row
                class="cursor-pointer transition-colors border-border/40 group border-l-2 {store.selectedProfile?.id === profile.id && store.drawerOpen ? 'bg-primary/5 hover:bg-primary/10 border-l-primary' : 'hover:bg-secondary/30 border-l-transparent'} {profile.is_running ? '!border-l-emerald-500' : ''}"
                onclick={() => { store.setSelectedProfile(profile); store.setDrawerOpen(true); }}
              >
                <Table.Cell class="py-2.5 px-4 text-sm font-semibold">{profile.name}</Table.Cell>
                <Table.Cell class="py-2.5 px-4 text-sm">{engineIcon(profile.engine)} {engineLabel(profile.engine)}</Table.Cell>
                <Table.Cell class="py-2.5 px-4 text-sm font-mono">v{profile.engine_version}</Table.Cell>
                <Table.Cell class="py-2.5 px-4 text-sm">
                  {profile.is_running ? "🟢 Running" : "⚪ Idle"}
                </Table.Cell>
                <Table.Cell class="py-2.5 px-4 text-sm">{profile.has_proxy ? "✅" : "—"}</Table.Cell>
                <Table.Cell class="py-2.5 px-4 text-sm text-muted-foreground">{formatDate(profile.created_at)}</Table.Cell>
                <Table.Cell class="py-1 px-4">
                  <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    {#if profile.is_running}
                      <Button variant="ghost" size="icon" class="h-7 w-7 text-xs text-emerald-500 hover:bg-emerald-500/20 border-0" title="Stop"
                        onclick={(e) => { e.stopPropagation(); store.stopProfile(profile.id); }}>⏹</Button>
                    {:else}
                      <Button variant="ghost" size="icon" class="h-7 w-7 text-xs hover:bg-primary/20 hover:text-primary border-0" title="Launch"
                        onclick={(e) => { e.stopPropagation(); store.launchProfile(profile.id); }} disabled={store.launching}>▶</Button>
                    {/if}
                    <Button variant="ghost" size="icon" class="h-7 w-7 text-xs text-destructive hover:bg-destructive/20 border-0" title="Delete"
                      onclick={(e) => { e.stopPropagation(); if (confirm(`Delete ${profile.name}?`)) store.deleteProfile(profile.id); }}>🗑</Button>
                  </div>
                </Table.Cell>
              </Table.Row>
            {/each}
            {#if filteredProfiles.length === 0}
              <Table.Row>
                <Table.Cell colspan={7} class="h-32 text-center text-muted-foreground italic">
                  No profiles match your search.
                </Table.Cell>
              </Table.Row>
            {/if}
          </Table.Body>
        </Table.Root>
      </div>
    {/if}
  </div>
</div>

<!-- Detail Drawer -->
{#if store.drawerOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="absolute inset-0 bg-background/30 backdrop-blur-sm z-40"
    transition:fly={{ duration: 200, opacity: 0 }}
    onclick={() => store.setDrawerOpen(false)}
  ></div>

  <div
    class="absolute top-0 right-0 bottom-0 w-[400px] bg-card border-l border-border/60 shadow-2xl z-50 flex flex-col"
    transition:fly={{ x: 400, duration: 350, easing: cubicOut }}
    use:clickOutside={() => store.setDrawerOpen(false)}
  >
    {#if store.selectedProfile}
      <!-- Drawer Header -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-border/40 bg-secondary/10 shrink-0">
        <div class="flex items-center gap-2 flex-1 min-w-0 pr-4">
          {#if renamingId === store.selectedProfile.id}
            <Input
              class="h-8 font-bold text-base -ml-2 w-full"
              bind:value={renameValue}
              onkeydown={(e) => { if (e.key === "Enter") confirmRename(); if (e.key === "Escape") renamingId = null; }}
              onblur={confirmRename}
            />
          {:else}
            <h3 class="m-0 text-lg font-bold truncate tracking-tight">{store.selectedProfile.name}</h3>
            <button
              class="bg-transparent border-0 p-1.5 rounded-md text-sm opacity-0 group-hover:opacity-60 hover:!opacity-100 hover:bg-secondary/80 transition-all cursor-pointer flex-shrink-0"
              onclick={() => startRename(store.selectedProfile!)}
            >✏️</button>
          {/if}
        </div>
        <Button variant="ghost" size="icon" class="h-8 w-8 text-muted-foreground" onclick={() => store.setDrawerOpen(false)}>✕</Button>
      </div>

      <!-- Drawer Body -->
      <div class="flex-1 overflow-y-auto p-6 flex flex-col gap-4">
        <div class="flex flex-col gap-0">
          {#each [
            { label: "Status", value: store.selectedProfile.is_running ? "Running" : "Idle", badge: true },
            { label: "Engine", value: `${engineIcon(store.selectedProfile.engine)} ${engineLabel(store.selectedProfile.engine)}` },
            { label: "Version", value: `v${store.selectedProfile.engine_version}`, mono: true },
            { label: "Created", value: formatDateTime(store.selectedProfile.created_at) },
            { label: "Last Launch", value: store.selectedProfile.last_launch ? formatDateTime(store.selectedProfile.last_launch) : "Never" },
            { label: "Proxy", value: store.selectedProfile.has_proxy ? "Configured" : "None" },
            { label: "Fingerprint OS", value: store.selectedProfile.fingerprint_os || "Random" },
          ] as row}
            <div class="flex justify-between items-center py-2.5 border-b border-border/40 last:border-0">
              <span class="text-xs uppercase tracking-widest font-semibold text-muted-foreground">{row.label}</span>
              {#if row.badge}
                <Badge variant={store.selectedProfile.is_running ? "default" : "secondary"}
                  class={store.selectedProfile.is_running ? "bg-emerald-500/15 text-emerald-500 hover:bg-emerald-500/20" : ""}>
                  {row.value}
                </Badge>
              {:else}
                <span class="text-sm font-medium {row.mono ? 'font-mono' : ''}">{row.value}</span>
              {/if}
            </div>
          {/each}
        </div>

        {#if store.selectedProfile.tags.length > 0}
          <div class="flex flex-wrap gap-1.5 pt-1">
            {#each store.selectedProfile.tags as tag}
              <Badge variant="outline" class="bg-primary/5 text-primary border-primary/20 text-[10px] uppercase tracking-wider">{tag}</Badge>
            {/each}
          </div>
        {/if}

        {#if store.selectedProfile.note}
          <div class="p-3 bg-amber-500/10 text-amber-600 border border-amber-500/20 rounded-lg text-sm leading-relaxed">
            <strong>Note:</strong> {store.selectedProfile.note}
          </div>
        {/if}
      </div>

      <!-- Drawer Footer -->
      <div class="p-5 border-t border-border/40 bg-secondary/10 flex items-center gap-2 shrink-0">
        {#if store.selectedProfile.is_running}
          <Button class="flex-1 bg-emerald-500 hover:bg-emerald-600 text-white font-semibold" onclick={() => store.stopProfile(store.selectedProfile!.id)}>
            ⏹ Stop Browser
          </Button>
        {:else}
          <Button class="flex-1 font-semibold" onclick={() => store.launchProfile(store.selectedProfile!.id)} disabled={store.launching}>
            {store.launching ? "Launching..." : "▶ Launch Browser"}
          </Button>
        {/if}
        <Button variant="outline" size="icon" title="Reinstall engine" onclick={() => { store.setSetupEngine(store.selectedProfile!.engine); store.setSetupVersion(store.selectedProfile!.engine_version); store.installEngine(); }} disabled={store.downloading}>⬇️</Button>
        <Button variant="ghost" size="icon" class="text-destructive hover:bg-destructive/10" onclick={() => { if (confirm(`Delete ${store.selectedProfile!.name}?`)) { store.deleteProfile(store.selectedProfile!.id); store.setDrawerOpen(false); } }}>🗑</Button>
      </div>
    {/if}
  </div>
{/if}
