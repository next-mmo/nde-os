<script lang="ts">
  import type { Snippet } from "svelte";

  let { 
    tag = "div", 
    style = "", 
    attrs = {}, 
    children, 
    text
  }: { 
    tag?: string; 
    style?: string; 
    attrs?: Record<string, any>; 
    children?: Snippet; 
    text?: string; 
  } = $props();
</script>

<svelte:element this={tag} {style} {...attrs}>
  {#if text}
    {text}
  {/if}
  {#if children}
    {@render children()}
  {/if}
</svelte:element>
