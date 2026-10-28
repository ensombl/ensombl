//! Commands that start agent CLIs in PTYs and drive them.

use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use serde::Deserialize;
use ts_rs::TS;

use crate::application::cqrs::{Command, CommandHandler};
use crate::application::ports::{
    PtyControl, PtySpawner, ShellEnvironment, SpawnSpec, TerminalSink, WorktreeResolver,
};
use crate::application::{Error, Result};
use crate::domain::terminal::{AgentKind, AgentProfile, Scrollback, TerminalId, TerminalInfo};
use crate::domain::workspace::WorktreeId;

/// Starts an agent profile in a worktree. Output goes to the sink passed alongside.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StartTerminal {
    pub worktree_id: WorktreeId,
    pub profile_id: String,
    pub cols: u16,
    pub rows: u16,
    /// Continue an agent session (Claude), e.g. taking over a task.
    #[serde(default)]
    #[ts(optional)]
    pub resume: Option<String>,
}

/// Runs an arbitrary command line (e.g. a terminal editor) through the login shell.
#[derive(Debug)]
pub struct StartCommand {
    pub worktree_id: WorktreeId,
    pub command_line: String,
    pub title: String,
    pub cols: u16,
    pub rows: u16,
}

/// Re-attaches a view: replays scrollback into the new sink, then streams.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AttachTerminal {
    pub terminal_id: TerminalId,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WriteTerminal {
    pub terminal_id: TerminalId,
    pub data: String,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ResizeTerminal {
    pub terminal_id: TerminalId,
    pub cols: u16,
    pub rows: u16,
}

/// Kills the process and forgets the terminal.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CloseTerminal {
    pub terminal_id: TerminalId,
}

/// A command plus the sink its output should flow into.
pub struct WithSink<C> {
    pub command: C,
    pub sink: Arc<dyn TerminalSink>,
}

impl Command for WithSink<StartTerminal> {
    type Output = TerminalInfo;
}
impl Command for WithSink<StartCommand> {
    type Output = TerminalInfo;
}
impl Command for WithSink<AttachTerminal> {
    type Output = TerminalInfo;
}
impl Command for WriteTerminal {
    type Output = ();
}
impl Command for ResizeTerminal {
    type Output = ();
}
impl Command for CloseTerminal {
    type Output = ();
}

struct Terminal {
    info: Mutex<TerminalInfo>,
    control: Mutex<Box<dyn PtyControl>>,
    scrollback: Mutex<Scrollback>,
    sink: Mutex<Arc<dyn TerminalSink>>,
}

/// Running terminals, shared by commands and queries.
#[derive(Default)]
pub struct TerminalRegistry {
    terminals: RwLock<HashMap<TerminalId, Arc<Terminal>>>,
    next: Mutex<u64>,
}

impl TerminalRegistry {
    pub fn list(&self, worktree_id: &str) -> Vec<TerminalInfo> {
        let mut list: Vec<_> = self
            .terminals
            .read()
            .unwrap()
            .values()
            .map(|t| t.info.lock().unwrap().clone())
            .filter(|i| i.worktree_id == worktree_id)
            .collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }

    fn get(&self, id: &str) -> Result<Arc<Terminal>> {
        self.terminals
            .read()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| Error::Invalid(format!("no terminal {id}")))
    }

    fn next_id(&self) -> TerminalId {
        let mut n = self.next.lock().unwrap();
        *n += 1;
        format!("t{:04}", *n)
    }

    /// Kills every process; called on shutdown.
    pub fn kill_all(&self) {
        for t in self.terminals.read().unwrap().values() {
            t.control.lock().unwrap().kill();
        }
    }
}

pub struct TerminalCommands {
    worktrees: Arc<dyn WorktreeResolver>,
    shell: Arc<dyn ShellEnvironment>,
    spawner: Arc<dyn PtySpawner>,
    registry: Arc<TerminalRegistry>,
    profiles: Vec<AgentProfile>,
}

impl TerminalCommands {
    pub fn new(
        worktrees: Arc<dyn WorktreeResolver>,
        shell: Arc<dyn ShellEnvironment>,
        spawner: Arc<dyn PtySpawner>,
        registry: Arc<TerminalRegistry>,
        profiles: Vec<AgentProfile>,
    ) -> Self {
        Self {
            worktrees,
            shell,
            spawner,
            registry,
            profiles,
        }
    }

    fn resolve_program(&self, profile: &AgentProfile) -> Result<(PathBuf, Vec<String>)> {
        if profile.command == "$SHELL" {
            return Ok((self.shell.shell(), vec!["-l".into()]));
        }
        let program = self.shell.find(&profile.command).ok_or_else(|| {
            Error::Invalid(format!(
                "`{}` isn't installed or isn't on your PATH",
                profile.command
            ))
        })?;
        Ok((program, profile.args.clone()))
    }
}

/// Pumps PTY output into the scrollback and the current sink until EOF.
fn pump(terminal: Arc<Terminal>, mut output: Box<dyn Read + Send>) {
    std::thread::spawn(move || {
        let mut buf = [0u8; 16 * 1024];
        loop {
            match output.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    terminal.scrollback.lock().unwrap().push(&buf[..n]);
                    terminal.sink.lock().unwrap().output(&buf[..n]);
                }
            }
        }
        let code = terminal.control.lock().unwrap().wait();
        {
            let mut info = terminal.info.lock().unwrap();
            info.running = false;
            info.exit_code = code;
        }
        terminal.sink.lock().unwrap().exited(code);
    });
}

/// What a new terminal runs and how it's labelled.
struct Launch {
    worktree_id: WorktreeId,
    profile_id: String,
    title: String,
    program: PathBuf,
    args: Vec<String>,
    cols: u16,
    rows: u16,
}

impl TerminalCommands {
    fn worktree_path(&self, worktree_id: &str) -> Result<PathBuf> {
        self.worktrees.root(worktree_id)
    }

    fn launch(&self, launch: Launch, sink: Arc<dyn TerminalSink>) -> Result<TerminalInfo> {
        let spawned = self.spawner.spawn(SpawnSpec {
            program: launch.program,
            args: launch.args,
            cwd: self.worktree_path(&launch.worktree_id)?,
            env: self.shell.env(),
            cols: launch.cols.max(10),
            rows: launch.rows.max(4),
        })?;
        let id = self.registry.next_id();
        let info = TerminalInfo {
            id: id.clone(),
            worktree_id: launch.worktree_id,
            profile_id: launch.profile_id,
            title: launch.title,
            running: true,
            exit_code: None,
        };
        let terminal = Arc::new(Terminal {
            info: Mutex::new(info.clone()),
            control: Mutex::new(spawned.control),
            scrollback: Mutex::default(),
            sink: Mutex::new(sink),
        });
        self.registry
            .terminals
            .write()
            .unwrap()
            .insert(id, terminal.clone());
        pump(terminal, spawned.output);
        Ok(info)
    }
}

impl CommandHandler<WithSink<StartTerminal>> for TerminalCommands {
    fn handle(
        &self,
        WithSink { command: cmd, sink }: WithSink<StartTerminal>,
    ) -> Result<TerminalInfo> {
        self.worktree_path(&cmd.worktree_id)?;
        let profile = self
            .profiles
            .iter()
            .find(|p| p.id == cmd.profile_id)
            .ok_or_else(|| Error::Invalid(format!("unknown agent {}", cmd.profile_id)))?;
        let (program, mut args) = self.resolve_program(profile)?;
        if let (Some(session), AgentKind::Claude) = (cmd.resume, profile.kind) {
            args.extend(["--resume".into(), session]);
        }
        let siblings = self
            .registry
            .list(&cmd.worktree_id)
            .iter()
            .filter(|t| t.profile_id == profile.id)
            .count();
        let title = if siblings == 0 {
            profile.label.clone()
        } else {
            format!("{} {}", profile.label, siblings + 1)
        };
        self.launch(
            Launch {
                worktree_id: cmd.worktree_id,
                profile_id: profile.id.clone(),
                title,
                program,
                args,
                cols: cmd.cols,
                rows: cmd.rows,
            },
            sink,
        )
    }
}

impl CommandHandler<WithSink<StartCommand>> for TerminalCommands {
    fn handle(
        &self,
        WithSink { command: cmd, sink }: WithSink<StartCommand>,
    ) -> Result<TerminalInfo> {
        self.launch(
            Launch {
                worktree_id: cmd.worktree_id,
                profile_id: "command".into(),
                title: cmd.title,
                program: self.shell.shell(),
                args: vec!["-lc".into(), cmd.command_line],
                cols: cmd.cols,
                rows: cmd.rows,
            },
            sink,
        )
    }
}

impl CommandHandler<WithSink<AttachTerminal>> for TerminalCommands {
    fn handle(
        &self,
        WithSink { command: cmd, sink }: WithSink<AttachTerminal>,
    ) -> Result<TerminalInfo> {
        let terminal = self.registry.get(&cmd.terminal_id)?;
        // Hold the sink lock while replaying so no live output slips in between.
        let mut current = terminal.sink.lock().unwrap();
        sink.output(&terminal.scrollback.lock().unwrap().snapshot());
        let info = terminal.info.lock().unwrap().clone();
        if !info.running {
            sink.exited(info.exit_code);
        }
        *current = sink;
        Ok(info)
    }
}

impl CommandHandler<WriteTerminal> for TerminalCommands {
    fn handle(&self, cmd: WriteTerminal) -> Result<()> {
        self.registry
            .get(&cmd.terminal_id)?
            .control
            .lock()
            .unwrap()
            .write(cmd.data.as_bytes())
    }
}

impl CommandHandler<ResizeTerminal> for TerminalCommands {
    fn handle(&self, cmd: ResizeTerminal) -> Result<()> {
        self.registry
            .get(&cmd.terminal_id)?
            .control
            .lock()
            .unwrap()
            .resize(cmd.cols.max(10), cmd.rows.max(4))
    }
}

impl CommandHandler<CloseTerminal> for TerminalCommands {
    fn handle(&self, cmd: CloseTerminal) -> Result<()> {
        if let Some(t) = self
            .registry
            .terminals
            .write()
            .unwrap()
            .remove(&cmd.terminal_id)
        {
            t.control.lock().unwrap().kill();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::Path;
    use std::time::{Duration, Instant};

    use super::*;
    use crate::application::ports::SessionRepository;
    use crate::application::worktrees::Worktrees;
    use crate::domain::workspace::{Session, Worktree};

    struct Sessions(Session);
    impl SessionRepository for Sessions {
        fn get(&self) -> Result<Session> {
            Ok(self.0.clone())
        }
        fn update(&self, _: &mut dyn FnMut(&mut Session) -> Result<()>) -> Result<()> {
            Ok(())
        }
    }

    struct FakeShell;
    impl ShellEnvironment for FakeShell {
        fn shell(&self) -> PathBuf {
            "/bin/zsh".into()
        }
        fn find(&self, program: &str) -> Option<PathBuf> {
            (program == "claude").then(|| "/usr/local/bin/claude".into())
        }
        fn env(&self) -> Vec<(String, String)> {
            vec![]
        }
    }

    #[derive(Default)]
    struct FakeControl(Arc<Mutex<Vec<u8>>>);
    impl PtyControl for FakeControl {
        fn write(&mut self, data: &[u8]) -> Result<()> {
            self.0.lock().unwrap().extend(data);
            Ok(())
        }
        fn resize(&mut self, _: u16, _: u16) -> Result<()> {
            Ok(())
        }
        fn kill(&mut self) {}
        fn wait(&mut self) -> Option<i32> {
            Some(0)
        }
    }

    #[derive(Default)]
    struct FakeSpawner {
        specs: Mutex<Vec<SpawnSpec>>,
        written: Arc<Mutex<Vec<u8>>>,
    }
    impl PtySpawner for FakeSpawner {
        fn spawn(&self, spec: SpawnSpec) -> Result<crate::application::ports::SpawnedPty> {
            self.specs.lock().unwrap().push(spec);
            Ok(crate::application::ports::SpawnedPty {
                output: Box::new(Cursor::new(b"hello from agent".to_vec())),
                control: Box::new(FakeControl(self.written.clone())),
            })
        }
    }

    #[derive(Default)]
    struct Collect(Mutex<Vec<u8>>, Mutex<Option<Option<i32>>>);
    impl TerminalSink for Collect {
        fn output(&self, bytes: &[u8]) {
            self.0.lock().unwrap().extend(bytes);
        }
        fn exited(&self, code: Option<i32>) {
            *self.1.lock().unwrap() = Some(code);
        }
    }

    fn setup() -> (TerminalCommands, Arc<FakeSpawner>, Arc<TerminalRegistry>) {
        let mut session = Session::default();
        session.add_project("p".into(), "proj".into(), "/repo".into());
        session
            .open_tab(Worktree {
                id: "wt".into(),
                project_id: "p".into(),
                path: "/repo".into(),
                branch: Some("main".into()),
                is_main: true,
            })
            .unwrap();
        let spawner = Arc::new(FakeSpawner::default());
        let registry = Arc::new(TerminalRegistry::default());
        let cmds = TerminalCommands::new(
            Arc::new(Worktrees::new(Arc::new(Sessions(session)), None)),
            Arc::new(FakeShell),
            spawner.clone(),
            registry.clone(),
            AgentProfile::builtin(),
        );
        (cmds, spawner, registry)
    }

    fn wait_until(f: impl Fn() -> bool) {
        let start = Instant::now();
        while !f() && start.elapsed() < Duration::from_secs(2) {
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn start(cmds: &TerminalCommands, profile: &str, sink: Arc<Collect>) -> Result<TerminalInfo> {
        cmds.handle(WithSink {
            command: StartTerminal {
                worktree_id: "wt".into(),
                profile_id: profile.into(),
                cols: 80,
                rows: 24,
                resume: None,
            },
            sink,
        })
    }

    #[test]
    fn resumes_a_claude_session_when_asked() {
        let (cmds, spawner, _) = setup();
        cmds.handle(WithSink {
            command: StartTerminal {
                worktree_id: "wt".into(),
                profile_id: "claude".into(),
                cols: 80,
                rows: 24,
                resume: Some("s1".into()),
            },
            sink: Arc::new(Collect::default()),
        })
        .unwrap();
        let spec = spawner.specs.lock().unwrap()[0].clone();
        assert_eq!(spec.args, ["--resume", "s1"]);
    }

    #[test]
    fn starts_an_agent_in_the_worktree_and_streams_output() {
        let (cmds, spawner, registry) = setup();
        let sink = Arc::new(Collect::default());
        let info = start(&cmds, "claude", sink.clone()).unwrap();

        let spec = spawner.specs.lock().unwrap()[0].clone();
        assert_eq!(spec.program, Path::new("/usr/local/bin/claude"));
        assert_eq!(spec.cwd, Path::new("/repo"));
        assert_eq!(info.title, "Claude Code");

        wait_until(|| sink.1.lock().unwrap().is_some());
        assert_eq!(&*sink.0.lock().unwrap(), b"hello from agent");
        assert_eq!(*sink.1.lock().unwrap(), Some(Some(0)));
        assert!(!registry.list("wt")[0].running);
    }

    #[test]
    fn attaching_replays_scrollback() {
        let (cmds, _, _) = setup();
        let first = Arc::new(Collect::default());
        let info = start(&cmds, "claude", first.clone()).unwrap();
        wait_until(|| first.1.lock().unwrap().is_some());

        let second = Arc::new(Collect::default());
        cmds.handle(WithSink {
            command: AttachTerminal {
                terminal_id: info.id,
            },
            sink: second.clone(),
        })
        .unwrap();
        assert_eq!(&*second.0.lock().unwrap(), b"hello from agent");
        assert_eq!(*second.1.lock().unwrap(), Some(Some(0)));
    }

    #[test]
    fn writes_reach_the_pty() {
        let (cmds, spawner, _) = setup();
        let info = start(&cmds, "claude", Arc::new(Collect::default())).unwrap();
        cmds.handle(WriteTerminal {
            terminal_id: info.id,
            data: "hi\r".into(),
        })
        .unwrap();
        assert_eq!(&*spawner.written.lock().unwrap(), b"hi\r");
    }

    #[test]
    fn missing_cli_is_a_clear_error() {
        let (cmds, _, _) = setup();
        let err = start(&cmds, "codex", Arc::new(Collect::default())).unwrap_err();
        assert!(err.to_string().contains("isn't installed"));
    }

    #[test]
    fn second_instance_gets_a_numbered_title() {
        let (cmds, _, _) = setup();
        start(&cmds, "shell", Arc::new(Collect::default())).unwrap();
        let second = start(&cmds, "shell", Arc::new(Collect::default())).unwrap();
        assert_eq!(second.title, "Shell 2");
    }
}
