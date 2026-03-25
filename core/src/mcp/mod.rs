/// MCP (Model Context Protocol) — client and server for ecosystem interop.
/// - Client: connect to external MCP tool servers (stdio transport)
/// - Server: expose NDE-OS sandboxed tools as an MCP server
/// - Builtin: auto-register all NDE-OS tools as MCP-accessible

pub mod builtin;
pub mod client;
pub mod server;
