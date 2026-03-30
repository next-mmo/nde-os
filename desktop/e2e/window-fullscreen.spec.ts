import { test as base, chromium, expect } from '@playwright/test';

base.describe("Window fullscreen on open", () => {
  base("all apps open in fullscreen mode", async () => {
    const browser = await chromium.launch();
    const page = await browser.newPage();
    await page.goto('http://localhost:5174', { waitUntil: 'domcontentloaded' });
    await page.waitForTimeout(3000);

    const dock = page.getByRole("toolbar", { name: "Dock" });
    await expect(dock).toBeVisible({ timeout: 10000 });

    // Open Settings
    await dock.getByRole("button", { name: "settings", exact: true }).click();
    await page.waitForTimeout(1000);

    const settingsWin = page.locator('[data-window="settings"]');
    await expect(settingsWin).toBeVisible({ timeout: 10000 });

    const settingsInfo = await settingsWin.evaluate((el) => {
      const rect = el.getBoundingClientRect();
      const cs = getComputedStyle(el);
      return {
        fullscreenClass: el.className.includes("inset-0"),
        rect: { x: Math.round(rect.x), y: Math.round(rect.y), w: Math.round(rect.width), h: Math.round(rect.height) },
        viewport: { w: window.innerWidth, h: window.innerHeight },
        translate: (el as HTMLElement).style.translate,
        inset: cs.inset,
      };
    });
    console.log("Settings:", JSON.stringify(settingsInfo, null, 2));

    // Open Logs
    await dock.getByRole("button", { name: /logs/i }).click();
    await page.waitForTimeout(1000);

    const logsWin = page.locator('[data-window="logs"]');
    await expect(logsWin).toBeVisible({ timeout: 10000 });

    const logsInfo = await logsWin.evaluate((el) => {
      const rect = el.getBoundingClientRect();
      return {
        fullscreenClass: el.className.includes("inset-0"),
        rect: { x: Math.round(rect.x), y: Math.round(rect.y), w: Math.round(rect.width), h: Math.round(rect.height) },
        translate: (el as HTMLElement).style.translate,
      };
    });
    console.log("Logs:", JSON.stringify(logsInfo, null, 2));

    // Verify fullscreen: x should be 0, width should match viewport
    expect(settingsInfo.fullscreenClass, "Settings should have fullscreen class").toBe(true);
    expect(settingsInfo.rect.x, "Settings x should be 0").toBe(0);
    expect(settingsInfo.rect.w, "Settings width should match viewport").toBe(settingsInfo.viewport.w);

    expect(logsInfo.fullscreenClass, "Logs should have fullscreen class").toBe(true);
    expect(logsInfo.rect.x, "Logs x should be 0").toBe(0);

    await browser.close();
  });
});
