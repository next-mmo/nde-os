<svelte:options runes={true} />

<script lang="ts">
  import { unlockScreen } from "🍎/state/desktop.svelte";
  import { onMount } from "svelte";

  // Simple clock for the lock screen
  let time = $state(new Date());

  onMount(() => {
    const timer = setInterval(() => {
      time = new Date();
    }, 1000);
    return () => clearInterval(timer);
  });

  const formattedTime = $derived(
    time.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  );
  const formattedDate = $derived(
    time.toLocaleDateString([], { weekday: 'long', month: 'long', day: 'numeric' })
  );
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<main
  class="fixed inset-0 z-[99999] flex flex-col items-center justify-center select-none bg-black/40 backdrop-blur-3xl"
  onclick={unlockScreen}
  role="dialog"
  aria-modal="true"
>
  <div class="flex flex-col items-center text-white drop-shadow-lg mb-20 animate-in fade-in zoom-in duration-500">
    <!-- Lock Icon -->
    <svg class="w-10 h-10 mb-4 opacity-80" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <rect x="3" y="11" width="18" height="11" rx="2" ry="2" />
      <path d="M7 11V7a5 5 0 0 1 10 0v4" />
    </svg>

    <h1 class="text-7xl font-light tracking-tight mb-2">{formattedTime}</h1>
    <h2 class="text-2xl font-normal opacity-90">{formattedDate}</h2>
  </div>

  <div class="absolute bottom-12 flex flex-col items-center animate-pulse opacity-70">
    <span class="text-white/90 text-sm tracking-wide font-medium">Click anywhere to unlock</span>
  </div>
</main>
