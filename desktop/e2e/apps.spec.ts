import { test, expect } from "@playwright/test";

test.describe("Installed & Running Pages (Empty State)", () => {
  test("installed page shows empty state with link to catalog", async ({ page }) => {
    await page.goto("/catalog");
    await expect(page.locator("text=Sample Counter")).toBeVisible({ timeout: 5000 });

    await page.locator(".sidebar-nav").getByRole("link", { name: /Installed/ }).click();
    await page.waitForURL("**/installed");
    await expect(page.locator("text=No apps installed")).toBeVisible({ timeout: 5000 });

    // Click the catalog link in the empty state
    await page.locator(".empty-hint a").click();
    await page.waitForURL("**/catalog");
    expect(page.url()).toContain("/catalog");
  });

  test("running page shows empty state with link to installed", async ({ page }) => {
    await page.goto("/catalog");
    await expect(page.locator("text=Sample Counter")).toBeVisible({ timeout: 5000 });

    await page.locator(".sidebar-nav").getByRole("link", { name: /Running/ }).click();
    await page.waitForURL("**/running");
    await expect(page.locator("text=No apps running")).toBeVisible({ timeout: 5000 });
  });
});

test.describe("Multiple App Install", () => {
  test("should install multiple apps and see correct counts", async ({ page }) => {
    await page.goto("/catalog");
    await expect(page.locator("text=Sample Counter")).toBeVisible({ timeout: 5000 });

    // Install Sample Counter
    const counterCard = page.locator("div.card", { has: page.locator("text=Sample Counter") });
    await counterCard.locator("button", { hasText: "Install" }).click();
    await expect(counterCard.locator("button", { hasText: "Launch" })).toBeVisible({ timeout: 10000 });

    // Install Ollama
    const ollamaCard = page.locator("div.card", { has: page.locator("text=Ollama") });
    await ollamaCard.locator("button", { hasText: "Install" }).click();
    await expect(ollamaCard.locator("button", { hasText: "Launch" })).toBeVisible({ timeout: 10000 });

    // Navigate to Installed page using sidebar link
    await page.locator(".sidebar-nav").getByRole("link", { name: /Installed/ }).click();
    await page.waitForURL("**/installed");
    await expect(page.locator("text=2 app(s) installed")).toBeVisible({ timeout: 5000 });
    await expect(page.locator("text=Sample Counter")).toBeVisible();
    await expect(page.locator("text=Ollama")).toBeVisible();
  });
});
