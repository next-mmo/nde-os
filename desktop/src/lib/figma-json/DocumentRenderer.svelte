<svelte:options runes={true} />

<script lang="ts">
  import type { FDocument } from "$lib/figma-json/types";
  import { fColorToCSS } from "$lib/figma-json/style-resolver";
  import JsonRenderer from "$lib/figma-json/JsonRenderer.svelte";

  interface Props {
    document: FDocument;
    onClick?: (id: string) => void;
    /** Scale factor for preview (1 = 100%) */
    scale?: number;
  }

  let { document, onClick, scale = 1 }: Props = $props();

  const bgColor = $derived(
    document.background
      ? fColorToCSS(document.background)
      : "transparent",
  );

  const canvasStyle = $derived(
    [
      `background-color: ${bgColor}`,
      document.width ? `width: ${document.width}px` : "",
      document.height ? `height: ${document.height}px` : "",
      scale !== 1 ? `transform: scale(${scale})` : "",
      scale !== 1 ? "transform-origin: top left" : "",
    ]
      .filter(Boolean)
      .join("; "),
  );
</script>

<div class="figma-canvas relative" style={canvasStyle}>
  {#each document.children as child (child.id)}
    <JsonRenderer node={child} {onClick} />
  {/each}
</div>
