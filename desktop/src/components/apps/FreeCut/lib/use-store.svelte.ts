/**
 * Zustand → Svelte 5 adapter (performance-optimized).
 *
 * Subscribes to a zustand vanilla store and synchronizes its state
 * into a Svelte 5 `$state` rune, so components reactively update
 * whenever the zustand store changes.
 *
 * PERF: Uses shallow equality to skip Svelte state updates when the
 * zustand state hasn't meaningfully changed. This prevents cascading
 * DOM diffs (the #1 cause of timeline lag in the previous version).
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

/**
 * Shallow-compare two objects. Returns true if they are the same reference
 * or if all own keys are === equal (ignoring functions which never change).
 */
function shallowEqual(a: any, b: any): boolean {
  if (a === b) return true;
  if (!a || !b) return false;
  const keysA = Object.keys(a);
  const keysB = Object.keys(b);
  if (keysA.length !== keysB.length) return false;
  for (let i = 0; i < keysA.length; i++) {
    const key = keysA[i]!;
    const va = a[key];
    const vb = b[key];
    // Skip function comparisons — store actions never change identity
    if (typeof va === 'function') continue;
    if (va !== vb) return false;
  }
  return true;
}

export function useStore<T extends object>(store: StoreApi<T>): T;
export function useStore<T extends object, U>(store: StoreApi<T>, selector: (s: T) => U): U;
export function useStore<T extends object, U>(store: StoreApi<T>, selector?: (s: T) => U): T | U {
  let state = $state({ current: selector ? selector(store.getState()) : store.getState() });

  $effect(() => {
    const unsub = store.subscribe((next) => {
      const derived = selector ? selector(next) : next;
      // PERF: Skip Svelte $state update if nothing meaningful changed.
      // This is critical because zustand fires on every `set()` call,
      // even if the resulting state is identical (e.g. setCurrentFrame
      // during playback fires 30×/s but the reactive UI doesn't need
      // to re-render on every frame).
      if (shallowEqual(state.current, derived)) return;
      state.current = derived;
    });
    return unsub;
  });

  if (selector) {
    if (typeof state.current !== 'object' || state.current === null) {
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
