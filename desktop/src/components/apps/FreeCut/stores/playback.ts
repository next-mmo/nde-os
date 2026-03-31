/**
 * FreeCut playback store — ported from React Zustand to zustand/vanilla.
 *
 * Controls current frame, play/pause, scrub, volume, zoom, and playback rate.
 * Connected to the Rust backend for frame rendering via Tauri events.
 */

import { createStore } from 'zustand/vanilla';

export type PreviewQuality = 1 | 0.5 | 0.33 | 0.25;

export interface PlaybackState {
  currentFrame: number;
  currentFrameEpoch: number;
  displayedFrame: number | null;
  isPlaying: boolean;
  playbackRate: number;
  loop: boolean;
  volume: number;
  muted: boolean;
  zoom: number;
  previewFrame: number | null;
  previewFrameEpoch: number;
  frameUpdateEpoch: number;
  previewItemId: string | null;
  useProxy: boolean;
  previewQuality: PreviewQuality;
}

export interface PlaybackActions {
  setCurrentFrame: (frame: number) => void;
  setScrubFrame: (frame: number, itemId?: string) => void;
  play: () => void;
  pause: () => void;
  togglePlayPause: () => void;
  setPlaybackRate: (rate: number) => void;
  toggleLoop: () => void;
  setVolume: (volume: number) => void;
  toggleMute: () => void;
  setZoom: (zoom: number) => void;
  setPreviewFrame: (frame: number | null, itemId?: string) => void;
  setDisplayedFrame: (frame: number | null) => void;
  toggleUseProxy: () => void;
  setPreviewQuality: (quality: PreviewQuality) => void;
}

function normalizeFrame(frame: number): number {
  if (!Number.isFinite(frame)) return 0;
  return Math.max(0, Math.round(frame));
}

function normalizePreviewQuality(quality: PreviewQuality): PreviewQuality {
  if (quality === 0.5 || quality === 0.33 || quality === 0.25) return quality;
  return 1;
}

export const playbackStore = createStore<PlaybackState & PlaybackActions>()((set) => ({
  // State
  currentFrame: 0,
  currentFrameEpoch: 0,
  displayedFrame: null,
  isPlaying: false,
  playbackRate: 1,
  loop: false,
  volume: 1,
  muted: false,
  zoom: -1, // -1 = auto-fit
  previewFrame: null,
  previewFrameEpoch: 0,
  frameUpdateEpoch: 0,
  previewItemId: null,
  useProxy: true,
  previewQuality: 1 as PreviewQuality,

  // Actions
  setCurrentFrame: (frame) =>
    set((state) => {
      const nextFrame = normalizeFrame(frame);
      if (state.currentFrame === nextFrame) return state;
      const nextEpoch = state.frameUpdateEpoch + 1;
      return {
        currentFrame: nextFrame,
        currentFrameEpoch: nextEpoch,
        frameUpdateEpoch: nextEpoch,
      };
    }),

  setScrubFrame: (frame, itemId) =>
    set((state) => {
      const nextFrame = normalizeFrame(frame);
      const nextItemId = itemId ?? null;
      if (
        state.currentFrame === nextFrame &&
        state.previewFrame === nextFrame &&
        state.previewItemId === nextItemId
      ) {
        return state;
      }
      const nextEpoch = state.frameUpdateEpoch + 1;
      return {
        currentFrame: nextFrame,
        currentFrameEpoch: nextEpoch,
        previewFrame: nextFrame,
        previewItemId: nextItemId,
        previewFrameEpoch: nextEpoch,
        frameUpdateEpoch: nextEpoch,
      };
    }),

  play: () => set((state) => (state.isPlaying ? state : { isPlaying: true })),
  pause: () => set((state) => (state.isPlaying ? { isPlaying: false } : state)),
  togglePlayPause: () => set((state) => ({ isPlaying: !state.isPlaying })),
  setPlaybackRate: (rate) => set({ playbackRate: rate }),
  toggleLoop: () => set((state) => ({ loop: !state.loop })),
  setVolume: (volume) => set({ volume }),
  toggleMute: () => set((state) => ({ muted: !state.muted })),
  setZoom: (zoom) => set({ zoom }),

  setPreviewFrame: (frame, itemId) =>
    set((state) => {
      const nextFrame = frame == null ? null : normalizeFrame(frame);
      const nextItemId = frame == null ? null : (itemId ?? null);
      if (state.previewFrame === nextFrame && state.previewItemId === nextItemId) {
        return state;
      }
      const nextEpoch = state.frameUpdateEpoch + 1;
      return {
        previewFrame: nextFrame,
        previewItemId: nextItemId,
        previewFrameEpoch: nextEpoch,
        frameUpdateEpoch: nextEpoch,
      };
    }),

  setDisplayedFrame: (frame) =>
    set((state) => {
      const nextFrame = frame == null ? null : normalizeFrame(frame);
      if (state.displayedFrame === nextFrame) return state;
      return { displayedFrame: nextFrame };
    }),

  toggleUseProxy: () => set((state) => ({ useProxy: !state.useProxy })),

  setPreviewQuality: (quality) =>
    set((state) => {
      const nextQuality = normalizePreviewQuality(quality);
      if (state.previewQuality === nextQuality) return state;
      return { previewQuality: nextQuality };
    }),
}));
