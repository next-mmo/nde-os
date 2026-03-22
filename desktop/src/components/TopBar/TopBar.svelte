<svelte:options runes={true} />

<script lang="ts">
  import MenuBar from "🍎/components/TopBar/MenuBar.svelte";
  import ActionCenter from "🍎/components/TopBar/ActionCenter.svelte";
  import TopBarTime from "🍎/components/TopBar/TopBarTime.svelte";
</script>

<header class="topbar">
  <MenuBar />

  <span class="spacer"></span>

  <ActionCenter />

  <button class="topbar-button" aria-label="Current time">
    <TopBarTime />
  </button>
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    position: relative;
    width: 100%;
    height: 1.65rem;

    background-color: hsla(var(--system-color-light-hsl) / 0.3);
    color: var(--system-color-text);
    fill: var(--system-color-text);

    z-index: 9990;
  }

  .topbar::before {
    content: "";
    width: inherit;
    height: inherit;
    position: fixed;
    left: 0;
    top: 0;
    z-index: 0;
    backdrop-filter: blur(14px);
  }

  .spacer {
    flex: 1 1 auto;
  }

  .topbar-button {
    font-weight: 500;
    font-size: 0.78rem;
    font-family: var(--system-font-family);
    letter-spacing: 0.3px;
    position: relative;
    height: 100%;
    padding: 0 0.6rem;
    color: var(--system-color-text);
    white-space: nowrap;
    border-radius: 0.25rem;
  }

  .topbar-button:hover {
    background-color: hsla(var(--system-color-dark-hsl) / 0.1);
  }
</style>
