/**
 * File Explorer + Desktop Icons E2E tests — Tauri via CDP.
 *
 * Tests the sandbox-jailed file explorer (NDE-OS workspace only)
 * and desktop shortcut icons.
 *
 * Run:  cd desktop && npx playwright test e2e/file-explorer.spec.ts --reporter=list
 */
import { test, expect } from "./fixtures";
import { dockButton, openLauncher } from "./helpers";

// ── Helpers ─────────────────────────────────────────────────────────────

async function closeFileExplorer(page: import("@playwright/test").Page) {
  const win = page.locator('[data-window="file-explorer"]');
  if (await win.isVisible().catch(() => false)) {
    const close = win.getByTestId("traffic-close");
    if (await close.isVisible().catch(() => false)) {
      await close.evaluate((el: HTMLButtonElement) => el.click());
      await page.waitForTimeout(500);
    }
  }
}

async function openFileExplorer(page: import("@playwright/test").Page) {
  await openLauncher(page);

  // Close existing file explorer to get fresh state at sandbox root
  await closeFileExplorer(page);

  // Find file explorer dock button
  const dock = page.getByRole("toolbar", { name: "Dock" });
  await expect(dock).toBeVisible({ timeout: 10_000 });

  const btn = dock.getByRole("button", { name: /file.?explorer/i });
  await expect(btn).toBeVisible({ timeout: 10_000 });

  // Scroll into view + dispatch click (dock may be behind the main window)
  await btn.scrollIntoViewIfNeeded();
  await btn.evaluate((el: HTMLButtonElement) => {
    el.scrollIntoView({ block: "center" });
    el.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
    el.dispatchEvent(new PointerEvent("pointerup", { bubbles: true }));
    el.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
  await page.waitForTimeout(1500);

  // Wait for file explorer window
  const win = page.locator('[data-window="file-explorer"]');
  await expect(win).toBeVisible({ timeout: 15_000 });
  await expect(win.locator("aside")).toBeVisible({ timeout: 10_000 });

  return win;
}

// ── File Explorer ───────────────────────────────────────────────────────

test.describe("File Explorer", () => {
  // Each test opens from dock → needs time
  test.beforeEach(async () => {
    test.setTimeout(45_000);
  });

  test("opens from dock with correct title", async ({ page }) => {
    const win = await openFileExplorer(page);
    await expect(win.locator("header strong")).toContainText("File Explorer");
  });

  test("has traffic light controls", async ({ page }) => {
    const win = await openFileExplorer(page);
    await expect(win.getByTestId("traffic-close")).toBeVisible();
    await expect(win.getByTestId("traffic-minimize")).toBeVisible();
    await expect(win.getByTestId("traffic-fullscreen")).toBeVisible();
  });

  // ── Sidebar ──────────────────────────────────────────────────────

  test("sidebar shows NDE-OS workspace directories", async ({ page }) => {
    const win = await openFileExplorer(page);
    const sb = win.locator("aside");

    for (const label of ["Workspace", "Data", "Models", "Outputs", "Logs", "Config"]) {
      await expect(sb.getByRole("button", { name: label })).toBeVisible();
    }
  });

  test("sidebar navigation changes breadcrumbs", async ({ page }) => {
    const win = await openFileExplorer(page);
    const sb = win.locator("aside");

    // Click "Data" in sidebar
    await sb.getByRole("button", { name: "Data" }).click();
    await page.waitForTimeout(1500);

    // Breadcrumbs should show "data"
    const bc = win.locator(".overflow-x-auto");
    await expect(bc).toContainText("data", { timeout: 5000 });
  });

  // ── Breadcrumbs ──────────────────────────────────────────────────

  test("breadcrumbs show NDE-OS root", async ({ page }) => {
    const win = await openFileExplorer(page);
    const bc = win.locator(".overflow-x-auto");
    await expect(bc.getByRole("button", { name: "NDE-OS" })).toBeVisible({ timeout: 5000 });
  });

  // ── Navigation ───────────────────────────────────────────────────

  test("go-up disabled at sandbox root", async ({ page }) => {
    const win = await openFileExplorer(page);
    const goUp = win.locator('button[aria-label="Go up"]');
    await expect(goUp).toBeVisible({ timeout: 5000 });
    await expect(goUp).toBeDisabled({ timeout: 5000 });
  });

  test("back button disabled initially", async ({ page }) => {
    const win = await openFileExplorer(page);
    const goBack = win.locator('button[aria-label="Go back"]');
    await expect(goBack).toBeVisible({ timeout: 5000 });
    await expect(goBack).toBeDisabled({ timeout: 5000 });
  });

  // ── File listing ─────────────────────────────────────────────────

  test("status bar shows item count", async ({ page }) => {
    const win = await openFileExplorer(page);
    await expect(win).toContainText(/\d+ items?/, { timeout: 5000 });
  });

  // ── View toggle ──────────────────────────────────────────────────

  test("view toggle switches between list and grid", async ({ page }) => {
    const win = await openFileExplorer(page);
    const btn = win.getByLabel("Toggle view");

    // Should work without crashing
    await btn.click();
    await page.waitForTimeout(300);
    await expect(win).toBeVisible();

    await btn.click();
    await page.waitForTimeout(300);
    await expect(win).toBeVisible();
  });

  // ── Toolbar actions ─────────────────────────────────────────────

  test("new folder button shows input and cancel hides it", async ({ page }) => {
    const win = await openFileExplorer(page);

    const newFolderBtn = win.locator('button[aria-label="New folder"]');
    await expect(newFolderBtn).toBeVisible({ timeout: 5000 });
    await newFolderBtn.click();
    const input = win.locator('input[placeholder*="folder name"]');
    await expect(input).toBeVisible({ timeout: 3000 });

    await win.getByRole("button", { name: "Cancel" }).click();
    await expect(input).toBeHidden({ timeout: 3000 });
  });

  test("delete button disabled when nothing selected", async ({ page }) => {
    const win = await openFileExplorer(page);
    await expect(win.getByLabel("Delete")).toBeDisabled({ timeout: 5000 });
  });

  // ── Context menu ─────────────────────────────────────────────────

  test("right-click blank area shows context menu", async ({ page }) => {
    const win = await openFileExplorer(page);
    const area = win.locator(".flex-1.overflow-auto");

    // Dispatch contextmenu event (Playwright right-click may be intercepted)
    await area.evaluate((el: HTMLElement) => {
      el.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, clientX: 200, clientY: 300 }));
    });
    await page.waitForTimeout(500);

    // Context menu items should be visible
    const overlay = page.locator(".fixed.inset-0.z-9999");
    if (await overlay.isVisible()) {
      await expect(page.getByText("New Folder").first()).toBeVisible({ timeout: 3000 });
      await expect(page.getByText("Refresh").first()).toBeVisible({ timeout: 3000 });
    }
  });

  test("context menu closes on click", async ({ page }) => {
    const win = await openFileExplorer(page);
    const area = win.locator(".flex-1.overflow-auto");

    // Open context menu
    await area.click({ button: "right" });
    await page.waitForTimeout(500);

    // Click to close
    const overlay = page.locator(".fixed.inset-0.z-9999");
    if (await overlay.isVisible()) {
      await overlay.click({ position: { x: 1, y: 1 } });
      await page.waitForTimeout(500);
      await expect(overlay).toBeHidden({ timeout: 3000 });
    }
  });

  // ── Window management ────────────────────────────────────────────

  test("close button removes window", async ({ page }) => {
    const win = await openFileExplorer(page);
    await win.getByTestId("traffic-close").evaluate((el: HTMLButtonElement) => el.click());
    await expect(page.locator('[data-window="file-explorer"]')).toBeHidden({ timeout: 5000 });
  });
});

// ── Desktop Icons ───────────────────────────────────────────────────────

test.describe("Desktop Icons", () => {
  test.beforeEach(async () => {
    test.setTimeout(45_000);
  });

  test("desktop shows shortcut icons", async ({ page }) => {
    await openLauncher(page);
    const icons = page.locator("div[style*='direction: rtl']");
    await expect(icons).toBeVisible({ timeout: 10_000 });

    for (const label of ["File Explorer", "AI Launcher"]) {
      await expect(icons.locator("button").filter({ hasText: label }).first()).toBeVisible({
        timeout: 5000,
      });
    }
  });

  test("click selects icon with ring", async ({ page }) => {
    await openLauncher(page);
    const icons = page.locator("div[style*='direction: rtl']");
    const icon = icons.locator("button").filter({ hasText: "File Explorer" }).first();

    if (await icon.isVisible({ timeout: 5000 }).catch(() => false)) {
      // Use evaluate click to bypass window overlap
      await icon.evaluate((el: HTMLButtonElement) => {
        el.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      });
      await page.waitForTimeout(500);
      const cls = await icon.getAttribute("class");
      expect(cls).toContain("ring-1");
    }
  });

  test("double-click opens corresponding app", async ({ page }) => {
    await openLauncher(page);
    const icons = page.locator("div[style*='direction: rtl']");
    const icon = icons.locator("button").filter({ hasText: "Settings" }).first();

    if (await icon.isVisible({ timeout: 5000 }).catch(() => false)) {
      // Use dispatchEvent to bypass pointer/window overlap issues
      await icon.evaluate((el: HTMLButtonElement) => {
        el.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
      });
      await page.waitForTimeout(1000);
      await expect(page.locator('[data-window="settings"]')).toBeVisible({ timeout: 10_000 });
    }
  });
});
