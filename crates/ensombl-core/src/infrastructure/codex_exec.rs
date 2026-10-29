//! [`AgentRuntime`] for Codex, driven through `codex exec --json`. Each turn
//! is its own process; follow-ups resume the thread with `codex exec resume`.
//! Codex can't ask for permission headlessly, so the permission mode picks a
//! sandbox instead.

use std::collections::HashSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use crate::application::ports::{AgentConnection, AgentLaunch, AgentRuntime, RunEventHandler};
use crate::application::{Error, Result};
use crate::domain::agent_run::{
    ContentBlock, NoticeLevel, PermissionDecision, PermissionMode, RunEvent,
};

const MAX_TOOL_OUTPUT: usize = 100_000;
const STDERR_TAIL: usize = 4_000;

pub struct Codex;

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn truncate(mut text: String) -> String {
    if text.len() > MAX_TOOL_OUTPUT {
        let mut cut = MAX_TOOL_OUTPUT;
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        text.truncate(cut);
        text.push_str("\n… (truncated)");
    }
    text
}

/// The sandbox flags standing in for a permission mode.
fn sandbox_args(mode: PermissionMode) -> Vec<String> {
    match mode {
        PermissionMode::Plan => vec!["--sandbox".into(), "read-only".into()],
        PermissionMode::BypassPermissions => {
            vec!["--dangerously-bypass-approvals-and-sandbox".into()]
        }
        PermissionMode::Default | PermissionMode::AcceptEdits => {
            vec!["--sandbox".into(), "workspace-write".into()]
        }
    }
}

/// Turns `codex exec --json` lines into run events.
#[derive(Default)]
struct Parser {
    model: Option<String>,
    permission_mode: Option<PermissionMode>,
    /// Tool items already announced by `item.started`.
    started: HashSet<String>,
    /// The latest agent message, reported as the turn's result.
    last_message: Option<String>,
}

impl Parser {
    fn parse(&mut self, line: &str) -> Vec<RunEvent> {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            return vec![];
        };
        match v.get("type").and_then(Value::as_str) {
            Some("thread.started") => match str_field(&v, "thread_id") {
                Some(session_id) => vec![RunEvent::Session {
                    session_id,
                    model: self.model.clone(),
                    permission_mode: self.permission_mode,
                }],
                None => vec![],
            },
            Some("item.started") => self.item(&v["item"], false),
            Some("item.updated") => vec![],
            Some("item.completed") => self.item(&v["item"], true),
            Some("turn.completed") => {
                let usage = &v["usage"];
                let input = ["input_tokens", "cache_write_input_tokens"]
                    .iter()
                    .filter_map(|k| usage[k].as_f64())
                    .sum::<f64>();
                let output = ["output_tokens", "reasoning_output_tokens"]
                    .iter()
                    .filter_map(|k| usage[k].as_f64())
                    .sum::<f64>();
                vec![RunEvent::TurnComplete {
                    is_error: false,
                    result: self.last_message.take(),
                    duration_ms: None,
                    cost_usd: None,
                    input_tokens: usage.is_object().then_some(input),
                    output_tokens: usage.is_object().then_some(output),
                }]
            }
            Some("turn.failed") => vec![RunEvent::TurnComplete {
                is_error: true,
                result: str_field(&v["error"], "message")
                    .or_else(|| Some("Codex reported an error".into())),
                duration_ms: None,
                cost_usd: None,
                input_tokens: None,
                output_tokens: None,
            }],
            Some("error") => vec![RunEvent::Notice {
                level: NoticeLevel::Error,
                text: str_field(&v, "message").unwrap_or_else(|| "Codex reported an error".into()),
            }],
            _ => vec![],
        }
    }

    fn item(&mut self, item: &Value, completed: bool) -> Vec<RunEvent> {
        let Some(id) = str_field(item, "id") else {
            return vec![];
        };
        let kind = item.get("type").and_then(Value::as_str).unwrap_or_default();
        let text_block = |block: ContentBlock| {
            vec![RunEvent::Assistant {
                message_id: id.clone(),
                parent: None,
                blocks: vec![block],
            }]
        };
        match kind {
            "agent_message" if completed => {
                let text = str_field(item, "text").unwrap_or_default();
                self.last_message = Some(text.clone());
                text_block(ContentBlock::Text { text })
            }
            "reasoning" if completed => text_block(ContentBlock::Thinking {
                text: str_field(item, "text").unwrap_or_default(),
            }),
            "error" => vec![RunEvent::Notice {
                level: NoticeLevel::Error,
                text: str_field(item, "message").unwrap_or_default(),
            }],
            _ => {
                let Some((name, input)) = tool_call(kind, item) else {
                    return vec![];
                };
                let mut events = Vec::new();
                if self.started.insert(id.clone()) {
                    events.push(RunEvent::Assistant {
                        message_id: id.clone(),
                        parent: None,
                        blocks: vec![ContentBlock::ToolUse {
                            id: id.clone(),
                            name,
                            input,
                        }],
                    });
                }
                if completed {
                    self.started.remove(&id);
                    let (content, is_error) = tool_result(kind, item);
                    events.push(RunEvent::ToolResult {
                        tool_use_id: id,
                        content: truncate(content),
                        is_error,
                    });
                }
                events
            }
        }
    }
}

/// Codex's tool items as Claude-style tool calls, so views show them alike.
fn tool_call(kind: &str, item: &Value) -> Option<(String, Value)> {
    Some(match kind {
        "command_execution" => (
            "Bash".into(),
            json!({ "command": str_field(item, "command").unwrap_or_default() }),
        ),
        "file_change" => (
            "Edit".into(),
            json!({ "changes": item.get("changes").cloned().unwrap_or(Value::Null) }),
        ),
        "mcp_tool_call" => (
            format!(
                "mcp__{}__{}",
                str_field(item, "server").unwrap_or_default(),
                str_field(item, "tool").unwrap_or_default()
            ),
            item.get("arguments").cloned().unwrap_or(Value::Null),
        ),
        "web_search" => (
            "WebSearch".into(),
            json!({ "query": str_field(item, "query").unwrap_or_default() }),
        ),
        "todo_list" => (
            "TodoWrite".into(),
            json!({ "todos": item.get("items").cloned().unwrap_or(Value::Null) }),
        ),
        _ => return None,
    })
}

fn tool_result(kind: &str, item: &Value) -> (String, bool) {
    let failed = item.get("status").and_then(Value::as_str) == Some("failed");
    match kind {
        "command_execution" => {
            let code = item.get("exit_code").and_then(Value::as_i64);
            (
                str_field(item, "aggregated_output").unwrap_or_default(),
                failed || code.is_some_and(|c| c != 0),
            )
        }
        "file_change" => {
            let changes = item["changes"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|c| {
                    format!(
                        "{} {}",
                        str_field(c, "kind").unwrap_or_default(),
                        str_field(c, "path").unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            (changes, failed)
        }
        "mcp_tool_call" => match item.get("error").filter(|e| !e.is_null()) {
            Some(error) => (
                str_field(error, "message").unwrap_or_else(|| error.to_string()),
                true,
            ),
            None => (
                item.get("result")
                    .map(|r| r.to_string())
                    .unwrap_or_default(),
                failed,
            ),
        },
        _ => (String::new(), failed),
    }
}

struct Connection {
    launch: Mutex<AgentLaunch>,
    events: Arc<RunEventHandler>,
    /// The thread to resume on the next turn.
    session: Arc<Mutex<Option<String>>>,
    child: Arc<Mutex<Option<Child>>>,
}

fn read_tail(mut stderr: impl Read) -> String {
    let mut buf = Vec::new();
    let _ = stderr.read_to_end(&mut buf);
    let text = String::from_utf8_lossy(&buf);
    let start = text.len().saturating_sub(STDERR_TAIL);
    let start = (start..text.len())
        .find(|&i| text.is_char_boundary(i))
        .unwrap_or(text.len());
    text[start..].trim().to_string()
}

impl Connection {
    fn args(&self) -> Vec<String> {
        let launch = self.launch.lock().unwrap();
        let mut args: Vec<String> = vec!["exec".into(), "--json".into()];
        args.extend(sandbox_args(launch.permission_mode));
        if let Some(model) = &launch.model {
            args.extend(["--model".into(), model.clone()]);
        }
        args.extend(launch.args.iter().cloned());
        if let Some(session) = self.session.lock().unwrap().clone() {
            args.extend(["resume".into(), session]);
        }
        // The prompt comes on stdin.
        args.push("-".into());
        args
    }
}

impl AgentConnection for Connection {
    /// Starts a turn. One turn runs at a time.
    fn send(&self, text: &str) -> Result<()> {
        let mut slot = self.child.lock().unwrap();
        if let Some(child) = slot.as_mut()
            && child.try_wait().ok().flatten().is_none()
        {
            return Err(Error::Invalid(
                "Codex is still working on the last turn".into(),
            ));
        }
        let (program, cwd, env, model, mode) = {
            let l = self.launch.lock().unwrap();
            (
                l.program.clone(),
                l.cwd.clone(),
                l.env.clone(),
                l.model.clone(),
                l.permission_mode,
            )
        };
        let mut child = Command::new(&program)
            .args(self.args())
            .current_dir(&cwd)
            .env_clear()
            .envs(env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::Invalid(format!("couldn't start codex: {e}")))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
        }
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        *slot = Some(child);

        let events = self.events.clone();
        let session = self.session.clone();
        let child = self.child.clone();
        std::thread::spawn(move || {
            let stderr = std::thread::spawn(move || read_tail(stderr));
            let mut parser = Parser {
                model,
                permission_mode: Some(mode),
                ..Parser::default()
            };
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                for event in parser.parse(&line) {
                    if let RunEvent::Session { session_id, .. } = &event {
                        *session.lock().unwrap() = Some(session_id.clone());
                    }
                    events(event);
                }
            }
            // Polled so `kill` can take the lock meanwhile.
            let code = loop {
                match child.lock().unwrap().as_mut().map(Child::try_wait) {
                    Some(Ok(None)) => {}
                    Some(Ok(Some(status))) => break status.code(),
                    _ => break None,
                }
                std::thread::sleep(Duration::from_millis(20));
            };
            let tail = stderr.join().unwrap_or_default();
            if code != Some(0) && !tail.is_empty() {
                events(RunEvent::Notice {
                    level: NoticeLevel::Error,
                    text: tail,
                });
            }
            events(RunEvent::Exited { code });
        });
        Ok(())
    }

    fn respond(&self, _: &str, _: &PermissionDecision) -> Result<()> {
        Err(Error::Invalid(
            "Codex doesn't ask for permission when run headless".into(),
        ))
    }

    fn interrupt(&self) -> Result<()> {
        if let Some(child) = self.child.lock().unwrap().as_mut() {
            let _ = child.kill();
        }
        Ok(())
    }

    /// Applies from the next turn.
    fn set_permission_mode(&self, mode: PermissionMode) -> Result<()> {
        self.launch.lock().unwrap().permission_mode = mode;
        Ok(())
    }

    fn set_model(&self, model: &str) -> Result<()> {
        self.launch.lock().unwrap().model = Some(model.into());
        Ok(())
    }

    fn kill(&self) {
        if let Some(child) = self.child.lock().unwrap().as_mut() {
            let _ = child.kill();
        }
    }
}

impl AgentRuntime for Codex {
    fn start(
        &self,
        launch: AgentLaunch,
        events: RunEventHandler,
    ) -> Result<Box<dyn AgentConnection>> {
        let session = launch.resume.clone();
        Ok(Box::new(Connection {
            launch: Mutex::new(launch),
            events: Arc::new(events),
            session: Arc::new(Mutex::new(session)),
            child: Arc::default(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(lines: &[Value]) -> Vec<RunEvent> {
        let mut p = Parser {
            permission_mode: Some(PermissionMode::AcceptEdits),
            ..Parser::default()
        };
        lines.iter().flat_map(|l| p.parse(&l.to_string())).collect()
    }

    #[test]
    fn turns_a_codex_turn_into_run_events() {
        let out = parse(&[
            json!({"type":"thread.started","thread_id":"t1"}),
            json!({"type":"turn.started"}),
            json!({"type":"item.completed","item":{"id":"item_0","type":"reasoning","text":"Thinking"}}),
            json!({"type":"item.started","item":{"id":"item_1","type":"command_execution","command":"ls","aggregated_output":"","exit_code":null,"status":"in_progress"}}),
            json!({"type":"item.completed","item":{"id":"item_1","type":"command_execution","command":"ls","aggregated_output":"a.txt\n","exit_code":0,"status":"completed"}}),
            json!({"type":"item.completed","item":{"id":"item_2","type":"file_change","changes":[{"path":"notes.txt","kind":"add"}],"status":"completed"}}),
            json!({"type":"item.completed","item":{"id":"item_3","type":"agent_message","text":"Done."}}),
            json!({"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":80,"cache_write_input_tokens":0,"output_tokens":7,"reasoning_output_tokens":3}}),
        ]);
        assert_eq!(
            out,
            [
                RunEvent::Session {
                    session_id: "t1".into(),
                    model: None,
                    permission_mode: Some(PermissionMode::AcceptEdits),
                },
                RunEvent::Assistant {
                    message_id: "item_0".into(),
                    parent: None,
                    blocks: vec![ContentBlock::Thinking {
                        text: "Thinking".into()
                    }],
                },
                RunEvent::Assistant {
                    message_id: "item_1".into(),
                    parent: None,
                    blocks: vec![ContentBlock::ToolUse {
                        id: "item_1".into(),
                        name: "Bash".into(),
                        input: json!({"command":"ls"}),
                    }],
                },
                RunEvent::ToolResult {
                    tool_use_id: "item_1".into(),
                    content: "a.txt\n".into(),
                    is_error: false,
                },
                RunEvent::Assistant {
                    message_id: "item_2".into(),
                    parent: None,
                    blocks: vec![ContentBlock::ToolUse {
                        id: "item_2".into(),
                        name: "Edit".into(),
                        input: json!({"changes":[{"path":"notes.txt","kind":"add"}]}),
                    }],
                },
                RunEvent::ToolResult {
                    tool_use_id: "item_2".into(),
                    content: "add notes.txt".into(),
                    is_error: false,
                },
                RunEvent::Assistant {
                    message_id: "item_3".into(),
                    parent: None,
                    blocks: vec![ContentBlock::Text {
                        text: "Done.".into()
                    }],
                },
                RunEvent::TurnComplete {
                    is_error: false,
                    result: Some("Done.".into()),
                    duration_ms: None,
                    cost_usd: None,
                    input_tokens: Some(100.0),
                    output_tokens: Some(10.0),
                },
            ]
        );
    }

    #[test]
    fn reports_failures() {
        let out = parse(&[
            json!({"type":"item.completed","item":{"id":"c","type":"command_execution","command":"false","aggregated_output":"","exit_code":1,"status":"failed"}}),
            json!({"type":"error","message":"stream disconnected"}),
            json!({"type":"turn.failed","error":{"message":"usage limit reached"}}),
        ]);
        assert!(matches!(
            &out[1],
            RunEvent::ToolResult { is_error: true, .. }
        ));
        assert_eq!(
            out[2],
            RunEvent::Notice {
                level: NoticeLevel::Error,
                text: "stream disconnected".into()
            }
        );
        assert!(matches!(
            &out[3],
            RunEvent::TurnComplete { is_error: true, result: Some(r), .. } if r == "usage limit reached"
        ));
    }

    fn launch(program: std::path::PathBuf, cwd: &std::path::Path) -> AgentLaunch {
        AgentLaunch {
            program,
            args: vec!["-c".into(), "x=1".into()],
            cwd: cwd.into(),
            env: vec![("PATH".into(), "/usr/bin:/bin".into())],
            resume: None,
            model: None,
            permission_mode: PermissionMode::AcceptEdits,
        }
    }

    /// A fake `codex` that logs its args and stdin, then prints one turn.
    #[cfg(unix)]
    #[test]
    fn runs_a_turn_per_process_and_resumes_the_thread() {
        use std::os::unix::fs::PermissionsExt;
        use std::sync::mpsc;
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("codex");
        std::fs::write(
            &script,
            r#"#!/bin/sh
echo "$@" >> args.log
cat >> stdin.log
echo
echo '{"type":"thread.started","thread_id":"t9"}'
echo '{"type":"item.completed","item":{"id":"i","type":"agent_message","text":"ok"}}'
echo '{"type":"turn.completed","usage":{"input_tokens":1,"output_tokens":1}}'
"#,
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let conn = Codex
            .start(
                launch(script, dir.path()),
                Box::new(move |e| {
                    let _ = tx.lock().unwrap().send(e);
                }),
            )
            .unwrap();
        conn.send("first").unwrap();
        let got: Vec<RunEvent> = rx.iter().take(4).collect();
        assert!(matches!(got[0], RunEvent::Session { .. }));
        assert!(matches!(got[2], RunEvent::TurnComplete { .. }));
        assert_eq!(got[3], RunEvent::Exited { code: Some(0) });

        conn.send("second").unwrap();
        let _: Vec<RunEvent> = rx.iter().take(4).collect();
        let args = std::fs::read_to_string(dir.path().join("args.log")).unwrap();
        let lines: Vec<&str> = args.lines().collect();
        assert_eq!(
            lines,
            [
                "exec --json --sandbox workspace-write -c x=1 -",
                "exec --json --sandbox workspace-write -c x=1 resume t9 -",
            ]
        );
        let stdin = std::fs::read_to_string(dir.path().join("stdin.log")).unwrap();
        assert_eq!(stdin, "firstsecond");
    }
}
