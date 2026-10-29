//! Composition root shared by every shell: wires adapters into handlers.
//! A shell supplies where data lives and how events leave the process.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};

use serde::Serialize;
use ts_rs::TS;

use crate::application::commands::architecture::ArchitectureCommands;
use crate::application::commands::indexing::{IndexingCommands, IndexingPorts, ReindexPaths};
use crate::application::commands::patterns::PatternCommands;
use crate::application::commands::review::ReviewCommands;
use crate::application::commands::tasks::{TaskCommands, TaskPorts};
use crate::application::commands::terminals::{TerminalCommands, TerminalRegistry};
use crate::application::commands::workspaces::WorkspaceCommands;
use crate::application::cqrs::CommandHandler;
use crate::application::index_registry::IndexRegistry;
use crate::application::ports::{
    AgentRuntime, EventBus, IndexEvents, ShellEnvironment, SourceScanner,
};
use crate::application::queries::architecture::ArchitectureQueries;
use crate::application::queries::files::FileQueries;
use crate::application::queries::get_app_info::GetAppInfoHandler;
use crate::application::queries::model::ModelQueries;
use crate::application::queries::patterns::PatternQueries;
use crate::application::queries::review::ReviewQueries;
use crate::application::queries::tasks::TaskQueries;
use crate::application::queries::terminals::TerminalQueries;
use crate::application::queries::workspaces::WorkspaceQueries;
use crate::application::worktrees::Worktrees;
use crate::application::{Error, Result};
use crate::domain::model::ModelDelta;
use crate::domain::terminal::{AgentKind, AgentProfile, LeadMcp};
use crate::infrastructure::build_info::CargoBuildInfo;
use crate::infrastructure::claude_headless::ClaudeCode;
use crate::infrastructure::codex_exec::Codex;
use crate::infrastructure::fs_scanner::FsScanner;
use crate::infrastructure::fs_workflows::FsWorkflows;
use crate::infrastructure::git_cli::GitCli;
use crate::infrastructure::json_architecture::JsonArchitecture;
use crate::infrastructure::json_comments::{JsonComments, JsonSettings};
use crate::infrastructure::json_decisions::JsonDecisions;
use crate::infrastructure::json_sessions::JsonSessions;
use crate::infrastructure::jsonl_run_log::JsonlRunLog;
use crate::infrastructure::login_shell::LoginShell;
use crate::infrastructure::native_pty::NativePty;
use crate::infrastructure::notify_watcher::{ChangeBatch, NotifyWatcher};
use crate::infrastructure::shell_launcher::ShellLauncher;
use crate::infrastructure::sqlite_cache::SqliteCache;
use crate::infrastructure::sqlite_tasks::SqliteTasks;
use crate::infrastructure::system_clock::SystemClock;
use crate::infrastructure::ts_parser::TreeSitterParser;
use crate::infrastructure::ts_resolver::TsResolverFactory;

/// Where an instance keeps its state.
#[derive(Debug, Clone)]
pub struct AppPaths {
    /// Session, caches, comments and settings.
    pub data_dir: PathBuf,
    /// New git worktrees go under here.
    pub worktrees_dir: PathBuf,
}

/// Shell-specific choices.
#[derive(Debug, Clone, Default)]
pub struct CoreOptions {
    /// Ensombl's MCP bridge; adds the lead agent profiles.
    pub lead_mcp: Option<LeadMcp>,
}

pub const INDEX_PROGRESS: &str = "index://progress";
pub const MODEL_UPDATED: &str = "index://updated";

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IndexProgress {
    pub worktree_id: String,
    pub done: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModelUpdated {
    pub worktree_id: String,
    /// Incremental change; `null` after a full (re)index, meaning refetch.
    pub delta: Option<ModelDelta>,
}

/// Indexing progress published on the shell's event bus.
struct BusIndexEvents(Arc<dyn EventBus>);

impl IndexEvents for BusIndexEvents {
    fn progress(&self, worktree: &str, done: usize, total: usize) {
        self.0.publish(
            INDEX_PROGRESS,
            to_value(IndexProgress {
                worktree_id: worktree.into(),
                done: done as u32,
                total: total as u32,
            }),
        );
    }

    fn updated(&self, worktree: &str, delta: Option<&ModelDelta>) {
        self.0.publish(
            MODEL_UPDATED,
            to_value(ModelUpdated {
                worktree_id: worktree.into(),
                delta: delta.cloned(),
            }),
        );
    }
}

fn to_value(payload: impl Serialize) -> serde_json::Value {
    serde_json::to_value(payload).unwrap_or_default()
}

/// Every command and query handler. `Arc`s so blocking work can move onto worker threads.
pub struct Handlers {
    pub get_app_info: GetAppInfoHandler,
    pub workspace_commands: Arc<WorkspaceCommands>,
    pub workspace_queries: Arc<WorkspaceQueries>,
    pub indexing_commands: Arc<IndexingCommands>,
    pub model_queries: Arc<ModelQueries>,
    pub terminal_commands: Arc<TerminalCommands>,
    pub terminal_queries: Arc<TerminalQueries>,
    pub architecture_commands: Arc<ArchitectureCommands>,
    pub architecture_queries: Arc<ArchitectureQueries>,
    pub file_queries: Arc<FileQueries>,
    pub pattern_commands: Arc<PatternCommands>,
    pub pattern_queries: Arc<PatternQueries>,
    pub review_commands: Arc<ReviewCommands>,
    pub review_queries: Arc<ReviewQueries>,
    pub task_commands: Arc<TaskCommands>,
    pub task_queries: Arc<TaskQueries>,
}

/// A wired-up instance.
pub struct Core {
    pub handlers: Arc<Handlers>,
    terminals: Arc<TerminalRegistry>,
}

impl Core {
    pub fn build(paths: AppPaths, bus: Arc<dyn EventBus>) -> Result<Core> {
        Self::build_with(paths, bus, CoreOptions::default())
    }

    pub fn build_with(
        paths: AppPaths,
        bus: Arc<dyn EventBus>,
        options: CoreOptions,
    ) -> Result<Core> {
        let AppPaths {
            data_dir,
            worktrees_dir,
        } = paths;
        std::fs::create_dir_all(&data_dir)?;
        let sessions = Arc::new(JsonSessions::open(data_dir.join("session.json")));
        let tasks = Arc::new(SqliteTasks::open(&data_dir.join("orchestrator.db"))?);
        let run_log = Arc::new(JsonlRunLog::new(data_dir.join("runs")));
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
                cache: Arc::new(
                    SqliteCache::open(&data_dir.join("index.db"))
                        .map_err(|e| Error::Storage(e.to_string()))?,
                ),
                watcher: Arc::new(NotifyWatcher::new(changes_tx, is_source)),
                events: Arc::new(BusIndexEvents(bus.clone())),
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
        let profiles = AgentProfile::builtin_with(options.lead_mcp.as_ref());

        let terminal_commands = Arc::new(TerminalCommands::new(
            Arc::new(Worktrees::new(sessions.clone(), Some(tasks.clone()))),
            shell.clone(),
            Arc::new(NativePty),
            terminals.clone(),
            profiles.clone(),
        ));
        let comments = Arc::new(JsonComments::open(data_dir.join("comments.json")));
        let settings = Arc::new(JsonSettings::open(data_dir.join("settings.json")));

        let mut runtimes: HashMap<AgentKind, Arc<dyn AgentRuntime>> = HashMap::new();
        runtimes.insert(AgentKind::Claude, Arc::new(ClaudeCode));
        runtimes.insert(AgentKind::Codex, Arc::new(Codex));
        let task_commands = Arc::new(TaskCommands::new(TaskPorts {
            tasks: tasks.clone(),
            log: run_log.clone(),
            runtimes,
            sessions: sessions.clone(),
            shell: shell.clone(),
            git: git.clone(),
            task_git: git.clone(),
            bus,
            clock: Arc::new(SystemClock),
            worktrees_dir: worktrees_dir.clone(),
            settings: settings.clone(),
            workflows: Arc::new(FsWorkflows),
        }));
        task_commands.recover()?;
        task_commands.dispatch();
        let task_queries = Arc::new(TaskQueries::new(
            tasks,
            run_log,
            sessions.clone(),
            git.clone(),
            Arc::new(FsWorkflows),
        ));

        let handlers = Handlers {
            get_app_info: GetAppInfoHandler::new(Arc::new(CargoBuildInfo)),
            workspace_commands: Arc::new(WorkspaceCommands::new(
                git.clone(),
                sessions.clone(),
                worktrees_dir,
            )),
            workspace_queries: Arc::new(WorkspaceQueries::new(git, sessions.clone())),
            indexing_commands: indexing,
            model_queries: Arc::new(ModelQueries::new(registry.clone())),
            architecture_commands: Arc::new(ArchitectureCommands::new(
                registry.clone(),
                Arc::new(JsonArchitecture),
            )),
            architecture_queries: Arc::new(ArchitectureQueries::new(
                registry.clone(),
                Arc::new(JsonArchitecture),
            )),
            pattern_commands: Arc::new(PatternCommands::new(sessions.clone(), decisions.clone())),
            pattern_queries: Arc::new(PatternQueries::new(sessions.clone(), registry, decisions)),
            file_queries: Arc::new(FileQueries::new(sessions.clone(), Arc::new(FsScanner))),
            terminal_commands: terminal_commands.clone(),
            terminal_queries: Arc::new(TerminalQueries::new(
                shell.clone(),
                terminals.clone(),
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
            task_commands,
            task_queries,
        };
        Ok(Core {
            handlers: Arc::new(handlers),
            terminals,
        })
    }

    /// Stops every agent process this instance started.
    pub fn shutdown(&self) {
        self.terminals.kill_all();
        self.handlers.task_commands.shutdown();
    }
}
