import { test, expect } from "./fixtures";
import { appRow, openLauncher, openRailSection } from "./helpers";

const API = "http://127.0.0.1:8080";
const APP_NAME = "Node.js Fullstack Counter";
const APP_ID = "sample-node";

async function ensureClean() {
  await fetch(`${API}/api/apps/${APP_ID}/stop`, { method: "POST" }).catch(
    () => {},
  );
  await fetch(`${API}/api/apps/${APP_ID}`, { method: "DELETE" }).catch(
    () => {},
  );
}

test.describe.serial("Node.js Fullstack Counter App", () => {
  test.setTimeout(180000);

  test.beforeEach(async () => {
    await ensureClean();
  });

  test.afterAll(async () => {
    await ensureClean();
  });

  test("installs and launches into a browser window", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Catalog");

    const row = appRow(page, APP_ID);
    await expect(row).toBeVisible({ timeout: 10000 });
    await expect(row.getByRole("button", { name: "Install" })).toBeVisible();

    await row.getByRole("button", { name: "Install" }).click();
    await expect(
      row.getByRole("button", { name: "Open in Window" }),
    ).toBeVisible({ timeout: 180000 });

    await row.getByRole("button", { name: "Open in Window" }).click();

    const browserWindow = page.locator('[data-window="browser"]').first();
    await expect(browserWindow).toBeVisible({ timeout: 15000 });
    await expect(
      browserWindow.getByRole("textbox", { name: "Browser address" }),
    ).toHaveValue(/localhost:3000/);
  });

  test("full lifecycle: overview to install to running to uninstall", async ({
    page,
  }) => {
    await openLauncher(page);

    await openRailSection(page, "Catalog");
    const row = appRow(page, APP_ID);
    await expect(row).toBeVisible({ timeout: 10000 });
    await row.getByRole("button", { name: "Install" }).click();
    await expect(
      row.getByRole("button", { name: "Open in Dashboard" }),
    ).toBeVisible({ timeout: 180000 });

    await row.getByRole("button", { name: "Open in Dashboard" }).click();

    await openRailSection(page, "Running");
    const runRow = page.locator("[data-session-id]").first();
    await expect(runRow).toBeVisible({ timeout: 15000 });

    await runRow.locator(".app-main").click();
    const detailPanel = page.locator(".detail-panel", {
      has: page.locator("h3", { hasText: APP_NAME }),
    });
    await expect(
      detailPanel.getByRole("button", { name: "New OS Window" }),
    ).toBeVisible();

    await detailPanel.getByRole("button", { name: "Stop" }).click();
    await expect(page.getByText("No running sessions yet.")).toBeVisible({
      timeout: 15000,
    });

    await openRailSection(page, "Installed");
    const installedRow = appRow(page, APP_ID);
    await expect(installedRow).toBeVisible({ timeout: 5000 });

    await installedRow.getByRole("button", { name: "Uninstall" }).click();
    await expect(
      page.getByText("Install an app from the catalog to see it here."),
    ).toBeVisible({ timeout: 15000 });
  });
});
