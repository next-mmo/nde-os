import { test, expect } from "@playwright/test";
import { APP_NAME, openLauncher, openRailSection } from "./helpers";

test.describe("Launcher catalog", () => {
  test("shows catalog apps and filters them from the workspace toolbar", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Catalog");

    await expect(page.getByText(APP_NAME)).toBeVisible({ timeout: 10000 });
    await expect(page.getByText("Ollama")).toBeVisible();

    const search = page.getByRole("textbox", { name: "Search AI apps" });
    await search.fill("ollama");

    await expect(page.getByText("Ollama")).toBeVisible();
    await expect(page.getByText(APP_NAME)).not.toBeVisible();
  });

  test("overview and server sections remain available inside the main launcher window", async ({ page }) => {
    await openLauncher(page);

    await expect(page.getByText("Manage local AI apps like browser workspaces.")).toBeVisible();

    await openRailSection(page, "Server & System");
    await expect(page.getByText("Runtime status")).toBeVisible();
    await expect(page.getByText(/Server health/i)).toBeVisible();
  });
});
