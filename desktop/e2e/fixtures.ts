import { test as base, chromium, type Page, expect as baseExpect } from '@playwright/test';

export const test = base.extend<{ page: Page }>({
  page: async ({}, use) => {
    // Retry to connect to the CDP port
    let wsEndpoint = "";
    for (let i = 0; i < 30; i++) {
        try {
            const res = await fetch('http://127.0.0.1:9222/json/version');
            if (res.ok) {
                const json = await res.json();
                wsEndpoint = json.webSocketDebuggerUrl;
                if (wsEndpoint) break;
            }
        } catch(e) { /* ignore */ }
        await new Promise(r => setTimeout(r, 1000));
    }

    if (!wsEndpoint) {
        throw new Error("Could not connect to Tauri remote debugging CDP server at 9222");
    }

    const browser = await chromium.connectOverCDP(wsEndpoint);
    
    // Find the correct Tauri WebView page among all contexts
    let tauriPage = null;
    for (const ctx of browser.contexts()) {
      for (const p of ctx.pages()) {
        const url = p.url();
        if (url.includes('localhost:5174') || url.includes('tauri')) {
          tauriPage = p;
          break;
        }
      }
      if (tauriPage) break;
    }
    
    if (!tauriPage) {
      throw new Error("Could not find Tauri WebView page. Found URLs: " + browser.contexts().flatMap(c => c.pages().map(p => p.url())).join(", "));
    }

    // Tauri sets its own URL, no need to navigate
    await use(tauriPage);

    await browser.close();
  }
});

export const expect = baseExpect;
