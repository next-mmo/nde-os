import { createStore } from "zustand/vanilla";

export interface Bookmark {
  id: string;
  title: string;
  url: string;
  icon?: string;
  isSpeedDial?: boolean;
}

export interface BrowserState {
  bookmarks: Bookmark[];
}

export interface BrowserActions {
  addBookmark: (bookmark: Omit<Bookmark, "id">) => void;
  removeBookmark: (id: string) => void;
  loadBookmarks: () => void;
}

const DEFAULT_SPEED_DIALS: Bookmark[] = [
  {
    id: "sd-1",
    title: "ChatGPT",
    url: "https://chatgpt.com",
    icon: "🤖",
    isSpeedDial: true,
  },
  {
    id: "sd-2",
    title: "Claude AI",
    url: "https://claude.ai",
    icon: "🧠",
    isSpeedDial: true,
  },
  {
    id: "sd-3",
    title: "Gemini",
    url: "https://gemini.google.com",
    icon: "✨",
    isSpeedDial: true,
  },
  {
    id: "sd-4",
    title: "ReelShort Drama",
    url: "https://m.quickplay.my.id/drama/69c23f5368325d2264072f05?category=reelshort&lang=en",
    icon: "🎬",
    isSpeedDial: true,
  },
  {
    id: "sd-5",
    title: "Music HiFi",
    url: "https://music-hifi.vercel.app/",
    icon: "🎵",
    isSpeedDial: true,
  },
];

export const browserStore = createStore<BrowserState & BrowserActions>()((set, get) => ({
  bookmarks: [...DEFAULT_SPEED_DIALS],

  addBookmark: (bookmark) => {
    set((state) => {
      const newBookmarks = [
        ...state.bookmarks,
        { ...bookmark, id: crypto.randomUUID() },
      ];
      localStorage.setItem("nde_browser_bookmarks", JSON.stringify(newBookmarks));
      return { bookmarks: newBookmarks };
    });
  },

  removeBookmark: (id) => {
    set((state) => {
      // Don't allow removing default speed dials for now, or maybe allow it
      const newBookmarks = state.bookmarks.filter((b) => b.id !== id);
      localStorage.setItem("nde_browser_bookmarks", JSON.stringify(newBookmarks));
      return { bookmarks: newBookmarks };
    });
  },

  loadBookmarks: () => {
    try {
      const stored = localStorage.getItem("nde_browser_bookmarks");
      if (stored) {
        const parsed = JSON.parse(stored);
        if (Array.isArray(parsed) && parsed.length > 0) {
          set({ bookmarks: parsed });
          return;
        }
      }
    } catch (e) {
      console.error("Failed to load bookmarks", e);
    }
    set({ bookmarks: [...DEFAULT_SPEED_DIALS] });
  },
}));
