import { test, expect } from "./fixtures";
import { openLauncher, openRailSection } from "./helpers";

test.describe("Vibe Code Studio IDE", () => {
  test("ide tab loads explorer and source control", async ({ page }) => {
    await openLauncher(page);
    
    // Switch to Vibe Code Studio section
    await openRailSection(page, "Vibe Code Studio");
    
    // Switch to IDE tab
    await page.getByRole('button', { name: "IDE 💻" }).click();
    
    // Check Explorer is visible
    await expect(page.getByText("Explorer", { exact: true })).toBeVisible({ timeout: 10000 });
    
    // Validate empty state
    await expect(page.getByText("OpenCode IDE")).toBeVisible();
    
    // Check Source Control activity bar button click
    await page.getByTitle("Source Control").click();
    await expect(page.getByText("Source Control", { exact: true })).toBeVisible({ timeout: 10000 });
  });
});
