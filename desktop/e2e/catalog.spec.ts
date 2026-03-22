import { test, expect } from "@playwright/test";

test.describe("Catalog Page", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/catalog");
    // Wait for mock catalog to load
    await expect(page.locator("text=Sample Counter")).toBeVisible({ timeout: 5000 });
  });

  test("should display page header", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "App Catalog" })).toBeVisible();
    await expect(page.locator("text=Browse and install AI applications")).toBeVisible();
  });

  test("should show all 3 catalog apps", async ({ page }) => {
    await expect(page.locator("text=Sample Counter")).toBeVisible();
    await expect(page.locator("text=Stable Diffusion WebUI")).toBeVisible();
    await expect(page.locator("text=Ollama")).toBeVisible();
  });

  test("should show app descriptions", async ({ page }) => {
    await expect(page.locator("text=A simple Gradio counter app")).toBeVisible();
    await expect(page.locator("text=AUTOMATIC1111 Stable Diffusion")).toBeVisible();
    await expect(page.locator("text=Run large language models locally")).toBeVisible();
  });

  test("should show GPU tags for GPU apps", async ({ page }) => {
    // Stable Diffusion and Ollama need GPU
    const gpuTags = page.locator(".tag-gpu");
    expect(await gpuTags.count()).toBe(2);
  });

  test("should show Install button for uninstalled apps", async ({ page }) => {
    const installButtons = page.locator("button", { hasText: "Install" });
    expect(await installButtons.count()).toBe(3);
  });

  test("should filter apps by search", async ({ page }) => {
    const search = page.getByPlaceholder("Search apps, tags...");
    await search.fill("ollama");
    await expect(page.locator("text=Ollama")).toBeVisible();
    await expect(page.locator("text=Sample Counter")).not.toBeVisible();
    await expect(page.locator("text=Stable Diffusion")).not.toBeVisible();
  });

  test("should filter apps by tag", async ({ page }) => {
    const search = page.getByPlaceholder("Search apps, tags...");
    await search.fill("gpu");
    await expect(page.locator("text=Stable Diffusion WebUI")).toBeVisible();
    await expect(page.locator("text=Ollama")).toBeVisible();
    await expect(page.locator("text=Sample Counter")).not.toBeVisible();
  });

  test("should show empty state for no matches", async ({ page }) => {
    const search = page.getByPlaceholder("Search apps, tags...");
    await search.fill("nonexistent-app-xyz");
    await expect(page.locator("text=No apps match")).toBeVisible();
  });

  test("should clear search and restore results", async ({ page }) => {
    const search = page.getByPlaceholder("Search apps, tags...");
    await search.fill("ollama");
    expect(await page.locator("text=Sample Counter").count()).toBe(0);

    await search.clear();
    await expect(page.locator("text=Sample Counter")).toBeVisible();
    await expect(page.locator("text=Ollama")).toBeVisible();
  });
});
