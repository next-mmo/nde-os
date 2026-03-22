import { test, expect } from "@playwright/test";
import { dockButton, openLauncher } from "./helpers";

test.describe("Settings window", () => {
  test("shows live RAM and disk usage in the shell and settings window", async ({ page }) => {
    await openLauncher(page);

    await expect(page.locator('[data-topbar-metric="memory"]')).toContainText(/RAM \d+%/, {
      timeout: 15000,
    });
    await expect(page.locator('[data-topbar-metric="disk"]')).toContainText(/Disk \d+%/, {
      timeout: 15000,
    });

    await dockButton(page, /settings/i).click();

    const settingsWindow = page.locator('[data-window="settings"]');
    await expect(settingsWindow).toBeVisible();
    await expect(settingsWindow.getByText("Server and runtime")).toBeVisible();
    await expect(settingsWindow.getByText("Host")).toBeVisible();
    await expect(settingsWindow.getByRole("heading", { name: "uv" })).toBeVisible();
    await expect(settingsWindow.getByRole("heading", { name: "Live resources" })).toBeVisible();
    await expect(settingsWindow.locator('[data-resource-card="memory"]')).toContainText(/Memory/);
    await expect(settingsWindow.locator('[data-resource-card="disk"]')).toContainText(/Disk/);

    const refreshButton = settingsWindow.getByRole("button", { name: "Refresh" });
    await refreshButton.click();
    await expect(settingsWindow.locator('[data-resource-card="memory"]')).toContainText(/%/);
    await expect(settingsWindow.locator('[data-resource-card="disk"]')).toContainText(/%/);
  });
});
