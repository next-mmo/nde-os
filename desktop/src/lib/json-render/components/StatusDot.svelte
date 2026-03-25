<script lang="ts">
  let p = $props();
  const statusColorMap: Record<string, string> = { online: "hsl(145 65% 50%)", offline: "hsl(0 0% 55%)", warning: "hsl(40 90% 55%)", busy: "hsl(0 70% 55%)" };
  let c = $derived(statusColorMap[(p.status as string) || "offline"] ?? "gray");
</script>
<div style="display:inline-flex;align-items:center;gap:0.35rem">
  <span style="width:8px;height:8px;border-radius:50%;background:{c}"></span>
  {#if p.label}
    <span style="font-size:0.78rem">{p.label}</span>
  {/if}
</div>