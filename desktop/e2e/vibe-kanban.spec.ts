import { test, expect } from "./fixtures";
import * as fs from "fs";
import * as path from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const TASKS_DIR = path.resolve(__dirname, "../../.agents/tasks");

/** Click element via JS (bypasses viewport bounds check) */
async function jsClick(page: import("@playwright/test").Page, selector: string) {
  await page.evaluate((sel) => {
    const el = document.querySelector(sel) as HTMLElement;
    if (el) el.click();
  }, selector);
}

/** Click element by matching its text content via JS */
async function jsClickByText(page: import("@playwright/test").Page, text: string) {
  await page.evaluate((t) => {
    const buttons = Array.from(document.querySelectorAll("button"));
    const btn = buttons.find(b => b.textContent?.trim() === t);
    if (btn) btn.click();
  }, text);
}

async function ensureCleanState(page: import("@playwright/test").Page) {
  await page.keyboard.press("Escape");
  await page.keyboard.press("Escape");
  await page.waitForTimeout(300);
}

test.describe("Vibe Code Studio Kanban", () => {
  test("kanban board loads and has columns", async ({ page }) => {
    await ensureCleanState(page);
    
    // Click Kanban tab via JS (bypasses viewport check)
    await jsClickByText(page, "Kanban 📋");
    await page.waitForTimeout(1000);
    
    // Screenshot
    await page.screenshot({ path: "test-results/vibe-kanban/01-columns-visible.png" });
    
    // Check columns (use evaluate to check text exists in DOM)
    const columnTexts = await page.evaluate(() => {
      return document.body.innerText;  
    });
    expect(columnTexts).toContain("Plan");
    expect(columnTexts).toContain("YOLO mode");
  });

  test("scrum master creates ticket via chat", async ({ page }) => {
    // Clean up any previous test ticket
    const testSlug = "e2e-test-counter-app";
    const testTicketPath = path.join(TASKS_DIR, `${testSlug}.md`);
    if (fs.existsSync(testTicketPath)) {
      fs.unlinkSync(testTicketPath);
    }

    await ensureCleanState(page);

    // Click Kanban tab via JS
    await jsClickByText(page, "Kanban 📋");
    await page.waitForTimeout(500);

    // Click Scrum Master button via JS  
    await jsClickByText(page, "Scrum Master");
    await page.waitForTimeout(500);

    // Screenshot: Scrum Master panel
    await page.screenshot({ path: "test-results/vibe-kanban/02-scrum-master-panel.png" });

    // Fill textarea via JS (put value and trigger events)
    await page.evaluate(() => {
      const ta = document.querySelector('textarea') as HTMLTextAreaElement;
      if (ta) {
        // Set value via native setter to trigger Svelte reactivity
        const nativeSetter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value')!.set!;
        nativeSetter.call(ta, 'create E2E Test Counter App');
        ta.dispatchEvent(new Event('input', { bubbles: true }));
      }
    });
    await page.waitForTimeout(300);

    // Press Enter to send
    await page.evaluate(() => {
      const ta = document.querySelector('textarea') as HTMLTextAreaElement;
      if (ta) {
        ta.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
      }
    });

    // Wait for ticket creation — check DOM for confirmation text
    let ticketCreated = false;
    for (let i = 0; i < 20; i++) {
      await page.waitForTimeout(500);
      const text = await page.evaluate(() => document.body.innerText);
      if (text.includes("Created ticket")) {
        ticketCreated = true;
        break;
      }
    }

    // Screenshot: ticket created
    await page.screenshot({ path: "test-results/vibe-kanban/03-ticket-created-chat.png" });

    expect(ticketCreated).toBe(true);

    // Verify the .md file was created on disk
    expect(fs.existsSync(testTicketPath)).toBe(true);
    const content = fs.readFileSync(testTicketPath, "utf-8");
    expect(content).toContain("# E2E Test Counter App");
    expect(content).toContain("- **Status:**");

    // Wait for Kanban to refresh
    await page.waitForTimeout(1500);
    const boardText = await page.evaluate(() => document.body.innerText);
    expect(boardText).toContain("E2E Test Counter App");

    // Screenshot: ticket on board
    await page.screenshot({ path: "test-results/vibe-kanban/04-ticket-on-board.png" });

    // Cleanup
    if (fs.existsSync(testTicketPath)) {
      fs.unlinkSync(testTicketPath);
    }
  });
});
