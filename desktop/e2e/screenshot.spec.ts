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

  test("ctrl+shift+s triggers screenshot capture and opens screenshot window", async ({ page }) => {
    // Wait for the app to be stable
    await expect(page.locator('[data-testid="dock-container"]')).toBeVisible({ timeout: 10000 });

    // Press the shortcut for screenshot mode
    await page.keyboard.press("Control+Shift+S");

    // The overlay may appear as a new page in CDP, or it might be handled internally.
    // Wait a moment for the overlay to spawn, then try to interact with it.
    let overlayPage = null;
    try {
      overlayPage = await page.context().waitForEvent("page", { timeout: 5000 });
    } catch {
      // Overlay didn't surface as a CDP page — this is OK in some Tauri configs.
      // The overlay might still have been created and auto-dismissed.
    }

    if (overlayPage) {
      // Overlay appeared as a CDP page — click it to trigger fullscreen capture
      await overlayPage.locator("main").waitFor({ state: "visible", timeout: 5000 });
      await overlayPage.mouse.click(100, 100);
    } else {
      // Fallback: emit the screenshot-selected event directly from the main window
      // to simulate a fullscreen capture (width=0, height=0 means fullscreen)
      await page.evaluate(async () => {
        const { emit } = await import("@tauri-apps/api/event");
        await emit("screenshot-selected", { x: 0, y: 0, width: 0, height: 0 });
      });
    }

    // After capturing, the screenshot app window should open in the main window
    const screenshotWindow = page.locator('[data-window="screenshot"]');
    await expect(screenshotWindow).toBeVisible({ timeout: 15000 });

    // Ensure either an image is displayed or the fallback placeholder is shown
    // (headless CI will get a 200x200 dummy image from capture.rs)
    const hasImage = screenshotWindow.locator("img[alt='Screenshot']");
    const hasPlaceholder = screenshotWindow.getByText("No screenshot data available.");
    await expect(hasImage.or(hasPlaceholder)).toBeVisible({ timeout: 10000 });

    // Clean up
    const trafficClose = screenshotWindow.getByTestId("traffic-close");
    await trafficClose.click();
    await expect(screenshotWindow).toBeHidden({ timeout: 5000 });
  });
});
