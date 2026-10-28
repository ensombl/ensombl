//! [`AgentRuntime`] for Claude Code, driven through its stream-json protocol
//! (`claude -p --input-format stream-json --output-format stream-json`), the
//! same one the Agent SDK uses. Permission prompts come back as
//! `can_use_tool` control requests that the user answers in the UI.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use crate::application::ports::{AgentConnection, AgentLaunch, AgentRuntime, ChatEventHandler};
use crate::application::{Error, Result};
use crate::domain::chat::{
    ChatEvent, ContentBlock, NoticeLevel, PermissionDecision, PermissionMode,
};

const MAX_TOOL_OUTPUT: usize = 100_000;
const STDERR_TAIL: usize = 4_000;

pub struct ClaudeCode;

/// What one protocol line turns into.
#[derive(Debug, PartialEq)]
enum Parsed {
    Event(ChatEvent),
    /// A permission request whose input and suggestions we need when answering.
    Pending {
        request_id: String,
        input: Value,
        suggestions: Value,
    },
    /// A control request we don't handle; it still needs an answer.
    Unsupported {
        request_id: String,
    },
}

/// Turns stream-json lines into chat events. Stream events only carry block
/// indexes, so it remembers the current message per (sub)agent.
#[derive(Default)]
struct Parser {
    current: HashMap<Option<String>, String>,
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn content_block(v: &Value) -> Option<ContentBlock> {
    match v.get("type")?.as_str()? {
        "text" => Some(ContentBlock::Text {
            text: str_field(v, "text").unwrap_or_default(),
        }),
        "thinking" => Some(ContentBlock::Thinking {
            text: str_field(v, "thinking").unwrap_or_default(),
        }),
        "tool_use" => Some(ContentBlock::ToolUse {
            id: str_field(v, "id")?,
            name: str_field(v, "name")?,
            input: v.get("input").cloned().unwrap_or(Value::Null),
        }),
        _ => None,
    }
}

/// Tool results are a string or a list of text/image blocks.
fn tool_output(content: Option<&Value>) -> String {
    let mut text = match content {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .map(|p| match p.get("type").and_then(Value::as_str) {
                Some("text") => str_field(p, "text").unwrap_or_default(),
                Some(other) => format!("[{other}]"),
                None => String::new(),
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    };
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

impl Parser {
    fn parse(&mut self, line: &str) -> Vec<Parsed> {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            return vec![];
        };
        let parent = str_field(&v, "parent_tool_use_id");
        let event = |e| vec![Parsed::Event(e)];
        match v.get("type").and_then(Value::as_str) {
            Some("system") => match v.get("subtype").and_then(Value::as_str) {
                Some("init") => match str_field(&v, "session_id") {
                    Some(session_id) => event(ChatEvent::Session {
                        session_id,
                        model: str_field(&v, "model"),
                        permission_mode: v
                            .get("permissionMode")
                            .and_then(|m| serde_json::from_value(m.clone()).ok()),
                    }),
                    None => vec![],
                },
                Some("compact_boundary") => event(ChatEvent::Notice {
                    level: NoticeLevel::Info,
                    text: "Conversation compacted".into(),
                }),
                _ => vec![],
            },
            Some("stream_event") => self.stream_event(&v["event"], parent),
            Some("assistant") => {
                let message = &v["message"];
                let Some(message_id) = str_field(message, "id") else {
                    return vec![];
                };
                let blocks = message["content"]
                    .as_array()
                    .map(|c| c.iter().filter_map(content_block).collect())
                    .unwrap_or_default();
                event(ChatEvent::Assistant {
                    message_id,
                    parent,
                    blocks,
                })
            }
            Some("user") => v["message"]["content"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|b| b.get("type").and_then(Value::as_str) == Some("tool_result"))
                .filter_map(|b| {
                    Some(Parsed::Event(ChatEvent::ToolResult {
                        tool_use_id: str_field(b, "tool_use_id")?,
                        content: tool_output(b.get("content")),
                        is_error: b.get("is_error").and_then(Value::as_bool).unwrap_or(false),
                    }))
                })
                .collect(),
            Some("result") => {
                let usage = &v["usage"];
                let tokens = |keys: &[&str]| {
                    let found: Vec<f64> = keys.iter().filter_map(|k| usage[k].as_f64()).collect();
                    (!found.is_empty()).then(|| found.iter().sum())
                };
                event(ChatEvent::TurnComplete {
                    is_error: v["is_error"].as_bool().unwrap_or(false),
                    result: str_field(&v, "result"),
                    duration_ms: v["duration_ms"].as_f64(),
                    cost_usd: v["total_cost_usd"].as_f64(),
                    input_tokens: tokens(&[
                        "input_tokens",
                        "cache_creation_input_tokens",
                        "cache_read_input_tokens",
                    ]),
                    output_tokens: tokens(&["output_tokens"]),
                })
            }
            Some("control_request") => {
                let request_id = str_field(&v, "request_id").unwrap_or_default();
                let request = &v["request"];
                if request["subtype"].as_str() != Some("can_use_tool") {
                    return vec![Parsed::Unsupported { request_id }];
                }
                let input = request.get("input").cloned().unwrap_or(json!({}));
                vec![
                    Parsed::Pending {
                        request_id: request_id.clone(),
                        input: input.clone(),
                        suggestions: request
                            .get("permission_suggestions")
                            .cloned()
                            .unwrap_or(Value::Null),
                    },
                    Parsed::Event(ChatEvent::Permission {
                        request_id,
                        tool_name: str_field(request, "tool_name").unwrap_or_default(),
                        tool_use_id: str_field(request, "tool_use_id"),
                        input,
                        description: str_field(request, "description"),
                    }),
                ]
            }
            Some("control_response") => Self::control_response(&v["response"]),
            _ => vec![],
        }
    }

    fn stream_event(&mut self, e: &Value, parent: Option<String>) -> Vec<Parsed> {
        match e.get("type").and_then(Value::as_str) {
            Some("message_start") => {
                if let Some(id) = str_field(&e["message"], "id") {
                    self.current.insert(parent, id);
                }
                vec![]
            }
            Some("content_block_start") => {
                let (Some(message_id), Some(index), Some(block)) = (
                    self.current.get(&parent).cloned(),
                    e["index"].as_u64(),
                    content_block(&e["content_block"]),
                ) else {
                    return vec![];
                };
                vec![Parsed::Event(ChatEvent::BlockStart {
                    message_id,
                    parent,
                    index: index as u32,
                    block,
                })]
            }
            Some("content_block_delta") => {
                let delta = &e["delta"];
                let text = match delta["type"].as_str() {
                    Some("text_delta") => str_field(delta, "text"),
                    Some("thinking_delta") => str_field(delta, "thinking"),
                    // Tool input arrives whole with the assistant message.
                    _ => None,
                };
                match (text, self.current.get(&parent), e["index"].as_u64()) {
                    (Some(text), Some(message_id), Some(index)) if !text.is_empty() => {
                        vec![Parsed::Event(ChatEvent::BlockDelta {
                            message_id: message_id.clone(),
                            index: index as u32,
                            text,
                        })]
                    }
                    _ => vec![],
                }
            }
            Some("message_delta") => match (
                self.current.get(&parent),
                e["usage"]["output_tokens"].as_f64(),
            ) {
                (Some(message_id), Some(output_tokens)) => {
                    vec![Parsed::Event(ChatEvent::Usage {
                        message_id: message_id.clone(),
                        output_tokens,
                    })]
                }
                _ => vec![],
            },
            _ => vec![],
        }
    }

    /// Answers to our own control requests; ids are `<kind>-<n>`.
    fn control_response(r: &Value) -> Vec<Parsed> {
        let id = r["request_id"].as_str().unwrap_or_default();
        if r["subtype"].as_str() == Some("error") {
            return vec![Parsed::Event(ChatEvent::Notice {
                level: NoticeLevel::Error,
                text: str_field(r, "error").unwrap_or_else(|| "Request failed".into()),
            })];
        }
        let body = &r["response"];
        if id.starts_with("mode-") {
            if let Some(mode) = body
                .get("mode")
                .and_then(|m| serde_json::from_value::<PermissionMode>(m.clone()).ok())
            {
                return vec![Parsed::Event(ChatEvent::ModeChanged { mode })];
            }
        } else if id.starts_with("model-")
            && let Some(model) = str_field(body, "model")
        {
            return vec![Parsed::Event(ChatEvent::ModelChanged { model })];
        }
        vec![]
    }
}

struct Pending {
    input: Value,
    suggestions: Value,
}

struct Connection {
    stdin: Mutex<Option<ChildStdin>>,
    child: Mutex<Child>,
    pending: Mutex<HashMap<String, Pending>>,
    next: AtomicU64,
}

impl Connection {
    fn write(&self, message: &Value) -> Result<()> {
        let mut stdin = self.stdin.lock().unwrap();
        let stdin = stdin
            .as_mut()
            .ok_or_else(|| Error::Invalid("the agent has exited".into()))?;
        writeln!(stdin, "{message}")?;
        stdin.flush()?;
        Ok(())
    }

    fn control(&self, kind: &str, request: Value) -> Result<()> {
        let n = self.next.fetch_add(1, Ordering::Relaxed);
        self.write(&json!({
            "type": "control_request",
            "request_id": format!("{kind}-{n}"),
            "request": request,
        }))
    }

    fn reply(&self, request_id: &str, response: Value) -> Result<()> {
        self.write(&json!({
            "type": "control_response",
            "response": { "subtype": "success", "request_id": request_id, "response": response },
        }))
    }

    /// Waits for exit without holding the child lock, so `kill` stays responsive.
    fn wait(&self) -> Option<i32> {
        loop {
            match self.child.lock().unwrap().try_wait() {
                Ok(Some(status)) => return status.code(),
                Ok(None) => {}
                Err(_) => return None,
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

impl AgentConnection for Arc<Connection> {
    fn send(&self, text: &str) -> Result<()> {
        self.write(&json!({
            "type": "user",
            "message": { "role": "user", "content": text },
            "parent_tool_use_id": null,
        }))
    }

    fn respond(&self, request_id: &str, decision: &PermissionDecision) -> Result<()> {
        let pending = self.pending.lock().unwrap().remove(request_id);
        let response = if decision.allow {
            let mut allow = json!({
                "behavior": "allow",
                "updatedInput": decision
                    .updated_input
                    .clone()
                    .or_else(|| pending.as_ref().map(|p| p.input.clone()))
                    .unwrap_or(json!({})),
            });
            if decision.remember
                && let Some(p) = pending.filter(|p| p.suggestions.is_array())
            {
                allow["updatedPermissions"] = p.suggestions;
            }
            allow
        } else {
            json!({
                "behavior": "deny",
                "message": decision.message.clone().unwrap_or_else(|| "The user declined this.".into()),
            })
        };
        self.reply(request_id, response)
    }

    fn interrupt(&self) -> Result<()> {
        self.control("interrupt", json!({ "subtype": "interrupt" }))
    }

    fn set_permission_mode(&self, mode: PermissionMode) -> Result<()> {
        self.control(
            "mode",
            json!({ "subtype": "set_permission_mode", "mode": mode.as_cli() }),
        )
    }

    fn set_model(&self, model: &str) -> Result<()> {
        self.control("model", json!({ "subtype": "set_model", "model": model }))
    }

    fn kill(&self) {
        self.stdin.lock().unwrap().take();
        let _ = self.child.lock().unwrap().kill();
    }
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

impl AgentRuntime for ClaudeCode {
    fn start(
        &self,
        launch: AgentLaunch,
        events: ChatEventHandler,
    ) -> Result<Box<dyn AgentConnection>> {
        let mut args: Vec<String> = [
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--include-partial-messages",
            "--permission-prompt-tool",
            "stdio",
            "--permission-mode",
            launch.permission_mode.as_cli(),
        ]
        .map(String::from)
        .into();
        if let Some(model) = launch.model {
            args.extend(["--model".into(), model]);
        }
        if let Some(session) = launch.resume {
            args.extend(["--resume".into(), session]);
        }
        args.extend(launch.args);

        let mut child = Command::new(&launch.program)
            .args(&args)
            .current_dir(&launch.cwd)
            .env_clear()
            .envs(launch.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::Invalid(format!("couldn't start claude: {e}")))?;
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let connection = Arc::new(Connection {
            stdin: Mutex::new(child.stdin.take()),
            child: Mutex::new(child),
            pending: Mutex::default(),
            next: AtomicU64::new(1),
        });

        let reader = connection.clone();
        std::thread::spawn(move || {
            let stderr = std::thread::spawn(move || read_tail(stderr));
            let mut parser = Parser::default();
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                for parsed in parser.parse(&line) {
                    match parsed {
                        Parsed::Event(event) => events(event),
                        Parsed::Pending {
                            request_id,
                            input,
                            suggestions,
                        } => {
                            reader
                                .pending
                                .lock()
                                .unwrap()
                                .insert(request_id, Pending { input, suggestions });
                        }
                        Parsed::Unsupported { request_id } => {
                            let _ = reader.write(&json!({
                                "type": "control_response",
                                "response": {
                                    "subtype": "error",
                                    "request_id": request_id,
                                    "error": "not supported by ensombl",
                                },
                            }));
                        }
                    }
                }
            }
            reader.stdin.lock().unwrap().take();
            let code = reader.wait();
            let tail = stderr.join().unwrap_or_default();
            if code != Some(0) && !tail.is_empty() {
                events(ChatEvent::Notice {
                    level: NoticeLevel::Error,
                    text: tail,
                });
            }
            events(ChatEvent::Exited { code });
        });
        Ok(Box::new(connection))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn events(parser: &mut Parser, lines: &[Value]) -> Vec<Parsed> {
        lines
            .iter()
            .flat_map(|l| parser.parse(&l.to_string()))
            .collect()
    }

    #[test]
    fn streams_blocks_of_the_current_message() {
        let mut p = Parser::default();
        let out = events(
            &mut p,
            &[
                json!({"type":"stream_event","parent_tool_use_id":null,"event":{"type":"message_start","message":{"id":"msg_1"}}}),
                json!({"type":"stream_event","parent_tool_use_id":null,"event":{"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}}),
                json!({"type":"stream_event","parent_tool_use_id":null,"event":{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Done."}}}),
                json!({"type":"stream_event","parent_tool_use_id":null,"event":{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{"}}}),
                json!({"type":"stream_event","parent_tool_use_id":null,"event":{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":42}}}),
            ],
        );
        assert_eq!(
            out,
            [
                Parsed::Event(ChatEvent::BlockStart {
                    message_id: "msg_1".into(),
                    parent: None,
                    index: 1,
                    block: ContentBlock::Text { text: "".into() },
                }),
                Parsed::Event(ChatEvent::BlockDelta {
                    message_id: "msg_1".into(),
                    index: 1,
                    text: "Done.".into(),
                }),
                Parsed::Event(ChatEvent::Usage {
                    message_id: "msg_1".into(),
                    output_tokens: 42.0,
                }),
            ]
        );
    }

    #[test]
    fn parses_assistant_messages_and_tool_results() {
        let mut p = Parser::default();
        let out = events(
            &mut p,
            &[
                json!({"type":"assistant","parent_tool_use_id":"toolu_task","message":{"id":"msg_2","content":[
                    {"type":"tool_use","id":"toolu_1","name":"Write","input":{"file_path":"/b.txt","content":"hi"}}
                ]}}),
                json!({"type":"user","message":{"role":"user","content":[
                    {"type":"tool_result","tool_use_id":"toolu_1","content":[{"type":"text","text":"ok"},{"type":"image"}],"is_error":true}
                ]}}),
            ],
        );
        assert_eq!(
            out,
            [
                Parsed::Event(ChatEvent::Assistant {
                    message_id: "msg_2".into(),
                    parent: Some("toolu_task".into()),
                    blocks: vec![ContentBlock::ToolUse {
                        id: "toolu_1".into(),
                        name: "Write".into(),
                        input: json!({"file_path":"/b.txt","content":"hi"}),
                    }],
                }),
                Parsed::Event(ChatEvent::ToolResult {
                    tool_use_id: "toolu_1".into(),
                    content: "ok\n[image]".into(),
                    is_error: true,
                }),
            ]
        );
    }

    #[test]
    fn permission_requests_keep_what_the_answer_needs() {
        let mut p = Parser::default();
        let out = events(
            &mut p,
            &[
                json!({"type":"control_request","request_id":"r1","request":{"subtype":"can_use_tool","tool_name":"Write","input":{"file_path":"b.txt"},"description":"b.txt","permission_suggestions":[{"type":"setMode","mode":"acceptEdits"}],"tool_use_id":"toolu_1"}}),
                json!({"type":"control_request","request_id":"r2","request":{"subtype":"hook_callback"}}),
            ],
        );
        assert_eq!(out.len(), 3);
        assert!(
            matches!(&out[0], Parsed::Pending { request_id, suggestions, .. } if request_id == "r1" && suggestions.is_array())
        );
        assert!(
            matches!(&out[1], Parsed::Event(ChatEvent::Permission { tool_name, tool_use_id: Some(t), .. }) if tool_name == "Write" && t == "toolu_1")
        );
        assert_eq!(
            out[2],
            Parsed::Unsupported {
                request_id: "r2".into()
            }
        );
    }

    #[test]
    fn session_results_and_control_responses() {
        let mut p = Parser::default();
        let out = events(
            &mut p,
            &[
                json!({"type":"system","subtype":"init","session_id":"s1","model":"claude-opus","permissionMode":"default"}),
                json!({"type":"result","is_error":false,"result":"Done","duration_ms":4213,"total_cost_usd":0.5,"usage":{"input_tokens":4,"cache_read_input_tokens":10,"output_tokens":7}}),
                json!({"type":"control_response","response":{"subtype":"success","request_id":"mode-1","response":{"mode":"acceptEdits"}}}),
                json!({"type":"control_response","response":{"subtype":"success","request_id":"model-2","response":{"model":"claude-sonnet"}}}),
                json!({"type":"control_response","response":{"subtype":"error","request_id":"interrupt-3","error":"nothing to interrupt"}}),
            ],
        );
        assert_eq!(
            out,
            [
                Parsed::Event(ChatEvent::Session {
                    session_id: "s1".into(),
                    model: Some("claude-opus".into()),
                    permission_mode: Some(PermissionMode::Default),
                }),
                Parsed::Event(ChatEvent::TurnComplete {
                    is_error: false,
                    result: Some("Done".into()),
                    duration_ms: Some(4213.0),
                    cost_usd: Some(0.5),
                    input_tokens: Some(14.0),
                    output_tokens: Some(7.0),
                }),
                Parsed::Event(ChatEvent::ModeChanged {
                    mode: PermissionMode::AcceptEdits
                }),
                Parsed::Event(ChatEvent::ModelChanged {
                    model: "claude-sonnet".into()
                }),
                Parsed::Event(ChatEvent::Notice {
                    level: NoticeLevel::Error,
                    text: "nothing to interrupt".into()
                }),
            ]
        );
    }

    #[test]
    fn long_tool_output_is_truncated_on_a_char_boundary() {
        let long = "é".repeat(MAX_TOOL_OUTPUT);
        let out = tool_output(Some(&Value::String(long)));
        assert!(out.ends_with("(truncated)"));
        assert!(out.len() < MAX_TOOL_OUTPUT + 20);
    }

    /// Talks to a fake `claude` script to check the process plumbing end to end.
    #[cfg(unix)]
    #[test]
    fn runs_a_session_over_pipes() {
        use std::os::unix::fs::PermissionsExt;
        use std::sync::mpsc;

        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("claude");
        std::fs::write(
            &script,
            r#"#!/bin/sh
echo '{"type":"system","subtype":"init","session_id":"s1","model":"m"}'
read line
case "$line" in *'"content":"hi"'*) ;; *) exit 3 ;; esac
echo '{"type":"assistant","message":{"id":"m1","content":[{"type":"text","text":"hello"}]}}'
echo '{"type":"result","is_error":false}'
"#,
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let conn = ClaudeCode
            .start(
                AgentLaunch {
                    program: script,
                    args: vec![],
                    cwd: dir.path().into(),
                    env: vec![("PATH".into(), "/usr/bin:/bin".into())],
                    resume: None,
                    model: None,
                    permission_mode: PermissionMode::Default,
                },
                Box::new(move |e| {
                    let _ = tx.lock().unwrap().send(e);
                }),
            )
            .unwrap();
        conn.send("hi").unwrap();
        let got: Vec<ChatEvent> = rx.iter().take(4).collect();
        assert!(matches!(got[0], ChatEvent::Session { .. }));
        assert!(matches!(&got[1], ChatEvent::Assistant { blocks, .. } if blocks.len() == 1));
        assert!(matches!(got[2], ChatEvent::TurnComplete { .. }));
        assert_eq!(got[3], ChatEvent::Exited { code: Some(0) });
    }

    /// Against the real CLI: `CRM_CLAUDE=$(which claude) cargo test claude_smoke -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn claude_smoke() {
        use std::sync::mpsc;
        let program = std::env::var("CRM_CLAUDE").expect("CRM_CLAUDE");
        let dir = tempfile::tempdir().unwrap();
        let env = std::env::vars()
            .filter(|(k, _)| k != "CLAUDECODE" && !k.starts_with("CLAUDE_CODE_"))
            .collect();
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        let conn = ClaudeCode
            .start(
                AgentLaunch {
                    program: program.into(),
                    args: vec![],
                    cwd: dir.path().into(),
                    env,
                    resume: None,
                    model: Some("haiku".into()),
                    permission_mode: PermissionMode::Default,
                },
                Box::new(move |e| {
                    let _ = tx.lock().unwrap().send(e);
                }),
            )
            .unwrap();
        conn.send("Create notes.txt containing 'hi' with the Write tool, then reply 'done'.")
            .unwrap();
        for event in rx.iter() {
            println!("{}", serde_json::to_string(&event).unwrap());
            match event {
                ChatEvent::Permission { request_id, .. } => conn
                    .respond(
                        &request_id,
                        &PermissionDecision {
                            allow: true,
                            remember: false,
                            updated_input: None,
                            message: None,
                        },
                    )
                    .unwrap(),
                ChatEvent::TurnComplete { is_error, .. } => {
                    assert!(!is_error);
                    break;
                }
                ChatEvent::Exited { .. } => panic!("exited early"),
                _ => {}
            }
        }
        assert_eq!(
            std::fs::read_to_string(dir.path().join("notes.txt"))
                .unwrap()
                .trim(),
            "hi"
        );
        conn.kill();
    }
}
