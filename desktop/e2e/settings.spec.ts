import { test, expect } from "@playwright/test";
import { dockButton, openLauncher } from "./helpers";

test.describe("Settings window", () => {
  test("shows system and uv information and refreshes in place", async ({ page }) => {
    await openLauncher(page);
    await dockButton(page, /settings/i).click();

    const settingsWindow = page.locator('[data-window="settings"]');
    await expect(settingsWindow).toBeVisible();
    await expect(settingsWindow.getByText("Server and runtime")).toBeVisible();
    await expect(settingsWindow.getByText("Host")).toBeVisible();
    await expect(settingsWindow.getByRole("heading", { name: "uv" })).toBeVisible();

    const refreshButton = settingsWindow.getByRole("button", { name: "Refresh" });
    await refreshButton.click();
    await expect(settingsWindow.getByText("Server and runtime")).toBeVisible();
  });
});
