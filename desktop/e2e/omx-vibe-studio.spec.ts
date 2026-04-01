import { test, expect } from "./fixtures";

const API = "http://localhost:8080";

test.describe("OMX Provider + Vibe Studio IDE", () => {
  test.setTimeout(180_000);

  // ── 1. OMX provider registered and active ───────────────────────
  test("omx provider is registered and active", async () => {
    const providers = await fetch(`${API}/api/models`).then((r) => r.json());
    expect(providers.success).toBe(true);

    const omxProvider = providers.data?.find(
      (p: any) => p.provider_type === "omx"
    );
    expect(omxProvider).toBeTruthy();
    expect(omxProvider.name).toBe("omx");

    const active = await fetch(`${API}/api/models/active`).then((r) =>
      r.json()
    );
    expect(active.success).toBe(true);
    expect(active.data).toBe("omx");
  });

  // ── 2. OMX sandbox binary resolvable ────────────────────────────
  test("omx binary exists in sandbox", async () => {
    const verify = await fetch(`${API}/api/models/verify`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        name: "omx-test",
        provider_type: "omx",
        model: "gpt-5.4",
      }),
    }).then((r) => r.json());

    expect(verify.success).toBe(true);
  });

  // ── 3. Streaming SSE works with OMX ─────────────────────────────
  test("streaming chat returns valid SSE with text_delta", async () => {
    const resp = await fetch(`${API}/api/agent/chat/stream`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ message: "Reply with exactly: HELLO_OMX_TEST" }),
    });

    expect(resp.ok).toBe(true);
    expect(resp.headers.get("content-type")).toContain("text/event-stream");

    const body = await resp.text();
    expect(body).toContain("data:");
    expect(body).toContain('"type":"text_delta"');
    expect(body).toContain("data: [DONE]");
    expect(body).toContain('"type":"done"');
    expect(body).toContain('"conversation_id"');
  });

  // ── 4. Vibe Studio IDE + Agent Chat panel ───────────────────────
  test("vibe studio ide tab has agent chat panel", async ({ page }) => {
    // Open Vibe Studio
    await page.evaluate(() =>
      (window as any).__svelteDesktop?.openStaticApp("vibe-studio")
    );
    await page.waitForTimeout(2000);

    const vibeWindow = page.locator('[data-window="vibe-studio"]');
    await expect(vibeWindow).toBeAttached({ timeout: 15000 });

    // Click IDE tab
    const ideTab = vibeWindow.getByRole("button", { name: "IDE 💻" });
    await expect(ideTab).toBeVisible({ timeout: 5000 });
    await ideTab.click();
    await page.waitForTimeout(500);

    // Verify Agent Workspace text exists
    const chatHeader = vibeWindow.getByText("AI Agent Workspace");
    await expect(chatHeader).toBeVisible({ timeout: 5000 });

    // Verify textarea
    const textarea = vibeWindow.locator("textarea");
    await expect(textarea).toBeVisible({ timeout: 5000 });

    // Clean up
    await page.evaluate(() => {
      document
        .querySelector('[data-window="vibe-studio"] button[aria-label="Close Desktop Window"]')
        ?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await page.waitForTimeout(500);
  });

  // ── 5. Full E2E: send chat via Vibe Studio, get OMX response ────
  test("agent chat sends message and receives OMX response", async ({ page }) => {
    // Open Vibe Studio fresh
    await page.evaluate(() =>
      (window as any).__svelteDesktop?.openStaticApp("vibe-studio")
    );
    await page.waitForTimeout(2000);

    const vibeWindow = page.locator('[data-window="vibe-studio"]');
    await expect(vibeWindow).toBeAttached({ timeout: 15000 });

    // Switch to IDE tab
    const ideTab = vibeWindow.getByRole("button", { name: "IDE 💻" });
    await expect(ideTab).toBeVisible({ timeout: 5000 });
    await ideTab.click();
    await page.waitForTimeout(500);

    // Type and send message
    const textarea = vibeWindow.locator("textarea");
    await expect(textarea).toBeVisible({ timeout: 5000 });
    await textarea.fill("Reply with exactly one word: WORKING");
    await textarea.press("Enter");

    // Verify user message appeared
    await expect(vibeWindow.locator("text=WORKING").first()).toBeVisible({ timeout: 10000 });

    // Wait for OMX response (up to 90s)
    const assistantBubbles = vibeWindow.locator('.flex.gap-3:not(.flex-row-reverse)');
    await expect(async () => {
      const count = await assistantBubbles.count();
      expect(count).toBeGreaterThanOrEqual(2);
    }).toPass({ timeout: 90000, intervals: [2000] });

    // Verify response text exists
    const lastAssistant = assistantBubbles.last();
    const content = await lastAssistant.locator('.rounded-xl').textContent();
    expect(content).toBeTruthy();
    expect(content!.length).toBeGreaterThan(0);

    // Clean up
    await page.evaluate(() => {
      document
        .querySelector('[data-window="vibe-studio"] button[aria-label="Close Desktop Window"]')
        ?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    await page.waitForTimeout(500);
  });
});
