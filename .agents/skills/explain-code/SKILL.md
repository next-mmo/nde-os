---
name: explain-code
description: Explains code in plain language. Breaks down complex logic, traces execution flow, and creates documentation.
triggers:
  - explain this code
  - what does this do
  - how does this work
  - walk me through
  - explain the architecture
---

# Explain Code Skill

## Objective

Provide clear, layered explanations of code from high-level architecture down to implementation details.

## Steps

1. **Read the code** using `file_read` and identify the scope (function, module, system)
2. **Map the structure** using `code_symbols` to understand the hierarchy
3. **Trace the flow**: Follow the execution path from entry point through dependencies
4. **Explain in layers**:
   - **What**: One-sentence purpose
   - **How**: High-level algorithm/approach
   - **Why**: Design decisions and tradeoffs
   - **Details**: Key implementation specifics

## Output Format

```
## Explanation: {scope}

### Purpose
{one-sentence description of what this code does}

### How It Works
{step-by-step explanation of the algorithm/flow}

### Key Design Decisions
- {decision}: {why it was made}

### Important Details
- {notable implementation detail}

### Dependencies
- {what this code depends on and why}
```

## Tools Used

- `file_read` — read the source code
- `code_symbols` — map structure
- `code_search` — find usages and dependencies
- `git` with `log` — understand history and evolution
