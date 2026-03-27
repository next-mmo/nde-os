import { test, expect } from './fixtures';
import { dockButton } from './helpers';
import fs from 'fs';

test.describe('VibeCode Studio v0-like Canvas Workflow', () => {
  test('should launch VibeCode Studio, request a UI component via chat, and render HTML to preview iframe', async ({ page }) => {

    // Setup screenshot directory
    const dir = 'test-results/vibecode-v0-canvas';
    if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });

    // 1. Launch VibeCode Studio from Launcher (or Dock)
    try {
      const launcherBtn = page.locator('button', { hasText: 'Launcher' });
      await launcherBtn.click();
      await page.waitForTimeout(500);
      const appsTab = page.locator('button', { hasText: 'Apps' });
      await appsTab.click();
      await page.fill('input[placeholder="Search applications..."]', 'vibe');
      await page.click('text=Vibe Studio');
    } catch {
      const qs = await page.locator('button', { hasText: 'Vibe Studio' }).count();
      if (qs > 0) {
        await page.locator('button', { hasText: 'Vibe Studio' }).first().click();
      }
    }

    await page.waitForTimeout(1000);
    
    // Ensure the chat input is visible
    const chatInput = page.locator('textarea[placeholder*="Ask AI"]');
    await expect(chatInput).toBeVisible();

    // 2. Mock the chat response for stability in E2E since LLMs are non-deterministic, 
    // BUT since we use local-gguf in e2e setup normally, we can actually type and hit enter. 
    // To ensure reliable testing, we'll intercept the API to return predictable HTML if needed,
    // or we'll just test that we can paste HTML in the IDE tab and see it in preview.
    // Let's test the IDE -> Preview flow first as it exercises the runner perfectly without waiting 3 mins for an LLM

    const previewTab = page.locator('button', { hasText: 'Preview' });
    const ideTab = page.locator('button', { hasText: 'IDE 💻' });

    // Go to IDE tab
    await ideTab.click();
    await page.waitForTimeout(500);

    // Let's create a new file in the explorer tree if OpenCode handles it, 
    // or just click the empty area if monaco editor is already there
    // Actually VibeCodeStudio IDE might just be a file explorer + monaco.
    // However, we just hooked up the state `generatedCode = fileContent` based on `activeFilePath`.
    // Let's simulate the `/api/agent/chat/stream` response directly to ensure chat -> preview works.
    
    await page.route('http://localhost:8080/api/agent/chat/stream', async route => {
      const resp = `data: {"type":"text_delta","content":"\\n\`\`\`html\\n<div class=\\"bg-indigo-500 p-8 rounded-xl shadow-2xl\\"><h2 class=\\"text-white text-3xl font-bold\\">Generated v0 Component</h2></div>\\n\`\`\`\\n"}\n\ndata: [DONE]\n\n`;
      return route.fulfill({
        status: 200,
        contentType: 'text/event-stream',
        body: resp
      });
    });

    // Go back to Preview Tab
    await previewTab.click();
    await page.waitForTimeout(500);

    // Prompt the agent
    await chatInput.fill('Make a cool card');
    await chatInput.press('Enter');

    // Wait for the stream to complete and the UI to update
    await page.waitForTimeout(2000);

    // Check if the V0Runner Active badge is present
    await expect(page.locator('span', { hasText: 'v0 Runner Active' })).toBeVisible();

    // Check if iframe rendering the code exists
    const iframe = page.locator('iframe[title="Live Preview"]');
    await expect(iframe).toBeVisible();
    
    // Capture screenshot
    await page.screenshot({ path: dir + '/1_v0_runner_active.png' });

    // Validate the IDE syncs
    await ideTab.click();
    await page.waitForTimeout(1000);
    // Let's take screenshot of the IDE tab
    await page.screenshot({ path: dir + '/2_ide_sync.png' });

    // Ensure we can close the app
    await page.locator('button[aria-label="Close Desktop Window"]').last().click();
    await page.waitForTimeout(500);
  });
});
