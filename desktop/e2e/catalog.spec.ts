import { test, expect } from "@playwright/test";
import { APP_NAME, appRow, openLauncher, openRailSection } from "./helpers";

test.describe("Launcher catalog", () => {
  test("shows catalog apps and filters them from the workspace toolbar", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Catalog");

    await expect(page.locator('[data-catalog-surface][data-layout="grid"]')).toBeVisible();
    await expect(appRow(page, "sample-gradio")).toContainText(APP_NAME, { timeout: 10000 });
    await expect(appRow(page, "ollama")).toContainText("Ollama");

    await page.getByRole("button", { name: "List" }).click();
    await expect(page.locator('[data-catalog-surface][data-layout="list"]')).toBeVisible();
    await expect(appRow(page, "sample-gradio")).toContainText(APP_NAME);

    const search = page.getByRole("textbox", { name: "Search AI apps" });
    await search.fill("ollama");

    await expect(appRow(page, "ollama")).toContainText("Ollama");
    await expect(page.locator('[data-app-id="sample-gradio"]')).toHaveCount(0);
  });

  test("overview and server sections remain available inside the main launcher window", async ({ page }) => {
    await openLauncher(page);

    await expect(page.getByText("Manage local AI apps like browser workspaces.")).toBeVisible();

    await openRailSection(page, "Server & System");
    await expect(page.getByText("Runtime status")).toBeVisible();
    await expect(page.getByText(/Server health/i)).toBeVisible();
  });
});
