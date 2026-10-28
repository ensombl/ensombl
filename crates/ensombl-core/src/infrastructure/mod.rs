//! Infrastructure layer: adapters implementing application ports.

pub mod build_info;
pub mod claude_headless;
pub mod fs_scanner;
pub mod git_cli;
pub mod json_architecture;
pub mod json_comments;
pub mod json_decisions;
pub mod json_sessions;
pub mod jsonl_run_log;
pub mod login_shell;
pub mod native_pty;
pub mod notify_watcher;
pub mod shell_launcher;
pub mod sqlite_cache;
pub mod sqlite_tasks;
pub mod ts_parser;
pub mod ts_resolver;
