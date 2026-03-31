/**
 * FreeCut zoom store — ported from React Zustand to zustand/vanilla.
 *
 * Controls timeline zoom level with throttled updates for smooth scrolling.
 */

import { createStore } from 'zustand/vanilla';

export interface ZoomState {
  level: number;
  pixelsPerSecond: number;
}

export interface ZoomActions {
  setZoomLevel: (level: number) => void;
  setZoomLevelImmediate: (level: number) => void;
  zoomIn: () => void;
  zoomOut: () => void;
  zoomToFit: (containerWidth: number, contentDurationSeconds: number) => void;
}

const ZOOM_THROTTLE_MS = 120;
let lastZoomUpdate = 0;
let pendingZoomLevel: number | null = null;
let zoomThrottleTimeout: ReturnType<typeof setTimeout> | null = null;

export const zoomStore = createStore<ZoomState & ZoomActions>()((set) => ({
  level: 1,
  pixelsPerSecond: 100,

  setZoomLevel: (level) => {
    const now = performance.now();
    pendingZoomLevel = level;

    if (now - lastZoomUpdate >= ZOOM_THROTTLE_MS) {
      lastZoomUpdate = now;
      set({ level, pixelsPerSecond: level * 100 });
      pendingZoomLevel = null;
      return;
    }

    if (!zoomThrottleTimeout) {
      zoomThrottleTimeout = setTimeout(() => {
        zoomThrottleTimeout = null;
        if (pendingZoomLevel !== null) {
          lastZoomUpdate = performance.now();
          set({ level: pendingZoomLevel, pixelsPerSecond: pendingZoomLevel * 100 });
          pendingZoomLevel = null;
        }
      }, ZOOM_THROTTLE_MS - (now - lastZoomUpdate));
    }
  },

  setZoomLevelImmediate: (level) => {
    if (zoomThrottleTimeout) {
      clearTimeout(zoomThrottleTimeout);
      zoomThrottleTimeout = null;
    }
    pendingZoomLevel = null;
    lastZoomUpdate = performance.now();
    set({ level, pixelsPerSecond: level * 100 });
  },

  zoomIn: () => set((state) => {
    const newLevel = Math.min(state.level * 1.1, 50);
    return { level: newLevel, pixelsPerSecond: newLevel * 100 };
  }),

  zoomOut: () => set((state) => {
    const newLevel = Math.max(state.level / 1.1, 0.01);
    return { level: newLevel, pixelsPerSecond: newLevel * 100 };
  }),

  zoomToFit: (containerWidth, contentDurationSeconds) => {
    if (contentDurationSeconds <= 0) return;
    const newLevel = containerWidth / (contentDurationSeconds * 100);
    set({ level: newLevel, pixelsPerSecond: newLevel * 100 });
  },
}));
