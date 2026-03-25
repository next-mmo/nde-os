<script lang="ts">
  let p = $props();
  const sizeMap: Record<string, string> = { sm: "16px", md: "24px", lg: "36px" };
  let s = $derived(sizeMap[(p.size as string) ?? "md"] ?? "24px");
</script>
<div style="display:flex;align-items:center;gap:0.5rem">
  <div style="width:{s};height:{s};border:2px solid var(--system-color-border);border-top-color:hsl(215 90% 55%);border-radius:50%;animation:spin 0.8s linear infinite"></div>
  {#if p.label}
    <span style="font-size:0.82rem;color:var(--system-color-text-muted)">{p.label}</span>
  {/if}
</div>
<style>
  @keyframes spin { to { transform: rotate(360deg); } }
</style>