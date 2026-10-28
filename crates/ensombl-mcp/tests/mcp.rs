use std::sync::{Arc, Mutex};

use ensombl_core::app::{AppPaths, Core};
use ensombl_mcp::{Backend, HttpApi, handle, serve};
use serde_json::{Value, json};

/// Answers from a script and records what was called.
struct Fake(Mutex<Vec<(String, Value)>>);
impl Backend for Fake {
    fn call(&self, name: &str, input: Value) -> Result<Value, String> {
        self.0.lock().unwrap().push((name.into(), input.clone()));
        match name {
            "list_worktrees" => {
                Ok(json!([{ "worktree": { "isMain": true, "branch": "develop" } }]))
            }
            "create_task" => Ok(
                json!({ "id": "k1", "title": input["title"], "status": "running", "baseBranch": input["baseBranch"] }),
            ),
            "get_task" => Ok(json!({
                "task": { "id": "k1", "status": "needsInput" },
                "runs": [{ "id": "r1", "costUsd": 0.2 }],
                "pending": { "requestId": "q1", "toolName": "Bash", "input": { "command": "ls" } },
            })),
            "respond_to_task" => Ok(Value::Null),
            _ => Err(format!("unexpected {name}")),
        }
    }
}

fn call(backend: &dyn Backend, tool: &str, args: Value) -> Value {
    handle(backend, &json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": tool, "arguments": args } })).unwrap()
}

#[test]
fn speaks_mcp() {
    let fake = Fake(Mutex::default());
    let init = handle(&fake, &json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": { "protocolVersion": "2025-03-26" } })).unwrap();
    assert_eq!(init["result"]["protocolVersion"], "2025-03-26");
    assert_eq!(init["result"]["serverInfo"]["name"], "ensombl");
    assert!(
        handle(
            &fake,
            &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })
        )
        .is_none()
    );
    let list = handle(
        &fake,
        &json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
    )
    .unwrap();
    let names: Vec<_> = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"create_task") && names.contains(&"wait_for_task"));
    let unknown = handle(
        &fake,
        &json!({ "jsonrpc": "2.0", "id": 3, "method": "nope" }),
    )
    .unwrap();
    assert_eq!(unknown["error"]["code"], -32601);
}

#[test]
fn creating_a_task_defaults_to_the_projects_branch() {
    let fake = Fake(Mutex::default());
    let reply = call(
        &fake,
        "create_task",
        json!({ "project_id": "p", "title": "Add x", "prompt": "do x" }),
    );
    assert!(reply["result"]["isError"].is_null());
    let calls = fake.0.lock().unwrap();
    let (_, input) = calls.iter().find(|(n, _)| n == "create_task").unwrap();
    assert_eq!(input["baseBranch"], "develop");
    assert_eq!(input["permission"], "acceptEdits");
    assert_eq!(input["queue"], true);
}

#[test]
fn answers_the_pending_permission() {
    let fake = Fake(Mutex::default());
    let reply = call(
        &fake,
        "answer_task",
        json!({ "task_id": "k1", "allow": true }),
    );
    assert_eq!(reply["result"]["content"][0]["text"], "Allowed.");
    let responded = fake
        .0
        .lock()
        .unwrap()
        .iter()
        .find(|(n, _)| n == "respond_to_task")
        .map(|(_, i)| i.clone());
    assert_eq!(responded.unwrap()["requestId"], "q1");
    let report = call(&fake, "get_task", json!({ "task_id": "k1" }));
    assert!(
        report["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("waiting_on")
    );
}

#[test]
fn errors_become_tool_errors() {
    let fake = Fake(Mutex::default());
    let reply = call(&fake, "create_task", json!({ "title": "x" }));
    assert_eq!(reply["result"]["isError"], true);
    assert_eq!(
        reply["result"]["content"][0]["text"],
        "project_id is required"
    );
}

#[test]
fn condenses_run_logs() {
    let text = ensombl_mcp::tools::activity(&[
        json!({ "type": "assistant", "blocks": [{ "type": "text", "text": "Looking." }, { "type": "toolUse", "name": "Read", "input": { "file_path": "a.ts" } }] }),
        json!({ "type": "toolResult", "isError": true, "content": "no such file" }),
        json!({ "type": "turnComplete", "isError": false, "result": "Done" }),
    ]);
    assert_eq!(
        text,
        "Looking.\n> Read a.ts\n! no such file\n= finished: Done"
    );
}

#[test]
fn drives_a_real_server_over_stdio() {
    let dir = tempfile::tempdir().unwrap();
    let bus = ensombl_server::BroadcastBus::new();
    let core = Arc::new(
        Core::build(
            AppPaths {
                data_dir: dir.path().join("data"),
                worktrees_dir: dir.path().join("w"),
            },
            Arc::new(bus.clone()),
        )
        .unwrap(),
    );
    let file = dir.path().join("api.json");
    ensombl_server::spawn_local(core.handlers.clone(), &bus, &file).unwrap();
    let api = HttpApi::from_file(&file).unwrap();
    let input = [
        json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": "list_tasks", "arguments": {} } }),
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "get_task", "arguments": { "task_id": "ghost" } } }),
    ]
    .iter()
    .map(|m| m.to_string())
    .collect::<Vec<_>>()
    .join("\n");
    let mut out = Vec::new();
    serve(&api, input.as_bytes(), &mut out).unwrap();
    let replies: Vec<Value> = String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(replies[0]["result"]["content"][0]["text"], "[]");
    assert_eq!(replies[1]["result"]["isError"], true);
    assert_eq!(replies[1]["result"]["content"][0]["text"], "no task ghost");
    core.shutdown();
}
