//! The tools a lead agent gets, each a thin call into Ensombl's API with
//! answers phrased for an agent to read.

use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::Backend;

pub const INSTRUCTIONS: &str = "Ensombl runs coding agents on tasks, each alone in its own git worktree and branch. Split work into independent tasks with create_task (use depends_on when one needs another merged first), then wait_for_task. When a task is in review, read its summary and diff, send_follow_up for changes, and approve_task to merge it. Tasks that need input are waiting on a permission: answer them with answer_task.";

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": { "type": "object", "properties": properties, "required": required },
    })
}

fn id_only(name: &str, description: &str) -> Value {
    tool(
        name,
        description,
        json!({ "task_id": { "type": "string" } }),
        &["task_id"],
    )
}

pub fn definitions() -> Vec<Value> {
    vec![
        tool(
            "list_projects",
            "Projects (git repositories) open in Ensombl, with their ids and default branches.",
            json!({}),
            &[],
        ),
        tool(
            "create_task",
            "Hand a piece of work to an agent. It runs in its own worktree on a new branch and lands in review when done.",
            json!({
                "project_id": { "type": "string" },
                "title": { "type": "string", "description": "Short name, becomes the branch name" },
                "prompt": { "type": "string", "description": "Everything the agent needs: goal, constraints, how to verify" },
                "base_branch": { "type": "string", "description": "Branch to start from and merge into; defaults to the project's checked-out branch" },
                "depends_on": { "type": "array", "items": { "type": "string" }, "description": "Task ids that must be merged first" },
                "permission": { "type": "string", "enum": ["acceptEdits", "default", "bypassPermissions", "plan"], "description": "acceptEdits (default) lets it edit files but asks before commands" },
            }),
            &["project_id", "title", "prompt"],
        ),
        tool(
            "list_tasks",
            "Tasks and their statuses.",
            json!({ "project_id": { "type": "string" } }),
            &[],
        ),
        id_only(
            "get_task",
            "A task's status, summary, error, and the permission it is waiting on, if any.",
        ),
        tool(
            "wait_for_task",
            "Wait until a task stops running (review, needs input, failed, done or cancelled), then report it.",
            json!({ "task_id": { "type": "string" }, "timeout_seconds": { "type": "number", "description": "Default 600" } }),
            &["task_id"],
        ),
        id_only(
            "get_task_activity",
            "What the agent did in its latest run: messages, tools used, errors and result.",
        ),
        id_only(
            "get_task_diff",
            "The task branch's committed changes against its base, as a diff.",
        ),
        tool(
            "send_follow_up",
            "Ask a task in review for changes; the agent continues its session.",
            json!({ "task_id": { "type": "string" }, "prompt": { "type": "string" } }),
            &["task_id", "prompt"],
        ),
        tool(
            "answer_task",
            "Allow or deny the permission a task is waiting on.",
            json!({ "task_id": { "type": "string" }, "allow": { "type": "boolean" }, "message": { "type": "string", "description": "Why, when denying" } }),
            &["task_id", "allow"],
        ),
        id_only(
            "approve_task",
            "Merge a reviewed task into its base branch and clean up its worktree.",
        ),
        id_only("cancel_task", "Stop a task."),
        id_only("retry_task", "Run a failed or cancelled task again."),
    ]
}

fn arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("{key} is required"))
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}

/// The parts of a task an agent cares about.
fn describe(task: &Value) -> Value {
    json!({
        "id": task["id"],
        "title": task["title"],
        "status": task["status"],
        "branch": task["branch"],
        "base_branch": task["baseBranch"],
        "depends_on": task["dependsOn"],
        "summary": task["summary"],
        "error": task["error"],
    })
}

fn task_report(detail: &Value) -> String {
    let mut report = describe(&detail["task"]);
    if !detail["pending"].is_null() {
        report["waiting_on"] = json!({
            "tool": detail["pending"]["toolName"],
            "input": detail["pending"]["input"],
            "description": detail["pending"]["description"],
        });
    }
    if let Some(run) = detail["runs"].as_array().and_then(|r| r.last()) {
        report["last_run_cost_usd"] = run["costUsd"].clone();
    }
    pretty(&report)
}

/// A run log condensed to what happened, one line each.
pub fn activity(events: &[Value]) -> String {
    let mut lines = Vec::new();
    for event in events {
        match event["type"].as_str().unwrap_or_default() {
            "assistant" => {
                for block in event["blocks"].as_array().into_iter().flatten() {
                    match block["type"].as_str() {
                        Some("text") => lines.push(
                            block["text"]
                                .as_str()
                                .unwrap_or_default()
                                .trim()
                                .to_string(),
                        ),
                        Some("toolUse") => {
                            let input = &block["input"];
                            let target = ["file_path", "command", "pattern", "path", "url"]
                                .iter()
                                .find_map(|k| input[*k].as_str())
                                .unwrap_or_default();
                            lines.push(format!(
                                "> {} {target}",
                                block["name"].as_str().unwrap_or("tool")
                            ));
                        }
                        _ => {}
                    }
                }
            }
            "toolResult" if event["isError"] == true => {
                let text: String = event["content"]
                    .as_str()
                    .unwrap_or_default()
                    .chars()
                    .take(400)
                    .collect();
                lines.push(format!("! {text}"));
            }
            "permission" => lines.push(format!(
                "? asked to use {}",
                event["toolName"].as_str().unwrap_or("a tool")
            )),
            "turnComplete" => lines.push(format!(
                "= {}: {}",
                if event["isError"] == true {
                    "failed"
                } else {
                    "finished"
                },
                event["result"].as_str().unwrap_or_default()
            )),
            "notice" => lines.push(format!("! {}", event["text"].as_str().unwrap_or_default())),
            _ => {}
        }
    }
    lines.retain(|l| !l.is_empty());
    if lines.is_empty() {
        "Nothing yet.".into()
    } else {
        lines.join("\n")
    }
}

fn get_task(backend: &dyn Backend, id: &str) -> Result<Value, String> {
    backend.call("get_task", json!({ "taskId": id }))
}

pub fn call(backend: &dyn Backend, name: &str, args: &Value) -> Result<String, String> {
    match name {
        "list_projects" => {
            let session = backend.call("get_session", Value::Null)?;
            let mut projects = Vec::new();
            for group in session["groups"].as_array().into_iter().flatten() {
                let project = &group["project"];
                let id = project["id"].as_str().unwrap_or_default();
                let branch = backend
                    .call("list_worktrees", json!({ "projectId": id }))
                    .ok()
                    .and_then(|w| {
                        w.as_array()?
                            .iter()
                            .find(|l| l["worktree"]["isMain"] == true)
                            .map(|l| l["worktree"]["branch"].clone())
                    })
                    .unwrap_or(Value::Null);
                projects.push(json!({ "id": id, "name": project["name"], "root": project["root"], "default_branch": branch }));
            }
            Ok(pretty(&Value::Array(projects)))
        }
        "create_task" => {
            let project_id = arg(args, "project_id")?;
            let base = match args["base_branch"].as_str() {
                Some(b) if !b.is_empty() => b.to_string(),
                _ => backend
                    .call("list_worktrees", json!({ "projectId": project_id }))?
                    .as_array()
                    .and_then(|w| w.iter().find(|l| l["worktree"]["isMain"] == true))
                    .and_then(|l| l["worktree"]["branch"].as_str().map(String::from))
                    .ok_or("couldn't tell the project's branch; pass base_branch")?,
            };
            let task = backend.call(
                "create_task",
                json!({
                    "projectId": project_id,
                    "title": arg(args, "title")?,
                    "prompt": arg(args, "prompt")?,
                    "agent": "claude",
                    "baseBranch": base,
                    "dependsOn": args["depends_on"].as_array().cloned().unwrap_or_default(),
                    "permission": args["permission"].as_str().unwrap_or("acceptEdits"),
                    "queue": true,
                }),
            )?;
            Ok(pretty(&describe(&task)))
        }
        "list_tasks" => {
            let input = match args["project_id"].as_str() {
                Some(p) => json!({ "projectId": p }),
                None => json!({}),
            };
            let tasks = backend.call("list_tasks", input)?;
            Ok(pretty(&Value::Array(
                tasks
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(describe)
                    .collect(),
            )))
        }
        "get_task" => Ok(task_report(&get_task(backend, arg(args, "task_id")?)?)),
        "wait_for_task" => {
            let id = arg(args, "task_id")?;
            let timeout = Duration::from_secs_f64(
                args["timeout_seconds"]
                    .as_f64()
                    .unwrap_or(600.0)
                    .clamp(1.0, 3600.0),
            );
            let deadline = Instant::now() + timeout;
            loop {
                let detail = get_task(backend, id)?;
                let status = detail["task"]["status"].as_str().unwrap_or_default();
                if !matches!(status, "queued" | "running") {
                    return Ok(task_report(&detail));
                }
                if Instant::now() >= deadline {
                    return Ok(format!(
                        "Still {status} after {}s.\n{}",
                        timeout.as_secs(),
                        task_report(&detail)
                    ));
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        }
        "get_task_activity" => {
            let detail = get_task(backend, arg(args, "task_id")?)?;
            let Some(run) = detail["runs"].as_array().and_then(|r| r.last()) else {
                return Ok("The task hasn't run yet.".into());
            };
            let log = backend.call("get_run_log", json!({ "runId": run["id"] }))?;
            Ok(activity(
                log.as_array().map(Vec::as_slice).unwrap_or_default(),
            ))
        }
        "get_task_diff" => {
            let diff = backend.call("get_task_diff", json!({ "taskId": arg(args, "task_id")? }))?;
            let diff = diff.as_str().unwrap_or_default();
            Ok(if diff.trim().is_empty() {
                "No committed changes.".into()
            } else {
                diff.to_string()
            })
        }
        "send_follow_up" => {
            let task = backend.call(
                "send_follow_up",
                json!({ "taskId": arg(args, "task_id")?, "prompt": arg(args, "prompt")? }),
            )?;
            Ok(pretty(&describe(&task)))
        }
        "answer_task" => {
            let id = arg(args, "task_id")?;
            let detail = get_task(backend, id)?;
            let request = detail["pending"]["requestId"]
                .as_str()
                .ok_or("that task isn't waiting on anything")?;
            let allow = args["allow"].as_bool().ok_or("allow is required")?;
            backend.call(
                "respond_to_task",
                json!({
                    "taskId": id,
                    "requestId": request,
                    "decision": { "allow": allow, "remember": false, "message": args["message"] },
                }),
            )?;
            Ok(if allow {
                "Allowed.".into()
            } else {
                "Denied.".into()
            })
        }
        "list_workflows" => {
            let workflows = backend.call(
                "list_workflows",
                json!({ "projectId": arg(args, "project_id")? }),
            )?;
            Ok(pretty(&workflows))
        }
        "start_workflow" => {
            let project_id = arg(args, "project_id")?;
            let base = match args["base_branch"].as_str() {
                Some(b) if !b.is_empty() => b.to_string(),
                _ => backend
                    .call("list_worktrees", json!({ "projectId": project_id }))?
                    .as_array()
                    .and_then(|w| w.iter().find(|l| l["worktree"]["isMain"] == true))
                    .and_then(|l| l["worktree"]["branch"].as_str().map(String::from))
                    .ok_or("couldn't tell the project's branch; pass base_branch")?,
            };
            let tasks = backend.call(
                "start_workflow",
                json!({ "projectId": project_id, "workflow": arg(args, "workflow")?, "input": arg(args, "input")?, "baseBranch": base }),
            )?;
            Ok(pretty(&Value::Array(
                tasks
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(describe)
                    .collect(),
            )))
        }
        "approve_task" | "cancel_task" | "retry_task" => {
            let task = backend.call(name, json!({ "taskId": arg(args, "task_id")? }))?;
            Ok(pretty(&describe(&task)))
        }
        other => Err(format!("unknown tool {other}")),
    }
}
