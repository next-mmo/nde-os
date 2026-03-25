<script lang="ts">
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
<span style="display:inline-block;padding:0.15rem 0.5rem;border-radius:999px;font-size:0.7rem;font-weight:500;background:{c.bg};color:{c.text}">{p.text}</span>