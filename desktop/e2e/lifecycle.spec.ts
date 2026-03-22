import { test, expect } from "@playwright/test";
import {
  APP_ID,
  appRow,
  clickButton,
  ensureCleanApp,
  installSampleCounter,
  launchSampleCounterInDashboard,
  openLauncher,
  openRailSection,
} from "./helpers";

test.describe("Dashboard-first lifecycle", () => {
  test.setTimeout(180000);

  test.beforeEach(async () => {
    await ensureCleanApp();
  });

  test.afterAll(async () => {
    await ensureCleanApp();
  });

  test("installs, opens inside the dashboard, pops out to browser, returns, stops, and uninstalls", async ({ page }) => {
    await openLauncher(page);
    await installSampleCounter(page);

    await launchSampleCounterInDashboard(page);
    await expect(page.locator(".session-strip").getByRole("button", { name: "Dashboard" })).toBeVisible();

    await openRailSection(page, "Running");
    const runningRow = page.locator('[data-session-id]').first();
    await expect(runningRow).toBeVisible({ timeout: 15000 });
    await clickButton(runningRow.getByRole("button", { name: "Open in Window" }));

    const browserWindow = page.locator('[data-window="browser"]').first();
    await expect(browserWindow).toBeVisible({ timeout: 10000 });
    await clickButton(browserWindow.getByRole("button", { name: "Return to Dashboard" }));
    await expect(page.locator('[data-window="browser"]')).toHaveCount(0);

    await openRailSection(page, "Installed");
    const installedRow = appRow(page, APP_ID);
    await expect(installedRow).toBeVisible();

    await clickButton(installedRow.getByRole("button", { name: "Open in Dashboard" }));
    await expect(page.getByText("Active preview")).toBeVisible({ timeout: 30000 });

    const detailPanel = page.locator(".detail-pane").first();
    await detailPanel.getByRole("button", { name: "Stop" }).click();
    await openRailSection(page, "Running");
    await expect(page.getByText("No running sessions yet.")).toBeVisible({ timeout: 15000 });

    await openRailSection(page, "Installed");
    await clickButton(installedRow.getByRole("button", { name: "Uninstall" }));
    await expect(page.locator(`[data-app-id="${APP_ID}"]`)).toHaveCount(0, { timeout: 15000 });

    await openRailSection(page, "Catalog");
    await expect(appRow(page, APP_ID).getByRole("button", { name: "Install" })).toBeVisible({ timeout: 15000 });
  });
});
