import { test, expect } from "@playwright/test";

test.describe("Activity Logs Page", () => {
  test("should show empty state initially", async ({ page }) => {
    await page.goto("/logs");
    await expect(page.getByRole("heading", { name: "Activity Logs" })).toBeVisible();
    await expect(page.locator("text=No activity yet")).toBeVisible();
  });

  test("should record log entries when installing an app", async ({ page }) => {
    // Install an app to generate log entries
    await page.goto("/catalog");
    await expect(page.locator("text=Sample Counter")).toBeVisible({ timeout: 5000 });

    const counterCard = page.locator("div.card", { has: page.locator("text=Sample Counter") });
    await counterCard.locator("button", { hasText: "Install" }).click();
    await expect(counterCard.locator("button", { hasText: "Launch" })).toBeVisible({ timeout: 10000 });

    // Navigate to logs
    await page.getByRole("link", { name: /Logs/ }).click();
    await page.waitForURL("**/logs");

    // Should have log entries for the install
    await expect(page.locator("text=Installing Sample Counter")).toBeVisible();
    await expect(page.locator("text=Sample Counter installed")).toBeVisible();
  });

  test("should clear logs", async ({ page }) => {
    // Install to get some logs
    await page.goto("/catalog");
    await expect(page.locator("text=Sample Counter")).toBeVisible({ timeout: 5000 });
    const counterCard = page.locator("div.card", { has: page.locator("text=Sample Counter") });
    await counterCard.locator("button", { hasText: "Install" }).click();
    await expect(counterCard.locator("button", { hasText: "Launch" })).toBeVisible({ timeout: 10000 });

    await page.getByRole("link", { name: /Logs/ }).click();
    await page.waitForURL("**/logs");
    await expect(page.locator("text=Installing Sample Counter")).toBeVisible();

    // Clear logs
    await page.locator("button", { hasText: "Clear" }).click();
    await expect(page.locator("text=No activity yet")).toBeVisible();
  });
});
