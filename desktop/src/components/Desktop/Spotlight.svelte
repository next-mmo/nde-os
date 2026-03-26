<script lang="ts">
  import { onMount } from "svelte";
  import { fade, fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { apps_config, type StaticAppID } from "🍎/configs/apps/apps-config";
  import { desktop, openStaticApp, openSessionInWindow, toggleSpotlight } from "🍎/state/desktop.svelte";

  let searchInput = $state("");
  let inputElement: HTMLInputElement | null = null;
  let selectedIndex = $state(0);

  $effect(() => {
    if (desktop.spotlight_open && inputElement) {
      inputElement.focus();
    }
    // reset selection on input change
    searchInput;
    selectedIndex = 0;
  });

  interface SearchResult {
    id: string;
    title: string;
    type: "app" | "session" | "command";
    icon: string;
  }

  const results = $derived.by(() => {
    const q = searchInput.toLowerCase();
    const res: SearchResult[] = [];

    // Static Apps
    for (const [id, config] of Object.entries(apps_config)) {
      if (id !== "launchpad" && config.title.toLowerCase().includes(q)) {
        res.push({
          id,
          title: config.title,
          type: "app",
          icon: "📱",
        });
      }
    }

    // Running Sessions
    for (const sess of desktop.sessions) {
      if (sess.title.toLowerCase().includes(q)) {
        res.push({
          id: sess.id,
          title: sess.title,
          type: "session",
          icon: "⚡",
        });
      }
    }

    // If query starts with >, treat as terminal/command
    if (q.startsWith(">") && q.trim().length > 1) {
      res.push({
        id: "cmd_" + q,
        title: `Run command: ${q.slice(1).trim()}`,
        type: "command",
        icon: "💻",
      });
    }

    return res;
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      toggleSpotlight(false);
      e.stopPropagation();
      e.preventDefault();
      return;
    }

    if (results.length === 0) return;

    if (e.key === "ArrowDown") {
      selectedIndex = (selectedIndex + 1) % results.length;
      e.preventDefault();
      e.stopPropagation();
    } else if (e.key === "ArrowUp") {
      selectedIndex = (selectedIndex - 1 + results.length) % results.length;
      e.preventDefault();
      e.stopPropagation();
    } else if (e.key === "Enter") {
      executeSelected();
      e.preventDefault();
      e.stopPropagation();
    }
  }

  async function executeSelected() {
    if (results.length === 0) return;
    const item = results[selectedIndex];
    
    if (item.type === "app") {
      openStaticApp(item.id as StaticAppID);
    } else if (item.type === "session") {
      openSessionInWindow(item.id);
    } else if (item.type === "command") {
      openStaticApp("terminal");
      // Actually running terminal commands would require IPC to rust.
      // E.g., spawn process. We'll simply open terminal.
    }
    
    toggleSpotlight(false);
    searchInput = "";
  }
</script>

<div 
  class="fixed inset-0 z-100000 flex items-start justify-center pt-[15vh]"
  transition:fade={{ duration: 150 }}
>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="absolute inset-0 bg-black/20 backdrop-blur-[2px]" onclick={() => toggleSpotlight(false)}></div>

  <div 
    class="relative w-full max-w-2xl bg-white/70 dark:bg-black/60 backdrop-blur-2xl border border-black/10 dark:border-white/10 rounded-2xl shadow-2xl shadow-black/30 overflow-hidden flex flex-col"
    transition:fly={{ y: -20, duration: 250, easing: cubicOut }}
  >
    <div class="flex items-center px-6 py-4 border-b border-black/10 dark:border-white/10">
      <span class="text-2xl mr-4 opacity-50">🔍</span>
      <input
        bind:this={inputElement}
        bind:value={searchInput}
        onkeydown={handleKeydown}
        placeholder="Spotlight Search"
        class="w-full bg-transparent border-none outline-none text-2xl font-light text-black dark:text-white placeholder:text-black/30 dark:placeholder:text-white/30"
        autocomplete="off"
        spellcheck="false"
      />
    </div>

    {#if searchInput.length > 0 && results.length > 0}
      <div class="max-h-[60vh] overflow-y-auto py-2">
        {#each results as result, i}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div 
            class="flex items-center px-6 py-3 cursor-pointer select-none transition-colors {i === selectedIndex ? 'bg-blue-500 text-white' : 'hover:bg-black/5 dark:hover:bg-white/10'}"
            onclick={() => { selectedIndex = i; executeSelected(); }}
          >
            <span class="text-xl mr-4">{result.icon}</span>
            <span class="text-lg font-medium">{result.title}</span>
            <span class="ml-auto text-sm opacity-50">{result.type}</span>
          </div>
        {/each}
      </div>
    {:else if searchInput.length > 0 && results.length === 0}
      <div class="px-6 py-8 text-center opacity-50 text-lg">
        No results found
      </div>
    {/if}
  </div>
</div>
