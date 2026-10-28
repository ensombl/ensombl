//! Commands that run agents as native chats and drive them.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::application::cqrs::{Command, CommandHandler};
use crate::application::ports::{
    AgentConnection, AgentLaunch, AgentRuntime, ChatSink, SessionRepository, ShellEnvironment,
};
use crate::application::{Error, Result};
use crate::domain::chat::{
    ChatEvent, ChatId, ChatInfo, PermissionDecision, PermissionMode, Transcript,
};
use crate::domain::terminal::{AgentKind, AgentMode, AgentProfile};
use crate::domain::workspace::WorktreeId;

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StartChat {
    pub worktree_id: WorktreeId,
    pub profile_id: String,
    /// Session id to continue.
    #[serde(default)]
    #[ts(optional)]
    pub resume: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub model: Option<String>,
    #[serde(default)]
    #[ts(optional)]
    pub permission_mode: Option<PermissionMode>,
}

/// Re-attaches a view: returns the transcript so far, then streams new events.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AttachChat {
    pub chat_id: ChatId,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChatSnapshot {
    pub info: ChatInfo,
    pub events: Vec<ChatEvent>,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SendChatMessage {
    pub chat_id: ChatId,
    pub text: String,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RespondPermission {
    pub chat_id: ChatId,
    pub request_id: String,
    pub decision: PermissionDecision,
}

/// Stops the current turn; the session stays open.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InterruptChat {
    pub chat_id: ChatId,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SetChatPermissionMode {
    pub chat_id: ChatId,
    pub mode: PermissionMode,
}

#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SetChatModel {
    pub chat_id: ChatId,
    pub model: String,
}

/// Kills the agent and forgets the chat.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CloseChat {
    pub chat_id: ChatId,
}

/// A command plus the sink live events should flow into.
pub struct WithChatSink<C> {
    pub command: C,
    pub sink: Arc<dyn ChatSink>,
}

impl Command for WithChatSink<StartChat> {
    type Output = ChatInfo;
}
impl Command for WithChatSink<AttachChat> {
    type Output = ChatSnapshot;
}
impl Command for SendChatMessage {
    type Output = ();
}
impl Command for RespondPermission {
    type Output = ();
}
impl Command for InterruptChat {
    type Output = ();
}
impl Command for SetChatPermissionMode {
    type Output = ();
}
impl Command for SetChatModel {
    type Output = ();
}
impl Command for CloseChat {
    type Output = ();
}

/// What the runtime's events update: the summary, the transcript and the live view.
struct ChatState {
    info: Mutex<ChatInfo>,
    transcript: Mutex<Transcript>,
    sink: Mutex<Arc<dyn ChatSink>>,
}

impl ChatState {
    fn record(&self, event: ChatEvent) {
        self.info.lock().unwrap().apply(&event);
        // The sink lock orders live delivery against attach snapshots.
        let sink = self.sink.lock().unwrap();
        sink.event(&event);
        self.transcript.lock().unwrap().push(event);
    }
}

struct Chat {
    state: Arc<ChatState>,
    connection: Box<dyn AgentConnection>,
}

/// Running chats, shared by commands and queries.
#[derive(Default)]
pub struct ChatRegistry {
    chats: RwLock<HashMap<ChatId, Arc<Chat>>>,
    next: Mutex<u64>,
}

impl ChatRegistry {
    pub fn list(&self, worktree_id: &str) -> Vec<ChatInfo> {
        let mut list: Vec<_> = self
            .chats
            .read()
            .unwrap()
            .values()
            .map(|c| c.state.info.lock().unwrap().clone())
            .filter(|i| i.worktree_id == worktree_id)
            .collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }

    fn get(&self, id: &str) -> Result<Arc<Chat>> {
        self.chats
            .read()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| Error::Invalid(format!("no chat {id}")))
    }

    fn next_id(&self) -> ChatId {
        let mut n = self.next.lock().unwrap();
        *n += 1;
        format!("c{:04}", *n)
    }

    /// Kills every agent; called on shutdown.
    pub fn kill_all(&self) {
        for c in self.chats.read().unwrap().values() {
            c.connection.kill();
        }
    }
}

pub struct ChatCommands {
    sessions: Arc<dyn SessionRepository>,
    shell: Arc<dyn ShellEnvironment>,
    runtimes: HashMap<AgentKind, Arc<dyn AgentRuntime>>,
    registry: Arc<ChatRegistry>,
    profiles: Vec<AgentProfile>,
}

impl ChatCommands {
    pub fn new(
        sessions: Arc<dyn SessionRepository>,
        shell: Arc<dyn ShellEnvironment>,
        runtimes: HashMap<AgentKind, Arc<dyn AgentRuntime>>,
        registry: Arc<ChatRegistry>,
        profiles: Vec<AgentProfile>,
    ) -> Self {
        Self {
            sessions,
            shell,
            runtimes,
            registry,
            profiles,
        }
    }

    fn worktree_path(&self, worktree_id: &str) -> Result<PathBuf> {
        self.sessions
            .get()?
            .tabs
            .iter()
            .find(|t| t.id == worktree_id)
            .map(|t| PathBuf::from(&t.path))
            .ok_or_else(|| Error::Invalid(format!("worktree {worktree_id} isn't open")))
    }
}

impl CommandHandler<WithChatSink<StartChat>> for ChatCommands {
    fn handle(
        &self,
        WithChatSink { command: cmd, sink }: WithChatSink<StartChat>,
    ) -> Result<ChatInfo> {
        let cwd = self.worktree_path(&cmd.worktree_id)?;
        let profile = self
            .profiles
            .iter()
            .find(|p| p.id == cmd.profile_id && p.mode == AgentMode::Chat)
            .ok_or_else(|| Error::Invalid(format!("{} can't run as a chat", cmd.profile_id)))?;
        let runtime = self
            .runtimes
            .get(&profile.kind)
            .ok_or_else(|| Error::Invalid(format!("no chat runtime for {}", profile.label)))?;
        let program = self.shell.find(&profile.command).ok_or_else(|| {
            Error::Invalid(format!(
                "`{}` isn't installed or isn't on your PATH",
                profile.command
            ))
        })?;

        let siblings = self
            .registry
            .list(&cmd.worktree_id)
            .iter()
            .filter(|c| c.profile_id == profile.id)
            .count();
        let title = if siblings == 0 {
            profile.label.clone()
        } else {
            format!("{} {}", profile.label, siblings + 1)
        };
        let permission_mode = cmd.permission_mode.unwrap_or_default();
        let info = ChatInfo {
            id: self.registry.next_id(),
            worktree_id: cmd.worktree_id,
            profile_id: profile.id.clone(),
            agent: profile.kind,
            title,
            running: true,
            busy: false,
            session_id: cmd.resume.clone(),
            model: cmd.model.clone(),
            permission_mode,
        };
        let state = Arc::new(ChatState {
            info: Mutex::new(info.clone()),
            transcript: Mutex::default(),
            sink: Mutex::new(sink),
        });
        let events = {
            let state = state.clone();
            Box::new(move |event| state.record(event))
        };
        let connection = runtime.start(
            AgentLaunch {
                program,
                args: profile.args.clone(),
                cwd,
                env: self.shell.env(),
                resume: cmd.resume,
                model: cmd.model,
                permission_mode,
            },
            events,
        )?;
        self.registry
            .chats
            .write()
            .unwrap()
            .insert(info.id.clone(), Arc::new(Chat { state, connection }));
        Ok(info)
    }
}

impl CommandHandler<WithChatSink<AttachChat>> for ChatCommands {
    fn handle(
        &self,
        WithChatSink { command: cmd, sink }: WithChatSink<AttachChat>,
    ) -> Result<ChatSnapshot> {
        let chat = self.registry.get(&cmd.chat_id)?;
        // Hold the sink lock so no event lands between the snapshot and the swap.
        let mut current = chat.state.sink.lock().unwrap();
        let snapshot = ChatSnapshot {
            info: chat.state.info.lock().unwrap().clone(),
            events: chat.state.transcript.lock().unwrap().events().to_vec(),
        };
        *current = sink;
        Ok(snapshot)
    }
}

impl CommandHandler<SendChatMessage> for ChatCommands {
    fn handle(&self, cmd: SendChatMessage) -> Result<()> {
        let chat = self.registry.get(&cmd.chat_id)?;
        if !chat.state.info.lock().unwrap().running {
            return Err(Error::Invalid("the agent has exited".into()));
        }
        chat.connection.send(&cmd.text)?;
        let id = format!("u{}", chat.state.transcript.lock().unwrap().events().len());
        chat.state.record(ChatEvent::User { id, text: cmd.text });
        Ok(())
    }
}

impl CommandHandler<RespondPermission> for ChatCommands {
    fn handle(&self, cmd: RespondPermission) -> Result<()> {
        let chat = self.registry.get(&cmd.chat_id)?;
        chat.connection.respond(&cmd.request_id, &cmd.decision)?;
        chat.state.record(ChatEvent::PermissionResolved {
            request_id: cmd.request_id,
            allowed: cmd.decision.allow,
        });
        Ok(())
    }
}

impl CommandHandler<InterruptChat> for ChatCommands {
    fn handle(&self, cmd: InterruptChat) -> Result<()> {
        self.registry.get(&cmd.chat_id)?.connection.interrupt()
    }
}

impl CommandHandler<SetChatPermissionMode> for ChatCommands {
    fn handle(&self, cmd: SetChatPermissionMode) -> Result<()> {
        self.registry
            .get(&cmd.chat_id)?
            .connection
            .set_permission_mode(cmd.mode)
    }
}

impl CommandHandler<SetChatModel> for ChatCommands {
    fn handle(&self, cmd: SetChatModel) -> Result<()> {
        self.registry
            .get(&cmd.chat_id)?
            .connection
            .set_model(&cmd.model)
    }
}

impl CommandHandler<CloseChat> for ChatCommands {
    fn handle(&self, cmd: CloseChat) -> Result<()> {
        if let Some(chat) = self.registry.chats.write().unwrap().remove(&cmd.chat_id) {
            chat.connection.kill();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::ChatEventHandler;
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

    /// Records what the app asked of the agent and lets tests emit events.
    #[derive(Default)]
    struct FakeRuntime {
        launches: Mutex<Vec<AgentLaunch>>,
        emit: Mutex<Option<ChatEventHandler>>,
        sent: Arc<Mutex<Vec<String>>>,
    }
    impl FakeRuntime {
        fn emit(&self, event: ChatEvent) {
            (self.emit.lock().unwrap().as_ref().unwrap())(event)
        }
    }
    struct FakeConnection(Arc<Mutex<Vec<String>>>);
    impl AgentConnection for FakeConnection {
        fn send(&self, text: &str) -> Result<()> {
            self.0.lock().unwrap().push(format!("send:{text}"));
            Ok(())
        }
        fn respond(&self, request_id: &str, d: &PermissionDecision) -> Result<()> {
            self.0
                .lock()
                .unwrap()
                .push(format!("respond:{request_id}:{}", d.allow));
            Ok(())
        }
        fn interrupt(&self) -> Result<()> {
            self.0.lock().unwrap().push("interrupt".into());
            Ok(())
        }
        fn set_permission_mode(&self, mode: PermissionMode) -> Result<()> {
            self.0
                .lock()
                .unwrap()
                .push(format!("mode:{}", mode.as_cli()));
            Ok(())
        }
        fn set_model(&self, model: &str) -> Result<()> {
            self.0.lock().unwrap().push(format!("model:{model}"));
            Ok(())
        }
        fn kill(&self) {
            self.0.lock().unwrap().push("kill".into());
        }
    }
    impl AgentRuntime for Arc<FakeRuntime> {
        fn start(
            &self,
            launch: AgentLaunch,
            events: ChatEventHandler,
        ) -> Result<Box<dyn AgentConnection>> {
            self.launches.lock().unwrap().push(launch);
            *self.emit.lock().unwrap() = Some(events);
            Ok(Box::new(FakeConnection(self.sent.clone())))
        }
    }

    #[derive(Default)]
    struct Collect(Mutex<Vec<ChatEvent>>);
    impl ChatSink for Collect {
        fn event(&self, event: &ChatEvent) {
            self.0.lock().unwrap().push(event.clone());
        }
    }

    fn setup() -> (ChatCommands, Arc<FakeRuntime>, Arc<ChatRegistry>) {
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
        let runtime = Arc::new(FakeRuntime::default());
        let registry = Arc::new(ChatRegistry::default());
        let mut runtimes: HashMap<AgentKind, Arc<dyn AgentRuntime>> = HashMap::new();
        runtimes.insert(AgentKind::Claude, Arc::new(runtime.clone()));
        let cmds = ChatCommands::new(
            Arc::new(Sessions(session)),
            Arc::new(FakeShell),
            runtimes,
            registry.clone(),
            AgentProfile::builtin(),
        );
        (cmds, runtime, registry)
    }

    fn start(cmds: &ChatCommands, profile: &str, sink: Arc<Collect>) -> Result<ChatInfo> {
        cmds.handle(WithChatSink {
            command: StartChat {
                worktree_id: "wt".into(),
                profile_id: profile.into(),
                resume: None,
                model: None,
                permission_mode: Some(PermissionMode::AcceptEdits),
            },
            sink,
        })
    }

    #[test]
    fn starts_the_runtime_in_the_worktree() {
        let (cmds, runtime, registry) = setup();
        let info = start(&cmds, "claude", Arc::new(Collect::default())).unwrap();
        let launch = runtime.launches.lock().unwrap()[0].clone();
        assert_eq!(launch.cwd, PathBuf::from("/repo"));
        assert_eq!(launch.program, PathBuf::from("/usr/local/bin/claude"));
        assert_eq!(launch.permission_mode, PermissionMode::AcceptEdits);
        assert_eq!(info.title, "Claude Code");
        assert_eq!(registry.list("wt").len(), 1);
    }

    #[test]
    fn terminal_only_profiles_are_rejected() {
        let (cmds, _, _) = setup();
        let err = start(&cmds, "shell", Arc::new(Collect::default())).unwrap_err();
        assert!(err.to_string().contains("can't run as a chat"));
    }

    #[test]
    fn sending_records_the_user_turn_and_streams_events() {
        let (cmds, runtime, registry) = setup();
        let sink = Arc::new(Collect::default());
        let info = start(&cmds, "claude", sink.clone()).unwrap();
        cmds.handle(SendChatMessage {
            chat_id: info.id.clone(),
            text: "hello".into(),
        })
        .unwrap();
        assert_eq!(*runtime.sent.lock().unwrap(), ["send:hello"]);
        assert!(registry.list("wt")[0].busy);

        runtime.emit(ChatEvent::TurnComplete {
            is_error: false,
            result: None,
            duration_ms: None,
            cost_usd: Some(0.1),
            input_tokens: None,
            output_tokens: None,
        });
        assert!(!registry.list("wt")[0].busy);
        assert_eq!(sink.0.lock().unwrap().len(), 2);
    }

    #[test]
    fn attaching_returns_the_transcript_and_moves_the_stream() {
        let (cmds, runtime, _) = setup();
        let first = Arc::new(Collect::default());
        let info = start(&cmds, "claude", first.clone()).unwrap();
        runtime.emit(ChatEvent::BlockDelta {
            message_id: "m".into(),
            index: 0,
            text: "Hi".into(),
        });

        let second = Arc::new(Collect::default());
        let snapshot = cmds
            .handle(WithChatSink {
                command: AttachChat { chat_id: info.id },
                sink: second.clone(),
            })
            .unwrap();
        assert_eq!(snapshot.events.len(), 1);

        runtime.emit(ChatEvent::Exited { code: Some(0) });
        assert_eq!(first.0.lock().unwrap().len(), 1);
        assert_eq!(second.0.lock().unwrap().len(), 1);
        assert!(!snapshot.info.busy);
    }

    #[test]
    fn controls_reach_the_connection() {
        let (cmds, runtime, _) = setup();
        let info = start(&cmds, "claude", Arc::new(Collect::default())).unwrap();
        let id = info.id;
        cmds.handle(RespondPermission {
            chat_id: id.clone(),
            request_id: "r1".into(),
            decision: PermissionDecision {
                allow: true,
                remember: false,
                updated_input: None,
                message: None,
            },
        })
        .unwrap();
        cmds.handle(InterruptChat {
            chat_id: id.clone(),
        })
        .unwrap();
        cmds.handle(SetChatPermissionMode {
            chat_id: id.clone(),
            mode: PermissionMode::Plan,
        })
        .unwrap();
        cmds.handle(SetChatModel {
            chat_id: id.clone(),
            model: "opus".into(),
        })
        .unwrap();
        cmds.handle(CloseChat { chat_id: id }).unwrap();
        assert_eq!(
            *runtime.sent.lock().unwrap(),
            [
                "respond:r1:true",
                "interrupt",
                "mode:plan",
                "model:opus",
                "kill"
            ]
        );
    }
}
