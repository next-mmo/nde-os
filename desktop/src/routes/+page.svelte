<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  onMount(() => { goto("/catalog"); });
</script>

<div class="loading">
  <span class="animate-spin" style="font-size: 24px;">🚀</span>
  <p style="color: var(--color-text-dim); font-size: 12px; margin-top: 8px;">Loading...</p>
</div>

<style>
  .loading { display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100vh; }
</style>
