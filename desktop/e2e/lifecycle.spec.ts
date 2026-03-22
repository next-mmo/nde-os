import { test, expect } from "@playwright/test";

test.describe("App Lifecycle (Install → Launch → Stop → Uninstall)", () => {
  test("full lifecycle: install, verify, launch, stop, uninstall", async ({ page }) => {
    // Helper: click sidebar nav link
    const navClick = (name: RegExp) => page.locator(".sidebar-nav").getByRole("link", { name }).click();

    // ── STEP 1: Navigate to catalog, install Sample Counter ──
    await page.goto("/catalog");
    await expect(page.locator("text=Sample Counter")).toBeVisible({ timeout: 5000 });

    const counterCard = page.locator("div.card", { has: page.locator("text=Sample Counter") });
    await counterCard.locator("button", { hasText: "Install" }).click();

    // Wait for install to complete — button should change to Launch
    await expect(counterCard.locator("button", { hasText: "Launch" })).toBeVisible({ timeout: 10000 });
    await expect(counterCard.locator("text=Installed")).toBeVisible();

    // ── STEP 2: Verify app appears in Installed page ──
    await navClick(/Installed/);
    await page.waitForURL("**/installed");
    await expect(page.locator("text=Sample Counter")).toBeVisible({ timeout: 5000 });
    await expect(page.locator("text=1 app(s) installed")).toBeVisible();

    // ── STEP 3: Launch the app ──
    const installedCard = page.locator("div.card", { has: page.locator("text=Sample Counter") });
    await installedCard.locator("button", { hasText: "Launch" }).click();

    // Wait for the app to transition to "Running"
    await expect(installedCard.locator(".status-running")).toBeVisible({ timeout: 10000 });

    // Should show Open and Stop buttons when running
    await expect(installedCard.locator("button", { hasText: "Open" })).toBeVisible();
    await expect(installedCard.locator("button", { hasText: "Stop" })).toBeVisible();

    // ── STEP 4: Check Running page ──
    await navClick(/Running/);
    await page.waitForURL("**/running");
    await expect(page.locator("text=1 app(s) running")).toBeVisible({ timeout: 5000 });
    await expect(page.locator("text=Sample Counter")).toBeVisible();

    // Running badge should appear in sidebar nav
    await expect(page.locator(".nav-badge")).toHaveText("1");

    // ── STEP 5: Stop the app ──
    const runningCard = page.locator("div.card", { has: page.locator("text=Sample Counter") });
    await runningCard.locator("button", { hasText: "Stop" }).click();

    // After stop, should show empty state
    await expect(page.locator("text=No apps running")).toBeVisible({ timeout: 10000 });

    // ── STEP 6: Verify app goes back to Installed state ──
    await navClick(/Installed/);
    await page.waitForURL("**/installed");
    const reinstalledCard = page.locator("div.card", { has: page.locator("text=Sample Counter") });
    await expect(reinstalledCard.locator(".status-installed")).toBeVisible({ timeout: 5000 });

    // ── STEP 7: Uninstall the app ──
    page.on("dialog", async (dialog) => {
      await dialog.accept();
    });

    await reinstalledCard.locator("button", { hasText: "Uninstall" }).click();

    // Should show empty state after uninstall
    await expect(page.locator("text=No apps installed")).toBeVisible({ timeout: 10000 });

    // ── STEP 8: Catalog should show Install button again ──
    await navClick(/Catalog/);
    await page.waitForURL("**/catalog");
    const catalogCard = page.locator("div.card", { has: page.locator("text=Sample Counter") });
    await expect(catalogCard.locator("button", { hasText: "Install" })).toBeVisible({ timeout: 5000 });
  });
});
