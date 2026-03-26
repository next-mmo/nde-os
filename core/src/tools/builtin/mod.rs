// ── Filesystem tools ─────────────────────────────────────────────────────────
mod file_read;
mod file_write;
mod file_delete;
mod file_list;
mod file_search;
mod file_patch;

// ── Shell ────────────────────────────────────────────────────────────────────
mod shell_exec;

// ── Code tools ───────────────────────────────────────────────────────────────
pub mod code_tools;

// ── Memory tools ─────────────────────────────────────────────────────────────
mod memory_tools;
mod conversation_tools;

// ── Knowledge tools ──────────────────────────────────────────────────────────
mod knowledge_tools;

// ── System integration tools ─────────────────────────────────────────────────
mod app_tools;
mod system_info;
mod http_fetch;
mod skill_list;

#[cfg(feature = "screenshot")]
pub mod screenshot;

// ── Web tools (Phase 3) ─────────────────────────────────────────────────────
mod web_browse;
mod web_search;

// ── Git tools (Phase 3) ─────────────────────────────────────────────────────
mod git_tools;

use super::ToolRegistry;

/// Creates the default tool registry with all 27 built-in tools.
///
/// Categories:
///   Filesystem (6): file_read, file_write, file_delete, file_list, file_search, file_patch
///   Shell (1):      shell_exec
///   Code (3):       code_search, code_edit, code_symbols
///   Memory (4):     memory_store, memory_recall, conversation_save, conversation_search
///   Knowledge (2):  knowledge_store, knowledge_query
///   Web (3):        web_browse, web_search, http_fetch
///   Git (1):        git
///   System (7):     app_list, app_install, app_launch, app_stop, system_info, skill_list
pub fn default_registry() -> ToolRegistry {
    let mut reg = ToolRegistry::new();

    // Filesystem
    reg.register(Box::new(file_read::FileReadTool));
    reg.register(Box::new(file_write::FileWriteTool));
    reg.register(Box::new(file_delete::FileDeleteTool));
    reg.register(Box::new(file_list::FileListTool));
    reg.register(Box::new(file_search::FileSearchTool));
    reg.register(Box::new(file_patch::FilePatchTool));

    // Shell
    reg.register(Box::new(shell_exec::ShellExecTool));

    // Code
    reg.register(Box::new(code_tools::CodeSearchTool));
    reg.register(Box::new(code_tools::CodeEditTool));
    reg.register(Box::new(code_tools::CodeSymbolsTool));

    // Memory
    reg.register(Box::new(memory_tools::MemoryStoreTool));
    reg.register(Box::new(memory_tools::MemoryRecallTool));
    reg.register(Box::new(conversation_tools::ConversationSaveTool));
    reg.register(Box::new(conversation_tools::ConversationSearchTool));

    // Knowledge
    reg.register(Box::new(knowledge_tools::KnowledgeStoreTool));
    reg.register(Box::new(knowledge_tools::KnowledgeQueryTool));

    // Web (Phase 3)
    reg.register(Box::new(web_browse::WebBrowseTool));
    reg.register(Box::new(web_search::WebSearchTool));
    reg.register(Box::new(http_fetch::HttpFetchTool));

    // Git (Phase 3)
    reg.register(Box::new(git_tools::GitTool));

    // System integration
    reg.register(Box::new(app_tools::AppListTool));
    reg.register(Box::new(app_tools::AppInstallTool));
    reg.register(Box::new(app_tools::AppLaunchTool));
    reg.register(Box::new(app_tools::AppStopTool));
    reg.register(Box::new(system_info::SystemInfoTool));
    reg.register(Box::new(skill_list::SkillListTool));

    #[cfg(feature = "screenshot")]
    reg.register(Box::new(screenshot::ScreenshotTool));

    reg
}

/// Creates a minimal registry with only filesystem + shell tools.
/// Useful for restricted sandbox contexts.
pub fn minimal_registry() -> ToolRegistry {
    let mut reg = ToolRegistry::new();
    reg.register(Box::new(file_read::FileReadTool));
    reg.register(Box::new(file_write::FileWriteTool));
    reg.register(Box::new(shell_exec::ShellExecTool));
    reg
}
