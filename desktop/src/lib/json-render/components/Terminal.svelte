<script lang="ts">
  let { props: p } = $props();
</script>
<div class="rounded-xl bg-black border border-zinc-800 overflow-hidden flex flex-col" style="max-height: {p.maxHeight ?? 300}px">
  {#if p.title}
    <div class="px-3 py-1.5 text-xs text-zinc-400 border-b border-zinc-800 font-mono">
      {p.title}
    </div>
  {/if}
  <div class="p-3 font-mono text-xs text-green-400 overflow-y-auto whitespace-pre-wrap">
    {#each (p.lines ?? []) as line}
      <div>{line}</div>
    {/each}
  </div>
</div>