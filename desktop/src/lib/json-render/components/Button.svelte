<script lang="ts">
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
</button>