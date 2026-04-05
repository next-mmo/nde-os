import { test, expect } from "./fixtures";
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
    await expect(settingsWindow.getByText("Host Information")).toBeVisible();
    await expect(settingsWindow.getByText("OS / Arch")).toBeVisible();
    await expect(settingsWindow.getByText("Python")).toBeVisible();
    await expect(settingsWindow.getByText("Live Resources")).toBeVisible();

    const refreshButton = settingsWindow.getByRole("button", { name: "Refresh Status" });
    await refreshButton.click();
    await expect(settingsWindow.getByText(/Memory Usage/i)).toBeVisible();
    await expect(settingsWindow.getByText(/Disk \(/i)).toBeVisible();
  });
});
