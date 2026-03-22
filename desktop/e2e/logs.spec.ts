import { test, expect } from "@playwright/test";
import { APP_NAME, dockButton, ensureCleanApp, installSampleCounter, openLauncher } from "./helpers";

test.describe("Logs window", () => {
  test.setTimeout(180000);

  test.beforeEach(async () => {
    await ensureCleanApp();
  });

  test.afterAll(async () => {
    await ensureCleanApp();
  });

  test("captures launcher activity and can clear the feed", async ({ page }) => {
    await openLauncher(page);
    await installSampleCounter(page);

    await dockButton(page, /logs/i).click();
    const logsWindow = page.locator('[data-window="logs"]');
    await expect(logsWindow).toBeVisible();
    await expect(logsWindow.getByText(`Installing ${APP_NAME}...`)).toBeVisible({ timeout: 10000 });
    await expect(logsWindow.getByText(`${APP_NAME} installed`)).toBeVisible({ timeout: 10000 });

    await logsWindow.getByRole("button", { name: "Clear" }).click();
    await expect(logsWindow.getByText("No activity yet.")).toBeVisible();
  });
});
