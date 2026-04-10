<script lang="ts">
  let { props: p } = $props();
</script>
<div class="flex items-center gap-3 p-3 rounded-xl border bg-card cursor-pointer transition-colors hover:bg-accent">
  <span class="text-2xl">{p.icon ?? "📦"}</span>
  <div class="flex-1 min-w-0">
    <strong class="text-sm block font-medium">{p.name}</strong>
    <span class="text-xs text-muted-foreground">{p.description}</span>
  </div>
  <span class="text-[10px] font-medium uppercase {p.status === 'installed' ? 'text-amber-500' : p.status === 'running' ? 'text-green-500' : p.status === 'error' ? 'text-red-500' : 'text-muted-foreground'}">{p.status}</span>
</div>