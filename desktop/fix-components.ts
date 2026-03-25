import * as fs from 'fs';

const mappings = {
  Progress: `<script lang="ts">
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
</div>`,

  StatusDot: `<script lang="ts">
  let p = $props();
  const statusColorMap: Record<string, string> = { online: "hsl(145 65% 50%)", offline: "hsl(0 0% 55%)", warning: "hsl(40 90% 55%)", busy: "hsl(0 70% 55%)" };
  let c = $derived(statusColorMap[(p.status as string) || "offline"] ?? "gray");
</script>
<div style="display:inline-flex;align-items:center;gap:0.35rem">
  <span style="width:8px;height:8px;border-radius:50%;background:{c}"></span>
  {#if p.label}
    <span style="font-size:0.78rem">{p.label}</span>
  {/if}
</div>`,

  Select: `<script lang="ts">
  let p = $props();
</script>
<div style="display:flex;flex-direction:column;gap:0.25rem">
  {#if p.label}
    <label style="font-size:0.75rem;color:var(--system-color-text-muted)">{p.label}</label>
  {/if}
  <select style="padding:0.5rem 0.75rem;border-radius:0.5rem;border:1px solid var(--system-color-border);background:var(--system-color-panel);color:var(--system-color-text);font-size:0.85rem">
    {#each (p.options ?? []) as opt}
      <option value={opt}>{opt}</option>
    {/each}
  </select>
</div>`,

  Spinner: `<script lang="ts">
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
</style>`,

  Empty: `<script lang="ts">
  let p = $props();
</script>
<div style="display:flex;flex-direction:column;align-items:center;gap:0.5rem;padding:2rem;text-align:center">
  {#if p.icon}
    <span style="font-size:2.5rem">{p.icon}</span>
  {/if}
  <strong style="font-size:0.92rem">{p.title}</strong>
  {#if p.description}
    <p style="margin:0;font-size:0.82rem;color:var(--system-color-text-muted);max-width:280px">{p.description}</p>
  {/if}
  {#if p.actionLabel}
    <button style="margin-top:0.5rem;padding:0.45rem 1rem;border-radius:999px;background:hsl(215 90% 55%);color:#fff;border:none;font-size:0.8rem;cursor:pointer">
      {p.actionLabel}
    </button>
  {/if}
</div>`,

  Terminal: `<script lang="ts">
  let p = $props();
</script>
<div style="border-radius:0.75rem;background:hsl(0 0% 8%);border:1px solid hsl(0 0% 15%);overflow:hidden;max-height:{p.maxHeight ?? 300}px">
  {#if p.title}
    <div style="padding:0.4rem 0.75rem;font-size:0.72rem;color:hsl(0 0% 50%);border-bottom:1px solid hsl(0 0% 15%);font-family:monospace">
      {p.title}
    </div>
  {/if}
  <div style="padding:0.5rem 0.75rem;font-family:monospace;font-size:0.75rem;color:hsl(145 65% 70%);overflow-y:auto;white-space:pre-wrap">
    {#each (p.lines ?? []) as line}
      <div>{line}</div>
    {/each}
  </div>
</div>`,

  Toggle: `<script lang="ts">
  let p = $props();
</script>
<label style="display:flex;align-items:center;gap:0.5rem;cursor:pointer;font-size:0.85rem">
  <input type="checkbox" style="accent-color:hsl(215 90% 55%)" />
  <span>{p.label}</span>
</label>`
};

for (const [name, code] of Object.entries(mappings)) {
  fs.writeFileSync('src/lib/json-render/components/'+name+'.svelte', code);
}
console.log('Done fixing missed components');
