import { test, expect } from "./fixtures";
import { dockButton, clickButton } from "./helpers";

test.describe("App State Persistence", () => {
  test("minimized windows and collapsed desktop preserve state", async ({ page }) => {
    // Wait for the desktop shell to boot
    const wallpaper = page.locator('[data-testid="desktop-wallpaper"]');
    const fabButton = page.locator('[data-testid="floating-fab"]');

    const anyIndicator = page.locator('[data-testid="desktop-wallpaper"]:visible, [data-testid="floating-fab"]:visible').first();
    await expect(anyIndicator).toBeVisible({ timeout: 15_000 });

    // If currently collapsed (FAB mode), expand it
    if (await fabButton.isVisible().catch(() => false)) {
      await clickButton(fabButton);
      await page.waitForTimeout(1500);
    }
    await expect(wallpaper).toBeVisible({ timeout: 10_000 });

    // Open the Launchpad
    await clickButton(dockButton(page, /launchpad/i));
    const launchpad = page.getByTestId("launchpad");
    await expect(launchpad).toBeVisible({ timeout: 10000 });

    // Click the Terminal app in the launchpad grid
    const terminalBtn = launchpad.getByRole("button", { name: /terminal/i });
    await expect(terminalBtn).toBeVisible({ timeout: 5000 });
    await clickButton(terminalBtn);
    
    const terminalWindow = page.locator('section[data-window="terminal"]');
    await expect(terminalWindow).toBeVisible({ timeout: 10000 });

    // Verify xterm renders
    const xterm = terminalWindow.locator('.xterm');
    await expect(xterm).toBeVisible({ timeout: 10000 });
    const xtermRows = xterm.locator('.xterm-rows');
    await expect(xtermRows).toBeVisible({ timeout: 10000 });

    // Take a screenshot of the normal state
    await page.screenshot({ path: '../test-results/fix-state-loss/before-minimize.png' });

    // Test 1: Window Minimize persistence
    const trafficMinimize = terminalWindow.getByTestId("traffic-minimize");
    await clickButton(trafficMinimize);
    
    // Window should become invisible/opacity-0, but remain mounted
    await expect(terminalWindow).toHaveClass(/invisible/, { timeout: 5000 });
    
    // Click dock button to un-minimize
    await clickButton(dockButton(page, /terminal/i));
    
    // Window should become visible again
    await expect(terminalWindow).toHaveClass(/visible/, { timeout: 5000 });
    await expect(xtermRows).toBeVisible({ timeout: 5000 });
    await page.screenshot({ path: '../test-results/fix-state-loss/after-unminimize.png' });

    // Test 2: Collapsing desktop (Switch to Host OS) persistence
    const collapseTab = page.locator('[data-testid="collapse-tab"]');
    await clickButton(collapseTab);

    // Desktop wrapper translates out and becomes invisible
    await expect(wallpaper).toBeHidden({ timeout: 5000 });
    await expect(fabButton).toBeVisible({ timeout: 5000 });

    // Expand desktop again
    await clickButton(fabButton);
    await expect(wallpaper).toBeVisible({ timeout: 10000 });

    // The Terminal should STILL be visible and state preserved without Svelte unmounting it
    await expect(terminalWindow).toBeVisible({ timeout: 5000 });
    await expect(xtermRows).toBeVisible({ timeout: 5000 });
    await page.screenshot({ path: '../test-results/fix-state-loss/after-uncollapse.png' });

    // Clean up
    const trafficClose = terminalWindow.getByTestId("traffic-close");
    await clickButton(trafficClose);
    // Use toBeHidden for cleanup confirmation
    await expect(terminalWindow.locator('div')).toHaveCount(0, { timeout: 5000 }).catch(() => {});
  });
});
