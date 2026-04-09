/**
 * Zustand → Svelte 5 adapter (shared).
 *
 * Subscribes to a zustand vanilla store and synchronizes its state
 * into a Svelte 5 `$state` rune, so components reactively update
 * whenever the zustand store changes.
 *
 * Usage:
 *   import { useStore } from '$lib/use-store.svelte';
 *   import { myStore } from '$state/my-store';
 *   const s = useStore(myStore);
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
    if (typeof state.current !== 'object' || state.current === null) {
      console.warn("useStore: selector must return an object to maintain proxy reactivity");
      return state.current as U;
    }
  }

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
