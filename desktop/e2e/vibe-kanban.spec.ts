import { test, expect } from "./fixtures";
import { openLauncher, openRailSection } from "./helpers";

test.describe("Vibe Code Studio Kanban", () => {
  test("kanban board loads and has columns", async ({ page }) => {
    await openLauncher(page);
    
    // Switch to Vibe Code Studio section
    await openRailSection(page, "Vibe Code Studio");
    
    // Switch to Kanban tab
    await page.getByRole('button', { name: "Kanban 📋" }).click();
    
    // Check columns
    await expect(page.getByText("Plan", { exact: true })).toBeVisible({ timeout: 10000 });
    await expect(page.getByText("Waiting Approval", { exact: true })).toBeVisible();
    await expect(page.getByText("YOLO mode", { exact: true })).toBeVisible();
    await expect(page.getByText("Done by AI", { exact: true })).toBeVisible();
    await expect(page.getByText("Verified by Human", { exact: true })).toBeVisible();
    await expect(page.getByText("Re-open", { exact: true })).toBeVisible();
  });
});
