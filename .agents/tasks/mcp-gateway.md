---
status: 🟢 done by AI
---

# MCP Gateway Support

## Purpose
Currently, NDE OS has the foundation for an MCP (Model Context Protocol) server (`core/src/mcp/server.rs`) and defines 22 builtin tools (`core/src/mcp/builtin.rs`). However, the `nde mcp` CLI command starts an empty server without registering these tools, and the handler in `server.rs` only processes `nde_kanban_` tools (returning a stub message for everything else). 

This ticket implements the actual tool execution routing so external agents and IDEs (like Cursor or Claude Code) can fully utilize NDE OS via the stdio MCP gateway.

## Feature
- Update the CLI to launch the MCP server populated with all built-in tools.
- Implement execution handlers in `core/src/mcp/server.rs` for core NDE OS tools (e.g., filesystem, shell commands, codebase search) so they actually perform the requested operations instead of returning stub messages.
- Ensure the NDE OS agent can act as a fully functional real gateway channel for agent-to-agent and human-to-agent interaction.

## Inputs & Outputs
- **Input**: external IDE or agent connects via `nde mcp` (stdio transport) and invokes tools (e.g., `nde_file_read`, `nde_shell`).
- **Output**: The NDE OS MCP server executes the operation correctly using safe Rust primitives (e.g., `std::fs`, `std::process::Command` within the sandbox constraints) and returns the JSON-RPC response.

## Edge Cases & Security
- **Sandbox Compliance**: `nde_file_read` / `nde_file_write` / `nde_shell` must run within the sandbox constraints, preventing traversal outside the allowed workspace.
- **Error Handling**: Missing files, syntax errors, or execution failures in the shell must return valid MCP JSON-RPC error responses rather than panicking.
- **Process Management**: `nde_shell` needs a timeout so runaway external agent commands don't hang the MCP stdio listener forever.

## Task Checklist
- [x] **CLI Fix**: Update `cli/src/main.rs` `Commands::Mcp` to use `builtin::create_executable_server()` instead of `McpServer::new()`.
- [x] **Tool Routing**: In `core/src/mcp/server.rs`, replace the ToolRegistry stub in `tools/call` with dispatch to ToolRegistry+Sandbox for all registered tools.
- [x] **Implementation**: Implement the Rust execution backend (`McpServer::with_executor`) using the existing `ToolRegistry` + `Sandbox` (path canonicalization defense) from `core/src/tools/`.
- [x] **Name Mapping**: Map MCP tool names (`nde_file_read` etc.) → internal registry names (`file_read` etc.) with explicit aliases for diverging names (`nde_shell` → `shell_exec`).
- [x] **Test & Validation**: 8 unit tests passing — tool listing, executable server creation, file read, file write+read roundtrip, shell execution, and sandbox escape prevention.

## Definition of Done
- [x] Local: The MCP CLI exposes 22+ builtin tools over `stdio`. Calling `nde_file_read` successfully reads a file relative to the current workspace. Calling `nde_shell` successfully executes a safe command. Calling `nde_file_write` + `nde_file_read` roundtrips correctly. Sandbox escape via `../../../etc/passwd` is blocked.
- [x] Global: No panics; `anyhow::Result` used correctly. Sandbox constraints respected. Stubs removed — all tools dispatch to real ToolRegistry implementations.
