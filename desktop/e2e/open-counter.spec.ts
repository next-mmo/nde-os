import { test, expect } from "@playwright/test";
import {
  clickButton,
  ensureCleanApp,
  installSampleCounter,
  launchSampleCounterInDashboard,
  openLauncher,
  openRailSection,
} from "./helpers";

test.describe("Browser window controls", () => {
  test.setTimeout(180000);

  test.beforeEach(async () => {
    await ensureCleanApp();
  });

  test.afterAll(async () => {
    await ensureCleanApp();
  });

  test("opens a browser window from the running session and exposes browser controls", async ({ page }) => {
    await openLauncher(page);
    await installSampleCounter(page);
    await launchSampleCounterInDashboard(page);

    await openRailSection(page, "Running");
    const sessionRow = page.locator('[data-session-id]').first();
    await expect(sessionRow).toBeVisible({ timeout: 15000 });
    await clickButton(sessionRow.getByRole("button", { name: "Open in Window" }));

    const browserWindow = page.locator('[data-window="browser"]').first();
    await expect(browserWindow).toBeVisible({ timeout: 10000 });
    await expect(browserWindow.getByRole("button", { name: "Back" })).toBeVisible();
    await expect(browserWindow.getByRole("button", { name: "Forward" })).toBeVisible();
    await expect(browserWindow.getByRole("button", { name: "Reload" })).toBeVisible();
    await expect(browserWindow.getByRole("button", { name: "Return to Dashboard" })).toBeVisible();
    await expect(browserWindow.getByRole("textbox", { name: "Browser address" })).toHaveValue(/localhost/);
  });
});
