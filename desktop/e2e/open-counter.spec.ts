import { test, expect, type Page } from "@playwright/test";

const API = "http://localhost:8080";
const APP_NAME = "Sample Counter";
const APP_ID = "sample-gradio";

/**
 * Ensure the app is fully cleaned up via API before each test.
 */
async function ensureClean() {
  // Stop if running
  await fetch(`${API}/api/apps/${APP_ID}/stop`, { method: "POST" }).catch(() => {});
  // Uninstall if installed
  await fetch(`${API}/api/apps/${APP_ID}`, { method: "DELETE" }).catch(() => {});
}

test.describe("Open Button & Counter App", () => {
  test.beforeEach(async () => {
    await ensureClean();
  });

  test.afterAll(async () => {
    await ensureClean();
  });

  test("Open button appears only when app is running", async ({ page }) => {
    await page.goto("/catalog");
    await expect(page.locator(`text=${APP_NAME}`)).toBeVisible({ timeout: 10000 });

    const card = page.locator("div.card", { has: page.locator(`text=${APP_NAME}`) });

    // Not installed → no Open button
    await expect(card.locator("button", { hasText: "Open" })).not.toBeVisible();

    // Install
    await card.locator("button", { hasText: "Install" }).click();
    await expect(card.locator("button", { hasText: "Launch" })).toBeVisible({ timeout: 90000 });

    // Installed → still no Open button
    await expect(card.locator("button", { hasText: "Open" })).not.toBeVisible();

    // Launch
    await card.locator("button", { hasText: "Launch" }).click();
    await expect(card.locator(".status-running")).toBeVisible({ timeout: 15000 });

    // Running → Open button appears
    await expect(card.locator("button", { hasText: "Open" })).toBeVisible();
  });

  test("Open button calls window.open with correct port", async ({ page }) => {
    await page.goto("/catalog");
    await expect(page.locator(`text=${APP_NAME}`)).toBeVisible({ timeout: 10000 });

    const card = page.locator("div.card", { has: page.locator(`text=${APP_NAME}`) });

    // Install + Launch
    await card.locator("button", { hasText: "Install" }).click();
    await expect(card.locator("button", { hasText: "Launch" })).toBeVisible({ timeout: 90000 });
    await card.locator("button", { hasText: "Launch" }).click();
    await expect(card.locator("button", { hasText: "Open" })).toBeVisible({ timeout: 15000 });

    // Intercept window.open — store captured URL on window object
    await page.evaluate(() => {
      (window as any).__capturedOpenUrl = null;
      const orig = window.open;
      window.open = (...args: any[]) => {
        (window as any).__capturedOpenUrl = String(args[0]);
        window.open = orig; // restore
        return null;
      };
    });

    // Click Open
    await card.locator("button", { hasText: "Open" }).click();

    // Read captured URL
    const url = await page.evaluate(() => (window as any).__capturedOpenUrl);
    expect(url).toContain("localhost:7860");
  });

  test("full lifecycle: install → launch → running page → stop → uninstall", async ({ page }) => {
    const navClick = (name: RegExp) =>
      page.locator(".sidebar-nav").getByRole("link", { name }).click();

    await page.goto("/catalog");
    await expect(page.locator(`text=${APP_NAME}`)).toBeVisible({ timeout: 10000 });

    const card = page.locator("div.card", { has: page.locator(`text=${APP_NAME}`) });

    // Install
    await card.locator("button", { hasText: "Install" }).click();
    await expect(card.locator(".status-installed")).toBeVisible({ timeout: 90000 });

    // Verify tags
    await expect(card.locator(".tag", { hasText: "counter" })).toBeVisible();
    await expect(card.locator(".tag", { hasText: "demo" })).toBeVisible();

    // Launch
    await card.locator("button", { hasText: "Launch" }).click();
    await expect(card.locator(".status-running")).toBeVisible({ timeout: 15000 });

    // Check Running page
    await navClick(/Running/);
    await page.waitForURL("**/running");
    await expect(page.locator(`text=${APP_NAME}`)).toBeVisible({ timeout: 5000 });
    await expect(page.locator(".nav-badge")).toHaveText("1");

    // Stop
    const runCard = page.locator("div.card", { has: page.locator(`text=${APP_NAME}`) });
    await runCard.locator("button", { hasText: "Stop" }).click();
    await expect(page.locator("text=No apps running")).toBeVisible({ timeout: 15000 });

    // Verify still installed
    await navClick(/Installed/);
    await page.waitForURL("**/installed");
    await expect(page.locator(`text=${APP_NAME}`)).toBeVisible({ timeout: 5000 });

    // Uninstall
    page.on("dialog", (d) => d.accept());
    const instCard = page.locator("div.card", { has: page.locator(`text=${APP_NAME}`) });
    await instCard.locator("button", { hasText: "Uninstall" }).click();
    await expect(page.locator("text=No apps installed")).toBeVisible({ timeout: 15000 });
  });

  test("re-launch after stop shows Open button again", async ({ page }) => {
    await page.goto("/catalog");
    await expect(page.locator(`text=${APP_NAME}`)).toBeVisible({ timeout: 10000 });

    const card = page.locator("div.card", { has: page.locator(`text=${APP_NAME}`) });

    // Install + Launch
    await card.locator("button", { hasText: "Install" }).click();
    await expect(card.locator("button", { hasText: "Launch" })).toBeVisible({ timeout: 90000 });
    await card.locator("button", { hasText: "Launch" }).click();
    await expect(card.locator("button", { hasText: "Open" })).toBeVisible({ timeout: 15000 });

    // Stop — Open disappears
    await card.locator("button", { hasText: "Stop" }).click();
    await expect(card.locator("button", { hasText: "Launch" })).toBeVisible({ timeout: 15000 });
    await expect(card.locator("button", { hasText: "Open" })).not.toBeVisible();

    // Re-launch — Open reappears
    await card.locator("button", { hasText: "Launch" }).click();
    await expect(card.locator("button", { hasText: "Open" })).toBeVisible({ timeout: 15000 });
  });
});
