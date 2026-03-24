import { test, expect } from "./fixtures";
import { clickButton, dock } from "./helpers";

test.describe("Shield Browser", () => {
  test.beforeEach(async ({ page }) => {
    // Wait for the dock to be ready
    await expect(dock(page)).toBeVisible({ timeout: 15000 });
  });

  async function openShieldBrowser(page: import("@playwright/test").Page) {
    const shieldBtn = dock(page).getByRole("button", { name: /shield/i });
    await clickButton(shieldBtn);
    const shieldWindow = page.locator('[data-window="shield-browser"]');
    await expect(shieldWindow).toBeVisible({ timeout: 10000 });
    return shieldWindow;
  }

  test("shield-browser dock icon opens the window", async ({ page }) => {
    const shieldWindow = await openShieldBrowser(page);

    // Should show either setup screen or header depending on engine state
    const setupScreen = shieldWindow.locator(".setup-screen");
    const header = shieldWindow.locator("h2");

    const isSetup = await setupScreen.isVisible().catch(() => false);
    if (isSetup) {
      // Onboarding: should show setup hero
      await expect(shieldWindow.getByText("Set Up Shield Browser")).toBeVisible();
    } else {
      // Profiles view: should show header
      await expect(header).toContainText("Shield Browser", { timeout: 10000 });
    }
  });

  test("shield-browser shows setup onboarding when no engines installed", async ({ page }) => {
    const shieldWindow = await openShieldBrowser(page);

    // Check for either setup view or profiles view
    const setupScreen = shieldWindow.locator(".setup-screen");
    const isSetup = await setupScreen.isVisible().catch(() => false);

    if (isSetup) {
      // Should show setup hero
      await expect(shieldWindow.getByText("Set Up Shield Browser")).toBeVisible();

      // Should show engine cards (loaded async from Tauri)
      await expect(shieldWindow.getByText(/Camoufox/)).toBeVisible({ timeout: 10000 });
      await expect(shieldWindow.getByText(/Wayfern/)).toBeVisible({ timeout: 5000 });

      // Wayfern should be marked as coming soon
      await expect(shieldWindow.getByText("Coming Soon")).toBeVisible();

      // Should have an install button
      const installBtn = shieldWindow.locator(".install-btn");
      await expect(installBtn).toBeVisible({ timeout: 10000 });

      // Skip button should be available
      await expect(shieldWindow.getByText(/skip setup/i)).toBeVisible();
    }
    // If engines are already installed, setup won't appear — that's fine
  });

  test("shield-browser skip setup goes to profiles view", async ({ page }) => {
    const shieldWindow = await openShieldBrowser(page);

    const setupScreen = shieldWindow.locator(".setup-screen");
    const isSetup = await setupScreen.isVisible().catch(() => false);

    if (isSetup) {
      // Click skip
      await clickButton(shieldWindow.getByText(/skip setup/i));

      // Should now show profiles view with header
      await expect(shieldWindow.locator(".header")).toBeVisible({ timeout: 5000 });
      await expect(shieldWindow.locator("h2")).toContainText("Shield Browser");
    }

    // Verify status bar is visible (always present in profiles view)
    await expect(shieldWindow.locator(".status-bar")).toBeVisible({ timeout: 5000 });
    await expect(shieldWindow.locator(".status-bar").getByText("Profiles")).toBeVisible();
    await expect(shieldWindow.locator(".status-bar").getByText("Running")).toBeVisible();
    await expect(shieldWindow.locator(".status-bar").getByText("Engines")).toBeVisible();
  });

  test("shield-browser can open create profile form after skip", async ({ page }) => {
    const shieldWindow = await openShieldBrowser(page);

    // Skip setup if shown
    const setupScreen = shieldWindow.locator(".setup-screen");
    const isSetup = await setupScreen.isVisible().catch(() => false);
    if (isSetup) {
      await clickButton(shieldWindow.getByText(/skip setup/i));
      await expect(shieldWindow.locator(".header")).toBeVisible({ timeout: 5000 });
    }

    // Click "New Profile" button
    await clickButton(shieldWindow.getByRole("button", { name: /new profile/i }));

    // Create form should appear
    await expect(shieldWindow.getByText("Create New Profile")).toBeVisible();
    await expect(shieldWindow.locator("#profile-name")).toBeVisible();
    await expect(shieldWindow.locator("#engine-select")).toBeVisible();

    // Only available engines shown (Camoufox only since Wayfern is coming soon)
    const options = shieldWindow.locator("#engine-select option");
    const count = await options.count();
    expect(count).toBeGreaterThanOrEqual(1);

    // Back button should return to profiles view
    await clickButton(shieldWindow.getByRole("button", { name: /back/i }));
    await expect(shieldWindow.getByText("Create New Profile")).not.toBeVisible();
  });

  test("shield-browser detail placeholder or empty state visible", async ({ page }) => {
    const shieldWindow = await openShieldBrowser(page);

    // Skip setup if shown
    const setupScreen = shieldWindow.locator(".setup-screen");
    const isSetup = await setupScreen.isVisible().catch(() => false);
    if (isSetup) {
      await clickButton(shieldWindow.getByText(/skip setup/i));
      await expect(shieldWindow.locator(".header")).toBeVisible({ timeout: 5000 });
    }

    // Either empty state or unselected detail panel
    const detailPlaceholder = shieldWindow.getByText("Select a profile to view details");
    const emptyState = shieldWindow.getByText("No Profiles Yet");

    const detailVisible = await detailPlaceholder.isVisible().catch(() => false);
    const emptyVisible = await emptyState.isVisible().catch(() => false);

    expect(detailVisible || emptyVisible).toBe(true);
  });
});
