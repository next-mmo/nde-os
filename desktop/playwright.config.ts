import { defineConfig, devices } from "@playwright/test";
import { join } from "node:path";

// Simple fallback if process is not globally defined in tsconfig
declare const process: any;

const testDataDir = join(process.cwd(), ".playwright-localappdata");
const isMac = process.platform === "darwin";
const isWin = process.platform === "win32";

const serverCommand = isWin
  ? `powershell -NoProfile -Command "Remove-Item -Recurse -Force '${testDataDir}' -ErrorAction SilentlyContinue; New-Item -ItemType Directory -Force '${testDataDir}' | Out-Null; cargo run -p ai-launcher-server"`
  : `rm -rf '${testDataDir}' && mkdir -p '${testDataDir}' && cargo run -p ai-launcher-server`;

// On macOS, WKWebView has no CDP endpoint.
// We check port 5174 (Vite dev server) instead of 9222.
// Fixtures.ts will fall back to localhost:5174 when 9222 is unavailable.
const desktopWebServer = isMac
  ? {
      // macOS: just verify the Vite dev server is running
      command: "pnpm dev",
      port: 5174,
      reuseExistingServer: true,
      timeout: 120000,
      cwd: ".",
      env: { ...process.env },
    }
  : {
      // Windows/Linux: Tauri exposes CDP via WebView2 remote debugging
      command: "pnpm tauri dev",
      port: 9222,
      reuseExistingServer: true,
      timeout: 300000,
      env: {
        ...process.env,
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: "--remote-debugging-port=9222",
        HOME: testDataDir,
        LOCALAPPDATA: testDataDir,
      },
    };

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: [["html", { open: "never" }], ["list"]],

  use: {
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    actionTimeout: 15000,
  },

  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],

  webServer: [
    {
      command: serverCommand,
      port: 8080,
      reuseExistingServer: true,
      timeout: 120000,
      cwd: "..",
      env: {
        ...process.env,
        HOME: testDataDir,
        LOCALAPPDATA: testDataDir,
      },
    },
    desktopWebServer,
  ],
});
