import type { Locator, Page } from "@playwright/test";
import { expect } from "@playwright/test";

const API = "http://localhost:8080";

export const APP_ID = "sample-gradio";
export const APP_NAME = "Sample Counter";

export async function ensureCleanApp() {
  await fetch(`${API}/api/apps/${APP_ID}/stop`, { method: "POST" }).catch(() => {});
  await fetch(`${API}/api/apps/${APP_ID}`, { method: "DELETE" }).catch(() => {});
}

export async function openLauncher(page: Page) {
  const win = page.locator('[data-window="ai-launcher"]');

  // If the launcher was closed by a previous test, reload to reset desktop state
  const visible = await win.isVisible().catch(() => false);
  if (!visible) {
    await page.reload({ waitUntil: "domcontentloaded" });
    await page.waitForTimeout(2000);
  }

  await expect(win).toBeVisible({ timeout: 15000 });
  await expect(win.locator("header.window-drag-handle strong").first()).toBeVisible({ timeout: 5000 });
  // Dispatch pointerdown to bring window to front (raise z-index above other windows)
  await win.evaluate((el) => (el as HTMLElement).dispatchEvent(new PointerEvent('pointerdown', { bubbles: true })));
  await page.waitForTimeout(200);
}

export function dock(page: Page): Locator {
  return page.getByRole("toolbar", { name: "Dock" });
}

export function dockButton(page: Page, name: string | RegExp): Locator {
  return dock(page).getByRole("button", { name });
}

export async function openRailSection(page: Page, name: string) {
  await page.locator(".rail").getByRole("button", { name }).click();
}

export function appRow(page: Page, appId: string): Locator {
  return page.locator(`[data-app-id="${appId}"]`).first();
}

export async function clickButton(locator: Locator) {
  await expect(locator).toBeVisible({ timeout: 30000 });
  await expect(locator).toBeEnabled({ timeout: 30000 });
  await locator.evaluate((element: HTMLButtonElement) => element.click());
}

export async function installSampleCounter(page: Page) {
  await openRailSection(page, "Catalog");
  const row = appRow(page, APP_ID);
  await expect(row).toBeVisible({ timeout: 30000 });
  await clickButton(row.getByRole("button", { name: "Install" }));
  await expect(row.getByRole("button", { name: "Open in Dashboard" })).toBeVisible({ timeout: 180000 });
}

export async function launchSampleCounterInDashboard(page: Page) {
  await openRailSection(page, "Catalog");
  const row = appRow(page, APP_ID);
  const openButton = row.getByRole("button", { name: "Open in Dashboard" });
  await clickButton(openButton);
  await expect(page.getByText("Active preview")).toBeVisible({ timeout: 30000 });
  await expect(page.locator(".session-headline")).toContainText(APP_NAME);
}
