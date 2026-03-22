<svelte:options runes={true} />

<script lang="ts">
  import {
    currentBrowserUrl,
    getSessionById,
    openGenericBrowserWindow,
    reloadBrowserWindow,
    sendSessionToDashboard,
    stepBrowserHistory,
    updateBrowserWindowUrl,
    type DesktopWindow,
  } from "🍎/state/desktop.svelte";

  interface Props {
    window: DesktopWindow;
  }

  let { window }: Props = $props();
  let address = $state("");

  $effect(() => {
    address = currentBrowserUrl(window);
  });

  const session = $derived(getSessionById(window.session_id));
  const iframeSrc = $derived(`${currentBrowserUrl(window)}${window.browser ? `#reload=${window.browser.reload_key}` : ""}`);

  function submitAddress(event: SubmitEvent) {
    event.preventDefault();
    if (!address) return;
    updateBrowserWindowUrl(window.id, address);
  }
</script>

<section class="browser-app">
  <form class="toolbar" onsubmit={submitAddress}>
    <div class="nav-buttons">
      <button type="button" onclick={() => stepBrowserHistory(window.id, -1)} aria-label="Back">Back</button>
      <button type="button" onclick={() => stepBrowserHistory(window.id, 1)} aria-label="Forward">Next</button>
      <button type="button" onclick={() => reloadBrowserWindow(window.id)} aria-label="Reload">Reload</button>
    </div>

    <input bind:value={address} aria-label="Browser address" />

    <div class="toolbar-actions">
      {#if session}
        <button type="button" onclick={() => sendSessionToDashboard(session.id)}>Return to Dashboard</button>
      {/if}
      <button type="button" onclick={() => openGenericBrowserWindow(currentBrowserUrl(window), window.title)}>
        New Window
      </button>
    </div>
  </form>

  <div class="surface">
    {#if currentBrowserUrl(window)}
      <iframe title={window.title} src={iframeSrc}></iframe>
    {:else}
      <div class="empty">
        <h2>Open a local app URL</h2>
        <p>Paste a localhost URL or open a running app from the dashboard.</p>
      </div>
    {/if}
  </div>
</section>

<style>
  .browser-app {
    height: 100%;
    display: grid;
    grid-template-rows: auto 1fr;
    min-height: 0;
  }

  .toolbar {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 0.8rem;
    align-items: center;
    padding: 0.85rem 1rem;
    border-bottom: 1px solid var(--system-color-border);
    background: var(--system-color-panel);
  }

  .nav-buttons,
  .toolbar-actions {
    display: flex;
    gap: 0.45rem;
  }

  .toolbar button {
    border-radius: 999px;
    padding: 0.55rem 0.8rem;
    border: 1px solid var(--system-color-border);
    background: hsla(var(--system-color-light-hsl) / 0.72);
    font-size: 0.8rem;
    cursor: pointer;
    transition: background 0.15s ease, transform 0.1s ease;
  }

  .toolbar button:hover {
    background: hsla(var(--system-color-light-hsl) / 0.92);
  }

  .toolbar button:active {
    transform: scale(0.96);
  }

  input {
    width: 100%;
    border-radius: 999px;
    border: 1px solid var(--system-color-border);
    padding: 0.75rem 1rem;
    background: hsla(var(--system-color-light-hsl) / 0.92);
    color: var(--system-color-text);
  }

  .surface {
    min-height: 0;
    background: hsl(220 28% 12%);
  }

  iframe {
    width: 100%;
    height: 100%;
    border: 0;
    background: white;
  }

  .empty {
    height: 100%;
    display: grid;
    place-items: center;
    text-align: center;
    color: hsl(210 24% 88%);
  }

  .empty p {
    color: hsla(210 24% 88% / 0.68);
  }
</style>
