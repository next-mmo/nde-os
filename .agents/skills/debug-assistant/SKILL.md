---
name: debug-assistant
description: Systematic debugging assistant. Analyzes errors, traces root causes, and suggests fixes with evidence.
triggers:
  - debug this
  - why is this failing
  - fix this error
  - help me debug
  - trace this bug
---

# Debug Assistant Skill

## Objective

Systematically diagnose and fix bugs by gathering evidence, forming hypotheses, and validating fixes.

## Steps

1. **Reproduce**: Understand the error message, stack trace, or unexpected behavior
2. **Gather context**: Read relevant source files, check recent git changes, search for related code
3. **Form hypotheses**: List 2-3 most likely root causes based on evidence
4. **Validate**: For each hypothesis, search for confirming/disconfirming evidence in the code
5. **Fix**: Propose the minimal fix with explanation of why it works
6. **Verify**: Suggest how to verify the fix (test command, manual check)

## Investigation Tools

- `file_read` — read source files around the error
- `code_search` — find where functions/variables are defined and used
- `git` with `log` and `diff` — check what changed recently
- `shell_exec` — run tests or reproduce the error
- `web_search` — look up error messages or library issues

## Output Format

```
## Debug Report

### Error
{error_message_or_behavior}

### Root Cause
{explanation with evidence from code}

### Fix
{code change with file path and line numbers}

### Verification
{how to verify the fix works}
```
