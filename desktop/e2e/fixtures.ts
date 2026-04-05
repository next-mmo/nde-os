import { test as base, chromium, type Page, expect as baseExpect } from '@playwright/test';

declare const process: any;
const isMac = process.platform === 'darwin';

export const test = base.extend<{ page: Page }>({
  page: async ({}, use) => {
    let tauriPage: Page | null = null;

    // ── Strategy 1: CDP (Windows/WebView2) ──────────────────────────────
    // `dev.sh` exports WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222
    // which makes the Tauri WebView2 (Windows) expose a CDP endpoint at 9222.
    if (!isMac) {
      for (let i = 0; i < 30; i++) {
        try {
          const res = await fetch('http://127.0.0.1:9222/json/version');
          if (res.ok) {
            const json = await res.json();
            const wsEndpoint: string = json.webSocketDebuggerUrl;
            if (wsEndpoint) {
              const browser = await chromium.connectOverCDP(wsEndpoint);
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
              if (tauriPage) break;
            }
          }
        } catch { /* ignore — retry */ }
        await new Promise(r => setTimeout(r, 1000));
      }
      if (!tauriPage) {
        throw new Error('Could not connect to Tauri remote debugging CDP server at 9222');
      }
    }

    // ── Strategy 2: Direct Vite URL (macOS / WKWebView) ─────────────────
    // WKWebView has no CDP endpoint. Launch a regular Chromium and point it
    // at the Vite dev server (localhost:5174). Tauri IPC (invoke) doesn't
    // work, but all UI/rendering tests pass correctly.
    if (isMac) {
      const browser = await chromium.launch({ headless: true });
      const context = await browser.newContext({
        viewport: { width: 1440, height: 900 },
        // Inject a no-op Tauri IPC shim so components don't crash on invoke()
        extraHTTPHeaders: {},
      });
      tauriPage = await context.newPage();

      // Inject __TAURI__ shim before any scripts load
      await context.addInitScript(() => {
        (window as any).__TAURI__ = { core: { invoke: async () => ({}) } };
        (window as any).__TAURI_IPC__ = () => {};
        // Stub Tauri invoke so components degrade gracefully
        (window as any).__tauri_invoke_stub = async (cmd: string, _args?: any) => {
          if (cmd === 'get_home_dir') return '/tmp/nde-test';
          if (cmd === 'list_directory') return [];
          if (cmd === 'get_agent_tasks') return [];
          return null;
        };
      });

      // Wait for the Vite dev server, then navigate
      let viteReady = false;
      for (let i = 0; i < 20; i++) {
        try {
          const r = await fetch('http://localhost:5174');
          if (r.ok) { viteReady = true; break; }
        } catch { /* ignore */ }
        await new Promise(r => setTimeout(r, 1000));
      }
      if (!viteReady) throw new Error('Vite dev server not available at localhost:5174');

      await tauriPage.goto('http://localhost:5174', { waitUntil: 'domcontentloaded', timeout: 30000 });
    }

    await use(tauriPage as Page);
  },
});

export const expect = baseExpect;
