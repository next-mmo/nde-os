import { test, expect } from "./fixtures";
import { clickButton, dockButton } from "./helpers";

test.describe("Service Hub window restore", () => {
  test("minimize and restore keeps the Service Hub content visible", async ({ page }) => {
    const anyIndicator = page
      .locator('[data-testid="desktop-wallpaper"]:visible, [data-testid="floating-fab"]:visible')
      .first();
    await expect(anyIndicator).toBeVisible({ timeout: 15_000 });

    const fabButton = page.locator('[data-testid="floating-fab"]');
    if (await fabButton.isVisible().catch(() => false)) {
      await clickButton(fabButton);
      await page.waitForTimeout(1500);
    }

    await page.evaluate(() => (window as any).__svelteDesktop?.openServiceHub());

    const serviceHubWindow = page.locator('section[data-window="service-hub"]');
    await expect(serviceHubWindow).toBeVisible({ timeout: 10_000 });

    const apiSwaggerButton = serviceHubWindow.getByRole("button", { name: /api swagger/i });
    const searchInput = serviceHubWindow.getByPlaceholder("Search services...");

    await expect(apiSwaggerButton).toBeVisible({ timeout: 10_000 });
    await expect(searchInput).toBeVisible({ timeout: 10_000 });

    await clickButton(serviceHubWindow.getByTestId("traffic-minimize"));
    await expect(serviceHubWindow).toHaveClass(/invisible/, { timeout: 5_000 });

    await page.waitForTimeout(4000);
    await clickButton(dockButton(page, /service hub/i));

    await expect(serviceHubWindow).toHaveClass(/visible/, { timeout: 5_000 });
    await expect(apiSwaggerButton).toBeVisible({ timeout: 10_000 });
    await expect(searchInput).toBeVisible({ timeout: 10_000 });
  });
});
