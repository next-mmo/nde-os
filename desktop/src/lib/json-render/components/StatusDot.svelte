<script lang="ts">
  let { props: p } = $props();
</script>
<div class="inline-flex items-center gap-1.5">
  <span class="w-2 h-2 rounded-full {p.status === 'online' ? 'bg-green-500' : p.status === 'offline' ? 'bg-gray-500' : p.status === 'warning' ? 'bg-amber-500' : p.status === 'busy' ? 'bg-red-500' : 'bg-gray-500'}"></span>
  {#if p.label}
    <span class="text-xs">{p.label}</span>
  {/if}
</div>