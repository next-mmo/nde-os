<script lang="ts">
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
</div>