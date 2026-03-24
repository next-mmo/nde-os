import { test, expect } from "./fixtures";
import { clickButton, dock } from "./helpers";

test.describe("Shield Browser", () => {
  test.beforeEach(async ({ page }) => {
    // Wait for the dock to be ready
    await expect(dock(page)).toBeVisible({ timeout: 15000 });
  });

  test("shield-browser dock icon opens the window", async ({ page }) => {
    // Find and click the Shield Browser dock button
    const shieldBtn = dock(page).getByRole("button", { name: /shield/i });
    await clickButton(shieldBtn);

    // Window should open
    const shieldWindow = page.locator('[data-window="shield-browser"]');
    await expect(shieldWindow).toBeVisible({ timeout: 10000 });

    // Header should be visible
    await expect(shieldWindow.locator("h2")).toContainText("Shield Browser", { timeout: 10000 });
    await expect(shieldWindow.locator(".eyebrow")).toContainText("Anti-Detect Browser");
  });

  test("shield-browser shows status bar with counts", async ({ page }) => {
    const shieldBtn = dock(page).getByRole("button", { name: /shield/i });
    await clickButton(shieldBtn);

    const shieldWindow = page.locator('[data-window="shield-browser"]');
    await expect(shieldWindow).toBeVisible({ timeout: 10000 });

    // Status bar should show Profiles, Running, and Engines counts
    const statusBar = shieldWindow.locator(".status-bar");
    await expect(statusBar).toBeVisible();
    await expect(statusBar.getByText("Profiles")).toBeVisible();
    await expect(statusBar.getByText("Running")).toBeVisible();
    await expect(statusBar.getByText("Engines")).toBeVisible();
  });

  test("shield-browser can open and close create profile form", async ({ page }) => {
    const shieldBtn = dock(page).getByRole("button", { name: /shield/i });
    await clickButton(shieldBtn);

    const shieldWindow = page.locator('[data-window="shield-browser"]');
    await expect(shieldWindow).toBeVisible({ timeout: 10000 });

    // Click "New Profile" button
    await clickButton(shieldWindow.getByRole("button", { name: /new profile/i }));

    // Create form should appear
    await expect(shieldWindow.getByText("Create New Profile")).toBeVisible();
    await expect(shieldWindow.locator("#profile-name")).toBeVisible();
    await expect(shieldWindow.locator("#engine-select")).toBeVisible();

    // Engine selector should have Wayfern and Camoufox options
    const select = shieldWindow.locator("#engine-select");
    await expect(select.locator("option")).toHaveCount(2);

    // Back button should return to profiles view
    await clickButton(shieldWindow.getByRole("button", { name: /back/i }));
    await expect(shieldWindow.getByText("Create New Profile")).not.toBeVisible();
  });

  test("shield-browser can create and delete a profile", async ({ page }) => {
    // Set up dialog handler before navigating
    page.on("dialog", (dialog) => dialog.accept());

    const shieldBtn = dock(page).getByRole("button", { name: /shield/i });
    await clickButton(shieldBtn);

    const shieldWindow = page.locator('[data-window="shield-browser"]');
    await expect(shieldWindow).toBeVisible({ timeout: 10000 });

    // Open create form
    await clickButton(shieldWindow.getByRole("button", { name: /new profile/i }));
    await expect(shieldWindow.getByText("Create New Profile")).toBeVisible();

    // Fill in the form
    const nameInput = shieldWindow.locator("#profile-name");
    const uniqueName = `E2E-Test-${Date.now()}`;
    await nameInput.fill(uniqueName);

    // Select Camoufox engine
    await shieldWindow.locator("#engine-select").selectOption("camoufox");

    // Create the profile
    await clickButton(shieldWindow.getByRole("button", { name: "Create Profile" }));

    // Should return to profile list and show the new profile
    await expect(shieldWindow.getByText(uniqueName)).toBeVisible({ timeout: 5000 });

    // Click the profile card to select it
    await shieldWindow.getByText(uniqueName).click();

    // Detail panel should show engine info
    await expect(shieldWindow.locator(".detail-panel")).toBeVisible();

    // Delete the profile
    await clickButton(shieldWindow.getByRole("button", { name: /delete/i }));

    // Wait for deletion and refresh
    await page.waitForTimeout(1500);

    // Profile should be removed
    await expect(shieldWindow.getByText(uniqueName)).not.toBeVisible({ timeout: 5000 });
  });

  test("shield-browser shows detail placeholder when nothing selected", async ({ page }) => {
    const shieldBtn = dock(page).getByRole("button", { name: /shield/i });
    await clickButton(shieldBtn);

    const shieldWindow = page.locator('[data-window="shield-browser"]');
    await expect(shieldWindow).toBeVisible({ timeout: 10000 });

    // Either empty state or unselected detail panel
    const detailPlaceholder = shieldWindow.getByText("Select a profile to view details");
    const emptyState = shieldWindow.getByText("No Profiles Yet");
    
    // One of these should be visible depending on whether profiles exist
    const detailVisible = await detailPlaceholder.isVisible().catch(() => false);
    const emptyVisible = await emptyState.isVisible().catch(() => false);
    
    expect(detailVisible || emptyVisible).toBe(true);
  });
});
