/**
 * Zustand → Svelte 5 adapter.
 *
 * Subscribes to a zustand vanilla store and synchronizes its state
 * into a Svelte 5 `$state` rune, so components reactively update
 * whenever the zustand store changes.
 *
 * Usage in a .svelte component:
 *
 *   import { useStore } from './lib/use-store.svelte';
 *   import { playbackStore } from './stores/playback';
 *
 *   const pb = useStore(playbackStore);
 *   // pb.currentFrame, pb.play(), pb.pause() — all reactive
 */

import type { StoreApi } from 'zustand';

export function useStore<T extends object>(store: StoreApi<T>): T;
export function useStore<T extends object, U>(store: StoreApi<T>, selector: (s: T) => U): U;
export function useStore<T extends object, U>(store: StoreApi<T>, selector?: (s: T) => U): T | U {
  let state = $state({ current: selector ? selector(store.getState()) : store.getState() });

  $effect(() => {
    const unsub = store.subscribe((next) => {
      state.current = selector ? selector(next) : next;
    });
    return unsub;
  });

  if (selector) {
    // For single-value selectors, we have to return an object getter.
    // In practice this project uses the full store approach without selector.
    // So we'll just handle scalar selector values safely.
    if (typeof state.current !== 'object' || state.current === null) {
      // Not a proxyable type, fallback to returning value (will lose reactivity)
      console.warn("useStore: selector must return an object to maintain proxy reactivity");
      return state.current as U;
    }
  }

  // Create a stable object that forwards getters/setters to the latest reactive state.
  return new Proxy({} as T | U, {
    get(_, prop) {
      return (state.current as any)[prop];
    },
    set() {
      console.warn("useStore: State should be mutated through store actions, not directly via Proxy.");
      return false;
    }
  });
}

export function getSnapshot<T>(store: StoreApi<T>): T {
  return store.getState();
}
