import { beforeEach, describe, expect, it, vi } from "vitest";

class MemoryStorage {
  private store = new Map<string, string>();

  clear() {
    this.store.clear();
  }

  getItem(key: string) {
    return this.store.has(key) ? this.store.get(key)! : null;
  }

  key(index: number) {
    return [...this.store.keys()][index] ?? null;
  }

  removeItem(key: string) {
    this.store.delete(key);
  }

  setItem(key: string, value: string) {
    this.store.set(key, value);
  }

  get length() {
    return this.store.size;
  }
}

function installBrowserStubs() {
  const storage = new MemoryStorage();
  const windowStub = {
    matchMedia: vi.fn().mockImplementation(() => ({
      matches: false,
      media: "",
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  };

  vi.stubGlobal("window", windowStub);
  vi.stubGlobal("localStorage", storage);
}

async function loadDesktopState() {
  vi.resetModules();
  return import("./desktop.svelte");
}

describe("desktop window state", () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
    installBrowserStubs();
  });

  it("does not treat a minimized window as active", async () => {
    const { activeWindow, minimizeWindow, openServiceHub, openStaticApp } = await loadDesktopState();

    const launcher = openStaticApp("ai-launcher");
    const serviceHub = openServiceHub();

    expect(activeWindow()?.id).toBe(serviceHub?.id);

    minimizeWindow(serviceHub!.id);

    expect(activeWindow()?.id).toBe(launcher?.id);
  });

  it("restores the same Service Hub window after minimizing it", async () => {
    const { activeWindow, minimizeWindow, openServiceHub, openStaticApp, windowForApp } = await loadDesktopState();

    const serviceHub = openServiceHub();
    expect(serviceHub?.minimized).toBe(false);

    minimizeWindow(serviceHub!.id);

    expect(windowForApp("service-hub")?.minimized).toBe(true);
    expect(activeWindow()).toBeNull();

    const restored = openStaticApp("service-hub");

    expect(restored?.id).toBe(serviceHub?.id);
    expect(restored?.minimized).toBe(false);
    expect(activeWindow()?.id).toBe(serviceHub?.id);
  });
});
