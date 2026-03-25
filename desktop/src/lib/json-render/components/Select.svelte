<script lang="ts">
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
</div>