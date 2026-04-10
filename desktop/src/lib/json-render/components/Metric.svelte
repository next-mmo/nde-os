<script lang="ts">
  let { props: p } = $props();
</script>
<div class="flex flex-col gap-1">
  <span class="text-xs text-muted-foreground uppercase tracking-wider">{p.label}</span>
  <div class="flex items-baseline gap-2">
    <strong class="text-2xl font-semibold tracking-tight">{p.value ?? ""}</strong>
    {#if p.change}
      <span class="text-xs font-medium {p.trend === 'up' ? 'text-green-500' : p.trend === 'down' ? 'text-red-500' : 'text-muted-foreground'}">
        {p.change}
      </span>
    {/if}
  </div>
</div>