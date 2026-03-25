<svelte:options runes={true} />

<script lang="ts">
  import type { FNode, FFrameNode, FTextNode, FImageNode, FVectorNode } from "$lib/figma-json/types";
  import { resolveNodeStyles, stylesToString } from "$lib/figma-json/style-resolver";

  interface Props {
    node: FNode;
    onClick?: (id: string) => void;
  }

  let { node, onClick }: Props = $props();

  const style = $derived(stylesToString(resolveNodeStyles(node)));

  function handleClick(e: MouseEvent) {
    if (node.onClick && onClick) {
      e.stopPropagation();
      onClick(node.onClick);
    }
  }

  const isFrame = $derived(
    node.type === "FRAME" ||
    node.type === "GROUP" ||
    node.type === "COMPONENT" ||
    node.type === "INSTANCE"
  );
  const isText = $derived(node.type === "TEXT");
  const isImage = $derived(node.type === "IMAGE");
  const isVector = $derived(
    node.type === "VECTOR" ||
    node.type === "LINE" ||
    node.type === "STAR" ||
    node.type === "POLYGON"
  );
  const isEllipse = $derived(node.type === "ELLIPSE");
  const isRectangle = $derived(node.type === "RECTANGLE");
</script>

{#if isFrame}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    data-fj-id={node.id}
    data-fj-name={node.name}
    data-fj-type={node.type}
    {style}
    onclick={node.onClick ? handleClick : undefined}
    class={node.onClick ? "cursor-pointer" : ""}
  >
    {#if (node as FFrameNode).children}
      {#each (node as FFrameNode).children! as child (child.id)}
        <svelte:self node={child} {onClick} />
      {/each}
    {/if}
  </div>
{:else if isText}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <span
    data-fj-id={node.id}
    data-fj-name={node.name}
    data-fj-type="TEXT"
    {style}
    onclick={node.onClick ? handleClick : undefined}
  >{(node as FTextNode).characters}</span>
{:else if isImage}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <img
    data-fj-id={node.id}
    data-fj-name={node.name}
    data-fj-type="IMAGE"
    src={(node as FImageNode).src}
    alt={(node as FImageNode).alt ?? node.name}
    {style}
    style:object-fit={(node as FImageNode).objectFit ?? "cover"}
    onclick={node.onClick ? handleClick : undefined}
  />
{:else if isVector && (node as FVectorNode).svgPath}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <svg
    data-fj-id={node.id}
    data-fj-name={node.name}
    data-fj-type={node.type}
    width={(node as FVectorNode).width}
    height={(node as FVectorNode).height}
    viewBox="0 0 {(node as FVectorNode).width} {(node as FVectorNode).height}"
    class={node.onClick ? "cursor-pointer" : ""}
    onclick={node.onClick ? handleClick : undefined}
  >
    <path d={(node as FVectorNode).svgPath} fill="currentColor" />
  </svg>
{:else}
  <!-- Rectangle, Ellipse, or Vector without path → rendered as div -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    data-fj-id={node.id}
    data-fj-name={node.name}
    data-fj-type={node.type}
    {style}
    onclick={node.onClick ? handleClick : undefined}
    class={node.onClick ? "cursor-pointer" : ""}
  ></div>
{/if}
