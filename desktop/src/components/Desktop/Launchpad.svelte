<svelte:options runes={true} />

<script lang="ts">
  import { click_outside } from "🍎/actions";
  import { apps_config } from "🍎/configs/apps/apps-config";
  import { revealOrLaunchManifest } from "$lib/session-actions";
  import { catalog, installedMap } from "$lib/stores/state";
  import { desktop, openStaticApp, selectLauncherSection, selectManifest, toggleLaunchpad } from "🍎/state/desktop.svelte";
  import { fade } from "svelte/transition";

  let panelEl = $state<HTMLElement>();
  let filter = $state("");

  const systemApps = (Object.entries(apps_config) as [
    keyof typeof apps_config,
    (typeof apps_config)[keyof typeof apps_config],
  ][]).filter(([app_id]) => app_id !== "ai-launcher");
  const filteredCatalog = $derived(
    $catalog.filter((app) => {
      if (!filter) return true;
      const query = filter.toLowerCase();
      return (
        app.name.toLowerCase().includes(query) ||
        app.id.toLowerCase().includes(query) ||
        app.tags.some((tag) => tag.toLowerCase().includes(query))
      );
    }),
  );

  function openShellApp(app_id: keyof typeof apps_config) {
    openStaticApp(app_id);
    toggleLaunchpad(false);
  }

  async function openManifest(appId: string, target: "embedded" | "windowed") {
    const manifest = $catalog.find((item) => item.id === appId);
    if (!manifest) return;
    const installed = $installedMap[appId] ?? null;
    if (!installed) {
      selectLauncherSection("catalog");
      selectManifest(appId);
      openStaticApp("ai-launcher");
      toggleLaunchpad(false);
      return;
    }
    await revealOrLaunchManifest(manifest, installed, target);
    toggleLaunchpad(false);
  }
</script>

<div class="absolute inset-0 z-[100] backdrop-blur-[26px] bg-linear-to-b from-[#070b16]/35 to-[#070b16]/20 pt-[4.8rem] pb-[6.5rem] px-4 md:px-16" transition:fade={{duration:200}}>
  <section
    data-testid="launchpad"
    class="w-full max-w-[1240px] h-full mx-auto rounded-[2rem] bg-white/35 dark:bg-black/35 border border-white/35 dark:border-white/10 shadow-[0_32px_96px_rgba(10,14,33,0.28)] flex flex-col gap-6 p-4 md:p-8"
    bind:this={panelEl}
    use:click_outside={() => toggleLaunchpad(false)}
    aria-label="Launchpad"
  >
    <header class="flex flex-col md:flex-row justify-between gap-4 items-start">
      <div>
        <p class="uppercase tracking-[0.14em] text-[0.72rem] font-bold m-0 text-gray-600 dark:text-gray-400">Launchpad</p>
        <h2 class="mt-[0.3rem] mb-0 max-w-[34rem] text-[1.9rem] leading-[1.05] text-black dark:text-white font-semibold">Open apps from the dashboard or a separate window.</h2>
      </div>
      <input class="w-full md:w-[18rem] rounded-full border border-black/10 dark:border-white/10 bg-white/75 dark:bg-gray-800/75 px-[1.2rem] py-[0.95rem] text-black dark:text-white outline-none focus:ring-2 focus:ring-blue-500" bind:value={filter} placeholder="Search apps" aria-label="Search apps" />
    </header>

    <div class="flex flex-col gap-[0.85rem]">
      <p class="uppercase tracking-[0.14em] text-[0.72rem] font-bold m-0 text-gray-600 dark:text-gray-400">System</p>
      <div class="grid grid-cols-[repeat(auto-fill,minmax(7.8rem,1fr))] gap-4">
        <button class="grid gap-[0.7rem] justify-items-center px-3 py-4 rounded-[1.4rem] bg-white/50 dark:bg-black/50 border border-black/10 dark:border-white/10 hover:bg-white/70 dark:hover:bg-black/70 transition-colors" onclick={() => openShellApp("ai-launcher")}>
          <img class="w-[4.6rem] h-[4.6rem]" src="/app-icons/ai-launcher/256.webp" alt="" />
          <span class="text-[0.88rem] text-center text-black dark:text-white font-medium">AI Launcher</span>
        </button>
        {#each systemApps as [app_id, app_config]}
          <button class="grid gap-[0.7rem] justify-items-center px-3 py-4 rounded-[1.4rem] bg-white/50 dark:bg-black/50 border border-black/10 dark:border-white/10 hover:bg-white/70 dark:hover:bg-black/70 transition-colors group" onclick={() => openShellApp(app_id)}>
            <img class="w-[4.6rem] h-[4.6rem]" src="/app-icons/{app_id}/256.webp" alt=""
              onerror={(e) => { const t = e.currentTarget; if (t instanceof HTMLImageElement) { t.style.display = 'none'; t.nextElementSibling?.classList.replace('hidden', 'grid'); } }}
            />
            <div class="hidden w-[4.6rem] h-[4.6rem] rounded-[22%] bg-linear-to-br from-blue-500/75 to-blue-700/75 place-items-center text-white font-bold text-[1.4rem] shadow-[0_4px_16px_rgba(0,0,0,0.2)] border border-white/20">{(app_config.title || String(app_id)).slice(0, 2).toUpperCase()}</div>
            <span class="text-[0.88rem] text-center text-black dark:text-white font-medium">{app_config.title}</span>
          </button>
        {/each}
      </div>
    </div>

    <div class="flex flex-col gap-[0.85rem]">
      <div class="flex justify-between items-center text-gray-600 dark:text-gray-400 text-[0.85rem]">
        <p class="uppercase tracking-[0.14em] text-[0.72rem] font-bold m-0">AI Apps</p>
        <span>{filteredCatalog.length} app(s)</span>
      </div>
      <div class="grid grid-cols-[repeat(auto-fill,minmax(17rem,1fr))] gap-4 overflow-y-auto pr-1">
        {#each filteredCatalog as app (app.id)}
          <article class="grid gap-[0.8rem] p-4 rounded-[1.2rem] bg-white/60 dark:bg-black/60 border border-black/10 dark:border-white/10 hover:bg-white/80 dark:hover:bg-black/80 transition-colors">
            <button class="grid grid-cols-[auto_1fr] gap-[0.9rem] items-start text-left" onclick={() => openManifest(app.id, desktop.default_session_mode)}>
              <div class="w-[3rem] h-[3rem] rounded-[0.95rem] bg-linear-to-b from-blue-500/20 to-blue-700/10 grid place-items-center font-bold text-blue-500">{app.icon ?? "AI"}</div>
              <div>
                <strong class="block mb-[0.3rem] text-black dark:text-white">{app.name}</strong>
                <p class="m-0 text-gray-600 dark:text-gray-400 text-[0.84rem]">{app.description}</p>
              </div>
            </button>
            <div class="flex gap-[0.55rem]">
              <button class="flex-1 rounded-full px-[0.8rem] py-[0.6rem] bg-white/80 dark:bg-gray-800/80 border border-black/10 dark:border-white/10 text-[0.8rem] hover:bg-white dark:hover:bg-gray-700 transition-colors text-black dark:text-white font-medium" onclick={() => openManifest(app.id, "embedded")}>Open in Dashboard</button>
              <button class="flex-1 rounded-full px-[0.8rem] py-[0.6rem] bg-white/80 dark:bg-gray-800/80 border border-black/10 dark:border-white/10 text-[0.8rem] hover:bg-white dark:hover:bg-gray-700 transition-colors text-black dark:text-white font-medium" onclick={() => openManifest(app.id, "windowed")}>Open in Window</button>
            </div>
          </article>
        {/each}
      </div>
    </div>
  </section>
</div>
