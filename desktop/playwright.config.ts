import { defineConfig, devices } from "@playwright/test";
import { join } from "node:path";

// Simple fallback if process is not globally defined in tsconfig
declare const process: any;

const testDataDir = join(process.cwd(), ".playwright-localappdata");
const serverCommand =
  process.platform === "win32"
    ? `powershell -NoProfile -Command "Remove-Item -Recurse -Force '${testDataDir}' -ErrorAction SilentlyContinue; New-Item -ItemType Directory -Force '${testDataDir}' | Out-Null; cargo run -p ai-launcher-server"`
    : `rm -rf '${testDataDir}' && mkdir -p '${testDataDir}' && cargo run -p ai-launcher-server`;

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  // Fail the build on CI if you accidentally left test.only in the source code.
  forbidOnly: !!process.env.CI,
  // Retry on CI only
  retries: process.env.CI ? 2 : 0,
  // Opt out of parallel tests on CI.
  workers: 1,
  reporter: [["html", { open: "never" }], ["list"]],

  use: {
    baseURL: "http://localhost:5173",
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
      reuseExistingServer: false,
      timeout: 60000,
      cwd: "..",
      env: {
        ...process.env,
        HOME: testDataDir,
        LOCALAPPDATA: testDataDir,
      },
    },
    {
      command: "pnpm dev",
      port: 5173,
      reuseExistingServer: false,
      timeout: 60000,
    }
  ],
});
