/**
 * FreeCut timeline items & tracks store — ported from React Zustand to zustand/vanilla.
 *
 * Core timeline content: items (clips), tracks, add/remove/move/split/trim operations.
 * Connected to Rust backend for persistence via Tauri IPC.
 */

import { createStore } from 'zustand/vanilla';

// ─── Types ─────────────────────────────────────────────────────────────────────

export type ItemType = 'video' | 'audio' | 'text' | 'image' | 'shape' | 'adjustment' | 'composition';

export interface TransformProperties {
  x?: number;
  y?: number;
  width?: number;
  height?: number;
  rotation?: number;
  opacity?: number;
  cornerRadius?: number;
  aspectRatioLocked?: boolean;
}

export interface ItemEffect {
  id: string;
  effectType: string;
  enabled: boolean;
  params: Record<string, unknown>;
}

export interface TimelineItem {
  id: string;
  trackId: string;
  from: number;
  durationInFrames: number;
  label: string;
  type: ItemType;
  mediaId?: string;
  originId?: string;
  linkedGroupId?: string;
  compositionId?: string;
  trimStart?: number;
  trimEnd?: number;
  sourceStart?: number;
  sourceEnd?: number;
  sourceDuration?: number;
  sourceFps?: number;
  speed?: number;
  transform?: TransformProperties;
  volume?: number;
  audioFadeIn?: number;
  audioFadeOut?: number;
  fadeIn?: number;
  fadeOut?: number;
  effects?: ItemEffect[];
  blendMode?: string;
  src?: string;
  thumbnailUrl?: string;
  sourceWidth?: number;
  sourceHeight?: number;
  waveformData?: number[];
  text?: string;
  fontSize?: number;
  fontFamily?: string;
  color?: string;
  textAlign?: string;
  shapeType?: string;
  fillColor?: string;
  strokeColor?: string;
  strokeWidth?: number;
}

export interface TimelineTrack {
  id: string;
  name: string;
  kind?: 'video' | 'audio';
  height: number;
  locked: boolean;
  visible: boolean;
  muted: boolean;
  solo: boolean;
  volume?: number;
  color?: string;
  order: number;
  parentTrackId?: string;
  isGroup: boolean;
  isCollapsed: boolean;
}

// ─── Helpers ───────────────────────────────────────────────────────────────────

function roundFrame(value: number, fallback = 0): number {
  if (!Number.isFinite(value)) return fallback;
  return Math.max(0, Math.round(value));
}

function roundDuration(value: number, fallback = 1): number {
  if (!Number.isFinite(value)) return fallback;
  return Math.max(1, Math.round(value));
}

function computeMaxItemEndFrame(items: TimelineItem[]): number {
  let max = 0;
  for (const item of items) {
    const end = item.from + item.durationInFrames;
    if (end > max) max = end;
  }
  return max;
}

function buildItemsByTrackId(items: TimelineItem[]): Record<string, TimelineItem[]> {
  const grouped: Record<string, TimelineItem[]> = {};
  for (const item of items) {
    (grouped[item.trackId] ??= []).push(item);
  }
  return grouped;
}

function buildItemById(items: TimelineItem[]): Record<string, TimelineItem> {
  const map: Record<string, TimelineItem> = {};
  for (const item of items) {
    map[item.id] = item;
  }
  return map;
}

function withItemIndexes(items: TimelineItem[]): Pick<ItemsState, 'items' | 'itemsByTrackId' | 'itemById' | 'maxItemEndFrame'> {
  return {
    items,
    itemsByTrackId: buildItemsByTrackId(items),
    itemById: buildItemById(items),
    maxItemEndFrame: computeMaxItemEndFrame(items),
  };
}

// ─── Store ─────────────────────────────────────────────────────────────────────

export interface ItemsState {
  items: TimelineItem[];
  itemsByTrackId: Record<string, TimelineItem[]>;
  itemById: Record<string, TimelineItem>;
  maxItemEndFrame: number;
  tracks: TimelineTrack[];
  fps: number;
  snapEnabled: boolean;
  scrollLeft: number;
}

export interface ItemsActions {
  setItems: (items: TimelineItem[]) => void;
  setTracks: (tracks: TimelineTrack[]) => void;
  setFps: (fps: number) => void;
  setSnapEnabled: (enabled: boolean) => void;
  setScrollLeft: (scrollLeft: number) => void;

  addItem: (item: TimelineItem) => void;
  addItems: (items: TimelineItem[]) => void;
  updateItem: (id: string, updates: Partial<TimelineItem>) => void;
  removeItems: (ids: string[]) => void;
  moveItem: (id: string, newFrom: number, newTrackId?: string) => void;
  moveItems: (updates: Array<{ id: string; from: number; trackId?: string }>) => void;
  duplicateItems: (itemIds: string[], positions: Array<{ from: number; trackId: string }>) => TimelineItem[];

  trimItemStart: (id: string, trimAmount: number) => void;
  trimItemEnd: (id: string, trimAmount: number) => void;
  splitItem: (id: string, splitFrame: number) => { leftItem: TimelineItem; rightItem: TimelineItem } | null;

  addTrack: (track: TimelineTrack) => void;
  removeTrack: (id: string) => void;
  updateTrack: (id: string, updates: Partial<TimelineTrack>) => void;
  toggleTrackLock: (id: string) => void;
  toggleTrackVisibility: (id: string) => void;
  toggleTrackMute: (id: string) => void;
  toggleTrackSolo: (id: string) => void;

  updateItemTransform: (id: string, transform: Partial<TransformProperties>) => void;
  resetItemTransform: (id: string) => void;
}

export const itemsStore = createStore<ItemsState & ItemsActions>()((set, get) => ({
  // State
  items: [],
  itemsByTrackId: {},
  itemById: {},
  maxItemEndFrame: 0,
  tracks: [],
  fps: 30,
  snapEnabled: true,
  scrollLeft: 0,

  // Bulk setters
  setItems: (items) => set(() => withItemIndexes(items)),
  setTracks: (tracks) => set({
    tracks: [...tracks].sort((a, b) => (a.order ?? 0) - (b.order ?? 0)),
  }),
  setFps: (fps) => set({ fps }),
  setSnapEnabled: (enabled) => set({ snapEnabled: enabled }),
  setScrollLeft: (scrollLeft) => set({ scrollLeft }),

  // Item operations
  addItem: (item) => set((state) => withItemIndexes([...state.items, item])),

  addItems: (items) => set((state) => withItemIndexes([...state.items, ...items])),

  updateItem: (id, updates) => set((state) => {
    const nextItems = state.items.map((i) =>
      i.id === id ? { ...i, ...updates } : i
    );
    return withItemIndexes(nextItems);
  }),

  removeItems: (ids) => set((state) => {
    const idsSet = new Set(ids);
    return withItemIndexes(state.items.filter((i) => !idsSet.has(i.id)));
  }),

  moveItem: (id, newFrom, newTrackId) => set((state) => {
    const nextItems = state.items.map((item) =>
      item.id === id
        ? { ...item, from: roundFrame(newFrom), ...(newTrackId && { trackId: newTrackId }) }
        : item
    );
    return withItemIndexes(nextItems);
  }),

  moveItems: (updates) => set((state) => {
    const updateMap = new Map(updates.map((u) => [u.id, u]));
    const nextItems = state.items.map((item) => {
      const update = updateMap.get(item.id);
      if (!update) return item;
      return {
        ...item,
        from: roundFrame(update.from),
        ...(update.trackId && { trackId: update.trackId }),
      };
    });
    return withItemIndexes(nextItems);
  }),

  duplicateItems: (itemIds, positions) => {
    const state = get();
    const itemsMap = new Map(state.items.map((i) => [i.id, i]));
    const newItems: TimelineItem[] = [];

    for (let i = 0; i < itemIds.length; i++) {
      const original = itemsMap.get(itemIds[i]!);
      const position = positions[i]!;
      if (!original || !position) continue;

      const duplicate: TimelineItem = {
        ...original,
        id: crypto.randomUUID(),
        from: roundFrame(position.from),
        trackId: position.trackId,
        originId: crypto.randomUUID(),
      };
      newItems.push(duplicate);
    }

    set((state) => withItemIndexes([...state.items, ...newItems]));
    return newItems;
  },

  trimItemStart: (id, trimAmount) => set((state) => {
    const nextItems = state.items.map((item) => {
      if (item.id !== id) return item;
      const clamped = Math.max(-item.from, Math.min(item.durationInFrames - 1, trimAmount));
      const newFrom = item.from + clamped;
      const newDuration = item.durationInFrames - clamped;
      if (newDuration <= 0) return item;
      return { ...item, from: roundFrame(newFrom), durationInFrames: roundDuration(newDuration) };
    });
    return withItemIndexes(nextItems);
  }),

  trimItemEnd: (id, trimAmount) => set((state) => {
    const nextItems = state.items.map((item) => {
      if (item.id !== id) return item;
      const newDuration = item.durationInFrames + trimAmount;
      if (newDuration <= 0) return item;
      return { ...item, durationInFrames: roundDuration(newDuration) };
    });
    return withItemIndexes(nextItems);
  }),

  splitItem: (id, splitFrame) => {
    const state = get();
    const item = state.items.find((i) => i.id === id);
    if (!item) return null;
    const splitAt = roundFrame(splitFrame);
    const itemEnd = item.from + item.durationInFrames;
    if (splitAt <= item.from || splitAt >= itemEnd) return null;

    const leftDuration = splitAt - item.from;
    const rightDuration = itemEnd - splitAt;
    const splitOriginId = item.originId ?? item.id;

    const leftItem: TimelineItem = {
      ...item,
      originId: splitOriginId,
      durationInFrames: leftDuration,
    };

    const rightItem: TimelineItem = {
      ...item,
      id: crypto.randomUUID(),
      originId: splitOriginId,
      from: splitAt,
      durationInFrames: rightDuration,
    };

    // Update source boundaries for media items
    if ((item.type === 'video' || item.type === 'audio') && item.sourceStart !== undefined) {
      const sourceFps = item.sourceFps ?? state.fps;
      const speed = item.speed ?? 1;
      const sourceStart = item.sourceStart ?? 0;
      const sourceFramesLeft = Math.round(leftDuration * speed * (sourceFps / state.fps));

      leftItem.sourceEnd = sourceStart + sourceFramesLeft;
      rightItem.sourceStart = sourceStart + sourceFramesLeft;
      if (item.sourceEnd !== undefined) {
        rightItem.sourceEnd = item.sourceEnd;
      }
    }

    set((state) => {
      const nextItems = state.items
        .map((i) => (i.id === id ? leftItem : i))
        .concat(rightItem);
      return withItemIndexes(nextItems);
    });

    return { leftItem, rightItem };
  },

  // Track operations
  addTrack: (track) => set((state) => ({
    tracks: [...state.tracks, track].sort((a, b) => a.order - b.order),
  })),

  removeTrack: (id) => set((state) => ({
    tracks: state.tracks.filter((t) => t.id !== id),
    ...withItemIndexes(state.items.filter((i) => i.trackId !== id)),
  })),

  updateTrack: (id, updates) => set((state) => ({
    tracks: state.tracks.map((t) => (t.id === id ? { ...t, ...updates } : t)),
  })),

  toggleTrackLock: (id) => set((state) => ({
    tracks: state.tracks.map((t) => (t.id === id ? { ...t, locked: !t.locked } : t)),
  })),

  toggleTrackVisibility: (id) => set((state) => ({
    tracks: state.tracks.map((t) => (t.id === id ? { ...t, visible: !t.visible } : t)),
  })),

  toggleTrackMute: (id) => set((state) => ({
    tracks: state.tracks.map((t) => (t.id === id ? { ...t, muted: !t.muted } : t)),
  })),

  toggleTrackSolo: (id) => set((state) => ({
    tracks: state.tracks.map((t) => (t.id === id ? { ...t, solo: !t.solo } : t)),
  })),

  // Transform operations
  updateItemTransform: (id, transform) => set((state) => {
    const nextItems = state.items.map((item) => {
      if (item.id !== id) return item;
      return { ...item, transform: { ...item.transform, ...transform } };
    });
    return withItemIndexes(nextItems);
  }),

  resetItemTransform: (id) => set((state) => {
    const nextItems = state.items.map((item) => {
      if (item.id !== id) return item;
      return { ...item, transform: { x: 0, y: 0, rotation: 0 } };
    });
    return withItemIndexes(nextItems);
  }),
}));
