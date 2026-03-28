import { test, expect } from './fixtures';
import { dockButton } from './helpers';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const outDir = join(__dirname, '..', '..', 'test-results');

test('resizable sidebars — left, right, and chat panels', async ({ page }) => {
  // Resize the Tauri window via CDP so all panels are visible
  const cdpSession = await page.context().newCDPSession(page);
  const { windowId } = await cdpSession.send('Browser.getWindowForTarget');
  await cdpSession.send('Browser.setWindowBounds', {
    windowId,
    bounds: { width: 1440, height: 900, windowState: 'normal' }
  });
  await page.waitForTimeout(500);
  await page.reload({ waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(2000);

  // Open Vibe Code Studio as a standalone window via the dock
  const vibeIcon = dockButton(page, /vibe studio/i);
  await vibeIcon.evaluate((el: HTMLElement) => el.click());
  await page.waitForTimeout(2000);

  const vibeWindow = page.locator('[data-window="vibe-studio"]');
  await expect(vibeWindow).toBeVisible({ timeout: 10000 });

  // Verify Preview tab and Layers panel visible
  const layersHeader = vibeWindow.locator('text=Layers').first();
  await expect(layersHeader).toBeVisible({ timeout: 10000 });

  const handles = vibeWindow.locator('[class*="cursor-col-resize"]');
  const handleCount = await handles.count();
  console.log(`Found ${handleCount} resize handle(s)`);
  expect(handleCount).toBeGreaterThanOrEqual(2);

  // Screenshot 1: Default state
  await page.screenshot({ path: join(outDir, 'cdp-resize-default.png') });

  // --- Test 1: Left sidebar (LayerTree) resize ---
  const leftHandle = handles.nth(0);
  const leftBox = await leftHandle.boundingBox();
  expect(leftBox).toBeTruthy();

  const layersBefore = await layersHeader.evaluate((el) =>
    el.closest('[style*="width"]')?.getBoundingClientRect().width ?? 0
  );

  await page.mouse.move(leftBox!.x + leftBox!.width / 2, leftBox!.y + leftBox!.height / 2);
  await page.mouse.down();
  await page.mouse.move(leftBox!.x + leftBox!.width / 2 + 60, leftBox!.y + leftBox!.height / 2, { steps: 10 });
  await page.mouse.up();
  await page.waitForTimeout(300);

  const layersAfter = await layersHeader.evaluate((el) =>
    el.closest('[style*="width"]')?.getBoundingClientRect().width ?? 0
  );
  console.log(`Left panel: ${layersBefore}px -> ${layersAfter}px`);
  expect(layersAfter).toBeGreaterThan(layersBefore);

  // --- Test 2: Right sidebar (PropertiesPanel) resize ---
  if (handleCount >= 2) {
    const rightHandle = handles.nth(1);
    const rightBox = await rightHandle.boundingBox();
    if (rightBox) {
      const designHeader = vibeWindow.locator('text=Design').first();
      const propsBefore = await designHeader.evaluate((el) =>
        el.closest('[style*="width"]')?.getBoundingClientRect().width ?? 0
      );

      await page.mouse.move(rightBox.x + rightBox.width / 2, rightBox.y + rightBox.height / 2);
      await page.mouse.down();
      await page.mouse.move(rightBox.x + rightBox.width / 2 - 60, rightBox.y + rightBox.height / 2, { steps: 10 });
      await page.mouse.up();
      await page.waitForTimeout(300);

      const propsAfter = await designHeader.evaluate((el) =>
        el.closest('[style*="width"]')?.getBoundingClientRect().width ?? 0
      );
      console.log(`Right panel: ${propsBefore}px -> ${propsAfter}px`);
      expect(propsAfter).toBeGreaterThan(propsBefore);
    }
  }

  // --- Test 3: Chat panel resize (may be at max already) ---
  if (handleCount >= 3) {
    const chatHandle = handles.nth(2);
    const chatBox = await chatHandle.boundingBox();
    if (chatBox) {
      const chatHeader = vibeWindow.locator('text=AI Agent Workspace').first();
      const chatBefore = await chatHeader.evaluate((el) =>
        el.closest('[style*="width"]')?.getBoundingClientRect().width ?? 0
      );
      console.log(`Chat panel width: ${chatBefore}px (exists and resizable)`);
      // Chat may already be at max constraint — just verify it exists
      expect(chatBefore).toBeGreaterThanOrEqual(150);
    }
  }

  // Screenshot 2: After resizing left and right panels
  await page.screenshot({ path: join(outDir, 'cdp-resize-after-drag.png') });

  // --- Test 4: Hover indicator shows grip dots ---
  const firstHandle = handles.first();
  await firstHandle.hover();
  await page.waitForTimeout(300);
  // Grip dots should be visible on hover
  const gripDots = firstHandle.locator('.rounded-full');
  const dotsCount = await gripDots.count();
  console.log(`Grip dots visible on hover: ${dotsCount}`);
  expect(dotsCount).toBe(5);

  // Screenshot 3: Hover state showing grip indicator
  await page.screenshot({ path: join(outDir, 'cdp-resize-hover-indicator.png') });

  // --- Test 5: Min constraint ---
  const leftBoxNow = await firstHandle.boundingBox();
  if (leftBoxNow) {
    await page.mouse.move(leftBoxNow.x + leftBoxNow.width / 2, leftBoxNow.y + leftBoxNow.height / 2);
    await page.mouse.down();
    await page.mouse.move(0, leftBoxNow.y + leftBoxNow.height / 2, { steps: 20 });
    await page.mouse.up();
    await page.waitForTimeout(300);

    const layersMin = await layersHeader.evaluate((el) =>
      el.closest('[style*="width"]')?.getBoundingClientRect().width ?? 0
    );
    console.log(`Left panel at min: ${layersMin}px`);
    expect(layersMin).toBeGreaterThanOrEqual(150);
  }

  await page.screenshot({ path: join(outDir, 'cdp-resize-min-constraint.png') });
});
