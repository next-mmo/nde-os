import { test, expect } from "./fixtures";
import { openLauncher, clickButton, dockButton } from "./helpers";

test.describe("Terminal App", () => {
  test("terminal app opens from launchpad and renders xterm pty", async ({ page }) => {
    // Clear local storage to prevent a stale saved Terminal bounds state from crashing the boot
    try { await page.evaluate(() => localStorage.clear()); } catch {}

    // Force reset desktop state
    await page.reload({ waitUntil: "domcontentloaded" });
    await page.waitForTimeout(2000);

    // Open launcher which expects everything to settle correctly
    await openLauncher(page);

    // Give a brief moment for any existing animations to settle
    await page.waitForTimeout(500);

    // Open the Launchpad
    await clickButton(dockButton(page, /launchpad/i));
    const launchpad = page.getByTestId("launchpad");
    await expect(launchpad).toBeVisible({ timeout: 10000 });

    // Click the Terminal app in the launchpad grid
    // We use a broader locator in case getByRole text computation fails
    const terminalBtn = launchpad.locator('button', { hasText: 'Terminal' }).first();
    await expect(terminalBtn).toBeVisible({ timeout: 5000 });
    await terminalBtn.click({ force: true });

    // The Terminal window should appear with data-window="terminal"
    const terminalWindow = page.locator('section[data-window="terminal"]');
    await expect(terminalWindow).toBeVisible({ timeout: 15000 });

    // The xterm element should render inside the window
    const xterm = terminalWindow.locator('.xterm');
    await expect(xterm).toBeVisible({ timeout: 10000 });

    // Wait for the xterm lines to populate (PTY spawns and sends prompt)
    const xtermRows = xterm.locator('.xterm-rows');
    await expect(xtermRows).toBeVisible({ timeout: 10000 });

    // Wait a brief moment for the pty to settle and render the prompt
    await page.waitForTimeout(2000);

    // Take screenshot to test-results/terminal-app/screenshot.png to satisfy DoD
    await page.screenshot({ path: '../test-results/terminal-app/screenshot.png' });

    // Clean up
    const trafficClose = terminalWindow.getByTestId("traffic-close");
    await trafficClose.click();
    await expect(terminalWindow).toBeHidden({ timeout: 5000 });
  });
});
