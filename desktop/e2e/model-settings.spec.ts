import { test, expect } from "./fixtures";
import { openLauncher, openRailSection } from "./helpers";

test.describe("Model Settings & LLM Providers", () => {
  test("can navigate to LLM Providers section", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "LLM Providers");

    await expect(page.locator(".model-settings")).toBeVisible({ timeout: 10000 });
    await expect(page.getByText("LLM Providers")).toBeVisible();
  });

  test("shows the existing GGUF provider", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "LLM Providers");

    // Should show at least the GGUF provider card
    const providerCard = page.locator(".provider-card").first();
    await expect(providerCard).toBeVisible({ timeout: 15000 });

    // Check that it's active and shows GGUF type
    await expect(providerCard.locator(".provider-type")).toContainText("gguf");
    await expect(providerCard.locator(".active-badge")).toBeVisible();
  });

  test("can open the Add Provider form", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "LLM Providers");

    // Click add provider button
    const addBtn = page.locator(".add-btn");
    await expect(addBtn).toBeVisible({ timeout: 10000 });
    await addBtn.click();

    // The add form should appear
    await expect(page.locator(".add-form")).toBeVisible({ timeout: 5000 });
    await expect(page.getByText("Add New Provider")).toBeVisible();

    // Should show provider type selector
    await expect(page.locator(".add-form select")).toBeVisible();
  });

  test("can select GGUF provider type and see GGUF panel", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "LLM Providers");

    const addBtn = page.locator(".add-btn");
    await addBtn.click();

    // Select GGUF provider type
    const select = page.locator(".add-form select");
    await select.selectOption("gguf");

    // Should show the GGUF panel with recommendations or local models
    await expect(page.locator(".gguf-panel")).toBeVisible({ timeout: 10000 });
    await expect(page.getByText("GGUF (Local Inference)")).toBeVisible();
  });

  test("can cancel adding a provider", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "LLM Providers");

    const addBtn = page.locator(".add-btn");
    await addBtn.click();
    await expect(page.locator(".add-form")).toBeVisible();

    // Click cancel
    await addBtn.click();
    await expect(page.locator(".add-form")).not.toBeVisible();
  });

  test("refresh button updates provider list", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "LLM Providers");

    const refreshBtn = page.locator(".refresh-btn");
    await expect(refreshBtn).toBeVisible({ timeout: 10000 });
    await refreshBtn.click();

    // After refresh, providers should still be visible
    await expect(page.locator(".provider-card").first()).toBeVisible({ timeout: 10000 });
  });
});
