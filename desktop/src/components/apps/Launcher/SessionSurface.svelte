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

<div class="w-full h-full rounded-[1.2rem] overflow-hidden bg-[hsl(220,28%,12%)] border border-black/10 dark:border-white/10 shadow-sm">
  <iframe class="w-full h-full border-0 bg-white" title={session.title} src={iframeSrc} onload={handleLoad}></iframe>
</div>
