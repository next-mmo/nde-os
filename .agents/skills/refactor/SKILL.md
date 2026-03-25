---
name: refactor
description: Refactors code for better structure, readability, and performance while preserving behavior.
triggers:
  - refactor this
  - clean up this code
  - improve this code
  - simplify this
  - make this more readable
---

# Refactor Skill

## Objective

Improve code structure and quality without changing behavior. Focus on readability, maintainability, and performance.

## Steps

1. **Read the code**: Understand current structure and behavior using `file_read` and `code_symbols`
2. **Identify issues**: Find code smells — duplication, long functions, deep nesting, unclear naming, unnecessary complexity
3. **Plan changes**: List specific refactoring operations (extract function, rename, simplify conditional, etc.)
4. **Apply changes**: Use `file_patch` or `code_edit` to make surgical edits
5. **Verify**: Run existing tests with `shell_exec` to ensure nothing broke

## Refactoring Catalog

- **Extract function**: Long function → smaller named pieces
- **Inline**: Unnecessary indirection → direct code
- **Rename**: Unclear names → descriptive names
- **Simplify conditional**: Nested if/else → early return or match
- **Remove duplication**: Copy-paste → shared function
- **Reduce nesting**: Deep nesting → guard clauses
- **Type narrowing**: `any/Object` → specific types

## Rules

- Never change behavior — refactoring is structure-only
- Run tests before and after
- Make one logical change per edit (reviewable)
- Prefer smaller, incremental changes over big rewrites
