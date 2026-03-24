<svelte:options runes={true} />

<script lang="ts">
  import {
    openStaticApp,
    desktop,
    selectIcon,
    setIconPosition,
    hideDesktopIcon,
    type StaticAppID,
  } from "🍎/state/desktop.svelte";
  import ContextMenu, { type ContextMenuItem } from "🍎/components/Desktop/ContextMenu.svelte";

  const GRID = 96;
  const ICON_W = 88;
  const DRAG_THRESHOLD = 5;

  const allShortcuts: { id: StaticAppID; label: string }[] = [
    { id: "file-explorer", label: "File Explorer" },
    { id: "ai-launcher", label: "AI Launcher" },
    { id: "shield-browser", label: "Shield Browser" },
    { id: "chat", label: "NDE Chat" },
    { id: "settings", label: "Settings" },
    { id: "code-editor", label: "Code Editor" },
  ];

  const visibleShortcuts = $derived(
    allShortcuts.filter((s) => !desktop.hidden_icons.has(s.id)),
  );

  // Compute default grid position for each icon (top-right, column-major)
  function defaultPosition(index: number): { x: number; y: number } {
    const cols = Math.max(1, Math.floor((window.innerWidth - 16) / GRID));
    const col = cols - 1 - (index % cols);
    const row = Math.floor(index / cols);
    return { x: col * GRID + 4, y: row * GRID + 40 };
  }

  function getIconPos(id: string, index: number): { x: number; y: number } {
    const saved = desktop.icon_positions[id];
    if (saved) return saved;
    return defaultPosition(index);
  }

  /* ---- Drag state ---- */
  let dragging = $state<{ id: string; startX: number; startY: number; origX: number; origY: number; moved: boolean } | null>(null);
  let dragPos = $state<{ x: number; y: number }>({ x: 0, y: 0 });

  function onPointerDown(e: PointerEvent, id: string, ix: number) {
    if (e.button !== 0) return; // left button only for drag
    const pos = getIconPos(id, ix);
    dragging = { id, startX: e.clientX, startY: e.clientY, origX: pos.x, origY: pos.y, moved: false };
    dragPos = { x: pos.x, y: pos.y };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    e.preventDefault();
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    const dx = e.clientX - dragging.startX;
    const dy = e.clientY - dragging.startY;
    if (!dragging.moved && Math.abs(dx) + Math.abs(dy) < DRAG_THRESHOLD) return;
    dragging.moved = true;
    dragPos = { x: dragging.origX + dx, y: dragging.origY + dy };
  }

  function onPointerUp(e: PointerEvent) {
    if (!dragging) return;
    if (dragging.moved) {
      // Snap to grid
      const snappedX = Math.round(dragPos.x / GRID) * GRID + 4;
      const snappedY = Math.round(dragPos.y / GRID) * GRID + 4;
      setIconPosition(dragging.id, snappedX, snappedY);
    }
    dragging = null;
  }

  function handleClick(e: MouseEvent, id: StaticAppID) {
    e.stopPropagation();
    selectIcon(id, e.shiftKey || e.ctrlKey || e.metaKey);
  }

  function handleDblClick(e: MouseEvent, id: StaticAppID) {
    e.stopPropagation();
    openStaticApp(id);
    selectIcon(null);
  }

  function handleClickOutside() {
    selectIcon(null);
    iconCtx = null;
  }

  /* ---- Icon right-click context menu ---- */
  let iconCtx = $state<{ x: number; y: number; id: StaticAppID } | null>(null);

  function handleIconContextMenu(e: MouseEvent, id: StaticAppID) {
    e.preventDefault();
    e.stopPropagation();
    selectIcon(id);
    iconCtx = { x: e.clientX, y: e.clientY, id };
  }

  const iconMenuItems = $derived<ContextMenuItem[]>(
    iconCtx
      ? [
          { kind: "action", icon: "📂", label: "Open", action: () => { openStaticApp(iconCtx!.id); iconCtx = null; } },
          { kind: "action", icon: "ℹ️", label: "Get Info", action: () => { iconCtx = null; }, disabled: true },
          { kind: "divider" },
          { kind: "action", icon: "🗑️", label: "Remove from Desktop", action: () => { hideDesktopIcon(iconCtx!.id); iconCtx = null; } },
        ]
      : [],
  );
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="absolute inset-0 z-0"
  onclick={handleClickOutside}
  onkeydown={undefined}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
>
  {#each visibleShortcuts as shortcut, ix (shortcut.id)}
    {@const pos = dragging?.id === shortcut.id && dragging.moved ? dragPos : getIconPos(shortcut.id, ix)}
    {@const isSelected = desktop.icon_selection.has(shortcut.id)}
    {@const isDragging = dragging?.id === shortcut.id && dragging.moved}
    <button
      type="button"
      class="absolute flex flex-col items-center gap-1 p-2 rounded-xl cursor-default select-none transition-shadow duration-100 bg-transparent border-none appearance-none outline-none focus-visible:ring-2 focus-visible:ring-white/50
        {isSelected ? 'bg-white/20 dark:bg-white/10 ring-1 ring-white/30 shadow-sm' : 'hover:bg-white/10 dark:hover:bg-white/5'}
        {isDragging ? 'opacity-80 scale-105 shadow-2xl' : ''}"
      style="left: {pos.x}px; top: {pos.y}px; width: {ICON_W}px; {isDragging ? 'z-index: 100; transition: none;' : ''}"
      onpointerdown={(e) => onPointerDown(e, shortcut.id, ix)}
      onclick={(e) => handleClick(e, shortcut.id)}
      ondblclick={(e) => handleDblClick(e, shortcut.id)}
      oncontextmenu={(e) => handleIconContextMenu(e, shortcut.id)}
    >
      <img
        src="/app-icons/{shortcut.id}/256.webp"
        alt=""
        class="w-14 h-14 drop-shadow-lg pointer-events-none"
        onerror={(e) => {
          const el = e.currentTarget as HTMLImageElement;
          el.style.display = "none";
          const parent = el.parentElement;
          if (parent && !parent.querySelector(".icon-fallback")) {
            const fallback = document.createElement("div");
            fallback.className = "icon-fallback w-14 h-14 rounded-[22%] bg-gradient-to-br from-blue-400/80 to-blue-600/80 grid place-items-center text-white font-bold text-xl shadow-lg border border-white/20 pointer-events-none";
            fallback.textContent = shortcut.label.slice(0, 2).toUpperCase();
            parent.insertBefore(fallback, el);
          }
        }}
      />
      <span class="text-[0.68rem] font-medium text-white text-center leading-tight line-clamp-2 w-full drop-shadow-[0_1px_2px_rgba(0,0,0,0.6)] pointer-events-none">
        {shortcut.label}
      </span>
    </button>
  {/each}
</div>

<!-- Icon context menu -->
{#if iconCtx}
  <ContextMenu x={iconCtx.x} y={iconCtx.y} items={iconMenuItems} onclose={() => (iconCtx = null)} />
{/if}
