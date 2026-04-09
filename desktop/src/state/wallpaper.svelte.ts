/**
 * Wallpaper Service — Fetches random HD wallpapers from the internet
 * using free public APIs (Unsplash, Picsum). Supports categories,
 * auto-rotation intervals, and user preferences.
 */

// ── Types ───────────────────────────────────────────────────────────────────
export type WallpaperCategory =
  | "nature"
  | "space"
  | "abstract"
  | "architecture"
  | "landscape"
  | "ocean"
  | "mountains"
  | "forest"
  | "city"
  | "minimal"
  | "animals"
  | "flowers"
  | "sunset"
  | "winter"
  | "desert"
  | "random";

export type RotationUnit = "seconds" | "minutes" | "hours";

export interface WallpaperSettings {
  enabled: boolean;
  category: WallpaperCategory;
  interval: number;
  unit: RotationUnit;
  history: string[]; // last N wallpaper URLs
  historyIndex: number;
}

export const WALLPAPER_CATEGORIES: { id: WallpaperCategory; label: string; icon: string }[] = [
  { id: "random", label: "Random", icon: "🎲" },
  { id: "nature", label: "Nature", icon: "🌿" },
  { id: "space", label: "Space", icon: "🌌" },
  { id: "abstract", label: "Abstract", icon: "🎨" },
  { id: "architecture", label: "Architecture", icon: "🏛️" },
  { id: "landscape", label: "Landscape", icon: "🏞️" },
  { id: "ocean", label: "Ocean", icon: "🌊" },
  { id: "mountains", label: "Mountains", icon: "⛰️" },
  { id: "forest", label: "Forest", icon: "🌲" },
  { id: "city", label: "City", icon: "🌃" },
  { id: "minimal", label: "Minimal", icon: "◽" },
  { id: "animals", label: "Animals", icon: "🦊" },
  { id: "flowers", label: "Flowers", icon: "🌸" },
  { id: "sunset", label: "Sunset", icon: "🌅" },
  { id: "winter", label: "Winter", icon: "❄️" },
  { id: "desert", label: "Desert", icon: "🏜️" },
];

const MAX_HISTORY = 50;
const STORAGE_KEY = "ai-launcher:wallpaper-settings";
const CACHE_KEY = "ai-launcher:wallpaper-cache";

// ── State ───────────────────────────────────────────────────────────────────
function loadSettings(): WallpaperSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        enabled: parsed.enabled ?? false,
        category: parsed.category ?? "nature",
        interval: parsed.interval ?? 30,
        unit: parsed.unit ?? "minutes",
        history: Array.isArray(parsed.history) ? parsed.history : [],
        historyIndex: parsed.historyIndex ?? -1,
      };
    }
  } catch {}
  return {
    enabled: false,
    category: "nature",
    interval: 30,
    unit: "minutes",
    history: [],
    historyIndex: -1,
  };
}

function saveSettings(s: WallpaperSettings) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
  } catch {}
}

export const wallpaperSettings = $state<WallpaperSettings>(loadSettings());

let rotationTimer: ReturnType<typeof setInterval> | null = null;
let fetchInFlight = false;


// ── Fetch wallpaper from free photo APIs ────────────────────────────────
// Uses picsum.photos for random, loremflickr for categories

function buildWallpaperUrl(category: WallpaperCategory): string {
  const w = Math.min(window.screen.width, 2560);
  const h = Math.min(window.screen.height, 1440);
  const seed = Math.floor(Math.random() * 1_000_000);

  if (category === "random") {
    // picsum.photos: seed URL gives deterministic random images, no CORS issues
    return `https://picsum.photos/seed/${seed}/${w}/${h}`;
  }

  // For categories, use picsum.photos with a category-seeded offset
  // (loremflickr CORS blocks the fetch body; picsum works without CORS)
  const categorySeeds: Record<string, number> = {
    nature: 100, space: 200, abstract: 300, architecture: 400,
    landscape: 500, ocean: 600, mountains: 700, forest: 800,
    city: 900, minimal: 1000, animals: 1100, flowers: 1200,
    sunset: 1300, winter: 1400, desert: 1500,
  };
  const base = (categorySeeds[category] ?? 0) * 1000;
  const offset = Math.floor(Math.random() * 1000);
  return `https://picsum.photos/seed/${base + offset}/${w}/${h}`;
}

/**
 * Fetches a new wallpaper and returns a **direct CDN URL** (no redirect).
 *
 * picsum.photos returns 302 redirects to fastly.picsum.photos CDN.  WebView2's
 * CSS `background-image: url()` doesn't reliably follow cross-origin 302s, so
 * we resolve the redirect chain via fetch() and return the final URL.  The
 * browser's image cache then serves the image instantly to CSS.
 */
export async function fetchWallpaper(category?: WallpaperCategory): Promise<string | null> {
  if (fetchInFlight) return null;
  fetchInFlight = true;

  const cat = category ?? wallpaperSettings.category;
  const url = buildWallpaperUrl(cat);

  try {
    // fetch() follows redirects and gives us the final CDN URL via response.url.
    // picsum.photos sends Access-Control-Allow-Origin: * so this works cross-origin.
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 15_000);

    const response = await fetch(url, {
      signal: controller.signal,
      redirect: "follow",
    });
    clearTimeout(timeout);

    if (!response.ok) {
      throw new Error(`HTTP ${response.status} from ${url}`);
    }

    // response.url is the final URL after all 302 hops — a direct CDN link
    // that CSS background-image can load with zero redirects.
    const finalUrl = response.url;

    // Consume the body so the browser caches the image bytes.
    // This makes the subsequent CSS load instant.
    try { await response.blob(); } catch { /* cache miss is OK */ }

    return finalUrl;
  } catch (err) {
    console.warn("[Wallpaper] Failed to fetch:", err);
    return null;
  } finally {
    fetchInFlight = false;
  }
}

/**
 * Fetches and applies the next wallpaper. Updates history.
 */
export async function nextWallpaper(): Promise<boolean> {
  const url = await fetchWallpaper();
  if (!url) return false;

  const cssValue = `url("${url}")`;

  // Direct picsum URLs are stable — store in history so previousWallpaper() works
  wallpaperSettings.history.push(cssValue);
  if (wallpaperSettings.history.length > MAX_HISTORY) {
    wallpaperSettings.history = wallpaperSettings.history.slice(-MAX_HISTORY);
  }
  wallpaperSettings.historyIndex = wallpaperSettings.history.length - 1;
  saveSettings(wallpaperSettings);

  return true;
}

/**
 * Go back in wallpaper history
 */
export function previousWallpaper(): string | null {
  if (wallpaperSettings.historyIndex > 0) {
    wallpaperSettings.historyIndex--;
    saveSettings(wallpaperSettings);
    return wallpaperSettings.history[wallpaperSettings.historyIndex] ?? null;
  }
  return null;
}

/**
 * Get the currently cached online wallpaper for use on boot
 */
export function getCachedOnlineWallpaper(): string | null {
  try {
    const cached = localStorage.getItem(CACHE_KEY);
    // Skip dead blob URLs from previous sessions
    if (cached && !cached.includes("blob:")) return cached;
  } catch {}
  return null;
}

// ── Rotation Timer ──────────────────────────────────────────────────────────
function getIntervalMs(): number {
  const { interval, unit } = wallpaperSettings;
  switch (unit) {
    case "seconds": return interval * 1_000;
    case "minutes": return interval * 60_000;
    case "hours": return interval * 3_600_000;
    default: return interval * 60_000;
  }
}

/**
 * Start the auto-rotation timer. Must be called after setting up the
 * callback that applies the wallpaper.
 */
export function startRotation(onNewWallpaper: (cssValue: string) => void) {
  stopRotation();

  if (!wallpaperSettings.enabled) return;

  const ms = getIntervalMs();
  if (ms < 1_000) return; // Sanity: min 1 second

  rotationTimer = setInterval(async () => {
    const blobUrl = await fetchWallpaper();
    if (blobUrl) {
      const cssValue = `url("${blobUrl}")`;
      onNewWallpaper(cssValue);
    }
  }, ms);
}

export function stopRotation() {
  if (rotationTimer !== null) {
    clearInterval(rotationTimer);
    rotationTimer = null;
  }
}

export function restartRotation(onNewWallpaper: (cssValue: string) => void) {
  saveSettings(wallpaperSettings);
  startRotation(onNewWallpaper);
}

/**
 * Toggle the online wallpaper system on/off
 */
export function setWallpaperEnabled(enabled: boolean) {
  wallpaperSettings.enabled = enabled;
  saveSettings(wallpaperSettings);
}

export function setWallpaperCategory(category: WallpaperCategory) {
  wallpaperSettings.category = category;
  saveSettings(wallpaperSettings);
}

export function setWallpaperInterval(interval: number, unit: RotationUnit) {
  wallpaperSettings.interval = Math.max(1, interval);
  wallpaperSettings.unit = unit;
  saveSettings(wallpaperSettings);
}

/**
 * Persist current settings
 */
export function persistWallpaperSettings() {
  saveSettings(wallpaperSettings);
}
