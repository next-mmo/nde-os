---
name: project-setup
description: Sets up new project scaffolding with proper structure, configuration, and tooling. Supports Rust, TypeScript, Python, and more.
triggers:
  - create a new project
  - set up a project
  - scaffold
  - init project
  - bootstrap
---

# Project Setup Skill

## Objective

Create a well-structured project with proper configuration, tooling, and best practices from the start.

## Steps

1. **Gather requirements**: Language, framework, project type (lib, app, CLI, API)
2. **Create directory structure** using `file_write`
3. **Generate config files**: Cargo.toml, package.json, pyproject.toml, etc.
4. **Set up tooling**: linting, formatting, testing, CI
5. **Create starter files**: main entry point, lib exports, basic tests
6. **Initialize git** using `git init`

## Templates by Language

### Rust (Binary)
```
project/
  Cargo.toml
  src/
    main.rs
    lib.rs
  tests/
    integration.rs
  .gitignore
```

### Rust (Library)
```
project/
  Cargo.toml
  src/
    lib.rs
    module.rs
  tests/
  examples/
  .gitignore
```

### TypeScript (Node)
```
project/
  package.json
  tsconfig.json
  src/
    index.ts
  tests/
    index.test.ts
  .gitignore
```

### Python
```
project/
  pyproject.toml
  src/
    __init__.py
    main.py
  tests/
    test_main.py
  .gitignore
```

## Tools Used

- `file_write` — create files
- `file_list` — verify structure
- `shell_exec` — run init commands
- `git` — initialize repository
