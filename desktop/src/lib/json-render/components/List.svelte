<script lang="ts">
  let { props: p } = $props();
</script>
<div class="flex flex-col">
  {#each (p.items ?? []) as item}
    <div class="flex items-center gap-2.5 py-2 px-2 border-b border-border last:border-0">
      {#if item.icon}
        <span class="text-lg">{item.icon}</span>
      {/if}
      <div class="flex-1 min-w-0">
        <strong class="text-sm block font-medium">{item.label}</strong>
        {#if item.description}
          <span class="text-xs text-muted-foreground">{item.description}</span>
        {/if}
      </div>
    </div>
  {/each}
</div>