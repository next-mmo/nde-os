import { test, expect } from './fixtures';
import { attachScreenshot } from './helpers';
import * as path from 'path';
import * as fs from 'fs';

test('Take FreeCut screenshot', async ({ page }) => {
  // Wait for the desktop shell to load
  await page.waitForSelector('[data-testid="dock"], [data-window]');

  // Open FreeCut programmatically
  await page.evaluate(() => {
    (window as any).__svelteDesktop?.openStaticApp("freecut");
  });

  // Wait for FreeCut window to be visible
  await page.waitForSelector('[data-window="freecut"]', { state: 'visible' });

  // Give it a moment to render timeline etc
  await page.waitForTimeout(2000);

  // Take screenshot
  const artifactsDir = path.resolve(process.cwd(), '..', 'artifacts');
  if (!fs.existsSync(artifactsDir)) {
    fs.mkdirSync(artifactsDir, { recursive: true });
  }
  
  await page.screenshot({ path: path.join(artifactsDir, 'freecut_screenshot.png') });
});
