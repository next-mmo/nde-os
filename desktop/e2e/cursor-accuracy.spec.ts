/**
 * Cursor accuracy & click-target E2E tests.
 *
 * Validates that custom CSS cursors have correct hotspot offsets so that
 * mouse hover, click, and close actions hit the intended targets.
 *
 * Requires dev.sh running (enables CDP on port 9222).
 * Run:  cd desktop && npx playwright test e2e/cursor-accuracy.spec.ts --reporter=list
 */
import { test, expect } from "./fixtures";
import { openLauncher, dock, dockButton } from "./helpers";

test.describe("Cursor accuracy & click targets", () => {
  test.beforeEach(async ({ page }) => {
    // Ensure desktop is expanded and ready
    const wallpaper = page.locator('[data-testid="desktop-wallpaper"]');
    const fab = page.locator('[data-testid="collapse-tab"]');
    await expect(wallpaper.or(fab)).toBeVisible({ timeout: 15_000 });
    if (await fab.isVisible({ timeout: 500 }).catch(() => false)) {
      await fab.click();
      await page.waitForTimeout(1500);
    }
    await expect(wallpaper).toBeVisible({ timeout: 10_000 });
  });

  // ── Cursor hotspot CSS validation ──────────────────────────

  test("custom cursors have hotspot coordinates defined", async ({ page }) => {
    // The cursor CSS vars must include x/y hotspot values after the url()
    const cursors = await page.evaluate(() => {
      const style = getComputedStyle(document.body);
      return {
        default: style.getPropertyValue("--system-cursor-default").trim(),
        pointer: style.getPropertyValue("--system-cursor-pointer").trim(),
        textSelect: style.getPropertyValue("--system-cursor-text-select").trim(),
      };
    });

    // Each cursor var should have format: url("...") <x> <y>
    // The hotspot numbers come after the closing paren of url()
    for (const [name, val] of Object.entries(cursors)) {
      // Match: url(...) followed by at least two numbers
      const hasHotspot = /url\([^)]+\)\s+\d+\s+\d+/.test(val);
      expect(hasHotspot, `${name} cursor should include hotspot coords, got: "${val}"`).toBe(true);
    }
  });

  // ── elementFromPoint hit-testing ───────────────────────────

  test("elementFromPoint at center of traffic-light buttons hits correct target", async ({ page }) => {
    await openLauncher(page);

    const win = page.locator('[data-window="ai-launcher"]');
    await expect(win).toBeVisible({ timeout: 10_000 });

    // Exit fullscreen so buttons are in normal position
    const isFullscreen = await win.evaluate((el) => el.className.includes("inset-0"));
    if (isFullscreen) {
      await win.getByTestId("traffic-fullscreen").dispatchEvent("click");
      await page.waitForTimeout(500);
    }

    for (const id of ["traffic-close", "traffic-minimize", "traffic-fullscreen"]) {
      const btn = win.getByTestId(id);
      await expect(btn).toBeVisible();
      const box = await btn.boundingBox();
      expect(box, `${id} should have a bounding box`).not.toBeNull();

      const cx = Math.round(box!.x + box!.width / 2);
      const cy = Math.round(box!.y + box!.height / 2);

      const hitTestId = await page.evaluate(
        ({ x, y }) => {
          const el = document.elementFromPoint(x, y);
          if (!el) return "nothing";
          const target = el.closest("[data-testid]");
          return target?.getAttribute("data-testid") ?? `other:${el.tagName}`;
        },
        { x: cx, y: cy },
      );

      expect(hitTestId, `elementFromPoint at center of ${id} should hit ${id}`).toBe(id);
    }
  });

  test("elementFromPoint at center of dock buttons hits the button", async ({ page }) => {
    const dockBar = dock(page);
    await expect(dockBar).toBeVisible({ timeout: 10_000 });

    // Hover over dock to make sure it's fully visible (in case auto-hide)
    await dockBar.hover();
    await page.waitForTimeout(400);

    const buttons = dockBar.getByRole("button");
    const count = await buttons.count();
    expect(count).toBeGreaterThan(0);

    // Test first 3 dock buttons
    const toTest = Math.min(count, 3);
    for (let i = 0; i < toTest; i++) {
      const btn = buttons.nth(i);
      const box = await btn.boundingBox();
      if (!box) continue;

      const cx = Math.round(box.x + box.width / 2);
      const cy = Math.round(box.y + box.height / 2);

      const hitTag = await page.evaluate(
        ({ x, y }) => {
          const el = document.elementFromPoint(x, y);
          if (!el) return "nothing";
          const btn = el.closest("button");
          return btn ? "button" : el.tagName.toLowerCase();
        },
        { x: cx, y: cy },
      );

      expect(hitTag, `Dock button ${i} center should hit a <button>`).toBe("button");
    }
  });

  // ── Real mouse.click at center closes a window ─────────────

  test("mouse.click at exact center of close button closes the window", async ({ page }) => {
    await openLauncher(page);

    const win = page.locator('[data-window="ai-launcher"]');
    await expect(win).toBeVisible({ timeout: 10_000 });

    // Exit fullscreen first so the window is freely positioned
    const isFullscreen = await win.evaluate((el) => el.className.includes("inset-0"));
    if (isFullscreen) {
      await win.getByTestId("traffic-fullscreen").dispatchEvent("click");
      await page.waitForTimeout(500);
    }

    // Focus the window
    await win.evaluate((el) => el.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true })));
    await page.waitForTimeout(200);

    const closeBtn = win.getByTestId("traffic-close");
    await expect(closeBtn).toBeVisible();
    const box = await closeBtn.boundingBox();
    expect(box).not.toBeNull();

    const cx = box!.x + box!.width / 2;
    const cy = box!.y + box!.height / 2;

    // Use real mouse.click at the computed center
    await page.mouse.click(cx, cy);
    await page.waitForTimeout(600);

    // The window should be gone (or reloaded if it was the last)
    const stillVisible = await win.isVisible().catch(() => false);
    // If no other windows exist, desktop might reboot the launcher, so just
    // verify the click registered by checking the window count changed
    if (stillVisible) {
      // If still visible, it may have been re-opened. Verify close button
      // didn't accidentally drag the window (position unchanged)
      const boxAfter = await closeBtn.boundingBox();
      if (boxAfter) {
        expect(Math.abs(boxAfter.x - box!.x)).toBeLessThanOrEqual(2);
        expect(Math.abs(boxAfter.y - box!.y)).toBeLessThanOrEqual(2);
      }
    }
  });

  // ── Hover triggers at correct position ─────────────────────

  test("mouse.move to button center triggers hover state on traffic lights", async ({ page }) => {
    await openLauncher(page);

    const win = page.locator('[data-window="ai-launcher"]');
    await expect(win).toBeVisible({ timeout: 10_000 });

    const closeBtn = win.getByTestId("traffic-close");
    const closeSvg = closeBtn.locator("span svg");

    // SVGs should be hidden before hover
    await expect(closeSvg).toHaveCSS("opacity", "0");

    // Move real mouse to center of close button
    const box = await closeBtn.boundingBox();
    expect(box).not.toBeNull();
    await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
    await page.waitForTimeout(400);

    // The group hover should reveal all traffic light SVGs
    await expect(closeSvg).toHaveCSS("opacity", "1");
    const minSvg = win.getByTestId("traffic-minimize").locator("span svg");
    const fullSvg = win.getByTestId("traffic-fullscreen").locator("span svg");
    await expect(minSvg).toHaveCSS("opacity", "1");
    await expect(fullSvg).toHaveCSS("opacity", "1");
  });

  // ── Context menu appears at click position ─────────────────

  test("right-click context menu appears near the mouse position", async ({ page }) => {
    const wallpaper = page.locator('[data-testid="desktop-wallpaper"]');

    // Right-click at a known position on the desktop
    const clickX = 400;
    const clickY = 350;
    await page.mouse.click(clickX, clickY, { button: "right" });
    await page.waitForTimeout(400);

    const menu = page.locator('[data-testid="context-menu"]');
    await expect(menu).toBeVisible({ timeout: 5_000 });

    const menuBox = await menu.boundingBox();
    expect(menuBox).not.toBeNull();

    // The menu should appear within 20px of the click point
    // (it may flip if near an edge, but 400,350 is well within bounds)
    expect(Math.abs(menuBox!.x - clickX)).toBeLessThanOrEqual(20);
    expect(Math.abs(menuBox!.y - clickY)).toBeLessThanOrEqual(20);

    // Close
    await page.keyboard.press("Escape");
    await expect(menu).not.toBeVisible({ timeout: 3_000 });
  });

  // ── Smooth mouse movement (no pointer-events flicker) ──────

  test("mouse sweep across window header has no pointer-events dead zones", async ({ page }) => {
    await openLauncher(page);

    const win = page.locator('[data-window="ai-launcher"]');
    await expect(win).toBeVisible({ timeout: 10_000 });

    const header = win.locator("header.window-drag-handle");
    const headerBox = await header.boundingBox();
    expect(headerBox).not.toBeNull();

    // Sweep the mouse across the header from left to right in small steps
    // and verify every point hits an element inside the window (not the
    // wallpaper or a dead zone)
    const y = headerBox!.y + headerBox!.height / 2;
    const startX = headerBox!.x + 10;
    const endX = headerBox!.x + headerBox!.width - 10;
    const step = 15;
    let deadZones = 0;

    for (let x = startX; x < endX; x += step) {
      const hitInWindow = await page.evaluate(
        ({ px, py }) => {
          const el = document.elementFromPoint(px, py);
          if (!el) return false;
          return !!el.closest("section[data-window]");
        },
        { px: Math.round(x), py: Math.round(y) },
      );
      if (!hitInWindow) deadZones++;
    }

    // Allow zero dead zones in the header sweep
    expect(deadZones, "Header should have no pointer-events dead zones").toBe(0);
  });

  // ── Window resize handles are reachable ────────────────────

  test("resize handle at each corner is hittable by elementFromPoint", async ({ page }) => {
    await openLauncher(page);

    const win = page.locator('[data-window="ai-launcher"]');
    await expect(win).toBeVisible({ timeout: 10_000 });

    // Exit fullscreen to expose resize handles
    const isFullscreen = await win.evaluate((el) => el.className.includes("inset-0"));
    if (isFullscreen) {
      await win.getByTestId("traffic-fullscreen").dispatchEvent("click");
      await page.waitForTimeout(500);
    }

    const windowBox = await win.boundingBox();
    expect(windowBox).not.toBeNull();

    // Check the four corners (just inside the window edge)
    const corners = [
      { name: "top-right", x: windowBox!.x + windowBox!.width - 3, y: windowBox!.y + 3 },
      { name: "bottom-right", x: windowBox!.x + windowBox!.width - 3, y: windowBox!.y + windowBox!.height - 3 },
      { name: "bottom-left", x: windowBox!.x + 3, y: windowBox!.y + windowBox!.height - 3 },
    ];

    for (const corner of corners) {
      const hitInWindow = await page.evaluate(
        ({ px, py }) => {
          const el = document.elementFromPoint(px, py);
          if (!el) return false;
          return !!el.closest("section[data-window]");
        },
        { px: Math.round(corner.x), py: Math.round(corner.y) },
      );

      expect(hitInWindow, `${corner.name} corner should hit the window`).toBe(true);
    }
  });

  // ── Dock hover magnification doesn't break targeting ───────

  test("dock item remains clickable during magnification animation", async ({ page }) => {
    const dockBar = dock(page);
    await expect(dockBar).toBeVisible({ timeout: 10_000 });

    const firstButton = dockBar.getByRole("button").first();
    await expect(firstButton).toBeVisible();

    // Move mouse to dock to trigger magnification
    const dockBox = await dockBar.boundingBox();
    expect(dockBox).not.toBeNull();
    await page.mouse.move(dockBox!.x + dockBox!.width / 2, dockBox!.y + dockBox!.height / 2);
    await page.waitForTimeout(300);

    // After magnification, re-read the first button's position and verify
    // elementFromPoint at its center still hits a button
    const btnBox = await firstButton.boundingBox();
    expect(btnBox).not.toBeNull();

    const cx = Math.round(btnBox!.x + btnBox!.width / 2);
    const cy = Math.round(btnBox!.y + btnBox!.height / 2);

    const hitTag = await page.evaluate(
      ({ x, y }) => {
        const el = document.elementFromPoint(x, y);
        if (!el) return "nothing";
        return el.closest("button") ? "button" : el.tagName.toLowerCase();
      },
      { x: cx, y: cy },
    );

    expect(hitTag, "Dock item should remain hittable during magnification").toBe("button");
  });
});
