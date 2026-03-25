/**
 * Integration test: Catalog prompt → LLM → Valid Spec
 *
 * Sends our catalog system prompt to an LLM and verifies
 * the response produces a valid @json-render spec via streaming patches.
 *
 * Usage: npx tsx src/lib/json-render/__tests__/llm-integration.test.ts
 */

import { validateSpec, createSpecStreamCompiler, applySpecStreamPatch } from "@json-render/core";
import { catalog, systemPrompt } from "../catalog";

const API_BASE = "https://dashscope.aliyuncs.com/compatible-mode/v1";
const API_KEY = "sk-ab3033f199eb4bcd93ea82f4c76cd117";

const MODELS = [
  "qwen2.5-coder-32b-instruct",
  "deepseek-v3",
] as const;

const USER_PROMPTS = [
  "Show a system health dashboard with CPU usage (72%), memory (8.2 GB), and disk (45%). Include a refresh button.",
  "Show a list of 3 AI apps: ComfyUI (running), Ollama (installed), Whisper (available). Each should be an AppTile.",
  "Show an error alert saying 'Server unreachable' with a retry button below it, inside a glass card.",
];

async function callLLM(model: string, userPrompt: string): Promise<string> {
  const res = await fetch(`${API_BASE}/chat/completions`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "Authorization": `Bearer ${API_KEY}`,
    },
    body: JSON.stringify({
      model,
      messages: [
        { role: "system", content: systemPrompt },
        { role: "user", content: userPrompt },
      ],
      temperature: 0.3,
    }),
  });

  if (!res.ok) {
    const text = await res.text();
    throw new Error(`API error ${res.status}: ${text.slice(0, 300)}`);
  }

  const data = await res.json();
  return data.choices?.[0]?.message?.content ?? "";
}

/**
 * Parse LLM output which should be JSONL (JSON Patch operations).
 * Each line is a JSON patch op like: {"op":"add","path":"/root","value":"main"}
 * We apply them incrementally to build the final spec.
 */
function parseStreamedSpec(raw: string): any {
  // Strip markdown code fences if present
  let content = raw.trim();
  const fenceMatch = content.match(/```(?:jsonl?|text)?\s*([\s\S]*?)```/);
  if (fenceMatch) {
    content = fenceMatch[1].trim();
  }

  // Try direct JSON parse first (some models may return plain JSON)
  try {
    const parsed = JSON.parse(content);
    if (parsed.root && parsed.elements) return parsed;
  } catch {
    // Not plain JSON, continue with JSONL parsing
  }

  // Parse as JSONL (one JSON object per line)
  const lines = content.split("\n").filter(l => l.trim().startsWith("{"));
  if (lines.length === 0) {
    throw new Error("No valid JSONL lines found in LLM response");
  }

  // Build spec by applying patches
  let spec: any = {};
  for (const line of lines) {
    try {
      const patch = JSON.parse(line.trim());
      if (patch.op === "add" || patch.op === "replace") {
        setByPath(spec, patch.path, patch.value);
      }
    } catch {
      // Skip unparseable lines
    }
  }

  return spec;
}

/**
 * Set a value at a JSON Pointer path (e.g. /root, /elements/card-1)
 */
function setByPath(obj: any, path: string, value: any): void {
  const parts = path.split("/").filter(Boolean);
  let current = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    const key = parts[i];
    if (!(key in current)) {
      current[key] = {};
    }
    current = current[key];
  }
  const lastKey = parts[parts.length - 1];
  if (lastKey !== undefined) {
    current[lastKey] = value;
  }
}

async function runTest(model: string, prompt: string): Promise<{ pass: boolean; issues: string[]; elementCount: number }> {
  try {
    const raw = await callLLM(model, prompt);
    const spec = parseStreamedSpec(raw);

    // Basic structure check
    if (!spec.root) {
      return { pass: false, issues: ["Missing 'root' in spec"], elementCount: 0 };
    }
    if (!spec.elements || Object.keys(spec.elements).length === 0) {
      return { pass: false, issues: ["Missing or empty 'elements' in spec"], elementCount: 0 };
    }

    const elementCount = Object.keys(spec.elements).length;

    // Validate with @json-render/core
    const result = validateSpec(spec);
    if (!result.valid) {
      const issueStrs = result.issues.map((i: any) =>
        typeof i === "string" ? i : `${i.path ?? ""}: ${i.message ?? JSON.stringify(i)}`
      );
      return { pass: false, issues: issueStrs, elementCount };
    }

    // Check root references an existing element
    if (!spec.elements[spec.root]) {
      return { pass: false, issues: [`Root '${spec.root}' not found in elements`], elementCount };
    }

    return { pass: true, issues: [], elementCount };
  } catch (e: any) {
    return { pass: false, issues: [e.message], elementCount: 0 };
  }
}

// ── Main ─────────────────────────────────────────────────────────────

async function main() {
  console.log("╔═══════════════════════════════════════════════════════╗");
  console.log("║  json-render LLM Integration Test                    ║");
  console.log("╚═══════════════════════════════════════════════════════╝\n");

  let passed = 0;
  let failed = 0;

  for (const model of MODELS) {
    console.log(`\n── Model: ${model} ──────────────────────────────\n`);

    for (const prompt of USER_PROMPTS) {
      const shortPrompt = prompt.slice(0, 60) + (prompt.length > 60 ? "..." : "");
      // @ts-ignore
      process.stdout.write(`  ${shortPrompt} ... `);

      const result = await runTest(model, prompt);

      if (result.pass) {
        console.log(`✅ PASS (${result.elementCount} elements)`);
        passed++;
      } else {
        console.log("❌ FAIL");
        result.issues.forEach(i => console.log(`    └─ ${i.slice(0, 120)}`));
        failed++;
      }
    }
  }

  console.log(`\n${"─".repeat(55)}`);
  console.log(`Results: ${passed} passed, ${failed} failed (${passed + failed} total)`);
  console.log(`${"─".repeat(55)}\n`);
  // @ts-ignore
  process.exit(failed > 0 ? 1 : 0);
}

main();
