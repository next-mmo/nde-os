import { test, expect } from '../e2e/fixtures';

test.describe('Desktop Interactions', () => {

  test.beforeEach(async ({ page }) => {
    // Expand from collapsed FAB if needed

    // Wait for desktop to be ready — check for presence of
    // either the wallpaper or the collapse-tab FAB
    const wallpaper = page.locator('[data-testid="desktop-wallpaper"]');
    const fab = page.locator('[data-testid="collapse-tab"]');

    await expect(wallpaper.or(fab)).toBeVisible({ timeout: 15_000 });

    // If still in collapsed FAB mode, click to expand
    if (await fab.isVisible({ timeout: 500 }).catch(() => false)) {
      await fab.click();
      await page.waitForTimeout(1500);
    }

    // Now desktop wallpaper should be visible
    await expect(wallpaper).toBeVisible({ timeout: 10_000 });
  });

  test('desktop icons are visible and can be right-clicked for context menu', async ({ page }) => {
    // Find the first icon on the desktop
    const firstIcon = page.locator('[data-testid^="desktop-icon-"]').first();
    await expect(firstIcon).toBeVisible({ timeout: 10_000 });

    // Take a screenshot of the desktop with icons visible
    await page.screenshot({ path: 'e2e-results/desktop-icons-visible.png' });

    // Right-click the icon to open the context menu
    await firstIcon.click({ button: 'right' });
    await page.waitForTimeout(300);

    // Context menu should appear
    const contextMenu = page.locator('[data-testid="context-menu"]');
    await expect(contextMenu).toBeVisible({ timeout: 5_000 });

    // Verify it has expected menu items
    await expect(contextMenu.getByRole('menuitem', { name: 'Open' })).toBeVisible();

    // Screenshot showing the icon context menu
    await page.screenshot({ path: 'e2e-results/desktop-icon-context-menu.png' });

    // Close by pressing Escape
    await page.keyboard.press('Escape');
    await expect(contextMenu).not.toBeVisible({ timeout: 3_000 });
  });

  test('desktop right-click shows full context menu with real OS actions', async ({ page }) => {
    // Right-click on the wallpaper (empty desktop area)
    const wallpaper = page.locator('[data-testid="desktop-wallpaper"]');
    await wallpaper.click({ button: 'right', position: { x: 200, y: 300 } });
    await page.waitForTimeout(300);

    const contextMenu = page.locator('[data-testid="context-menu"]');
    await expect(contextMenu).toBeVisible({ timeout: 5_000 });

    // Verify all the macOS-style items are present
    await expect(contextMenu.getByRole('menuitem', { name: 'New Folder' })).toBeVisible();
    await expect(contextMenu.getByRole('menuitem', { name: 'Sort By Name' })).toBeVisible();
    await expect(contextMenu.getByRole('menuitem', { name: 'Clean Up' })).toBeVisible();
    await expect(contextMenu.getByRole('menuitem', { name: 'Next Wallpaper' })).toBeVisible();
    await expect(contextMenu.getByRole('menuitem', { name: 'Random Wallpaper' })).toBeVisible();
    await expect(contextMenu.getByRole('menuitem', { name: 'Wallpaper Settings…' })).toBeVisible();
    await expect(contextMenu.getByRole('menuitem', { name: 'Toggle Dark Mode' })).toBeVisible();
    await expect(contextMenu.getByRole('menuitem', { name: 'Open Launchpad' })).toBeVisible();
    await expect(contextMenu.getByRole('menuitem', { name: 'Spotlight Search' })).toBeVisible();

    // Screenshot: full context menu
    await page.screenshot({ path: 'e2e-results/desktop-context-menu-full.png' });

    // Click a functional item (Sort By Name)
    await contextMenu.getByRole('menuitem', { name: 'Sort By Name' }).click();
    await page.waitForTimeout(300);

    // Context menu should close after click
    await expect(contextMenu).not.toBeVisible({ timeout: 3_000 });
  });

  test('dragging an icon does not open the app', async ({ page }) => {
    // Find the AI Launcher icon
    const icon = page.locator('[data-testid="desktop-icon-ai-launcher"]');

    // Skip if not visible (may be hidden)
    if (!(await icon.isVisible({ timeout: 3_000 }).catch(() => false))) {
      test.skip();
      return;
    }

    const box = await icon.boundingBox();
    if (!box) {
      test.skip();
      return;
    }

    // Record the number of windows before the interaction
    const windowsBefore = await page.locator('[data-window]').count();

    // Drag the icon by 100px to the right
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width / 2 + 100, box.y + box.height / 2, { steps: 10 });
    await page.mouse.up();
    await page.waitForTimeout(400);

    // The icon should have moved but the app should NOT have opened a new window
    const windowsAfter = await page.locator('[data-window]').count();
    expect(windowsAfter).toBe(windowsBefore);

    // Screenshot: icon moved, no app opened
    await page.screenshot({ path: 'e2e-results/desktop-icon-drag-no-open.png' });
  });

  test('icon context menu has Pin to Dock and Remove from Desktop options', async ({ page }) => {
    // Find a static app icon
    const icon = page.locator('[data-testid="desktop-icon-settings"]');

    if (!(await icon.isVisible({ timeout: 3_000 }).catch(() => false))) {
      test.skip();
      return;
    }

    // Right-click the icon
    await icon.click({ button: 'right' });
    await page.waitForTimeout(300);

    const contextMenu = page.locator('[data-testid="context-menu"]');
    await expect(contextMenu).toBeVisible({ timeout: 5_000 });

    // Verify expanded context menu items
    await expect(contextMenu.getByRole('menuitem', { name: 'Open' })).toBeVisible();
    await expect(contextMenu.getByRole('menuitem', { name: 'Get Info' })).toBeVisible();
    await expect(contextMenu.getByRole('menuitem', { name: 'Pin to Dock' })).toBeVisible();
    await expect(contextMenu.getByRole('menuitem', { name: 'Copy' })).toBeVisible();
    await expect(contextMenu.getByRole('menuitem', { name: 'Remove from Desktop' })).toBeVisible();

    // Screenshot: expanded icon context menu
    await page.screenshot({ path: 'e2e-results/desktop-icon-rich-context-menu.png' });

    // Close
    await page.keyboard.press('Escape');
  });

  test('window can be dragged by title bar', async ({ page }) => {
    // Find any open window's drag handle
    const dragHandle = page.locator('.window-drag-handle').first();

    if (!(await dragHandle.isVisible({ timeout: 5_000 }).catch(() => false))) {
      test.skip();
      return;
    }

    const box = await dragHandle.boundingBox();
    if (!box) {
      test.skip();
      return;
    }

    // Get the parent window's initial transform
    const windowEl = page.locator('section[data-window]').first();
    const initialTransform = await windowEl.evaluate((el) => el.style.transform);

    // Drag the window by its title bar
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width / 2 + 80, box.y + box.height / 2 + 40, { steps: 10 });
    await page.mouse.up();
    await page.waitForTimeout(400);

    // The window's transform should have changed (it moved)
    const finalTransform = await windowEl.evaluate((el) => el.style.transform);
    // If it was fullscreen it won't move, so only assert if not fullscreen
    const isFullscreen = await windowEl.evaluate((el) => el.classList.contains('inset-0!'));
    if (!isFullscreen) {
      expect(finalTransform).not.toBe(initialTransform);
    }

    // Screenshot: window after drag
    await page.screenshot({ path: 'e2e-results/desktop-window-dragged.png' });
  });

});
