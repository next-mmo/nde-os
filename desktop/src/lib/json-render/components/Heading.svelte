<script lang="ts">
  let p = $props();
  const headingSize: Record<string,string> = { "1": "1.5rem", "2": "1.2rem", "3": "1rem", "4": "0.88rem" };
  let level = $derived(p.level ?? "2");
  let tag = $derived(`h${level}`);
</script>
<svelte:element this={tag} style="margin:0;font-size:{headingSize[level]};font-weight:600{p.muted ? ';color:var(--system-color-text-muted)' : ''}">
  {p.text}
</svelte:element>