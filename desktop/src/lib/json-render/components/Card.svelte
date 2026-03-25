<script lang="ts">
  let p = $props();
  const padMap: Record<string,string> = { none: "0", sm: "0.5rem", md: "1rem", lg: "1.5rem" };
  let padding = $derived(padMap[(p.padding as string) ?? "md"] ?? "1rem");
  let variantStyle = $derived(p.variant === "glass"
    ? "background:hsla(0 0% 100%/0.04);backdrop-filter:blur(12px);border:1px solid hsla(0 0% 100%/0.08)"
    : p.variant === "outlined"
      ? "background:transparent;border:1px solid var(--system-color-border)"
      : "background:var(--system-color-panel);border:1px solid var(--system-color-border)");
</script>
<div style="border-radius:1rem;padding:{padding};{variantStyle}">
  {#if p.title}
    <strong style="display:block;margin-bottom:0.5rem;font-size:0.88rem">{p.title}</strong>
  {/if}
  {@render p.children?.()}
</div>