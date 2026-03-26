---
name: tickets-writer
description: Enforces a strict 4-phase methodology (Plan, Generate, Review, DoD) for development tickets to ensure high-quality, secure, and production-ready code.
---

# Tickets Writer Skill

Use when asked to implement a feature, write ticket code, or build a component.
ALWAYS follow this 4-phase methodology in order:

## Phase 1: Plan
Complete the planning template based on the user's request. Present it for approval before coding.
* **Feature:** [Name/Title]
* **Purpose:** [One-sentence summary]
* **Inputs & Outputs:** [Expected inputs, types, validation, and success/error responses]
* **Edge Cases & Security:** [Potential failures, injection risks, auth checks, etc.]

## Phase 2: Generate
Adapt implementation rules dynamically to the stack, ensuring:
- **Validation**: Strict input validation using schema libraries (e.g., Zod) or strict types.
- **Security**: Parameterized queries/ORM models, secure hashing, and zero hardcoded secrets.
- **Error Handling**: Graceful, context-aware errors (e.g., HTTP 400/409/500) without leaking internal stack traces.
- **Success State**: Exact match to desired output, excluding sensitive fields.

## Phase 3: Review
Perform a rigorous, automated self-review of the generated code:
- **Security:** Check for injections, XSS, exposed secrets, or missing auth.
- **Bugs:** Check for race conditions, missing `await` statements, or unhandled errors.
- **Performance:** Identify N+1 queries, unoptimized loops, or missing indexes.
- **Best Practices:** Ensure type safety and codebase-consistent error handling.

*Action:* Fix identified issues immediately in the code. Proceed to Phase 4 once passed.

## Phase 4: Definition of Done (DoD) Checklist
Verify the following NDE-OS project criteria. Present the completed checklist to the user:
- [ ] **No Mocks/TODOs:** Code is fully production-ready.
- [ ] **Core Tests (Rust):** If `core/` or `server/` changed, passing scoped unit tests (`cargo test -p <changed-crate> -- <test_name>`). Use `anyhow::Result`, no panics.
- [ ] **Desktop E2E:** If `desktop/` UI/Tauri changed, passing Playwright WebView2 tests (`cd desktop && npx playwright test e2e/<spec>.spec.ts`).
- [ ] **Cross-Platform:** No hardcoded paths (`PathBuf::join()`). Proper `cfg!(windows)` subprocess execution. Both `HOME` and `USERPROFILE` set.
- [ ] **Tauri IPC:** IPC data batched. Kept `Arc<Mutex<>>` locks minimal. Returned structs, not primitives.
- [ ] **Sandbox Integrity:** Filesystem constraints respected (`curl -s http://localhost:8080/api/sandbox/test/verify`).
