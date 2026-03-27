import { test, expect } from "./fixtures";

test("debug: find kanban tab", async ({ page }) => {
  await page.waitForTimeout(1000);
  
  // Close any menus by pressing Escape multiple times
  await page.keyboard.press("Escape");
  await page.keyboard.press("Escape");
  await page.waitForTimeout(500);

  // Screenshot 
  await page.screenshot({ path: "test-results/debug-kanban-2.png" });

  // Find all elements containing "kanban" text
  const kanbanEls = await page.getByText(/kanban/i).evaluateAll(
    (els) => els.map(el => ({
      tag: el.tagName,
      text: el.textContent?.trim().slice(0, 80),
      visible: el.offsetParent !== null,
      classes: el.className?.slice(0, 80),
    }))
  );
  console.log("Kanban elements:", JSON.stringify(kanbanEls, null, 2));

  // Find ALL visible buttons on the page
  const allButtons = await page.locator('button').evaluateAll(
    (els) => els.map(el => ({
      text: el.textContent?.trim().slice(0, 60),
      visible: el.offsetParent !== null,
    })).filter(b => b.visible && b.text && b.text.length > 0)
  );
  console.log("ALL visible buttons:", JSON.stringify(allButtons.slice(0, 40), null, 2));

  // Find the Scrum Master text
  const scrumEls = await page.getByText(/scrum/i).evaluateAll(
    (els) => els.map(el => ({
      tag: el.tagName,
      text: el.textContent?.trim().slice(0, 80),
      visible: el.offsetParent !== null,
    }))
  );
  console.log("Scrum elements:", JSON.stringify(scrumEls, null, 2));

  expect(true).toBe(true);
});
