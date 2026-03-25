<script lang="ts">
  let p = $props();
  const fontSizeMap: Record<string,string> = { xs: "0.72rem", sm: "0.82rem", md: "0.92rem", lg: "1.1rem" };
  let styles = $derived([
    `font-size:${fontSizeMap[p.size ?? "md"]}`,
    p.muted ? "color:var(--system-color-text-muted)" : "",
    p.bold ? "font-weight:600" : "",
    p.mono ? "font-family:monospace" : "",
    "margin:0",
  ].filter(Boolean).join(";"));
</script>
<p style={styles}>{p.text}</p>