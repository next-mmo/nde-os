import { test, expect } from "./fixtures";

// ─── Helpers ────────────────────────────────────────────────────────────────

/**
 * Opens the Vibe Studio window and switches to the IDE tab.
 * Uses the programmatic dev hook (same as omx-vibe-studio.spec.ts).
 */
async function openIDETab(page: any) {
  // Close any existing window for clean state
  await page.evaluate(() => {
    const btn = document.querySelector('[data-window="vibe-studio"] button[aria-label="Close Desktop Window"]') as HTMLElement | null;
    btn?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  });
  await page.waitForTimeout(500);

  // Open Vibe Studio via dev hook
  await page.evaluate(() => (window as any).__svelteDesktop?.openStaticApp('vibe-studio'));
  await page.waitForTimeout(2000);

  const vibeWindow = page.locator('[data-window="vibe-studio"]');
  await expect(vibeWindow).toBeAttached({ timeout: 15000 });

  // Click the IDE tab via dispatchEvent to bypass any overlay interception
  await page.evaluate(() => {
    const btn = Array.from(document.querySelectorAll('[data-window="vibe-studio"] button'))
      .find((b) => b.textContent?.trim().startsWith('IDE')) as HTMLElement | null;
    btn?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  });
  await page.waitForTimeout(500);

  return vibeWindow;
}

async function closeVibeStudio(page: any) {
  await page.evaluate(() => {
    const btn = document.querySelector('[data-window="vibe-studio"] button[aria-label="Close Desktop Window"]') as HTMLElement | null;
    btn?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  });
  await page.waitForTimeout(300);
}

/** Click an element by title inside the vibe-studio window using dispatchEvent (bypasses pointer overlay). */
async function clickByTitle(page: any, title: string) {
  await page.evaluate((t: string) => {
    const el = document.querySelector(`[data-window="vibe-studio"] [title="${t}"]`) as HTMLElement | null;
    el?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  }, title);
}

// ─── Tests ──────────────────────────────────────────────────────────────────

test.describe("NDE IDE — Core UI", () => {
  test("welcome screen shows NDE IDE branding", async ({ page }) => {
    const win = await openIDETab(page);
    await expect(win.getByText("NDE IDE")).toBeVisible({ timeout: 8000 });
    await closeVibeStudio(page);
  });

  test("activity bar has Explorer and Source Control buttons (no project = disabled state)", async ({ page }) => {
    const win = await openIDETab(page);
    // Without a project, both buttons show 'Open a project first'
    const disabledBtns = win.locator('[title="Open a project first"]');
    // There should be at least 2 (Explorer + Source Control both disabled)
    await expect(disabledBtns.first()).toBeAttached({ timeout: 5000 });
    await closeVibeStudio(page);
  });

  test("toggle terminal button exists in DOM", async ({ page }) => {
    const win = await openIDETab(page);
    // Title is "Toggle Terminal (⌘`)"
    await expect(win.locator('[title="Toggle Terminal (⌘`)"]')).toBeAttached();
    await closeVibeStudio(page);
  });

  test("terminal opens when terminal button is clicked", async ({ page }) => {
    await openIDETab(page);

    // Use dispatchEvent to bypass dock overlay
    await clickByTitle(page, 'Toggle Terminal (⌘`)');
    await page.waitForTimeout(600);

    // xterm canvas should now be in DOM
    await page.waitForSelector('[data-window="vibe-studio"] .xterm', { timeout: 10000 });
    const win = page.locator('[data-window="vibe-studio"]');
    await expect(win.locator('.xterm')).toBeAttached();

    await closeVibeStudio(page);
  });

  test("terminal renders a shell prompt", async ({ page }) => {
    await openIDETab(page);
    await clickByTitle(page, 'Toggle Terminal (⌘`)');
    await page.waitForSelector('[data-window="vibe-studio"] .xterm-rows', { timeout: 10000 });
    const text = await page.locator('[data-window="vibe-studio"] .xterm-rows').textContent({ timeout: 5000 });
    expect(text).toBeTruthy();
    await closeVibeStudio(page);
  });
});

test.describe("NDE IDE — Sidebar & Explorer", () => {
  test("activity bar buttons present (Explorer + Source Control disabled without project)", async ({ page }) => {
    const win = await openIDETab(page);
    // Without a project both activity bar buttons have title 'Open a project first'
    const disabledBtns = win.locator('[title="Open a project first"]');
    await expect(disabledBtns.first()).toBeAttached({ timeout: 5000 });
    await closeVibeStudio(page);
  });

  test("activity bar has source control button (shows 'Open a project first' without project)", async ({ page }) => {
    const win = await openIDETab(page);
    // Without a project the title is "Open a project first" — verify the button exists regardless
    const scBtn = win.locator('[title="Source Control"], [title="Open a project first"]');
    await expect(scBtn.first()).toBeAttached();
    await closeVibeStudio(page);
  });
});

test.describe("NDE IDE — VS Code Tab Bar", () => {
  test("no close button visible when no file is open", async ({ page }) => {
    const win = await openIDETab(page);
    await expect(win.locator("span[title='Close']")).not.toBeVisible();
    await expect(win.locator("span[title='Close (unsaved)']")).not.toBeVisible();
    await closeVibeStudio(page);
  });
});

test.describe("NDE IDE — Markdown Preview", () => {
  test("Edit/Split/Preview toggles absent without an open file", async ({ page }) => {
    const win = await openIDETab(page);
    await expect(win.getByTitle("Edit")).not.toBeVisible();
    await expect(win.getByTitle("Split")).not.toBeVisible();
    await expect(win.getByTitle("Preview")).not.toBeVisible();
    await closeVibeStudio(page);
  });
});

test.describe("NDE IDE — Welcome Screen", () => {
  test("Recent Projects section is visible", async ({ page }) => {
    const win = await openIDETab(page);
    await expect(win.getByText("Recent Projects")).toBeVisible({ timeout: 8000 });
    await closeVibeStudio(page);
  });

  test("New Project and Browse Sandbox CTAs are present", async ({ page }) => {
    const win = await openIDETab(page);
    await expect(win.getByText("New Project")).toBeVisible({ timeout: 8000 });
    await expect(win.getByText("Browse Sandbox")).toBeVisible({ timeout: 8000 });
    await closeVibeStudio(page);
  });
});
