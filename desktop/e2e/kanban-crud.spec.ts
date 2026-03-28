import { test, expect } from './fixtures';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

test.describe('Vibe Kanban - Full Manual Management CRUD', () => {
  const tasksDir = path.resolve(__dirname, '../src-tauri/.agents/tasks');

  test.beforeEach(async () => {
    // Ensure clean state before test
    if (!fs.existsSync(tasksDir)) {
      fs.mkdirSync(tasksDir, { recursive: true });
    }
    const files = fs.readdirSync(tasksDir);
    for (const file of files) {
      if (file.endsWith('.md')) {
        fs.unlinkSync(path.join(tasksDir, file));
      }
    }
  });

  test('should create, edit, move, and delete Kanban tasks via UI', async ({ page }) => {
    // Wait for the desktop shell to finish loading
    await page.waitForSelector('[data-testid="dock"], [data-window], .dock', { timeout: 15000 });

    // Open VibeCode Studio via global test hook
    await page.evaluate(() => {
        (window as any).__svelteDesktop?.openStaticApp('vibe-studio');
    });

    // Wait for Vibe Studio window
    await page.waitForSelector('[data-window="vibe-studio"]', { timeout: 10000 });
    await page.waitForTimeout(500);

    // Navigate to Kanban tab via JS to bypass overlay bounds
    await page.evaluate(() => {
        const btn = Array.from(document.querySelectorAll("button"))
            .find(b => b.textContent?.trim() === "Kanban 📋") as HTMLElement | undefined;
        if (btn) btn.click();
    });
    // Click away to dismiss any stray context menus just in case
    await page.evaluate(() => {
        document.body.click();
        const z = document.querySelector('.fixed.inset-0.z-40');
        if (z) z.remove();
    });

    // 1. Create a Task via the inline '+ New Task' button
    const planCol = page.locator('div[aria-label="Plan"]');
    await planCol.locator('button[title="New Task"]').evaluate(node => (node as HTMLButtonElement).click());
    
    // Fill the new task inline input
    await planCol.locator('input[placeholder="Task title..."]').fill('My Amazing E2E Task');
    await page.keyboard.press('Enter');

    // Wait for the card to be rendered after Tauri IPC completes
    const cardId = 'my-amazing-e2e-task.md';
    const cardSelector = `[data-card-id="${cardId}"]`;
    await expect(page.locator(cardSelector)).toBeVisible({ timeout: 5000 });

    // 2. Open Task Detail Slide-over -> Edit Content
    await page.locator(cardSelector).evaluate(node => (node as HTMLElement).click());
    
    // Verify Slide-over is visible
    const saveBtn = page.locator('button', { hasText: 'Save & Close' });
    await expect(saveBtn).toBeVisible({ timeout: 2000 });

    // Fill the text area
    const textarea = page.locator('textarea').first();
    await textarea.fill(`# My Amazing E2E Task\n\n- **Status:** 🔴 plan\n\n## Edited Details\nWe are editing inside the detail panel.`);
    await saveBtn.evaluate(node => (node as HTMLButtonElement).click());
    
    // Slide-over should disappear
    await expect(saveBtn).not.toBeVisible();
    await page.waitForTimeout(200);

    // 3. Move Task using the Quick Status ⋮ Menu
    await page.locator(`${cardSelector} button[title="Options"]`).evaluate(node => (node as HTMLButtonElement).click());
    
    // The context menu drops down with columns as buttons
    const doneItem = page.locator('button.text-left', { hasText: 'Done by AI' });
    await expect(doneItem).toBeVisible();
    await doneItem.evaluate(node => (node as HTMLButtonElement).click());

    // Verify it moved away from Plan column and into Done column
    const doneCol = page.locator('div[aria-label="Done by AI"]');
    await expect(doneCol.locator(cardSelector)).toBeVisible({ timeout: 3000 });
    await expect(planCol.locator(cardSelector)).not.toBeVisible();

    // 4. Delete the Task via Card Delete Button
    await doneCol.locator(`${cardSelector} button[title="Delete Task"]`).evaluate(node => (node as HTMLButtonElement).click());

    // Verify it disappears from UI
    await expect(page.locator(cardSelector)).not.toBeVisible({ timeout: 3000 });
  });
});
