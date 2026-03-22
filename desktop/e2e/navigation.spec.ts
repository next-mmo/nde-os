import { test, expect } from "@playwright/test";

test.describe("Navigation & Layout", () => {
  test("should redirect root to /catalog", async ({ page }) => {
    await page.goto("/");
    await page.waitForURL("**/catalog");
    expect(page.url()).toContain("/catalog");
  });

  test("should render sidebar with brand and nav items", async ({ page }) => {
    await page.goto("/catalog");
    await expect(page.locator("text=Sample Counter")).toBeVisible({ timeout: 5000 });
    await expect(page.locator("text=AI Launcher")).toBeVisible();
    await expect(page.locator("text=v0.2.0")).toBeVisible();
    await expect(page.getByRole("link", { name: /Catalog/ })).toBeVisible();
    await expect(page.getByRole("link", { name: /Installed/ })).toBeVisible();
    await expect(page.getByRole("link", { name: /Running/ })).toBeVisible();
    await expect(page.getByRole("link", { name: /Logs/ })).toBeVisible();
    await expect(page.getByRole("link", { name: /Settings/ })).toBeVisible();
  });

  test("should highlight active nav item", async ({ page }) => {
    await page.goto("/catalog");
    await expect(page.locator("text=Sample Counter")).toBeVisible({ timeout: 5000 });
    const catalogLink = page.getByRole("link", { name: /Catalog/ });
    await expect(catalogLink).toHaveClass(/active/);

    await page.getByRole("link", { name: /Settings/ }).click();
    await page.waitForURL("**/settings");
    const settingsLink = page.getByRole("link", { name: /Settings/ });
    await expect(settingsLink).toHaveClass(/active/);
  });

  test("should navigate to all 5 routes", async ({ page }) => {
    await page.goto("/catalog");
    await expect(page.locator("text=Sample Counter")).toBeVisible({ timeout: 5000 });

    for (const [name, path] of [
      ["Installed", "/installed"],
      ["Running", "/running"],
      ["Logs", "/logs"],
      ["Settings", "/settings"],
      ["Catalog", "/catalog"],
    ]) {
      await page.getByRole("link", { name: new RegExp(name) }).click();
      await page.waitForURL(`**${path}`);
      expect(page.url()).toContain(path);
    }
  });

  test("should show system info in sidebar footer", async ({ page }) => {
    await page.goto("/catalog");
    // Wait for mock data to load
    await expect(page.locator("text=Sample Counter")).toBeVisible({ timeout: 5000 });
    await expect(page.locator(".sys-value", { hasText: "windows/x86_64" })).toBeVisible({ timeout: 5000 });
    await expect(page.locator(".sys-value", { hasText: "Python 3.12.0" })).toBeVisible();
  });
});
