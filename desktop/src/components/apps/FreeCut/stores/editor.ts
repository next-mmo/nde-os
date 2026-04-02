/**
 * FreeCut editor layout store — ported from React Zustand to zustand/vanilla.
 *
 * Controls panel visibility, sidebar widths, active tabs, and timeline height.
 */

import { createStore } from 'zustand/vanilla';

export type EditorTab = 'media' | 'effects' | 'transitions' | 'text' | 'audio' | 'shapes' | 'dubbing';
export type ClipInspectorTab = 'transform' | 'effects' | 'audio' | 'speed' | 'color';

export interface EditorState {
  activePanel: string | null;
  leftSidebarOpen: boolean;
  rightSidebarOpen: boolean;
  keyframeEditorOpen: boolean;
  activeTab: EditorTab;
  clipInspectorTab: ClipInspectorTab;
  sidebarWidth: number;
  rightSidebarWidth: number;
  timelineHeight: number;
  sourcePreviewMediaId: string | null;
  linkedSelectionEnabled: boolean;
  colorScopesOpen: boolean;
}

export interface EditorActions {
  setActivePanel: (panel: string | null) => void;
  setLeftSidebarOpen: (open: boolean) => void;
  setRightSidebarOpen: (open: boolean) => void;
  setKeyframeEditorOpen: (open: boolean) => void;
  toggleLeftSidebar: () => void;
  toggleRightSidebar: () => void;
  setActiveTab: (tab: EditorTab) => void;
  setClipInspectorTab: (tab: ClipInspectorTab) => void;
  setSidebarWidth: (width: number) => void;
  setRightSidebarWidth: (width: number) => void;
  setTimelineHeight: (height: number) => void;
  setSourcePreviewMediaId: (mediaId: string | null) => void;
  toggleLinkedSelectionEnabled: () => void;
  toggleColorScopesOpen: () => void;
}

export const editorStore = createStore<EditorState & EditorActions>()((set) => ({
  // State
  activePanel: null,
  leftSidebarOpen: true,
  rightSidebarOpen: true,
  keyframeEditorOpen: false,
  activeTab: 'media',
  clipInspectorTab: 'transform',
  sidebarWidth: 340,
  rightSidebarWidth: 280,
  timelineHeight: 250,
  sourcePreviewMediaId: null,
  linkedSelectionEnabled: true,
  colorScopesOpen: false,

  // Actions
  setActivePanel: (panel) => set({ activePanel: panel }),
  setLeftSidebarOpen: (open) => set({ leftSidebarOpen: open }),
  setRightSidebarOpen: (open) => set({ rightSidebarOpen: open }),
  setKeyframeEditorOpen: (open) => set((state) => ({
    keyframeEditorOpen: open,
    leftSidebarOpen: open ? true : state.leftSidebarOpen,
  })),
  toggleLeftSidebar: () => set((state) => ({ leftSidebarOpen: !state.leftSidebarOpen })),
  toggleRightSidebar: () => set((state) => ({ rightSidebarOpen: !state.rightSidebarOpen })),
  setActiveTab: (tab) => set({ activeTab: tab }),
  setClipInspectorTab: (tab) => set({ clipInspectorTab: tab }),
  setSidebarWidth: (width) => set({ sidebarWidth: width }),
  setRightSidebarWidth: (width) => set({ rightSidebarWidth: width }),
  setTimelineHeight: (height) => set({ timelineHeight: height }),
  setSourcePreviewMediaId: (mediaId) => set({ sourcePreviewMediaId: mediaId }),
  toggleLinkedSelectionEnabled: () => set((state) => ({ linkedSelectionEnabled: !state.linkedSelectionEnabled })),
  toggleColorScopesOpen: () => set((state) => ({ colorScopesOpen: !state.colorScopesOpen })),
}));
