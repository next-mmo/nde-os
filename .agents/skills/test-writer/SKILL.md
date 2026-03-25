---
name: test-writer
description: Generates comprehensive unit tests and integration tests for existing code. Covers happy path, edge cases, and error conditions.
triggers:
  - write tests for
  - add tests
  - test this code
  - generate tests
  - improve test coverage
---

# Test Writer Skill

## Objective

Generate thorough, production-quality tests that cover functionality, edge cases, and error handling.

## Steps

1. **Read the code** to understand public API, inputs, outputs, and side effects
2. **Identify test cases**:
   - Happy path (normal usage)
   - Edge cases (empty input, max values, boundary conditions)
   - Error cases (invalid input, missing data, timeout, permission denied)
   - State transitions (if stateful)
3. **Write tests** following the existing test patterns in the project
4. **Add to test file** using `file_write` or `file_patch`
5. **Run tests** with `shell_exec` to verify they pass

## Test Quality Rules

- Each test tests ONE thing (single assertion focus)
- Test names describe the scenario: `test_{function}_{scenario}_{expected}`
- No test depends on another test's state
- Use proper setup/teardown (fixtures, temp dirs)
- Mock external dependencies, not internal logic
- Assert specific values, not just "no error"

## Language-Specific Patterns

### Rust
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_happy_path() {
        let result = function(valid_input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_function_edge_case() {
        let result = function(edge_input);
        assert!(result.is_ok());
    }
}
```

### TypeScript
```typescript
describe('Module', () => {
  it('should handle normal input', () => {
    expect(fn(input)).toBe(expected);
  });
});
```

## Tools Used

- `file_read` — read source code
- `code_symbols` — find functions to test
- `file_write` / `file_patch` — write test code
- `shell_exec` — run tests
