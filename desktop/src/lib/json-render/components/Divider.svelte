<script lang="ts">
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
{/if}