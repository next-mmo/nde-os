/**
 * FreeCut selection store — ported from React Zustand to zustand/vanilla.
 *
 * Manages item, track, marker, and transition selection with mutual exclusion.
 */

import { createStore } from 'zustand/vanilla';

export type SelectionType = 'item' | 'track' | 'marker' | 'transition' | null;
export type ActiveTool = 'select' | 'razor' | 'slip' | 'slide' | 'ripple' | 'rolling' | 'rate-stretch';

export interface DragState {
  type: string;
  startX: number;
  startY: number;
  itemId?: string;
  trackId?: string;
}

export interface SelectionState {
  selectedItemIds: string[];
  selectedMarkerId: string | null;
  selectedTransitionId: string | null;
  selectedTrackId: string | null;
  selectedTrackIds: string[];
  activeTrackId: string | null;
  selectionType: SelectionType;
  activeTool: ActiveTool;
  dragState: DragState | null;
  expandedKeyframeLanes: Set<string>;
}

export interface SelectionActions {
  selectItems: (ids: string[]) => void;
  selectMarker: (id: string | null) => void;
  selectTransition: (id: string | null) => void;
  selectTrack: (id: string | null) => void;
  selectTracks: (ids: string[], append?: boolean) => void;
  setActiveTrack: (id: string | null) => void;
  toggleTrackSelection: (id: string) => void;
  clearSelection: () => void;
  clearItemSelection: () => void;
  setDragState: (dragState: DragState | null) => void;
  setActiveTool: (tool: ActiveTool) => void;
  toggleKeyframeLanes: (itemId: string) => void;
  setKeyframeLanesExpanded: (itemId: string, expanded: boolean) => void;
}

export const selectionStore = createStore<SelectionState & SelectionActions>()((set) => ({
  // State
  selectedItemIds: [],
  selectedMarkerId: null,
  selectedTransitionId: null,
  selectedTrackId: null,
  selectedTrackIds: [],
  activeTrackId: null,
  selectionType: null,
  activeTool: 'select',
  dragState: null,
  expandedKeyframeLanes: new Set<string>(),

  // Actions
  selectItems: (ids) => set((state) => ({
    selectedItemIds: ids,
    selectedMarkerId: null,
    selectedTransitionId: null,
    selectionType: ids.length > 0 ? 'item' : (state.selectedTrackIds.length > 0 ? 'track' : null),
  })),

  selectMarker: (id) => set({
    selectedMarkerId: id,
    selectedTransitionId: null,
    selectedItemIds: [],
    selectionType: id ? 'marker' : null,
  }),

  selectTransition: (id) => set({
    selectedTransitionId: id,
    selectedMarkerId: null,
    selectedItemIds: [],
    selectionType: id ? 'transition' : null,
  }),

  selectTrack: (id) => set({
    selectedTrackId: id,
    activeTrackId: id,
    selectedTrackIds: id ? [id] : [],
    selectedItemIds: [],
    selectedMarkerId: null,
    selectionType: id ? 'track' : null,
  }),

  selectTracks: (ids, append = false) => set((state) => {
    const newSelectedIds = append
      ? Array.from(new Set([...state.selectedTrackIds, ...ids]))
      : ids;
    return {
      selectedTrackIds: newSelectedIds,
      activeTrackId: ids[0] || null,
      selectedTrackId: ids[0] || null,
      selectedItemIds: [],
      selectedMarkerId: null,
      selectionType: newSelectedIds.length > 0 ? 'track' : null,
    };
  }),

  setActiveTrack: (id) => set({
    activeTrackId: id,
    selectedTrackId: id,
    selectedTrackIds: id ? [id] : [],
    selectedItemIds: [],
    selectedMarkerId: null,
    selectionType: id ? 'track' : null,
  }),

  toggleTrackSelection: (id) => set((state) => {
    const isSelected = state.selectedTrackIds.includes(id);
    const newSelectedIds = isSelected
      ? state.selectedTrackIds.filter(trackId => trackId !== id)
      : [...state.selectedTrackIds, id];
    return {
      selectedTrackIds: newSelectedIds,
      activeTrackId: newSelectedIds[0] || null,
      selectedTrackId: newSelectedIds[0] || null,
      selectedItemIds: [],
      selectedMarkerId: null,
      selectionType: newSelectedIds.length > 0 ? 'track' : null,
    };
  }),

  clearSelection: () => set({
    selectedItemIds: [],
    selectedMarkerId: null,
    selectedTransitionId: null,
    selectedTrackId: null,
    selectedTrackIds: [],
    activeTrackId: null,
    selectionType: null,
  }),

  clearItemSelection: () => set((state) => ({
    selectedItemIds: [],
    selectionType: state.selectedTrackIds.length > 0 ? 'track' : null,
  })),

  setDragState: (dragState) => set({ dragState }),
  setActiveTool: (tool) => set({ activeTool: tool }),

  toggleKeyframeLanes: (itemId) => set((state) => {
    const newSet = new Set(state.expandedKeyframeLanes);
    if (newSet.has(itemId)) newSet.delete(itemId); else newSet.add(itemId);
    return { expandedKeyframeLanes: newSet };
  }),

  setKeyframeLanesExpanded: (itemId, expanded) => set((state) => {
    const newSet = new Set(state.expandedKeyframeLanes);
    if (expanded) newSet.add(itemId); else newSet.delete(itemId);
    return { expandedKeyframeLanes: newSet };
  }),
}));
