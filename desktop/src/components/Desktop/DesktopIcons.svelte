<svelte:options runes={true} />

<script lang="ts">
  import { openStaticApp, type StaticAppID } from "🍎/state/desktop.svelte";

  const desktopShortcuts: { id: StaticAppID; label: string }[] = [
    { id: "file-explorer", label: "File Explorer" },
    { id: "ai-launcher", label: "AI Launcher" },
    { id: "shield-browser", label: "Shield Browser" },
    { id: "chat", label: "NDE Chat" },
    { id: "settings", label: "Settings" },
    { id: "code-editor", label: "Code Editor" },
  ];

  let selectedId = $state<string | null>(null);

  function handleClick(id: StaticAppID) {
    selectedId = id;
  }

  function handleDblClick(id: StaticAppID) {
    openStaticApp(id);
    selectedId = null;
  }

  function handleClickOutside() {
    selectedId = null;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="absolute inset-0 z-0 p-4 grid auto-rows-min justify-items-end content-start gap-1"
  style="grid-template-columns: repeat(auto-fill, 88px); direction: rtl;"
  onclick={handleClickOutside}
  onkeydown={undefined}
>
  {#each desktopShortcuts as shortcut (shortcut.id)}
    <button
      type="button"
      class="flex flex-col items-center gap-1 p-2 rounded-xl cursor-default select-none transition-all duration-100 bg-transparent border-none appearance-none outline-none focus-visible:ring-2 focus-visible:ring-white/50 {selectedId === shortcut.id ? 'bg-white/20 dark:bg-white/10 ring-1 ring-white/30 shadow-sm' : 'hover:bg-white/10 dark:hover:bg-white/5'}"
      style="direction: ltr;"
      onclick={(e) => { e.stopPropagation(); handleClick(shortcut.id); }}
      ondblclick={(e) => { e.stopPropagation(); handleDblClick(shortcut.id); }}
    >
      <img
        src="/app-icons/{shortcut.id}/256.webp"
        alt=""
        class="w-14 h-14 drop-shadow-lg"
        onerror={(e) => {
          const el = e.currentTarget as HTMLImageElement;
          el.style.display = "none";
          const parent = el.parentElement;
          if (parent && !parent.querySelector(".icon-fallback")) {
            const fallback = document.createElement("div");
            fallback.className = "icon-fallback w-14 h-14 rounded-[22%] bg-gradient-to-br from-blue-400/80 to-blue-600/80 grid place-items-center text-white font-bold text-xl shadow-lg border border-white/20";
            fallback.textContent = shortcut.label.slice(0, 2).toUpperCase();
            parent.insertBefore(fallback, el);
          }
        }}
      />
      <span class="text-[0.68rem] font-medium text-white text-center leading-tight line-clamp-2 w-full drop-shadow-[0_1px_2px_rgba(0,0,0,0.6)]">
        {shortcut.label}
      </span>
    </button>
  {/each}
</div>
