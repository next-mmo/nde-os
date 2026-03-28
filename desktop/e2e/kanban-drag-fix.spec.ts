/**
 * E2E spec: Kanban drag-and-drop fix verification
 *
 * Verifies:
 *  1. Tasks with YAML frontmatter are parsed and displayed in the correct column.
 *  2. Dragging a card to another column rewrites the YAML frontmatter `status:` on disk.
 *  3. YOLO mode tasks are shown as locked (non-draggable).
 *
 * Requires: a single dev.sh running with CDP on port 9222.
 * In dev mode the desktop is always expanded (collapseDesktop is a no-op), so
 * no FAB-expansion logic is needed here.
 */

import { test, expect } from "./fixtures";
import * as fs from "fs";
import * as path from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
// The Tauri binary's CWD during `cargo tauri dev` is desktop/src-tauri/.
// tasks_dir() in Rust resolves relative to CWD, so it reads from:
//   desktop/src-tauri/.agents/tasks/
// NOT from the repo root .agents/tasks/ as one might expect.
const TASKS_DIR = path.resolve(__dirname, "../src-tauri/.agents/tasks");

// ── Task file helpers ─────────────────────────────────────────────────────────

const SLUG = "e2e-kanban-drag-verify";
const TASK_FILE = path.join(TASKS_DIR, `${SLUG}.md`);
const CARD_ID = `${SLUG}.md`; // Rust returns full filenames including .md extension

function writeTask(fmStatus: string) {
  fs.mkdirSync(TASKS_DIR, { recursive: true });
  fs.writeFileSync(
    TASK_FILE,
    `---\nstatus: ${fmStatus}\n---\n\n# E2E Kanban Drag Verify\n\n## Description\nAuto-generated test task for E2E verification.\n`,
    "utf-8"
  );
}

function readTaskStatus(): string {
  const content = fs.readFileSync(TASK_FILE, "utf-8");
  return content.match(/^status:\s*(.+)$/m)?.[1].trim() ?? "";
}

function cleanup() {
  if (fs.existsSync(TASK_FILE)) fs.unlinkSync(TASK_FILE);
}

// ── Desktop helpers ───────────────────────────────────────────────────────────

/**
 * Open the Vibe Studio window and navigate to the Kanban tab.
 * Dev mode is always expanded so no FAB handling needed.
 */
async function openKanban(page: import("@playwright/test").Page) {
  // Wait for the desktop shell to finish loading (dock or any window visible)
  await page.waitForSelector('[data-testid="dock"], [data-window], .dock', { timeout: 15000 });

  // Close the Vibe Studio window if already open (forces component unmount/remount
  // so onMount calls get_agent_tasks fresh with our newly-written task file)
  const existingWindow = page.locator('[data-window="vibe-studio"]');
  if (await existingWindow.isVisible({ timeout: 1000 }).catch(() => false)) {
    await page.evaluate(() => {
      // Click the close button of the vibe-studio window
      const win = document.querySelector('[data-window="vibe-studio"]');
      const closeBtn = win?.querySelector('button[aria-label="Close Desktop Window"]') as HTMLElement | null;
      closeBtn?.click();
    });
    await page.waitForTimeout(500);
  }

  // Open Vibe Studio window via the exposed test hook (set in main.ts dev-only block)
  await page.evaluate(() => {
    (window as any).__svelteDesktop?.openStaticApp("vibe-studio");
  });

  // Wait for Vibe Studio window
  const vibeWindow = page.locator('[data-window="vibe-studio"]');
  await expect(vibeWindow).toBeVisible({ timeout: 10000 });

  // Click the Kanban tab via JS to bypass any viewport bounds issues
  await page.evaluate(() => {
    const btn = Array.from(document.querySelectorAll("button"))
      .find(b => b.textContent?.trim() === "Kanban 📋") as HTMLElement | undefined;
    if (!btn) throw new Error("Kanban 📋 button not found");
    btn.click();
  });

  // Trigger Kanban board to re-fetch tasks.
  // Strategy: try the dev hook first; if unavailable, use Tauri's bundled invoke directly.
  await page.evaluate(async () => {
    const desktop = (window as any).__svelteDesktop;
    if (desktop?.refreshKanban) {
      await desktop.refreshKanban();
      return;
    }
    // Fallback: use the Tauri invoke bridge that's always bundled with the app
    const tauri = (window as any).__TAURI_INTERNALS__ ?? (window as any).__TAURI__;
    if (tauri?.invoke) {
      await tauri.invoke("get_agent_tasks").catch(() => {});
      await tauri.invoke("plugin:event|emit", {
        event: "tasks://updated",
        payload: null,
      }).catch(() => {});
    }
  });

  // Wait for Kanban columns to be present in DOM (confirms onMount ran + initial fetch done)
  await page.waitForSelector("[role='list'][aria-label='Plan']", { timeout: 10000 });

  // DEBUG: log what Rust actually returned
  const debugTasks = await page.evaluate(async () => {
    const desktop = (window as any).__svelteDesktop;
    if (desktop?.refreshKanban) await desktop.refreshKanban();
    // Wait a tick for Svelte to update
    await new Promise(r => setTimeout(r, 2000));
    // Return what cards are currently visible
    const cards = Array.from(document.querySelectorAll("[data-card-id]"));
    return cards.map(c => c.getAttribute("data-card-id"));
  });
  console.log("[DEBUG] visible card IDs:", JSON.stringify(debugTasks));
}

/**
 * Dispatch HTML5 DragEvents to simulate a card move.
 * Playwright's dragTo() uses Pointer/Mouse events which don't fire HTML5 DragEvent
 * in WebView2 — so we dispatch them manually from the page context.
 */
async function htmlDrag(
  page: import("@playwright/test").Page,
  sourceSelector: string,
  targetSelector: string
) {
  await page.evaluate(
    ({ src, tgt }: { src: string; tgt: string }) => {
      const sourceEl = document.querySelector(src) as HTMLElement | null;
      const targetEl = document.querySelector(tgt) as HTMLElement | null;
      if (!sourceEl || !targetEl) {
        throw new Error(`htmlDrag: elements not found. src="${src}" tgt="${tgt}"`);
      }
      const dt = new DataTransfer();
      const fire = (el: HTMLElement, type: string) =>
        el.dispatchEvent(new DragEvent(type, { bubbles: true, cancelable: true, dataTransfer: dt }));
      fire(sourceEl, "dragstart");
      fire(targetEl, "dragover");
      fire(targetEl, "drop");
      fire(sourceEl, "dragend");
    },
    { src: sourceSelector, tgt: targetSelector }
  );
}

// ── Tests ─────────────────────────────────────────────────────────────────────

test.describe("Kanban drag-and-drop fix", () => {
  test.beforeEach(() => cleanup());
  test.afterEach(() => cleanup());

  // ── 1. YAML frontmatter parsing ─────────────────────────────────────────────
  test("task with YAML frontmatter 'plan' status appears in Plan column", async ({ page }) => {
    writeTask("🔴 plan");
    await openKanban(page);

    await page.screenshot({ path: "test-results/kanban-drag/01-initial-board.png" });

    // Title visible anywhere on the board
    await expect(page.getByText("E2E Kanban Drag Verify")).toBeVisible({ timeout: 5000 });

    // Confirm it's in the Plan column (check by filename in card content)
    const inPlan = await page.evaluate((cardId: string) => {
      const col = document.querySelector("[role='list'][aria-label='Plan']");
      return !!col?.querySelector(`[data-card-id='${cardId}']`);
    }, CARD_ID);
    expect(inPlan).toBe(true);
  });

  // ── 2. Drag between columns + disk persistence ───────────────────────────────
  test("drag card from Plan to Done by AI updates YAML frontmatter on disk", async ({ page }) => {
    writeTask("🔴 plan");
    await openKanban(page);

    await page.screenshot({ path: "test-results/kanban-drag/02-before-drag.png" });

    const cardSel = `[data-card-id="${CARD_ID}"]`;
    const colSel  = `[role='list'][aria-label='Done by AI']`;

    // Card must be present before dragging
    await expect(page.locator(cardSel)).toBeVisible({ timeout: 5000 });

    // HTML5 drag from Plan card to Done by AI column
    await htmlDrag(page, cardSel, colSel);

    // Wait for Tauri IPC write + Svelte re-render
    await page.waitForTimeout(2500);
    await page.screenshot({ path: "test-results/kanban-drag/03-after-drag.png" });

    // UI: card appears in Done by AI column
    const inDone = await page.evaluate(
      ({ card, col }: { card: string; col: string }) =>
        !!document.querySelector(`[role='list'][aria-label='${col}']`)?.querySelector(card),
      { card: cardSel, col: "Done by AI" }
    );
    expect(inDone).toBe(true);

    // Disk: YAML frontmatter updated
    expect(readTaskStatus()).toMatch(/done by ai/i);
  });

  // ── 3. YOLO mode → locked card ──────────────────────────────────────────────
  test("YOLO mode task appears in YOLO column and has draggable=false", async ({ page }) => {
    writeTask("🟡 yolo mode");
    await openKanban(page);

    await page.screenshot({ path: "test-results/kanban-drag/04-locked-card.png" });

    // In the correct YOLO column (check by data-card-id)
    const inYolo = await page.evaluate((cardId: string) => {
      const col = document.querySelector("[role='list'][aria-label='YOLO mode']");
      return !!col?.querySelector(`[data-card-id='${cardId}']`);
    }, CARD_ID);
    expect(inYolo).toBe(true);

    // Not draggable
    const draggable = await page.evaluate(
      (sel: string) => document.querySelector(sel)?.getAttribute("draggable"),
      `[data-card-id="${CARD_ID}"]`
    );
    expect(draggable).not.toBe("true");
  });
});
