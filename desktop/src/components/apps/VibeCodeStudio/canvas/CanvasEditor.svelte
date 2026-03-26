<script lang="ts">
  import type { FDocument, FNode } from "$lib/figma-json/types";
  import NodeRenderer from "./NodeRenderer.svelte";
  import SelectionOverlay from "./SelectionOverlay.svelte";

  interface Props {
    document: FDocument;
    zoom?: number;
    panX?: number;
    panY?: number;
    selectedNodeId?: string | null;
    onSelectNode?: (nodeId: string | null) => void;
    onUpdateNodePosition?: (nodeId: string, x: number, y: number) => void;
    onUpdateNodeSize?: (nodeId: string, width: number, height: number) => void;
  }

  let { 
    document, 
    zoom = $bindable(1), 
    panX = $bindable(0), 
    panY = $bindable(0), 
    selectedNodeId = null,
    onSelectNode,
    onUpdateNodePosition,
    onUpdateNodeSize
  }: Props = $props();

  let canvasRef: HTMLDivElement | undefined = $state();
  
  // Interactive panning
  let isPanning = $state(false);
  let startPanX = $state(0);
  let startPanY = $state(0);
  
  function handlePointerDown(e: PointerEvent) {
    if (e.target !== canvasRef) return; // Only if clicking the empty canvas space

    if (e.button === 1 || e.altKey || e.ctrlKey) {
      // middle click or modifier for panning
      isPanning = true;
      startPanX = e.clientX - panX;
      startPanY = e.clientY - panY;
      (e.currentTarget as HTMLElement)?.setPointerCapture(e.pointerId);
      e.preventDefault();
    } else {
      // Clear selection if clicking empty canvas
      if (onSelectNode) onSelectNode(null);
    }
  }

  function handlePointerMove(e: PointerEvent) {
    if (isPanning) {
      panX = e.clientX - startPanX;
      panY = e.clientY - startPanY;
    }
  }

  function handlePointerUp(e: PointerEvent) {
    if (isPanning) {
      isPanning = false;
      (e.currentTarget as HTMLElement)?.releasePointerCapture(e.pointerId);
    }
  }

  function handleWheel(e: WheelEvent) {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      // Zoom
      const zoomFactor = -e.deltaY * 0.002;
      const newZoom = Math.min(Math.max(0.1, zoom * (1 + zoomFactor)), 5);
      
      // Calculate zoom to pointer
      const rect = canvasRef?.getBoundingClientRect();
      if (rect) {
        const mouseX = e.clientX - rect.left;
        const mouseY = e.clientY - rect.top;
        panX = mouseX - (mouseX - panX) * (newZoom / zoom);
        panY = mouseY - (mouseY - panY) * (newZoom / zoom);
      }
      zoom = newZoom;
    } else {
      // Pan
      panX -= e.deltaX;
      panY -= e.deltaY;
    }
  }

  // Checkered background pattern
  const gridBackground = `
    background-color: #0f1115;
    background-image: 
      linear-gradient(rgba(255, 255, 255, 0.03) 1px, transparent 1px),
      linear-gradient(90deg, rgba(255, 255, 255, 0.03) 1px, transparent 1px);
    background-size: 20px 20px;
  `;
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div 
  bind:this={canvasRef}
  class="w-full h-full relative overflow-hidden select-none touch-none outline-none"
  style={gridBackground}
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  onpointerleave={handlePointerUp}
  onwheel={handleWheel}
>
  <div 
    class="origin-top-left absolute will-change-transform"
    style="transform: translate({panX}px, {panY}px) scale({zoom});"
  >
    <!-- Root document container uses document bounds if set, 
         or just renders children absolutely -->
    <div 
      class="relative"
      style="
        {document.width ? `width: ${document.width}px;` : ''}
        {document.height ? `height: ${document.height}px;` : ''}
        {document.background ? `background: rgba(${document.background.r*255}, ${document.background.g*255}, ${document.background.b*255}, ${document.background.a});` : ''}
      "
    >
      {#each document.children as node (node.id)}
        <NodeRenderer 
          {node} 
          {selectedNodeId}
          {zoom}
          {onSelectNode}
          {onUpdateNodePosition}
          {onUpdateNodeSize}
        />
      {/each}
      
      {#if selectedNodeId}
        <SelectionOverlay 
          {document} 
          {selectedNodeId} 
          {zoom}
          {onUpdateNodePosition}
          {onUpdateNodeSize}
        />
      {/if}
    </div>
  </div>
</div>
