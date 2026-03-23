import { test, expect } from "./fixtures";
import { APP_ID, APP_NAME, appRow, clickButton, ensureCleanApp, openLauncher, openRailSection } from "./helpers";

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

  test("shows install loading state while the install request is pending", async ({ page }) => {
    await ensureCleanApp();

    await page.route("http://localhost:8080/api/apps", async (route, request) => {
      if (request.method() === "POST") {
        await page.waitForTimeout(1200);
      }
      await route.continue();
    });

    await openLauncher(page);
    await openRailSection(page, "Catalog");

    const row = appRow(page, APP_ID);
    const installResponse = page.waitForResponse((response) => {
      return response.url() === "http://localhost:8080/api/apps" && response.request().method() === "POST";
    });
    await expect(row).toContainText(APP_NAME, { timeout: 10000 });
    await clickButton(row.getByRole("button", { name: "Install" }));

    const pendingButton = row.getByRole("button", { name: "Installing..." });
    await expect(pendingButton).toBeVisible();
    await expect(pendingButton).toBeDisabled();

    const search = page.getByRole("textbox", { name: "Search AI apps" });
    await search.fill("sample");
    await expect(search).toHaveValue("sample");

    await installResponse;
    await ensureCleanApp();
  });
});
