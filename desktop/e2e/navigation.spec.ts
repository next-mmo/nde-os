import { test, expect } from "@playwright/test";
import { dockButton, openLauncher } from "./helpers";

test.describe("Desktop shell", () => {
  test("boots into the macOS shell with the launcher window", async ({ page }) => {
    await openLauncher(page);

    await expect(page.locator(".topbar")).toBeVisible();
    await expect(page.getByRole("toolbar", { name: "Dock" })).toBeVisible();
    await expect(page.locator('[data-window="ai-launcher"]')).toBeVisible();
  });

  test("dock opens utility windows and launchpad opens as an overlay", async ({ page }) => {
    await openLauncher(page);

    await dockButton(page, /logs/i).click();
    await expect(page.locator('[data-window="logs"]')).toBeVisible();

    await dockButton(page, /settings/i).click();
    await expect(page.locator('[data-window="settings"]')).toBeVisible();

    await dockButton(page, /launchpad/i).click();
    await expect(page.getByTestId("launchpad")).toBeVisible();
    await expect(page.locator('[data-window="launchpad"]')).toHaveCount(0);
  });
});
