import { test, expect } from "./fixtures";
import { APP_NAME, clickButton, dockButton, openLauncher } from "./helpers";

test.describe("Launchpad behavior", () => {
  test("launchpad can surface catalog apps and system apps from the shell", async ({ page }) => {
    await openLauncher(page);

    await clickButton(dockButton(page, /launchpad/i));
    const launchpad = page.getByTestId("launchpad");
    await expect(launchpad).toBeVisible();
    await expect(launchpad.getByText("Open apps from the dashboard or a separate window.")).toBeVisible();
    await expect(launchpad.getByText(APP_NAME)).toBeVisible();

    await launchpad.getByRole("button", { name: "App Store" }).click();
    await expect(page.locator('[data-window="app-store"]')).toBeVisible();
  });
});
