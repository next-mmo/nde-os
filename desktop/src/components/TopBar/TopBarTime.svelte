<svelte:options runes={true} />

<script lang="ts">
  import { onMount } from "svelte";

  let now = $state(new Date());

  onMount(() => {
    const interval = window.setInterval(() => {
      now = new Date();
    }, 1000);

    return () => window.clearInterval(interval);
  });

  const timeLabel = $derived(
    now.toLocaleString("en-US", {
      weekday: "short",
      hour: "2-digit",
      minute: "2-digit",
    }),
  );
</script>

<span>{timeLabel}</span>
