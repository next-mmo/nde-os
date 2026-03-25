<script lang="ts">
  let p = $props();
  let pct = $derived(Math.min(100, Math.max(0, ((p.value ?? 0) / (p.max ?? 100)) * 100)));
  let barColor = $derived(p.variant === "success" ? "hsl(145 65% 50%)" : p.variant === "warning" ? "hsl(40 90% 55%)" : p.variant === "danger" ? "hsl(0 70% 55%)" : "hsl(215 90% 55%)");
</script>
<div style="display:flex;flex-direction:column;gap:0.3rem">
  {#if p.label}
    <div style="display:flex;justify-content:space-between;font-size:0.75rem">
      <span>{p.label}</span>
      <span style="color:var(--system-color-text-muted)">{Math.round(pct)}%</span>
    </div>
  {/if}
  <div style="height:6px;border-radius:3px;background:hsla(0 0% 50%/0.15);overflow:hidden">
    <div style="width:{pct}%;height:100%;border-radius:3px;background:{barColor};transition:width 0.3s ease"></div>
  </div>
</div>