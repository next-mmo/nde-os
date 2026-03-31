/**
 * FreeCut undo/redo history store.
 *
 * Snapshot-based: captures items + tracks state before each edit.
 * Max 50 undo levels.
 */

import { createStore } from 'zustand/vanilla';
import { itemsStore, type TimelineItem, type TimelineTrack } from './items';

interface Snapshot {
  items: TimelineItem[];
  tracks: TimelineTrack[];
}

export interface HistoryState {
  past: Snapshot[];
  future: Snapshot[];
}

export interface HistoryActions {
  push: () => void;
  undo: () => void;
  redo: () => void;
  clear: () => void;
}

export const historyStore = createStore<HistoryState & HistoryActions>()((set, get) => ({
  past: [],
  future: [],

  push: () => {
    const { items, tracks } = itemsStore.getState();
    set((s) => ({
      past: s.past.slice(-49).concat({
        items: items.map((i) => ({ ...i })),
        tracks: tracks.map((t) => ({ ...t })),
      }),
      future: [],
    }));
  },

  undo: () => {
    const { past } = get();
    if (past.length === 0) return;

    const { items, tracks } = itemsStore.getState();
    const current: Snapshot = {
      items: items.map((i) => ({ ...i })),
      tracks: tracks.map((t) => ({ ...t })),
    };

    const prev = past[past.length - 1]!;
    itemsStore.getState().setItems(prev.items);
    itemsStore.getState().setTracks(prev.tracks);

    set((s) => ({
      past: s.past.slice(0, -1),
      future: [current, ...s.future],
    }));
  },

  redo: () => {
    const { future } = get();
    if (future.length === 0) return;

    const { items, tracks } = itemsStore.getState();
    const current: Snapshot = {
      items: items.map((i) => ({ ...i })),
      tracks: tracks.map((t) => ({ ...t })),
    };

    const next = future[0]!;
    itemsStore.getState().setItems(next.items);
    itemsStore.getState().setTracks(next.tracks);

    set((s) => ({
      past: [...s.past, current],
      future: s.future.slice(1),
    }));
  },

  clear: () => set({ past: [], future: [] }),
}));
