import { test, expect } from "./fixtures";
import { openLauncher, clickButton, dockButton } from "./helpers";

test.describe("Screenshot Feature", () => {
  test("screenshot app opens from launchpad and renders UI controls", async ({ page }) => {
    await openLauncher(page);

    // Open the Launchpad
    await clickButton(dockButton(page, /launchpad/i));
    const launchpad = page.getByTestId("launchpad");
    await expect(launchpad).toBeVisible({ timeout: 10000 });

    // Click the Screenshot app in the launchpad grid
    const screenshotBtn = launchpad.getByRole("button", { name: /screenshot/i });
    await expect(screenshotBtn).toBeVisible({ timeout: 5000 });
    await screenshotBtn.click();

    // The screenshot window should appear with data-window="screenshot"
    const screenshotWindow = page.locator('[data-window="screenshot"]');
    await expect(screenshotWindow).toBeVisible({ timeout: 15000 });

    // Verify the "no data" placeholder is shown (no actual capture was performed)
    await expect(screenshotWindow.getByText("No screenshot data available.")).toBeVisible({ timeout: 5000 });

    // Verify the Close button exists inside the window content (scroll into view if needed)
    const closeButton = screenshotWindow.locator("button", { hasText: "Close" });
    await closeButton.scrollIntoViewIfNeeded();
    await expect(closeButton).toBeVisible({ timeout: 5000 });

    // Verify the Copy to Clipboard button
    const copyButton = screenshotWindow.locator("button", { hasText: "Copy to Clipboard" });
    await copyButton.scrollIntoViewIfNeeded();
    await expect(copyButton).toBeVisible({ timeout: 5000 });

    // Close the screenshot window using the traffic-light close button (macOS-style)
    const trafficClose = screenshotWindow.getByTestId("traffic-close");
    await trafficClose.click();
    await expect(screenshotWindow).toBeHidden({ timeout: 5000 });
  });
});
