import { test, expect } from './fixtures';

test.describe('FreeCut Edit and Export Workflow', () => {
  test('creates a project, drops text, trims it, edit properties, and exports', async ({ page }) => {
    page.on('console', msg => console.log('BROWSER CONSOLE:', msg.text()));
    await page.evaluate(() => {
      (window as any).__E2E_TEST__ = true;
      console.log("Playwright E2E Mock Mode Enabled");
    });

    // 2. Open FreeCut via the exposed test hook
    await page.evaluate(() => (window as any).__svelteDesktop?.openStaticApp('freecut'));
    
    const win = page.locator('[data-window="freecut"]');
    await expect(win).toBeVisible({ timeout: 10000 });

    // 3. Create a new project
    const newProjectBtn = page.getByRole('button', { name: /New Project/i });
    if (await newProjectBtn.isVisible()) {
      await newProjectBtn.click();
    }

    // Ensure we are in the editor view
    await expect(page.locator('text=Properties').or(page.locator('text=Inspector'))).toBeVisible({ timeout: 15000 });

    // 4. Add Text preset to timeline via simulated HTML5 Drag & Drop
    // 4. Add Text preset to timeline via simulated HTML5 Drag & Drop
    await page.evaluate(() => {
      const tabs = Array.from(document.querySelectorAll('button'));
      const textTab = tabs.find(b => b.textContent?.trim().toLowerCase() === 'text');
      if (textTab) {
        textTab.click();
      }
    });

    // Wait for the preset to appear
    await expect(page.locator('.cursor-grab')).toBeVisible({ timeout: 10000 });

    await page.evaluate(() => {
      const source = document.querySelector('.cursor-grab') as HTMLElement;
      const target = document.querySelector('#tracks-container') as HTMLElement;
      if (!source || !target) throw new Error("Drag source or target not found");
      
      const dt = new DataTransfer();
      const dragStart = new DragEvent('dragstart', { dataTransfer: dt, bubbles: true });
      source.dispatchEvent(dragStart);
      
      const targetRect = target.getBoundingClientRect();
      const dropX = targetRect.left + 50; 
      const dropY = targetRect.top + 20;

      const dragOver = new DragEvent('dragover', { dataTransfer: dt, bubbles: true, clientX: dropX, clientY: dropY });
      target.dispatchEvent(dragOver);
      
      const drop = new DragEvent('drop', { dataTransfer: dt, bubbles: true, clientX: dropX, clientY: dropY });
      target.dispatchEvent(drop);
      
      const dragEnd = new DragEvent('dragend', { dataTransfer: dt, bubbles: true });
      source.dispatchEvent(dragEnd);
    });

    // Verify clip appears in timeline
    const timelineClip = page.locator('[role="button"][tabindex="-1"]').first();
    await expect(timelineClip).toBeVisible({ timeout: 5000 });

    // 5. Simulate Trim handles (resizing the clip)
    await page.evaluate(() => {
      const handle = document.querySelector('[data-drag-mode="trim"]') as HTMLElement;
      if (!handle) throw new Error("Trim handle not found");
      
      const handleRect = handle.getBoundingClientRect();
      const mousedown = new MouseEvent('mousedown', { clientX: handleRect.left, clientY: handleRect.top, bubbles: true });
      handle.dispatchEvent(mousedown);
      
      const mousemove = new MouseEvent('mousemove', { clientX: handleRect.left + 50, clientY: handleRect.top, bubbles: true });
      window.dispatchEvent(mousemove);
      
      const mouseup = new MouseEvent('mouseup', { bubbles: true });
      window.dispatchEvent(mouseup);
    });

    // 6. Click the clip to reveal Properties
    await timelineClip.click();
    await expect(page.locator('text=Text Content')).toBeVisible({ timeout: 5000 });
    
    // 7. Edit Text Content
    const textarea = page.locator('textarea').first();
    await expect(textarea).toBeVisible();
    await textarea.fill('Testing End-to-End Export!');
    
    // 8. Open Export Modal
    const exportToolbarBtn = page.getByRole('button', { name: 'Export' }).first();
    await exportToolbarBtn.click();
    
    // Ensure modal appears
    await expect(page.locator('h2', { hasText: 'Exporting...' })).toBeVisible({ timeout: 5000 });

    // 9. Confirm Export
    const confirmExportBtn = page.getByRole('button', { name: 'Confirm Export' });
    await confirmExportBtn.click();
    
    // Confirm the backend IPC to start export was called
    await expect(page.locator('body[data-export-called="true"]')).toBeVisible({ timeout: 5000 });
  });
});
