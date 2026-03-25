<script lang="ts">
  let p = $props();
</script>
<div style="border-radius:0.75rem;background:hsl(0 0% 8%);border:1px solid hsl(0 0% 15%);overflow:hidden;max-height:{p.maxHeight ?? 300}px">
  {#if p.title}
    <div style="padding:0.4rem 0.75rem;font-size:0.72rem;color:hsl(0 0% 50%);border-bottom:1px solid hsl(0 0% 15%);font-family:monospace">
      {p.title}
    </div>
  {/if}
  <div style="padding:0.5rem 0.75rem;font-family:monospace;font-size:0.75rem;color:hsl(145 65% 70%);overflow-y:auto;white-space:pre-wrap">
    {#each (p.lines ?? []) as line}
      <div>{line}</div>
    {/each}
  </div>
</div>