pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interface;

use std::sync::{Arc, mpsc};

use tauri::Manager;

use application::commands::indexing::{IndexingCommands, IndexingPorts, ReindexPaths};
use application::commands::patterns::PatternCommands;
use application::commands::review::ReviewCommands;
use application::commands::terminals::{TerminalCommands, TerminalRegistry};
use application::commands::workspaces::WorkspaceCommands;
use application::cqrs::CommandHandler;
use application::index_registry::IndexRegistry;
use application::ports::{ShellEnvironment, SourceScanner};
use application::queries::get_app_info::GetAppInfoHandler;
use application::queries::model::ModelQueries;
use application::queries::patterns::PatternQueries;
use application::queries::review::ReviewQueries;
use application::queries::terminals::TerminalQueries;
use application::queries::workspaces::WorkspaceQueries;
use domain::terminal::AgentProfile;
use infrastructure::build_info::CargoBuildInfo;
use infrastructure::fs_scanner::FsScanner;
use infrastructure::git_cli::GitCli;
use infrastructure::json_comments::{JsonComments, JsonSettings};
use infrastructure::json_decisions::JsonDecisions;
use infrastructure::json_sessions::JsonSessions;
use infrastructure::login_shell::LoginShell;
use infrastructure::native_pty::NativePty;
use infrastructure::notify_watcher::{ChangeBatch, NotifyWatcher};
use infrastructure::shell_launcher::ShellLauncher;
use infrastructure::sqlite_cache::SqliteCache;
use infrastructure::ts_parser::TreeSitterParser;
use infrastructure::ts_resolver::TsResolverFactory;
use interface::commands as cmd;
use interface::events::TauriEvents;
use interface::state::Handlers;

/// Composition root: wire adapters into handlers and expose them to Tauri.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let paths = app.path();
            let data_dir = paths.app_data_dir()?;
            let sessions = Arc::new(JsonSessions::open(data_dir.join("session.json")));
            let worktrees_dir = paths.home_dir()?.join(".crm").join("worktrees");
            let git = Arc::new(GitCli);

            let scanner = Arc::new(FsScanner);
            let (changes_tx, changes_rx) = mpsc::channel::<ChangeBatch>();
            let is_source = {
                let scanner = scanner.clone();
                Arc::new(move |p: &str| scanner.is_source(p))
            };
            let registry = Arc::new(IndexRegistry::default());
            let indexing = Arc::new(IndexingCommands::new(
                IndexingPorts {
                    sessions: sessions.clone(),
                    scanner,
                    parser: Arc::new(TreeSitterParser),
                    resolvers: Arc::new(TsResolverFactory),
                    cache: Arc::new(SqliteCache::open(&data_dir.join("index.db"))?),
                    watcher: Arc::new(NotifyWatcher::new(changes_tx, is_source)),
                    events: Arc::new(TauriEvents(app.handle().clone())),
                },
                registry.clone(),
            ));

            // File changes from watchers become ReindexPaths commands.
            let reindex = indexing.clone();
            std::thread::spawn(move || {
                for (worktree_id, paths) in changes_rx {
                    if let Err(e) = reindex.handle(ReindexPaths { worktree_id, paths }) {
                        eprintln!("reindex failed: {e}");
                    }
                }
            });

            let decisions = Arc::new(JsonDecisions::open(data_dir.join("patterns.json")));
            let shell = Arc::new(LoginShell::detect());
            let terminals = Arc::new(TerminalRegistry::default());
            let profiles = AgentProfile::builtin();

            let terminal_commands = Arc::new(TerminalCommands::new(
                sessions.clone(),
                shell.clone(),
                Arc::new(NativePty),
                terminals.clone(),
                profiles.clone(),
            ));
            let comments = Arc::new(JsonComments::open(data_dir.join("comments.json")));
            let settings = Arc::new(JsonSettings::open(data_dir.join("settings.json")));

            app.manage(terminals.clone());
            app.manage(Handlers {
                get_app_info: GetAppInfoHandler::new(Arc::new(CargoBuildInfo)),
                workspace_commands: Arc::new(WorkspaceCommands::new(
                    git.clone(),
                    sessions.clone(),
                    worktrees_dir,
                )),
                workspace_queries: Arc::new(WorkspaceQueries::new(git, sessions.clone())),
                indexing_commands: indexing,
                model_queries: Arc::new(ModelQueries::new(registry.clone())),
                pattern_commands: Arc::new(PatternCommands::new(
                    sessions.clone(),
                    decisions.clone(),
                )),
                pattern_queries: Arc::new(PatternQueries::new(
                    sessions.clone(),
                    registry,
                    decisions,
                )),
                terminal_commands: terminal_commands.clone(),
                terminal_queries: Arc::new(TerminalQueries::new(
                    shell.clone(),
                    terminals,
                    profiles,
                )),
                review_commands: Arc::new(ReviewCommands::new(
                    sessions.clone(),
                    comments.clone(),
                    settings.clone(),
                    shell.clone(),
                    Arc::new(ShellLauncher {
                        shell: shell.shell(),
                    }),
                    terminal_commands,
                )),
                review_queries: Arc::new(ReviewQueries::new(sessions, comments, settings)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd::get_app_info,
            cmd::get_session,
            cmd::list_worktrees,
            cmd::list_branches,
            cmd::get_model,
            cmd::add_project,
            cmd::create_worktree,
            cmd::open_worktree,
            cmd::remove_worktree,
            cmd::close_tab,
            cmd::activate_tab,
            cmd::toggle_group,
            cmd::remove_project,
            cmd::index_worktree,
            cmd::drop_index,
            cmd::get_patterns,
            cmd::set_pattern_decision,
            cmd::read_source,
            cmd::list_comments,
            cmd::get_settings,
            cmd::add_comment,
            cmd::resolve_comment,
            cmd::delete_comment,
            cmd::update_settings,
            cmd::open_in_editor,
            cmd::list_agents,
            cmd::list_terminals,
            cmd::start_terminal,
            cmd::attach_terminal,
            cmd::write_terminal,
            cmd::resize_terminal,
            cmd::close_terminal,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Don't leave agent processes behind.
            if let tauri::RunEvent::Exit = event {
                app.state::<Arc<TerminalRegistry>>().kill_all();
            }
        });
}
