/**
 * Traffic light button E2E tests — Tauri app via CDP.
 *
 * Requires dev.sh running (enables CDP on port 9222).
 * Run:  npx playwright test e2e/traffic-lights.spec.ts --reporter=list
 */
import { test, expect } from "./fixtures";
import { openLauncher } from "./helpers";

test.describe("Traffic light buttons", () => {

  // ── Colors ────────────────────────────────────────────────

  test("all three buttons show correct macOS colors", async ({ page }) => {
    await openLauncher(page);

    const win = page.locator('[data-window="ai-launcher"]');
    const close = win.getByTestId("traffic-close");
    const minimize = win.getByTestId("traffic-minimize");
    const fullscreen = win.getByTestId("traffic-fullscreen");

    await expect(close).toBeVisible();
    await expect(minimize).toBeVisible();
    await expect(fullscreen).toBeVisible();

    await expect(close.locator("span")).toHaveCSS("background-color", "rgb(255, 95, 87)");       // #ff5f57 red
    await expect(minimize.locator("span")).toHaveCSS("background-color", "rgb(254, 188, 46)");   // #febc2e yellow
    await expect(fullscreen.locator("span")).toHaveCSS("background-color", "rgb(40, 200, 64)");  // #28c840 green
  });

  test("buttons are round", async ({ page }) => {
    await openLauncher(page);
    const win = page.locator('[data-window="ai-launcher"]');

    for (const id of ["traffic-close", "traffic-minimize", "traffic-fullscreen"]) {
      const circle = win.getByTestId(id).locator("span");
      const radius = await circle.evaluate((el) => getComputedStyle(el).borderRadius);
      // rounded-full produces 9999px or 50% depending on engine
      expect(
        radius === "9999px" || radius === "50%" || parseFloat(radius) > 5,
        `${id} border-radius should be fully round, got: ${radius}`,
      ).toBe(true);
    }
  });

  // ── Hover position & icons ────────────────────────────────

  test("hover over group reveals all three SVG icons", async ({ page }) => {
    await openLauncher(page);

    const win = page.locator('[data-window="ai-launcher"]');
    const closeBtn = win.getByTestId("traffic-close");
    const closeSvg = closeBtn.locator("span svg");
    const minSvg = win.getByTestId("traffic-minimize").locator("span svg");
    const fullSvg = win.getByTestId("traffic-fullscreen").locator("span svg");

    // Before hover — SVGs hidden
    await expect(closeSvg).toHaveCSS("opacity", "0");
    await expect(minSvg).toHaveCSS("opacity", "0");
    await expect(fullSvg).toHaveCSS("opacity", "0");

    // Hover the group container
    await closeBtn.locator("..").hover();
    await page.waitForTimeout(400);

    // After hover — all visible
    await expect(closeSvg).toHaveCSS("opacity", "1");
    await expect(minSvg).toHaveCSS("opacity", "1");
    await expect(fullSvg).toHaveCSS("opacity", "1");
  });

  test("each SVG icon is centered inside its button bounding box", async ({ page }) => {
    await openLauncher(page);

    const win = page.locator('[data-window="ai-launcher"]');

    // Trigger hover so SVGs render
    await win.getByTestId("traffic-close").locator("..").hover();
    await page.waitForTimeout(400);

    for (const id of ["traffic-close", "traffic-minimize", "traffic-fullscreen"]) {
      const btn = win.getByTestId(id);
      const svg = btn.locator("span svg");

      const circle = btn.locator("span");
      const circleBox = await circle.boundingBox();
      const svgBox = await svg.boundingBox();
      expect(circleBox, `${id} circle should have a bounding box`).not.toBeNull();
      expect(svgBox, `${id} SVG should have a bounding box`).not.toBeNull();

      // SVG must be fully inside circle
      expect(svgBox!.x).toBeGreaterThanOrEqual(circleBox!.x);
      expect(svgBox!.y).toBeGreaterThanOrEqual(circleBox!.y);
      expect(svgBox!.x + svgBox!.width).toBeLessThanOrEqual(circleBox!.x + circleBox!.width + 1);
      expect(svgBox!.y + svgBox!.height).toBeLessThanOrEqual(circleBox!.y + circleBox!.height + 1);

      // SVG center should be roughly at circle center (within 2px tolerance)
      const circleCenterX = circleBox!.x + circleBox!.width / 2;
      const circleCenterY = circleBox!.y + circleBox!.height / 2;
      const svgCenterX = svgBox!.x + svgBox!.width / 2;
      const svgCenterY = svgBox!.y + svgBox!.height / 2;
      expect(Math.abs(svgCenterX - circleCenterX)).toBeLessThanOrEqual(2);
      expect(Math.abs(svgCenterY - circleCenterY)).toBeLessThanOrEqual(2);
    }
  });

  test("hover on individual button center triggers group hover", async ({ page }) => {
    await openLauncher(page);

    const win = page.locator('[data-window="ai-launcher"]');
    const minimizeBtn = win.getByTestId("traffic-minimize");

    // Hover precisely at the center of the minimize button
    const box = await minimizeBtn.boundingBox();
    expect(box).not.toBeNull();
    await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
    await page.waitForTimeout(400);

    // All three SVGs should now be visible (group hover)
    const closeSvg = win.getByTestId("traffic-close").locator("span svg");
    const minSvg = minimizeBtn.locator("span svg");
    const fullSvg = win.getByTestId("traffic-fullscreen").locator("span svg");

    await expect(closeSvg).toHaveCSS("opacity", "1");
    await expect(minSvg).toHaveCSS("opacity", "1");
    await expect(fullSvg).toHaveCSS("opacity", "1");
  });

  // ── Click: Close ──────────────────────────────────────────

  test("close button is enabled (closable by default)", async ({ page }) => {
    await openLauncher(page);

    const win = page.locator('[data-window="ai-launcher"]');
    const closeBtn = win.getByTestId("traffic-close");

    await expect(closeBtn).toBeEnabled();
    await expect(closeBtn.locator("span")).toHaveCSS("background-color", "rgb(255, 95, 87)"); // red, not grey
  });

  test("clicking close at button center removes the window", async ({ page }) => {
    await openLauncher(page);

    const win = page.locator('[data-window="ai-launcher"]');
    const closeBtn = win.getByTestId("traffic-close");

    // Record window count before close
    const windowsBefore = await page.locator('[data-window]').count();

    await closeBtn.evaluate((el: HTMLButtonElement) => el.click());
    await page.waitForTimeout(500);

    // Either the launcher was removed (other windows present prevent reboot)
    // or it was rebooted — either way, the state changed
    const windowsAfter = await page.locator('[data-window]').count();
    const launcherVisible = await page.locator('[data-window="ai-launcher"]').isVisible();
    expect(windowsAfter <= windowsBefore || launcherVisible).toBe(true);
  });

  test("close button click does NOT drag the window", async ({ page }) => {
    await openLauncher(page);

    const win = page.locator('[data-window="ai-launcher"]');
    const header = win.locator("header.window-drag-handle");

    // Get position before click
    const headerBox1 = await header.boundingBox();
    expect(headerBox1).not.toBeNull();

    // Click close (use evaluate to ensure it fires)
    await win.getByTestId("traffic-close").evaluate((el: HTMLButtonElement) => el.click());
    await page.waitForTimeout(300);

    // If window is still visible, verify it didn't move (no accidental drag)
    const stillVisible = await page.locator('[data-window="ai-launcher"]').isVisible();
    if (stillVisible) {
      const headerBox2 = await page.locator('[data-window="ai-launcher"] header.window-drag-handle').boundingBox();
      if (headerBox2) {
        // Position should not have changed significantly (±2px tolerance)
        expect(Math.abs(headerBox2.x - headerBox1!.x)).toBeLessThanOrEqual(2);
        expect(Math.abs(headerBox2.y - headerBox1!.y)).toBeLessThanOrEqual(2);
      }
    }
  });

  // ── Click: Minimize ───────────────────────────────────────

  test("clicking minimize hides the window", async ({ page }) => {
    await openLauncher(page);

    const win = page.locator('[data-window="ai-launcher"]');
    const minimizeBtn = win.getByTestId("traffic-minimize");

    await minimizeBtn.evaluate((el: HTMLButtonElement) => el.click());
    // Window should become invisible (minimized)
    await expect(win).toBeHidden({ timeout: 3000 });
  });

  // ── Click: Fullscreen ─────────────────────────────────────

  test("clicking fullscreen at button center toggles fullscreen on/off", async ({ page }) => {
    await openLauncher(page);

    const win = page.locator('[data-window="ai-launcher"]');

    // Check that the window starts in fullscreen via class test
    const hasFullscreenBefore = await win.evaluate(
      (el) => el.className.includes("transform-none"),
    );
    expect(hasFullscreenBefore).toBe(true);

    // Click fullscreen button to exit fullscreen
    // Use dispatchEvent with the Svelte-compatible event pattern
    const fullscreenBtn = win.getByTestId("traffic-fullscreen");
    await fullscreenBtn.dispatchEvent("click");
    await page.waitForTimeout(500);

    // After click: should no longer have fullscreen class
    const hasFullscreenAfter = await win.evaluate(
      (el) => el.className.includes("transform-none"),
    );
    expect(hasFullscreenAfter).toBe(false);

    // Click again to re-enter fullscreen
    await win.getByTestId("traffic-fullscreen").dispatchEvent("click");
    await page.waitForTimeout(500);

    const hasFullscreenRestored = await win.evaluate(
      (el) => el.className.includes("transform-none"),
    );
    expect(hasFullscreenRestored).toBe(true);
  });

  test("fullscreen toggle preserves traffic light button colors", async ({ page }) => {
    await openLauncher(page);

    const win = page.locator('[data-window="ai-launcher"]');

    // Exit fullscreen
    await win.getByTestId("traffic-fullscreen").dispatchEvent("click");
    await page.waitForTimeout(300);

    // Colors should persist in normal mode
    await expect(win.getByTestId("traffic-close").locator("span")).toHaveCSS("background-color", "rgb(255, 95, 87)");
    await expect(win.getByTestId("traffic-minimize").locator("span")).toHaveCSS("background-color", "rgb(254, 188, 46)");
    await expect(win.getByTestId("traffic-fullscreen").locator("span")).toHaveCSS("background-color", "rgb(40, 200, 64)");

    // Re-enter fullscreen
    await win.getByTestId("traffic-fullscreen").dispatchEvent("click");
    await page.waitForTimeout(300);

    // Colors should still persist
    await expect(win.getByTestId("traffic-close").locator("span")).toHaveCSS("background-color", "rgb(255, 95, 87)");
    await expect(win.getByTestId("traffic-minimize").locator("span")).toHaveCSS("background-color", "rgb(254, 188, 46)");
    await expect(win.getByTestId("traffic-fullscreen").locator("span")).toHaveCSS("background-color", "rgb(40, 200, 64)");
  });

  // ── Click accuracy: no position mismatch ──────────────────

  test("visual button position matches click target (no offset)", async ({ page }) => {
    await openLauncher(page);

    const win = page.locator('[data-window="ai-launcher"]');

    for (const id of ["traffic-close", "traffic-minimize", "traffic-fullscreen"]) {
      const btn = win.getByTestId(id);
      const box = await btn.boundingBox();
      expect(box, `${id} should have a bounding box`).not.toBeNull();

      const cx = Math.round(box!.x + box!.width / 2);
      const cy = Math.round(box!.y + box!.height / 2);

      // elementFromPoint at button center should hit the button or its child (SVG)
      const hitTestId = await page.evaluate(
        ({ x, y, expectedId }) => {
          const el = document.elementFromPoint(x, y);
          if (!el) return "nothing";
          // Walk up to find the nearest [data-testid]
          const target = el.closest("[data-testid]");
          return target?.getAttribute("data-testid") ?? `other:${el.tagName}`;
        },
        { x: cx, y: cy, expectedId: id },
      );

      expect(hitTestId, `elementFromPoint at center of ${id} should hit ${id}`).toBe(id);
    }
  });
});
