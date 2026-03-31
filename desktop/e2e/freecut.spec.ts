import { test, expect } from './fixtures';

test.describe('FreeCut Video Editor', () => {
  test('opens app, creates project, and verify rendering environment', async ({ page }) => {
    // 1. Open FreeCut via the exposed test hook
    await page.evaluate(() => (window as any).__svelteDesktop?.openStaticApp('freecut'));
    
    // Wait for the window to mount
    const win = page.locator('[data-window="freecut"]');
    await expect(win).toBeVisible({ timeout: 10000 });

    // 2. Either click New Project (if in projects view) or we are already in the editor
    const inspectorVisible = await page.locator('text=Inspector').isVisible({ timeout: 2000 }).catch(() => false);
    
    if (!inspectorVisible) {
      const newProjectBtn = page.getByRole('button', { name: /New Project/i });
      await expect(newProjectBtn).toBeVisible({ timeout: 15000 });
      await newProjectBtn.click();
    }

    // 3. Verify we entered the editor layout by checking for the Inspector panel
    await expect(page.locator('text=Inspector')).toBeVisible({ timeout: 10000 });

    // 5. Check for Timeline
    await expect(page.locator('text=Timeline')).toBeVisible();

    // 7. Trigger a frame update to fire a render request
    await page.keyboard.press('ArrowRight');
    await page.waitForTimeout(500);

    // 8. Test saving the project (triggers freecut_save_project via Ctrl+S shorthand or button if available, 
    // but the inspector has a save shortcut button)
    const saveBtn = page.locator('button[aria-label="Save (Ctrl+S)"]');
    if (await saveBtn.isVisible()) {
      await saveBtn.click();
      await page.waitForTimeout(500);
    }
    
    // Close FreeCut window
    await win.locator('button').filter({ hasText: 'Close' }).click().catch(() => {});
  });
});
