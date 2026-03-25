<script lang="ts">
  let p = $props();
  const gapMap: Record<string,string> = { none: "0", xs: "0.25rem", sm: "0.5rem", md: "0.75rem", lg: "1.25rem", xl: "2rem" };
</script>
<div style="display:grid;grid-template-columns:repeat({p.columns ?? 2},1fr);gap:{gapMap[p.gap ?? 'md'] ?? '0.75rem'}">
  {@render p.children?.()}
</div>