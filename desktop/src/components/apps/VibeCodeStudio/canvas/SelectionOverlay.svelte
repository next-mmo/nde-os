<script lang="ts">
  import type { FDocument } from "$lib/figma-json/types";
  
  interface Props {
    document: FDocument;
    selectedNodeId: string | null;
    zoom: number;
    onUpdateNodePosition?: (id: string, x: number, y: number) => void;
    onUpdateNodeSize?: (id: string, w: number, h: number) => void;
  }

  let { document, selectedNodeId, zoom, onUpdateNodePosition, onUpdateNodeSize }: Props = $props();

  type BoundsData = { x: number, y: number, w: number, h: number, localX: number, localY: number, node: any };
  function getAbsoluteBounds(root: FDocument, targetId: string): BoundsData | null {
    let found: BoundsData | null = null;
    
    function walk(node: any, currentX: number, currentY: number) {
      const lx = node.x ?? 0;
      const ly = node.y ?? 0;
      
      if (node.id === targetId) {
        found = { 
          x: currentX + lx, 
          y: currentY + ly, 
          w: node.width ?? 0, 
          h: node.height ?? 0, 
          localX: lx,
          localY: ly,
          node 
        };
        return true;
      }
      
      const nextX = currentX + lx;
      const nextY = currentY + ly;
      
      if (node.children) {
        for (const child of node.children) {
          if (walk(child, nextX, nextY)) return true;
        }
      }
      return false;
    }
    
    for (const child of root.children) {
      if (walk(child, 0, 0)) break;
    }
    return found;
  }

  const bounds = $derived(selectedNodeId ? getAbsoluteBounds(document, selectedNodeId) : null);

  let isResizing = $state(false);
  let resizeHandle = $state("");
  let dragStartX = $state(0);
  let dragStartY = $state(0);
  let startW = $state(0);
  let startH = $state(0);
  let startLocalX = $state(0);
  let startLocalY = $state(0);

  function handlePointerDown(e: PointerEvent, handle: string) {
    if (!bounds || !selectedNodeId) return;
    e.stopPropagation();
    isResizing = true;
    resizeHandle = handle;
    dragStartX = e.clientX;
    dragStartY = e.clientY;
    startW = bounds.w;
    startH = bounds.h;
    startLocalX = bounds.localX;
    startLocalY = bounds.localY;
    (e.currentTarget as HTMLElement)?.setPointerCapture(e.pointerId);
  }

  function handlePointerMove(e: PointerEvent) {
    if (!isResizing || !bounds || !selectedNodeId) return;
    
    // Calculate delta taking canvas zoom into account
    const dx = (e.clientX - dragStartX) / zoom;
    const dy = (e.clientY - dragStartY) / zoom;
    
    let newW = startW;
    let newH = startH;
    let newX = startLocalX;
    let newY = startLocalY;
    
    if (resizeHandle.includes("e")) newW = Math.max(1, startW + dx);
    if (resizeHandle.includes("s")) newH = Math.max(1, startH + dy);
    
    if (resizeHandle.includes("w")) {
      const clampedDx = Math.min(dx, startW - 1);
      newW = startW - clampedDx;
      newX = startLocalX + clampedDx;
    }
    
    if (resizeHandle.includes("n")) {
      const clampedDy = Math.min(dy, startH - 1);
      newH = startH - clampedDy;
      newY = startLocalY + clampedDy;
    }
    
    if (onUpdateNodeSize) onUpdateNodeSize(selectedNodeId, Math.round(newW), Math.round(newH));
    if (onUpdateNodePosition) onUpdateNodePosition(selectedNodeId, Math.round(newX), Math.round(newY));
  }

  function handlePointerUp(e: PointerEvent) {
    if (isResizing) {
      isResizing = false;
      (e.currentTarget as HTMLElement)?.releasePointerCapture(e.pointerId);
    }
  }

  // Handle dimensions accounting for zoom inverse scaling to keep handles consistent size
  const handleSize = $derived(8 / zoom);
  const handleOffset = $derived(-4 / zoom);
  const strokeWidth = $derived(2 / zoom);
  
</script>

{#if bounds}
  <!-- Outline Box -->
  <div 
    class="pointer-events-none absolute border-blue-500 z-50"
    style="
      left: {bounds.x}px; 
      top: {bounds.y}px; 
      width: {bounds.w}px; 
      height: {bounds.h}px;
      border-width: {strokeWidth}px;
    "
  >
    <!-- Resize Handles -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="absolute bg-white border-blue-500 cursor-nwse-resize pointer-events-auto"
         style="width: {handleSize}px; height: {handleSize}px; left: {handleOffset}px; top: {handleOffset}px; border-width: {strokeWidth}px;"
         onpointerdown={(e) => handlePointerDown(e, 'nw')}
         onpointermove={handlePointerMove}
         onpointerup={handlePointerUp}></div>
    
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="absolute bg-white border-blue-500 cursor-ns-resize pointer-events-auto"
         style="width: {handleSize}px; height: {handleSize}px; left: calc(50% + {handleOffset}px); top: {handleOffset}px; border-width: {strokeWidth}px;"
         onpointerdown={(e) => handlePointerDown(e, 'n')}
         onpointermove={handlePointerMove}
         onpointerup={handlePointerUp}></div>
         
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="absolute bg-white border-blue-500 cursor-nesw-resize pointer-events-auto"
         style="width: {handleSize}px; height: {handleSize}px; right: {handleOffset}px; top: {handleOffset}px; border-width: {strokeWidth}px;"
         onpointerdown={(e) => handlePointerDown(e, 'ne')}
         onpointermove={handlePointerMove}
         onpointerup={handlePointerUp}></div>
         
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="absolute bg-white border-blue-500 cursor-ew-resize pointer-events-auto"
         style="width: {handleSize}px; height: {handleSize}px; right: {handleOffset}px; top: calc(50% + {handleOffset}px); border-width: {strokeWidth}px;"
         onpointerdown={(e) => handlePointerDown(e, 'e')}
         onpointermove={handlePointerMove}
         onpointerup={handlePointerUp}></div>
         
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="absolute bg-white border-blue-500 cursor-nwse-resize pointer-events-auto"
         style="width: {handleSize}px; height: {handleSize}px; right: {handleOffset}px; bottom: {handleOffset}px; border-width: {strokeWidth}px;"
         onpointerdown={(e) => handlePointerDown(e, 'se')}
         onpointermove={handlePointerMove}
         onpointerup={handlePointerUp}></div>
         
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="absolute bg-white border-blue-500 cursor-ns-resize pointer-events-auto"
         style="width: {handleSize}px; height: {handleSize}px; left: calc(50% + {handleOffset}px); bottom: {handleOffset}px; border-width: {strokeWidth}px;"
         onpointerdown={(e) => handlePointerDown(e, 's')}
         onpointermove={handlePointerMove}
         onpointerup={handlePointerUp}></div>
         
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="absolute bg-white border-blue-500 cursor-nesw-resize pointer-events-auto"
         style="width: {handleSize}px; height: {handleSize}px; left: {handleOffset}px; bottom: {handleOffset}px; border-width: {strokeWidth}px;"
         onpointerdown={(e) => handlePointerDown(e, 'sw')}
         onpointermove={handlePointerMove}
         onpointerup={handlePointerUp}></div>
         
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="absolute bg-white border-blue-500 cursor-ew-resize pointer-events-auto"
         style="width: {handleSize}px; height: {handleSize}px; left: {handleOffset}px; top: calc(50% + {handleOffset}px); border-width: {strokeWidth}px;"
         onpointerdown={(e) => handlePointerDown(e, 'w')}
         onpointermove={handlePointerMove}
         onpointerup={handlePointerUp}></div>
  </div>
{/if}
