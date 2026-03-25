<script lang="ts">
  let p = $props();
  const appStatusColors: Record<string,string> = {
    available: "var(--system-color-text-muted)",
    installed: "hsl(40 90% 55%)",
    running: "hsl(145 65% 55%)",
    error: "hsl(0 70% 55%)",
  };
  let sc = $derived(appStatusColors[p.status] ?? "gray");
</script>
<div style="display:flex;align-items:center;gap:0.75rem;padding:0.75rem;border-radius:0.85rem;border:1px solid var(--system-color-border);background:var(--system-color-panel);cursor:pointer;transition:border-color 0.15s">
  <span style="font-size:1.5rem">{p.icon ?? "📦"}</span>
  <div style="flex:1;min-width:0">
    <strong style="font-size:0.85rem;display:block">{p.name}</strong>
    <span style="font-size:0.72rem;color:var(--system-color-text-muted)">{p.description}</span>
  </div>
  <span style="font-size:0.7rem;font-weight:500;color:{sc};text-transform:uppercase">{p.status}</span>
</div>