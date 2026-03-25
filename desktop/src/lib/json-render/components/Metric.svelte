<script lang="ts">
  let p = $props();
  const trendColor: Record<string,string> = { up: "hsl(145 65% 55%)", down: "hsl(0 70% 60%)", neutral: "var(--system-color-text-muted)" };
  let tc = $derived(trendColor[p.trend ?? "neutral"] ?? trendColor.neutral);
</script>
<div style="display:flex;flex-direction:column;gap:0.15rem">
  <span style="font-size:0.72rem;color:var(--system-color-text-muted);text-transform:uppercase;letter-spacing:0.08em">{p.label}</span>
  <div style="display:flex;align-items:baseline;gap:0.4rem">
    <strong style="font-size:1.4rem">{p.value ?? ""}</strong>
    {#if p.change}
      <span style="font-size:0.75rem;color:{tc}">{p.change}</span>
    {/if}
  </div>
</div>