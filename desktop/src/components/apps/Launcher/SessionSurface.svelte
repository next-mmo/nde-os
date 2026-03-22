<svelte:options runes={true} />

<script lang="ts">
  import { openSessionInWindow, markSessionLoadState, type RunningSession } from "🍎/state/desktop.svelte";

  interface Props {
    session: RunningSession;
  }

  let { session }: Props = $props();
  let loaded = $state(false);

  const iframeSrc = $derived(`${session.url}#mode=${session.mode}`);

  $effect(() => {
    loaded = false;
    markSessionLoadState(session.id, "loading");
    const timeout = window.setTimeout(() => {
      if (!loaded && session.mode === "embedded") {
        markSessionLoadState(session.id, "fallback");
        openSessionInWindow(session.id);
      }
    }, 3500);

    return () => window.clearTimeout(timeout);
  });

  function handleLoad() {
    loaded = true;
    markSessionLoadState(session.id, "ready");
  }
</script>

<div class="surface">
  <iframe title={session.title} src={iframeSrc} onload={handleLoad}></iframe>
</div>

<style>
  .surface {
    width: 100%;
    height: 100%;
    border-radius: 1.2rem;
    overflow: hidden;
    background: hsl(220 28% 12%);
    border: 1px solid var(--system-color-border);
  }

  iframe {
    width: 100%;
    height: 100%;
    border: 0;
    background: white;
  }
</style>
