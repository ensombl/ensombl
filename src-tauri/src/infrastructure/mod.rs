//! Infrastructure layer: adapters implementing application ports.

pub mod build_info;
pub mod fs_scanner;
pub mod git_cli;
pub mod json_sessions;
pub mod login_shell;
pub mod native_pty;
pub mod notify_watcher;
pub mod sqlite_cache;
pub mod ts_parser;
pub mod ts_resolver;
