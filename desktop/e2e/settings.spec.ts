import { test, expect } from "@playwright/test";

test.describe("Settings Page", () => {
  test("should display system information", async ({ page }) => {
    await page.goto("/catalog");
    // Wait for data to load first
    await expect(page.locator("text=Sample Counter")).toBeVisible({ timeout: 5000 });
    // Navigate to settings
    await page.getByRole("link", { name: /Settings/ }).click();
    await page.waitForURL("**/settings");

    await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
    await expect(page.locator(".info-value", { hasText: "windows / x86_64" })).toBeVisible({ timeout: 5000 });
    await expect(page.locator(".info-value", { hasText: "Python 3.12.0" })).toBeVisible();
    await expect(page.locator(".info-value", { hasText: "NVIDIA detected" })).toBeVisible();
  });

  test("should show UV package manager info", async ({ page }) => {
    await page.goto("/catalog");
    await expect(page.locator("text=Sample Counter")).toBeVisible({ timeout: 5000 });
    await page.getByRole("link", { name: /Settings/ }).click();
    await page.waitForURL("**/settings");

    await expect(page.getByRole("heading", { name: "UV Package Manager" })).toBeVisible({ timeout: 5000 });
    await expect(page.locator(".info-value", { hasText: "0.6.0" })).toBeVisible();
  });

  test("should show stats section", async ({ page }) => {
    await page.goto("/catalog");
    await expect(page.locator("text=Sample Counter")).toBeVisible({ timeout: 5000 });
    await page.getByRole("link", { name: /Settings/ }).click();
    await page.waitForURL("**/settings");

    await expect(page.getByRole("heading", { name: "Stats" })).toBeVisible({ timeout: 5000 });
    await expect(page.locator(".stat-label", { hasText: "Installed" })).toBeVisible();
    await expect(page.locator(".stat-label", { hasText: "Running" })).toBeVisible();
  });

  test("should have refresh button", async ({ page }) => {
    await page.goto("/catalog");
    await expect(page.locator("text=Sample Counter")).toBeVisible({ timeout: 5000 });
    await page.getByRole("link", { name: /Settings/ }).click();
    await page.waitForURL("**/settings");

    const refreshBtn = page.locator("button", { hasText: "Refresh" });
    await expect(refreshBtn).toBeVisible();
    await refreshBtn.click();
    // Should still show system info after refresh
    await expect(page.locator(".info-value", { hasText: "windows / x86_64" })).toBeVisible({ timeout: 5000 });
  });
});
