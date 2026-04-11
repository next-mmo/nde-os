<svelte:options runes={true} />

<script lang="ts">
  import { useStore } from "../../../lib/use-store.svelte";
  import { shieldBrowserStore } from "../../../state/shield";
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Switch } from "$lib/components/ui/switch";
  import { Badge } from "$lib/components/ui/badge";
  import { ScrollArea } from "$lib/components/ui/scroll-area";
  import type { ShieldExtension } from "./types";

  const store = useStore(shieldBrowserStore);

  onMount(() => {
    store.loadExtensions();
  });

  // ─── Developer Mode: Upload Unpacked Directory ──────────────
  async function handleLoadUnpacked() {
    const selected = await open({ directory: true, title: "Select unpacked extension folder" });
    if (selected) {
      await store.installExtensionFromDir(selected as string);
    }
  }

  // ─── Upload Packed File (CRX / XPI / ZIP) ──────────────────
  async function handleLoadPacked() {
    const selected = await open({
      title: "Select extension file",
      filters: [{ name: "Browser Extension", extensions: ["crx", "xpi", "zip"] }],
    });
    if (selected) {
      await store.installExtensionFromFile(selected as string);
    }
  }

  function formatEngine(format: string): string {
    return format === "chromium" ? "Wayfern" : format === "firefox" ? "Camoufox" : format;
  }

  function formatSource(source: string): string {
    return source === "developer" ? "Developer" : "Store";
  }

  function formatDate(epoch: number): string {
    return new Date(epoch * 1000).toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  }

  function confirmUninstall(ext: ShieldExtension) {
    if (confirm(`Uninstall "${ext.name}"? This removes it from all profiles.`)) {
      store.uninstallExtension(ext.id);
    }
  }
</script>

<div class="flex flex-col h-full gap-4 overflow-hidden">
  <!-- ═══ Header Bar ═══ -->
  <div class="flex items-center justify-between gap-4 shrink-0">
    <div class="flex flex-col">
      <h3 class="text-base font-semibold tracking-tight">Extensions</h3>
      <p class="text-xs text-muted-foreground">
        Manage browser extensions for Wayfern and Camoufox profiles
      </p>
    </div>

    <div class="flex items-center gap-2">
      <div class="flex items-center gap-2 px-3 py-1.5 rounded-md border border-border bg-background">
        <span class="text-xs font-medium text-muted-foreground">Developer Mode</span>
        <Switch
          checked={store.extensionDevMode}
          onCheckedChange={(v) => store.setExtensionDevMode(!!v)}
        />
      </div>
    </div>
  </div>

  <!-- ═══ Install Section ═══ -->
  <div class="flex items-center gap-2 shrink-0">
    {#if store.extensionDevMode}
      <Button
        variant="outline"
        size="sm"
        class="gap-1.5"
        disabled={store.extensionActionBusy}
        onclick={handleLoadUnpacked}
      >
        <span class="text-sm">+</span> Load Unpacked
      </Button>
    {/if}
    <Button
      variant="outline"
      size="sm"
      class="gap-1.5"
      disabled={store.extensionActionBusy}
      onclick={handleLoadPacked}
    >
      <span class="text-sm">+</span> Install from File
    </Button>

    <div class="flex-1"></div>

    <form
      class="flex items-center gap-2"
      onsubmit={(e) => { e.preventDefault(); store.installExtensionFromUrl(); }}
    >
      <Input
        type="url"
        placeholder="Extension URL (.crx / .xpi)"
        class="h-8 w-64 text-xs"
        value={store.extensionInstallUrl}
        oninput={(e) => store.setExtensionInstallUrl((e.target as HTMLInputElement).value)}
      />
      <Button
        variant="secondary"
        size="sm"
        type="submit"
        disabled={!store.extensionInstallUrl.trim() || store.extensionActionBusy}
      >
        Install
      </Button>
    </form>
  </div>

  <!-- ═══ Extension List ═══ -->
  <ScrollArea class="flex-1 min-h-0 -mx-1 px-1">
    {#if store.extensionsLoading}
      <div class="flex items-center justify-center py-12">
        <div class="w-6 h-6 border-2 border-white/20 border-t-white rounded-full animate-spin"></div>
      </div>
    {:else if store.extensionsError}
      <div class="flex items-center justify-center py-12 text-sm text-destructive">
        {store.extensionsError}
      </div>
    {:else if store.extensions.length === 0}
      <div class="flex flex-col items-center justify-center py-16 gap-3">
        <div class="text-4xl opacity-30">&#x1f9e9;</div>
        <p class="text-sm text-muted-foreground text-center max-w-xs">
          No extensions installed yet. Upload a CRX/XPI file, load an unpacked directory in developer mode, or paste a download URL.
        </p>
      </div>
    {:else}
      <div class="grid gap-3">
        {#each store.extensions as ext (ext.id)}
          <div class="group flex items-start gap-4 p-4 rounded-lg border border-border bg-card hover:border-border/80 transition-colors">
            <!-- Icon -->
            <div class="flex items-center justify-center w-10 h-10 rounded-lg bg-secondary border border-border/50 text-lg shrink-0 shadow-inner">
              {#if ext.format === "chromium"}
                &#x1f310;
              {:else}
                &#x1f98a;
              {/if}
            </div>

            <!-- Info -->
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2">
                <span class="text-sm font-medium truncate">{ext.name}</span>
                <Badge variant="secondary" class="text-[10px] px-1.5 py-0 h-4">
                  v{ext.version}
                </Badge>
                <Badge variant="outline" class="text-[10px] px-1.5 py-0 h-4">
                  {formatEngine(ext.format)}
                </Badge>
                {#if ext.source === "developer"}
                  <Badge variant="outline" class="text-[10px] px-1.5 py-0 h-4 border-amber-500/50 text-amber-500">
                    Dev
                  </Badge>
                {/if}
              </div>
              <p class="text-xs text-muted-foreground mt-0.5 line-clamp-1">
                {ext.description || "No description"}
              </p>
              <div class="flex items-center gap-3 mt-1.5 text-[10px] text-muted-foreground/70">
                <span>by {ext.author}</span>
                <span>Installed {formatDate(ext.installed_at)}</span>
                {#if ext.permissions.length > 0}
                  <span>{ext.permissions.length} permission{ext.permissions.length !== 1 ? "s" : ""}</span>
                {/if}
              </div>
            </div>

            <!-- Actions -->
            <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
              {#if store.extensionDevMode && ext.permissions.length > 0}
                <Button
                  variant="ghost"
                  size="sm"
                  class="h-7 text-xs"
                  title={ext.permissions.join(", ")}
                >
                  Permissions
                </Button>
              {/if}
              <Button
                variant="ghost"
                size="sm"
                class="h-7 text-xs text-destructive hover:text-destructive"
                disabled={store.extensionActionBusy}
                onclick={() => confirmUninstall(ext)}
              >
                Uninstall
              </Button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </ScrollArea>

  <!-- ═══ Footer Stats ═══ -->
  {#if store.extensions.length > 0}
    <div class="flex items-center gap-4 text-[11px] text-muted-foreground border-t border-border pt-3 shrink-0">
      <span>{store.extensions.length} extension{store.extensions.length !== 1 ? "s" : ""} installed</span>
      <span>
        {store.extensions.filter((e) => e.format === "chromium").length} Chromium
        /
        {store.extensions.filter((e) => e.format === "firefox").length} Firefox
      </span>
      {#if store.extensions.some((e) => e.source === "developer")}
        <span class="text-amber-500/70">
          {store.extensions.filter((e) => e.source === "developer").length} sideloaded
        </span>
      {/if}
    </div>
  {/if}
</div>
