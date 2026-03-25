---
name: code-review
description: Performs thorough code review on files or diffs. Checks for bugs, security issues, performance problems, and code quality.
triggers:
  - review this code
  - code review
  - check this for bugs
  - review my changes
---

# Code Review Skill

## Objective

Perform a production-grade code review covering correctness, security, performance, and maintainability.

## Steps

1. **Read the target files** using `file_read` or `code_search` to understand the code
2. **Check for bugs**: null/undefined access, off-by-one errors, race conditions, resource leaks
3. **Check for security**: injection vulnerabilities (SQL, XSS, command), hardcoded secrets, path traversal, unsafe deserialization
4. **Check for performance**: N+1 queries, unnecessary allocations, missing indexes, O(n^2) loops on large data
5. **Check for maintainability**: unclear naming, missing error handling, overly complex logic, dead code
6. **Summarize findings** with severity levels: CRITICAL, WARNING, INFO

## Output Format

```
## Code Review: {file_or_scope}

### CRITICAL
- [Line X] Description of critical issue

### WARNING
- [Line X] Description of warning

### INFO
- [Line X] Suggestion for improvement

### Summary
- X critical, Y warnings, Z info items
- Overall assessment: PASS / NEEDS FIXES / BLOCKED
```

## Tools Used

- `file_read` — read source files
- `code_search` — find patterns across codebase
- `code_symbols` — understand file structure
- `git` — check recent changes with `diff`
