import { test, expect } from "./fixtures";
import { openLauncher, openRailSection } from "./helpers";

test.describe("Launcher sections — all rail links navigate correctly", () => {
  test("Overview shows stat cards and quick launch", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Overview");

    await expect(page.locator(".overview-grid")).toBeVisible({ timeout: 10000 });
    await expect(page.locator(".stat-grid")).toBeVisible();
    await expect(page.getByText("Quick launch")).toBeVisible();
    await expect(page.getByText("Recent running apps")).toBeVisible();
  });

  test("Command Center opens", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Command Center");

    // CommandCenter component should render
    await expect(page.locator(".embedded-app, .command-center, [class*='command']")).toBeVisible({ timeout: 10000 });
  });

  test("Catalog shows app grid with action buttons", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Catalog");

    // Should show the catalog panel
    await expect(page.locator(".catalog-panel")).toBeVisible({ timeout: 10000 });
    await expect(page.getByText("Explore AI apps")).toBeVisible();

    // Should have at least one catalog card
    await expect(page.locator("[data-app-id]").first()).toBeVisible({ timeout: 10000 });

    // Grid/List toggle should exist
    await expect(page.locator("[data-catalog-layout='grid']")).toBeVisible();
    await expect(page.locator("[data-catalog-layout='list']")).toBeVisible();
  });

  test("catalog layout toggle switches between grid and list", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Catalog");

    // Default is grid
    await expect(page.locator("[data-catalog-surface][data-layout='grid']")).toBeVisible({ timeout: 10000 });

    // Click list button
    await page.locator("[data-catalog-layout='list']").click();
    await expect(page.locator("[data-catalog-surface][data-layout='list']")).toBeVisible({ timeout: 5000 });

    // Click grid button back
    await page.locator("[data-catalog-layout='grid']").click();
    await expect(page.locator("[data-catalog-surface][data-layout='grid']")).toBeVisible({ timeout: 5000 });
  });

  test("Installed section loads", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Installed");

    // Should show the Installed panel
    await expect(page.getByText("Ready to run")).toBeVisible({ timeout: 10000 });
  });

  test("Running section loads", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Running");

    await expect(page.getByText("Live sessions")).toBeVisible({ timeout: 10000 });
  });

  test("Server & System section shows system info", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Server & System");

    await expect(page.getByText("Runtime status")).toBeVisible({ timeout: 10000 });
    await expect(page.getByText("Server health")).toBeVisible({ timeout: 15000 });
  });

  test("Chat section renders", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Chat");
    await expect(page.locator(".chat-app")).toBeVisible({ timeout: 10000 });
  });

  test("LLM Providers section renders", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "LLM Providers");
    await expect(page.locator(".model-settings")).toBeVisible({ timeout: 10000 });
  });

  test("Plugins section renders", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Plugins");
    await expect(page.locator(".embedded-app")).toBeVisible({ timeout: 10000 });
  });

  test("Channels section renders", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Channels");
    await expect(page.locator(".embedded-app")).toBeVisible({ timeout: 10000 });
  });

  test("MCP Tools section renders", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "MCP Tools");
    await expect(page.locator(".embedded-app")).toBeVisible({ timeout: 10000 });
  });

  test("Skills section renders", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Skills");
    await expect(page.locator(".embedded-app")).toBeVisible({ timeout: 10000 });
  });

  test("Knowledge section renders", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Knowledge");
    await expect(page.locator(".embedded-app")).toBeVisible({ timeout: 10000 });
  });

  test("Code Editor section renders", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Code Editor");
    await expect(page.locator(".embedded-app")).toBeVisible({ timeout: 10000 });
  });
});
