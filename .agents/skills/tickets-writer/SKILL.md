---
name: tickets-writer
description: Enforces a strict 4-phase methodology (Plan, Generate, Review, DoD) for development tickets to ensure high-quality, secure, and production-ready code. Features a continuous YOLO mode loop for autonomous task completion.
---

# Tickets Writer Skill

Use when asked to implement a feature, write ticket code, build a component, or when picking up an existing task from `.agents/tasks/`.
ALWAYS follow this 4-phase methodology in order:

## Phase 1: Plan

If a plan does not exist, complete the planning template based on the user's request. You MUST write this plan to a new markdown file in `.agents/tasks/` (e.g., `.agents/tasks/feature-name.md`).
The plan must include a detailed checklist of tasks.

> **Best Practice:** See `references/vibe-code-studio.md` for a gold-standard ticket example. Use its exact structure (Status, Feature, Purpose, Inputs & Outputs, Edge Cases & Security, Task Checklist, Definition of Done) as your template when generating new tickets. Set initial `Status` to `🔴 plan`.

_Action:_ **STOP.** Present the plan to the user. Do NOT proceed to Phase 2 until the user explicitly approves the plan.
**EXCEPTION (YOLO Mode)**: If the user explicitly requests "YOLO mode", "auto approve", or you are continuing work on an approved ticket, automatically update the status to `🟡 yolo mode` in the ticket file and proceed directly to Phase 2.

## Phase 2: Generate (YOLO Loop)

In YOLO mode, you operate autonomously. You MUST continually loop through the "Task Checklist" until NO MORE uncompleted tasks remain.
For each task:

1. **Implement**: Adapt implementation rules dynamically to the stack, ensuring strict input validation, security, error handling, and matching success state.
2. **Review**: Fix any obvious bugs or issues immediately in the code.
3. **Sync Progress**: Check off the completed task in the `.agents/tasks/feature-name.md` file.

Do not stop to ask for permission between tasks. Keep working continuously until the entire task checklist is checked off.

## Phase 3: Review

Once the final task is completed, perform a rigorous, automated self-review of the generated code:

- **Security:** Check for injections, XSS, exposed secrets, or missing auth.
- **Bugs:** Check for race conditions, missing `await` statements, or unhandled errors.
- **Performance:** Identify N+1 queries, unoptimized loops, or missing indexes.
- **Best Practices:** Ensure type safety and codebase-consistent error handling.

_Action:_ Fix identified issues immediately in the code. Proceed to Phase 4 once passed.

## Phase 4: Definition of Done (DoD) & Handoff

Verify that the code meets both Local DoD (ticket-specific) and Global DoD criteria.

- [ ] **Local DoD**: All core requirements, edge cases, and constraints explicitly defined in Phase 1 are handled.
- [ ] **Global DoD**: Code complies strictly with the system-wide **Global Definition of Done** defined in `AGENTS.md` (no mocks, explicit passing unit tests, E2E WebView2 tests, 100% cross-platform path integrity, etc).

_Action:_
1. You MUST sync the final ticket status in the `.agents/tasks/feature-name.md` file. Pick off all completed DoD items.
2. Change **Status** to `🟢 done by AI`.
3. Stop and inform the user that the ticket is complete. The human will manually verify the implementation. If it is working, the human will mark it as `✅ verified`. If there are issues, the human will mark it as `🔴 re-open` and assign you to fix the remaining issues.
