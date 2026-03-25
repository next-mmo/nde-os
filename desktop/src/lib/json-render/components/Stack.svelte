<script lang="ts">
  let p = $props();
  const gapMap: Record<string,string> = { none: "0", xs: "0.25rem", sm: "0.5rem", md: "0.75rem", lg: "1.25rem", xl: "2rem" };
  const alignMap: Record<string,string> = { start: "flex-start", center: "center", end: "flex-end", stretch: "stretch" };
  const justMap: Record<string,string> = { start: "flex-start", center: "center", end: "flex-end", between: "space-between", around: "space-around" };
</script>
<div style="display:flex;flex-direction:{p.direction === 'horizontal' ? 'row' : 'column'};gap:{gapMap[p.gap ?? 'md'] ?? '0.75rem'};align-items:{alignMap[p.align ?? 'start']};justify-content:{justMap[p.justify ?? 'start']}{p.wrap ? ';flex-wrap:wrap' : ''}">
  {@render p.children?.()}
</div>