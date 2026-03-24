<svelte:options runes={true} />

<script lang="ts">
  import MenuBar from "🍎/components/TopBar/MenuBar.svelte";
  import ActionCenter from "🍎/components/TopBar/ActionCenter.svelte";
  import TopBarMetrics from "./TopBarMetrics.svelte";
  import TopBarTime from "🍎/components/TopBar/TopBarTime.svelte";
</script>

<header class="flex items-center relative w-full h-[1.65rem] bg-white/30 dark:bg-black/30 text-black dark:text-white fill-current z-[9990] backdrop-blur-md before:content-[''] before:fixed before:inset-x-0 before:top-0 before:h-inherit before:-z-10 before:backdrop-blur-md">
  <MenuBar />

  <span class="flex-1"></span>

  <ActionCenter />
  <TopBarMetrics />

  <button class="font-medium text-[0.78rem] tracking-wide relative h-full px-2.5 text-black dark:text-white whitespace-nowrap rounded hover:bg-black/10 dark:hover:bg-white/10" aria-label="Current time">
    <TopBarTime />
  </button>
</header>
