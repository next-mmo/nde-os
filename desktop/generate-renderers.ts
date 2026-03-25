import * as fs from 'fs';

const mappings = {
  Card: `<script lang="ts">
  let p = $props();
  const padMap: Record<string,string> = { none: "0", sm: "0.5rem", md: "1rem", lg: "1.5rem" };
  let padding = padMap[p.padding ?? "md"] ?? "1rem";
  let variantStyle = p.variant === "glass"
    ? "background:hsla(0 0% 100%/0.04);backdrop-filter:blur(12px);border:1px solid hsla(0 0% 100%/0.08)"
    : p.variant === "outlined"
      ? "background:transparent;border:1px solid var(--system-color-border)"
      : "background:var(--system-color-panel);border:1px solid var(--system-color-border)";
</script>
<div style="border-radius:1rem;padding:{padding};{variantStyle}">
  {#if p.title}
    <strong style="display:block;margin-bottom:0.5rem;font-size:0.88rem">{p.title}</strong>
  {/if}
  {@render p.children?.()}
</div>`,

  Stack: `<script lang="ts">
  let p = $props();
  const gapMap: Record<string,string> = { none: "0", xs: "0.25rem", sm: "0.5rem", md: "0.75rem", lg: "1.25rem", xl: "2rem" };
  const alignMap: Record<string,string> = { start: "flex-start", center: "center", end: "flex-end", stretch: "stretch" };
  const justMap: Record<string,string> = { start: "flex-start", center: "center", end: "flex-end", between: "space-between", around: "space-around" };
</script>
<div style="display:flex;flex-direction:{p.direction === 'horizontal' ? 'row' : 'column'};gap:{gapMap[p.gap ?? 'md'] ?? '0.75rem'};align-items:{alignMap[p.align ?? 'start']};justify-content:{justMap[p.justify ?? 'start']}{p.wrap ? ';flex-wrap:wrap' : ''}">
  {@render p.children?.()}
</div>`,

  Grid: `<script lang="ts">
  let p = $props();
  const gapMap: Record<string,string> = { none: "0", xs: "0.25rem", sm: "0.5rem", md: "0.75rem", lg: "1.25rem", xl: "2rem" };
</script>
<div style="display:grid;grid-template-columns:repeat({p.columns ?? 2},1fr);gap:{gapMap[p.gap ?? 'md'] ?? '0.75rem'}">
  {@render p.children?.()}
</div>`,

  Divider: `<script lang="ts">
  let p = $props();
</script>
{#if p.label}
  <div style="display:flex;align-items:center;gap:0.75rem;margin:0.25rem 0">
    <div style="flex:1;height:1px;background:var(--system-color-border)"></div>
    <span style="font-size:0.72rem;color:var(--system-color-text-muted);text-transform:uppercase;letter-spacing:0.1em">{p.label}</span>
    <div style="flex:1;height:1px;background:var(--system-color-border)"></div>
  </div>
{:else}
  <hr style="border:none;height:1px;background:var(--system-color-border);margin:0.25rem 0" />
{/if}`,

  Heading: `<script lang="ts">
  let p = $props();
  const headingSize: Record<string,string> = { "1": "1.5rem", "2": "1.2rem", "3": "1rem", "4": "0.88rem" };
  const level = p.level ?? "2";
  const tag = \`h\${level}\`;
</script>
<svelte:element this={tag} style="margin:0;font-size:{headingSize[level]};font-weight:600{p.muted ? ';color:var(--system-color-text-muted)' : ''}">
  {p.text}
</svelte:element>`,

  Text: `<script lang="ts">
  let p = $props();
  const fontSizeMap: Record<string,string> = { xs: "0.72rem", sm: "0.82rem", md: "0.92rem", lg: "1.1rem" };
  let styles = $derived([
    \`font-size:\${fontSizeMap[p.size ?? "md"]}\`,
    p.muted ? "color:var(--system-color-text-muted)" : "",
    p.bold ? "font-weight:600" : "",
    p.mono ? "font-family:monospace" : "",
    "margin:0",
  ].filter(Boolean).join(";"));
</script>
<p style={styles}>{p.text}</p>`,

  Code: `<script lang="ts"> let p = $props(); </script>
<pre style="margin:0;padding:0.75rem 1rem;border-radius:0.5rem;background:hsla(0 0% 0%/0.3);font-family:monospace;font-size:0.78rem;overflow-x:auto;white-space:pre-wrap;color:hsl(145 65% 70%)"><code>{p.code}</code></pre>`,

  Badge: `<script lang="ts">
  let p = $props();
  const badgeColors: Record<string, {bg:string;text:string}> = {
    default: { bg: "hsla(0 0% 50%/0.12)", text: "var(--system-color-text-muted)" },
    success: { bg: "hsla(145 65% 50%/0.12)", text: "hsl(145 65% 55%)" },
    warning: { bg: "hsla(40 90% 55%/0.12)", text: "hsl(40 90% 60%)" },
    danger:  { bg: "hsla(0 70% 55%/0.12)", text: "hsl(0 70% 60%)" },
    info:    { bg: "hsla(210 80% 55%/0.12)", text: "hsl(210 80% 65%)" },
  };
  let c = $derived(badgeColors[p.variant ?? "default"] ?? badgeColors.default);
</script>
<span style="display:inline-block;padding:0.15rem 0.5rem;border-radius:999px;font-size:0.7rem;font-weight:500;background:{c.bg};color:{c.text}">{p.text}</span>`,

  Metric: `<script lang="ts">
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
</div>`,

  Button: `<script lang="ts">
  let p = $props();
  const btnSizeMap: Record<string,string> = { sm: "0.35rem 0.7rem", md: "0.5rem 1rem", lg: "0.65rem 1.3rem" };
  const btnVariants: Record<string,string> = {
    primary: "background:hsl(215 90% 55%);color:#fff;border:none",
    secondary: "background:var(--system-color-panel);color:var(--system-color-text);border:1px solid var(--system-color-border)",
    outline: "background:transparent;color:var(--system-color-text);border:1px solid var(--system-color-border)",
    ghost: "background:transparent;color:var(--system-color-text);border:none",
    danger: "background:hsl(0 70% 55%);color:#fff;border:none",
  };
  let vs = $derived(btnVariants[p.variant ?? "primary"] ?? btnVariants.primary);
</script>
<button style="{vs};padding:{btnSizeMap[p.size ?? 'md']};border-radius:0.5rem;font-size:0.82rem;font-weight:500;cursor:pointer;transition:opacity 0.15s{p.disabled ? ';opacity:0.4;pointer-events:none' : ''}">
  {p.label ?? ""}
</button>`,

  Input: `<script lang="ts">
  let p = $props();
</script>
<div style="display:flex;flex-direction:column;gap:0.25rem">
  {#if p.label}
    <label style="font-size:0.75rem;color:var(--system-color-text-muted)">{p.label}</label>
  {/if}
  <input type={p.type ?? "text"} placeholder={p.placeholder ?? ""} bind:value={p.value} style="padding:0.5rem 0.75rem;border-radius:0.5rem;border:1px solid var(--system-color-border);background:var(--system-color-panel);color:var(--system-color-text);font-size:0.85rem;outline:none" />
</div>`,

  Alert: `<script lang="ts">
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
</div>`,

  AppTile: `<script lang="ts">
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
</div>`,

  List: `<script lang="ts">
  let p = $props();
</script>
<div style="display:flex;flex-direction:column">
  {#each (p.items ?? []) as item}
    <div style="display:flex;align-items:center;gap:0.6rem;padding:0.55rem 0.5rem;border-bottom:1px solid var(--system-color-border);cursor:default">
      {#if item.icon}
        <span style="font-size:1.1rem">{item.icon}</span>
      {/if}
      <div style="flex:1;min-width:0">
        <strong style="font-size:0.85rem;display:block">{item.label}</strong>
        {#if item.description}
          <span style="font-size:0.72rem;color:var(--system-color-text-muted)">{item.description}</span>
        {/if}
      </div>
    </div>
  {/each}
</div>`,

  Table: `<script lang="ts">
  let p = $props();
</script>
<table style="width:100%;border-collapse:collapse">
  <thead>
    <tr>
      {#each (p.headers ?? []) as h}
        <th style="padding:0.5rem 0.75rem;text-align:left;font-size:0.72rem;text-transform:uppercase;letter-spacing:0.08em;color:var(--system-color-text-muted);border-bottom:1px solid var(--system-color-border)">{h}</th>
      {/each}
    </tr>
  </thead>
  <tbody>
    {#each (p.rows ?? []) as row, i}
      <tr style="{p.striped && i % 2 ? 'background:hsla(0 0% 50%/0.04)' : ''}">
        {#each row as cell}
          <td style="padding:0.5rem 0.75rem;font-size:0.82rem;border-bottom:1px solid var(--system-color-border)">{cell}</td>
        {/each}
      </tr>
    {/each}
  </tbody>
</table>`
};

for(const [name, code] of Object.entries(mappings)) {
  fs.writeFileSync('src/lib/json-render/components/'+name+'.svelte', code);
}
console.log('Done rendering templates');
