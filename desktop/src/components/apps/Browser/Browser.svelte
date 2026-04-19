<svelte:options runes={true} />

<script lang="ts">
  import {
    currentBrowserUrl,
    getSessionById,
    openGenericBrowserWindow,
    openStaticApp,
    reloadBrowserWindow,
    sendSessionToDashboard,
    stepBrowserHistory,
    updateBrowserWindowUrl,
    type DesktopWindow,
  } from "🍎/state/desktop.svelte";

  import { useStore } from "../../../lib/use-store.svelte";
  import { browserStore } from "../../../stores/browser";
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-shell";

  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";

  interface Props {
    window: DesktopWindow;
  }

  let { window: win }: Props = $props();
  let address = $state("");
  const store = useStore(browserStore);

  const UNEMBEDDABLE_DOMAINS = [
    "chatgpt.com",
    "claude.ai",
    "gemini.google.com",
    "github.com",
    "google.com",
    "anthropic.com",
    "openai.com",
    "x.com",
    "twitter.com"
  ];

  onMount(() => {
    store.loadBookmarks();
  });

  $effect(() => {
    address = currentBrowserUrl(win) || "";
  });

  const session = $derived(getSessionById(win.session_id));
  const iframeSrc = $derived(`${currentBrowserUrl(win) || ""}${win.browser ? `#reload=${win.browser.reload_key}` : ""}`);
  
  const isCurrentBookmarked = $derived(
    store.bookmarks.some((b) => b.url === address && !b.isSpeedDial)
  );

  const parsedUrl = $derived.by(() => {
    try {
      const current = currentBrowserUrl(win) || "";
      if (!current) return null;
      return new URL(current.startsWith('http') ? current : `https://${current}`);
    } catch {
      return null;
    }
  });

  const isUnembeddable = $derived(
    parsedUrl && UNEMBEDDABLE_DOMAINS.some(d => parsedUrl.hostname === d || parsedUrl.hostname.endsWith('.' + d))
  );

  function submitAddress(event: SubmitEvent) {
    event.preventDefault();
    if (!address) return;
    let finalUrl = address;
    if (!finalUrl.startsWith('http://') && !finalUrl.startsWith('https://') && finalUrl.includes('.')) {
      finalUrl = `https://${finalUrl}`;
    }
    updateBrowserWindowUrl(win.id, finalUrl);
  }

  function toggleBookmark() {
    if (!address) return;
    if (isCurrentBookmarked) {
      const b = store.bookmarks.find((b) => b.url === address && !b.isSpeedDial);
      if (b) store.removeBookmark(b.id);
    } else {
      store.addBookmark({
        title: address, // fallback, since we can't easily read iframe title
        url: address,
        icon: "🔖",
        isSpeedDial: false,
      });
    }
  }

  function openUrl(url: string) {
    address = url;
    updateBrowserWindowUrl(win.id, url);
  }

  async function openInSystemBrowser() {
    if (iframeSrc) {
      // open is imported from @tauri-apps/plugin-shell
      await open(iframeSrc);
    }
  }

  function openInNdeShield() {
    openStaticApp("shield-browser", { returnView: "profiles" });
  }
</script>

<div class="flex flex-col w-full h-full bg-background text-foreground overflow-hidden">
  <form class="flex items-center gap-2 px-3 py-2 border-b border-border bg-muted/30 shrink-0" onsubmit={submitAddress}>
    <div class="flex items-center gap-1">
      <Button variant="outline" size="sm" class="h-8 px-2" type="button" onclick={() => stepBrowserHistory(win.id, -1)} aria-label="Back">
        ⬅️
      </Button>
      <Button variant="outline" size="sm" class="h-8 px-2" type="button" onclick={() => stepBrowserHistory(win.id, 1)} aria-label="Forward">
        ➡️
      </Button>
      <Button variant="ghost" size="sm" class="h-8 px-2" type="button" onclick={() => reloadBrowserWindow(win.id)} aria-label="Reload">
        🔄
      </Button>
    </div>

    <div class="flex-1 relative flex items-center">
      <Input 
        class="w-full h-8 bg-background pr-8" 
        bind:value={address} 
        aria-label="Browser address" 
        placeholder="Enter URL or open an app"
      />
      {#if address}
        <button 
          type="button" 
          class="absolute right-2 text-lg hover:scale-110 transition-transform"
          onclick={toggleBookmark}
          aria-label={isCurrentBookmarked ? "Remove Bookmark" : "Add Bookmark"}
        >
          {isCurrentBookmarked ? "⭐" : "☆"}
        </button>
      {/if}
    </div>

    <div class="flex items-center gap-2">
      {#if session}
        <Button variant="default" size="sm" class="h-8" type="button" onclick={() => sendSessionToDashboard(session.id)}>
          Return to Dashboard
        </Button>
      {/if}
      <Button variant="secondary" size="sm" class="h-8" type="button" onclick={() => openGenericBrowserWindow(currentBrowserUrl(win) || "", win.title)}>
        New Window
      </Button>
    </div>
  </form>

  <div class="flex-1 min-h-0 bg-background relative overflow-y-auto w-full h-full">
    {#if isUnembeddable}
      <div class="w-full h-full flex flex-col items-center justify-center p-8 bg-card shadow-inner">
        <div class="w-20 h-20 rounded-full bg-destructive/10 text-destructive flex items-center justify-center mb-6 text-4xl border border-destructive/20 shadow-sm animate-pulse">
          🛡️
        </div>
        <h2 class="text-2xl font-bold text-foreground mb-3 text-center">Protected Website</h2>
        <p class="text-sm text-muted-foreground max-w-md text-center mb-6 leading-relaxed">
          The website <strong>{parsedUrl?.hostname}</strong> actively blocks being embedded inside iframes for security reasons. 
          To securely use this site, please launch it in your native system browser or the secure NDE Shield.
        </p>
        <div class="flex items-center gap-4 mt-2">
          <Button variant="default" size="lg" onclick={openInSystemBrowser} class="gap-2 shadow-xl shadow-primary/20">
            <span class="text-xl">🌐</span> System Browser
          </Button>
          <Button variant="secondary" size="lg" onclick={openInNdeShield} class="gap-2 shadow-xl">
            <span class="text-xl">🛡️</span> NDE Shield
          </Button>
        </div>
      </div>
    {:else if currentBrowserUrl(win)}
      <!-- svelte-ignore a11y_missing_attribute -->
      <iframe 
        title={win.title} 
        src={iframeSrc}
        class="w-full h-full border-0 bg-background absolute inset-0 z-0"
        style="color-scheme: light dark;"
      ></iframe>
    {:else}
      <div class="w-full min-h-full flex flex-col p-8 max-w-4xl mx-auto gap-8">
        <div class="flex flex-col items-center justify-center text-muted-foreground mt-8 text-center">
          <div class="w-16 h-16 rounded-full bg-muted flex items-center justify-center mb-4 text-3xl border border-border shadow-sm">
            🌍
          </div>
          <h2 class="text-xl font-semibold text-foreground mb-2">NDE Browser</h2>
          <p class="text-sm max-w-sm mb-6">Enter a URL above to browse, or pick a speed dial below.</p>
        </div>

        <div>
          <h3 class="text-sm font-medium text-foreground tracking-wide uppercase mb-4 flex items-center gap-2">
            <span class="text-lg">⌨️</span> Speed Dial
          </h3>
          <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-4">
            {#each store.bookmarks.filter(b => b.isSpeedDial) as dial (dial.id)}
              <button 
                type="button"
                class="flex flex-col items-center gap-3 p-4 rounded-xl border border-border bg-card hover:bg-muted/50 hover:border-foreground/20 transition-all group shadow-sm hover:shadow"
                onclick={() => openUrl(dial.url)}
              >
                <div class="w-12 h-12 rounded-full bg-secondary text-2xl flex items-center justify-center group-hover:scale-110 transition-transform shadow-inner border border-border/50">
                  {dial.icon || "🌐"}
                </div>
                <div class="text-sm font-medium text-foreground truncate w-full text-center">
                  {dial.title}
                </div>
              </button>
            {/each}
          </div>
        </div>

        {#if store.bookmarks.some(b => !b.isSpeedDial)}
          <div class="mt-4">
            <h3 class="text-sm font-medium text-foreground tracking-wide uppercase mb-4 flex items-center gap-2">
              <span class="text-lg">⭐</span> Bookmarks
            </h3>
            <div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-3">
              {#each store.bookmarks.filter(b => !b.isSpeedDial) as bm (bm.id)}
                <div class="flex items-center justify-between p-3 rounded-lg border border-border bg-card/50 hover:bg-card transition-colors group">
                  <button 
                    type="button"
                    class="flex items-center gap-3 flex-1 overflow-hidden" 
                    onclick={() => openUrl(bm.url)}
                  >
                    <div class="w-8 h-8 rounded-full bg-secondary/80 text-sm flex items-center justify-center border border-border/50 shrink-0">
                      {bm.icon || "🔖"}
                    </div>
                    <div class="flex flex-col items-start min-w-0 flex-1">
                      <span class="text-sm font-medium text-foreground truncate w-full text-left">{bm.title}</span>
                      <span class="text-xs text-muted-foreground truncate w-full text-left">{bm.url}</span>
                    </div>
                  </button>
                  <button 
                    type="button"
                    class="ml-2 w-8 h-8 flex items-center justify-center rounded hover:bg-destructive/10 hover:text-destructive text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity"
                    onclick={() => store.removeBookmark(bm.id)}
                    title="Remove bookmark"
                  >
                    ✕
                  </button>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>
