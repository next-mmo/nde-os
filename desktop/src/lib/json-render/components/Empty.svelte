<script lang="ts">
  let { props: p } = $props();
</script>
<div class="flex flex-col items-center gap-2 p-8 text-center">
  {#if p.icon}
    <span class="text-4xl">{p.icon}</span>
  {/if}
  <strong class="text-sm font-semibold">{p.title}</strong>
  {#if p.description}
    <p class="m-0 text-xs text-muted-foreground max-w-[280px]">{p.description}</p>
  {/if}
  {#if p.actionLabel}
    <button class="mt-2 px-4 py-2 rounded-full bg-primary text-primary-foreground text-xs cursor-pointer hover:bg-primary/90 transition-colors">
      {p.actionLabel}
    </button>
  {/if}
</div>