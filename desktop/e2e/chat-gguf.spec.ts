import { test, expect } from "./fixtures";
import { openLauncher, openRailSection } from "./helpers";

test.describe("Chat with GGUF model", () => {
  test("can navigate to Chat section from the launcher rail", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Chat");

    // Should show the NDE Chat interface
    await expect(page.locator(".chat-app")).toBeVisible({ timeout: 10000 });
    await expect(page.getByText("NDE Chat")).toBeVisible();
  });

  test("shows agent info with GGUF provider details", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Chat");

    // Wait for agent config to load
    await expect(page.locator(".provider-badge")).toBeVisible({ timeout: 15000 });
    // The badge should mention "gguf" since we have a GGUF provider configured
    await expect(page.locator(".provider-badge")).toContainText("gguf", { timeout: 5000 });
  });

  test("shows empty state with suggestion buttons", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Chat");

    // Verify the empty state with suggestions
    const emptyState = page.locator(".empty-state");
    await expect(emptyState).toBeVisible({ timeout: 10000 });
    await expect(emptyState.getByText("NDE-OS Agent")).toBeVisible();

    // Check suggestion buttons exist  
    const suggestions = page.locator(".suggestion");
    await expect(suggestions).toHaveCount(4);
  });

  test("can send a message and receive a streaming response", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Chat");

    // Type a message in the input area
    const textarea = page.locator(".chat-input textarea, .chat-input input");
    await expect(textarea).toBeVisible({ timeout: 10000 });
    await textarea.fill("What is 1+1?");

    // Click the send button
    const sendBtn = page.locator(".chat-input button.send-btn, .chat-input button[type='submit'], .send-action");
    await sendBtn.click();

    // The user message should appear
    await expect(page.locator(".msg.user").first()).toBeVisible({ timeout: 5000 });
    await expect(page.locator(".msg.user .msg-text").first()).toContainText("What is 1+1?");

    // Wait for assistant response (GGUF can take up to 90s to boot + respond)
    await expect(page.locator(".msg.assistant").first()).toBeVisible({ timeout: 120000 });
    // Verify the streaming badge appears then disappears
    await expect(page.locator(".msg.assistant .msg-text").first()).not.toBeEmpty({
      timeout: 120000,
    });
  });

  test("can click a suggestion to send a message", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Chat");

    // Click the first suggestion button
    const suggestion = page.locator(".suggestion").first();
    await expect(suggestion).toBeVisible({ timeout: 10000 });
    await suggestion.click();

    // Message should be sent automatically
    await expect(page.locator(".msg.user").first()).toBeVisible({ timeout: 10000 });

    // Should start getting a response
    await expect(page.locator(".msg.assistant").first()).toBeVisible({ timeout: 120000 });
  });

  test("can open conversations sidebar and start new chat", async ({ page }) => {
    await openLauncher(page);
    await openRailSection(page, "Chat");

    // Click the menu button to toggle sidebar
    const menuBtn = page.locator(".menu-btn");
    await expect(menuBtn).toBeVisible({ timeout: 10000 });
    await menuBtn.click();

    // Sidebar should appear
    await expect(page.locator(".sidebar")).toBeVisible({ timeout: 5000 });

    // Check that sidebar has "Conversations" heading
    await expect(page.locator(".sidebar h3")).toContainText("Conversations");

    // Click "New Chat" button
    const newChatBtn = page.locator(".new-chat-btn");
    await expect(newChatBtn).toBeVisible();
    await newChatBtn.click();

    // Messages should be cleared
    await expect(page.locator(".empty-state")).toBeVisible({ timeout: 5000 });
  });
});
