<svelte:options runes={true} />

<script lang="ts">
  import type { FNode, FFrameNode, FTextNode, FImageNode, FVectorNode } from "$lib/figma-json/types";
  import { resolveNodeStyles, stylesToString } from "$lib/figma-json/style-resolver";
  import NodeRenderer from "./NodeRenderer.svelte";

  interface Props {
    node: FNode;
    selectedNodeId?: string | null;
    zoom?: number;
    onSelectNode?: (nodeId: string | null) => void;
    onUpdateNodePosition?: (id: string, x: number, y: number) => void;
    onUpdateNodeSize?: (id: string, w: number, h: number) => void;
  }

  let { node, selectedNodeId, zoom = 1, onSelectNode, onUpdateNodePosition, onUpdateNodeSize }: Props = $props();

  const isSelected = $derived(node.id === selectedNodeId);

  // Core base styles inherited from the JSON structure
  const baseStyle = $derived(resolveNodeStyles(node));

  // Determine if this node is absolutely positioned based on having x/y
  // If undefined, it falls back to flex/auto-layout flow
  const isAbsolute = $derived(node.x !== undefined && node.y !== undefined);

  // Extend styles with our interactive and positioning layers
  const mergedStyle = $derived({
    ...baseStyle,
    ...(isAbsolute && { position: 'absolute', left: `${node.x}px`, top: `${node.y}px` })
  });

  const styleStr = $derived(stylesToString(mergedStyle));

  // Interaction handlers
  let isDragging = $state(false);
  let dragStartX = $state(0);
  let dragStartY = $state(0);
  let initialNodeX = $state(0);
  let initialNodeY = $state(0);

  function handlePointerDown(e: PointerEvent) {
    e.stopPropagation(); // prevent canvas panning
    if (onSelectNode) onSelectNode(node.id);

    if (e.button === 0 && isAbsolute) {
      isDragging = true;
      dragStartX = e.clientX;
      dragStartY = e.clientY;
      initialNodeX = node.x ?? 0;
      initialNodeY = node.y ?? 0;
      (e.currentTarget as HTMLElement)?.setPointerCapture(e.pointerId);
    }
  }

  function handlePointerMove(e: PointerEvent) {
    if (isDragging && onUpdateNodePosition) {
      const dx = (e.clientX - dragStartX) / zoom;
      const dy = (e.clientY - dragStartY) / zoom;
      onUpdateNodePosition(node.id, initialNodeX + dx, initialNodeY + dy);
    }
  }

  function handlePointerUp(e: PointerEvent) {
    if (isDragging) {
      isDragging = false;
      (e.currentTarget as HTMLElement)?.releasePointerCapture(e.pointerId);
    }
  }

  // Type identification
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
</script>

{#if isFrame}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    data-fj-id={node.id}
    data-fj-type={node.type}
    style={styleStr}
    class="pointer-events-auto"
    onpointerdown={handlePointerDown}
    onpointermove={handlePointerMove}
    onpointerup={handlePointerUp}
    onpointercancel={handlePointerUp}
  >
    {#if (node as FFrameNode).children}
      {#each (node as FFrameNode).children! as child (child.id)}
        <NodeRenderer 
          node={child} 
          {selectedNodeId} 
          {zoom}
          {onSelectNode} 
          {onUpdateNodePosition} 
          {onUpdateNodeSize} 
        />
      {/each}
    {/if}
  </div>
{:else if isText}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <span
    data-fj-id={node.id}
    data-fj-type="TEXT"
    style={styleStr}
    class="pointer-events-auto select-none"
    onpointerdown={handlePointerDown}
    onpointermove={handlePointerMove}
    onpointerup={handlePointerUp}
    onpointercancel={handlePointerUp}
  >{(node as FTextNode).characters}</span>
{:else if isImage}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <img
    data-fj-id={node.id}
    data-fj-type="IMAGE"
    src={(node as FImageNode).src}
    alt={(node as FImageNode).alt ?? node.name}
    style={styleStr}
    style:object-fit={(node as FImageNode).objectFit ?? "cover"}
    class="pointer-events-auto"
    onpointerdown={handlePointerDown}
    onpointermove={handlePointerMove}
    onpointerup={handlePointerUp}
    onpointercancel={handlePointerUp}
  />
{:else if isVector && (node as FVectorNode).svgPath}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <svg
    data-fj-id={node.id}
    data-fj-type={node.type}
    width={(node as FVectorNode).width}
    height={(node as FVectorNode).height}
    viewBox="0 0 {(node as FVectorNode).width} {(node as FVectorNode).height}"
    class="pointer-events-auto {styleStr}"
    onpointerdown={handlePointerDown}
    onpointermove={handlePointerMove}
    onpointerup={handlePointerUp}
    onpointercancel={handlePointerUp}
  >
    <path d={(node as FVectorNode).svgPath} fill="currentColor" />
  </svg>
{:else}
  <!-- Rectangle, Ellipse, or basic shape wrapper -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    data-fj-id={node.id}
    data-fj-type={node.type}
    style={styleStr}
    class="pointer-events-auto"
    onpointerdown={handlePointerDown}
    onpointermove={handlePointerMove}
    onpointerup={handlePointerUp}
    onpointercancel={handlePointerUp}
  ></div>
{/if}
