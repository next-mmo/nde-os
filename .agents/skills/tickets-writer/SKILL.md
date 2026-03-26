---
name: tickets-writer
description: Enforces a strict 4-phase methodology (Plan, Generate, Review, DoD) for development tickets to ensure high-quality, secure, and production-ready code.
---

# Tickets Writer Skill

Use when asked to implement a feature, write ticket code, or build a component.
ALWAYS follow this 4-phase methodology in order:

## Phase 1: Plan

Complete the planning template based on the user's request. You MUST write this plan to a new markdown file in `.agents/tasks/` (e.g., `.agents/tasks/feature-name.md`).
The plan must include a detailed checklist of tasks. The `Status` must be set to `waiting-approval` by default.

**Ticket Template:**

- **Status:** 🔴 `waiting-approval` Plan (🔴 waiting-approval) ➔ Approve ➔ Generate (🟡 in-progress) ➔ Review ➔ DoD ➔ Finish (🟢 done)
- **Feature:** [Name/Title]
- **Purpose:** [One-sentence summary]
- **Inputs & Outputs:** [Expected inputs, types, validation, and success/error responses]
- **Edge Cases & Security:** [Potential failures, injection risks, auth checks, etc.]
- **Task Checklist:**
  - [ ] Task 1
    - [ ] Sub-task 1.1
  - [ ] Task 2

_Action:_ **STOP.** Present the plan to the user. Do NOT proceed to Phase 2 until the user explicitly approves the plan (e.g., they reply with "approved" or "/skill approve"). Once approved, update the status in the file to `🟡 in-progress`.

## Phase 2: Generate

Adapt implementation rules dynamically to the stack, ensuring:

- **Validation**: Strict input validation using schema libraries (e.g., Zod) or strict types.
- **Security**: Parameterized queries/ORM models, secure hashing, and zero hardcoded secrets.
- **Error Handling**: Graceful, context-aware errors (e.g., HTTP 400/409/500) without leaking internal stack traces.
- **Success State**: Exact match to desired output, excluding sensitive fields.

As you progress through your generation and implementation, continuously sync your progress back to the `.agents/tasks/feature-name.md` file by checking off the relevant tasks and sub-tasks.

## Phase 3: Review

Perform a rigorous, automated self-review of the generated code:

- **Security:** Check for injections, XSS, exposed secrets, or missing auth.
- **Bugs:** Check for race conditions, missing `await` statements, or unhandled errors.
- **Performance:** Identify N+1 queries, unoptimized loops, or missing indexes.
- **Best Practices:** Ensure type safety and codebase-consistent error handling.

_Action:_ Fix identified issues immediately in the code. Proceed to Phase 4 once passed.

## Phase 4: Definition of Done (DoD) Checklist

Verify that the code meets both Local DoD (ticket-specific) and Global DoD criteria. Present the completed checklist to the user:

- [ ] **Local DoD**: All core requirements, edge cases, and constraints explicitly defined in Phase 1 are handled.
- [ ] **Global DoD**: Code complies strictly with the system-wide **Global Definition of Done** defined in `AGENTS.md` (no mocks, explicit passing unit tests, E2E WebView2 tests, 100% cross-platform path integrity, etc).

_Action:_ You MUST sync the final ticket status in the `.agents/tasks/feature-name.md` file (check off all completed tasks and DoD items, and change **Status** to `🟢 done`) before concluding the ticket.
