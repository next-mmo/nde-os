<script lang="ts">
  let p = $props();
  const alertColors: Record<string, any> = {
    info:    { bg: "hsla(210 80% 55%/0.08)", border: "hsla(210 80% 55%/0.2)", text: "hsl(210 80% 65%)" },
    success: { bg: "hsla(145 65% 50%/0.08)", border: "hsla(145 65% 50%/0.2)", text: "hsl(145 65% 55%)" },
    warning: { bg: "hsla(40 90% 55%/0.08)",  border: "hsla(40 90% 55%/0.2)",  text: "hsl(40 90% 60%)" },
    error:   { bg: "hsla(0 70% 55%/0.08)",   border: "hsla(0 70% 55%/0.2)",   text: "hsl(0 70% 60%)" },
  };
  let c = $derived(alertColors[p.variant ?? "info"] ?? alertColors.info);
</script>
<div style="padding:0.75rem 1rem;border-radius:0.75rem;background:{c.bg};border:1px solid {c.border};color:{c.text};font-size:0.82rem">
  {#if p.title}
    <strong style="display:block;margin-bottom:0.2rem;font-size:0.85rem">{p.title}</strong>
  {/if}
  <span>{p.message}</span>
</div>