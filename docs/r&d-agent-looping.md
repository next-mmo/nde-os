| Project | Sandbox (no Docker) | Plugin/Extension System | Binary Size | RAM Usage |
| :--- | :--- | :--- | :--- | :--- |
| **OpenFang** | WASM dual-metered | 38+ native tools, MCP client+server, custom Hands (HAND.toml), skill system | 32 MB | ~50 MB |
| **IronClaw** | WASM capability-based | Drop in new WASM tools and channels without restarting GitHub, dynamic tool building from plain language, MCP protocol | varies | ~80 MB |
| **Moltis** | WASM (Wasmtime) + Apple Container | Skills + OpenClaw Store import, MCP (stdio+HTTP/SSE), 15 hook events | 44 MB | ~60 MB |
| **Carapace** | WASM plugins + OS-level subprocess sandboxing GitHub | Signed plugin runtime — plugins are signature-verified and run with strict permissions and resource limits GitHub, guarded filesystem tools | ~20 MB | ~30 MB |
| **OpenCrust** | WASM sandboxing — optional plugin sandbox via WebAssembly runtime with controlled host access GitHub | WASM plugin system (compile with --features plugins), MCP (stdio), self-update | 16 MB | 13 MB |
| **ZeroClaw** | WASM workspace-scoped | Rust trait extensions, MCP, skills, OpenClaw migration | 3.4 MB | <5 MB |
