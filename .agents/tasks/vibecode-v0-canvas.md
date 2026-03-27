# VibeCode Studio - Serverless v0-style UI Generator

- **Status:** 🟢 `done by AI`
- **Feature:** VibeCode Studio v0-like Canvas Upgrades
- **Purpose:** Transition VibeCode Studio from a basic "Figma-JSON" editor to a true Vercel v0-style UI generator. The current canvas only renders abstract rectangles and text elements (which feel "fake" or "not working" for building real web apps). We will update the agent to generate actual HTML/Tailwind/Svelte code and render it live in an interactive preview iframe, matching the v0 experience.

---

## Current State vs. Target State

| Feature | Current State | Target State (v0-like) |
|---------|---------------|------------------------|
| **Agent Output** | Figma-like JSON (`FDocument`) patches | Functional Web Code (HTML/Tailwind or Svelte) |
| **Canvas Render**| Custom `CanvasEditor` rendering colored divs based on JSON | An `<iframe>` or live-preview runner rendering real web UI |
| **Editing Mode** | Drag & drop abstract JSON nodes | Prompt-driven iteration over functional UI components |
| **Output Format**| `.fj` (Figma JSON) or `.json` | `.svelte`, `.html`, or `.tsx` viable for actual use |

---

## Inputs & Outputs

**Inputs:**
- User prompts in the chat panel ("Build a pricing table with 3 tiers")
- Mentions in chat to apply specific styles or libraries

**Outputs:**
- Streaming text containing code blocks (HTML/Tailwind)
- A live-reloading interactive preview of the generated UI code
- The actual source code visible in the "IDE" tab and saved to the file system

**Validation:**
- Validate that the generated payload contains executable/renderable code format.
- Secure the `<iframe>` sandbox to prevent executing malicious scripts from the LLM.

---

## Edge Cases & Security

- **XSS & Sandbox Security:** The live preview must be rendered inside a sandboxed `<iframe>` layout (`sandbox="allow-scripts"`) to prevent the LLM from executing malicious code inside the NDE OS desktop context.
- **Dependency Loading:** To make it truly v0-like, we need a Tailwind CSS CDN link injected into the iframe output so styling works immediately without a build step.
- **Malformed Outputs:** If the LLM generates broken HTML, gracefully display standard browser fallback or an error overlay rather than crashing the canvas.

---

## Task Checklist

- [x] **1. Agent Prompt Engineering & Parser**
  - Update `AgentChat.svelte` system prompt (`figmaPrompt`) to instruct the LLM to output valid HTML with Tailwind CSS classes (instead of Figma JSON patches).
  - Implement a parser in `AgentChat` that extracts the ````html ... ```` blocks from the stream and sends them to the parent component.

- [x] **2. Live Preview Canvas Engine (v0 Runner)**
  - Create a new `V0Runner.svelte` component to host an `<iframe>` using the `srcdoc` attribute.
  - Dynamically inject the Tailwind CSS CDN (`<script src="https://cdn.tailwindcss.com"></script>`) into the `srcdoc` head to provide immediate, zero-build styling.
  - Hook the extracted code block from the Chat into the iframe's content body for real-time rendering.

- [x] **3. VibeCodeStudio Component Wiring**
  - Update `VibeCodeStudio.svelte` to manage `generatedCode: string` state.
  - Toggle the main view to show the `V0Runner` iframe instead of the `LayerTree` and `PropertiesPanel` when dealing with generated web UI.
  - Sync the `generatedCode` to the IDE tab so the user can see and manually edit the generated source code.

- [x] **4. Polish & E2E Validation**
  - Remove/hide fake "Add Frame/Rectangle" buttons if they do not modify the real code.
  - Update Playwright E2E tests to verify the chat generates code blocks that are successfully embedded in the preview iframe.
  - Take a screenshot of the new v0-like interface showing a generated UI component.

---

## Definition of Done

- [x] **Local DoD**:
  - The chat agent generates HTML + Tailwind instead of Figma JSON patches.
  - The generated code is rendered live inside a sandboxed preview iframe with Tailwind support.
  - The source code is editable and viewable in the IDE tab.
  - Malicious scripts within the generated code cannot escape the iframe sandbox.

- [x] **Global DoD**:
  - `shadcn-svelte` + Tailwind only. No custom `<style>`.
  - Ensure zero panics, no TODOs, and no mocks.
  - Add E2E Playwright test proving the v0-like workflow and save screenshots to `test-results/vibecode-v0-canvas/`.
