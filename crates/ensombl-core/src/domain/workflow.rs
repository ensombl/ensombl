//! Workflows: reusable pipelines of steps kept in the repo under
//! `.ensombl/workflows/*.yml`. Running one expands it into dependent tasks.
//!
//! ```yaml
//! name: Feature
//! description: Plan, build in parallel, then review
//! steps:
//!   - id: plan
//!     prompt: Write PLAN.md for {{input}}
//!     auto_approve: true
//!   - id: build
//!     title: Build {{item}}
//!     prompt: Implement the {{item}} part of PLAN.md. {{deps}}
//!     matrix: [api, web]
//!   - id: review
//!     prompt: Review the changes and fix anything broken.
//! ```
//!
//! Steps run after the previous step unless `after` names others. `{{input}}`
//! is what you typed when starting the run, `{{item}}` the matrix entry, and
//! `{{deps}}` what the tasks it waits on reported (filled in when it starts).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::agent_run::PermissionMode;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workflow {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    pub prompt: String,
    #[serde(default)]
    pub permission: Option<PermissionMode>,
    /// Merge as soon as it finishes, so the pipeline keeps moving.
    #[serde(default)]
    pub auto_approve: bool,
    /// One task per entry, run in parallel.
    #[serde(default)]
    pub matrix: Vec<String>,
    /// Steps to wait for; the previous step when absent.
    #[serde(default)]
    pub after: Option<Vec<String>>,
}

/// Which workflow run a task belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WorkflowRef {
    pub run_id: String,
    pub name: String,
    pub step: String,
}

/// One task a run will create, before it has an id.
#[derive(Debug, Clone, PartialEq)]
pub struct PlannedTask {
    pub step: String,
    pub title: String,
    pub prompt: String,
    pub permission: PermissionMode,
    pub auto_approve: bool,
    /// Indexes into the plan of the tasks this waits for.
    pub depends_on: Vec<usize>,
}

pub fn parse(yaml: &str) -> Result<Workflow, String> {
    let workflow: Workflow = serde_yaml_ng::from_str(yaml).map_err(|e| e.to_string())?;
    validate(&workflow)?;
    Ok(workflow)
}

fn validate(workflow: &Workflow) -> Result<(), String> {
    if workflow.steps.is_empty() {
        return Err("a workflow needs at least one step".into());
    }
    let mut seen: Vec<&str> = Vec::new();
    for step in &workflow.steps {
        if step.id.trim().is_empty() {
            return Err("every step needs an id".into());
        }
        if seen.contains(&step.id.as_str()) {
            return Err(format!("step id {} is used twice", step.id));
        }
        for dep in step.after.iter().flatten() {
            if !seen.contains(&dep.as_str()) {
                return Err(format!(
                    "step {} waits for {dep}, which isn't an earlier step",
                    step.id
                ));
            }
        }
        seen.push(&step.id);
    }
    Ok(())
}

fn render(template: &str, input: &str, item: Option<&str>) -> String {
    let text = template.replace("{{input}}", input.trim());
    match item {
        Some(item) => text.replace("{{item}}", item),
        None => text.replace("{{item}}", ""),
    }
}

/// The tasks a run of `workflow` with `input` creates, in order.
pub fn expand(workflow: &Workflow, input: &str) -> Vec<PlannedTask> {
    let mut plan: Vec<PlannedTask> = Vec::new();
    // Where each step's tasks landed in the plan.
    let mut placed: Vec<(String, Vec<usize>)> = Vec::new();
    for step in &workflow.steps {
        let waits_for: Vec<usize> = match &step.after {
            Some(ids) => ids
                .iter()
                .flat_map(|id| {
                    placed
                        .iter()
                        .find(|(s, _)| s == id)
                        .map(|(_, at)| at.clone())
                        .unwrap_or_default()
                })
                .collect(),
            None => placed.last().map(|(_, at)| at.clone()).unwrap_or_default(),
        };
        let items: Vec<Option<&str>> = if step.matrix.is_empty() {
            vec![None]
        } else {
            step.matrix.iter().map(|i| Some(i.as_str())).collect()
        };
        let mut at = Vec::new();
        for item in items {
            let title = match &step.title {
                Some(t) => render(t, input, item),
                None => match item {
                    Some(item) => format!("{}: {} ({item})", workflow.name, step.id),
                    None => format!("{}: {}", workflow.name, step.id),
                },
            };
            at.push(plan.len());
            plan.push(PlannedTask {
                step: step.id.clone(),
                title,
                prompt: render(&step.prompt, input, item),
                permission: step.permission.unwrap_or(PermissionMode::AcceptEdits),
                auto_approve: step.auto_approve,
                depends_on: waits_for.clone(),
            });
        }
        placed.push((step.id.clone(), at));
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEATURE: &str = r#"
name: Feature
description: Plan, build in parallel, review
steps:
  - id: plan
    prompt: Write PLAN.md for {{input}}
    auto_approve: true
  - id: build
    title: Build {{item}}
    prompt: Implement the {{item}} part. {{deps}}
    matrix: [api, web]
  - id: review
    prompt: Review it.
    permission: default
  - id: docs
    prompt: Document {{input}}
    after: [plan]
"#;

    #[test]
    fn expands_steps_into_a_task_graph() {
        let plan = expand(&parse(FEATURE).unwrap(), " rate limits ");
        let titles: Vec<_> = plan.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "Feature: plan",
                "Build api",
                "Build web",
                "Feature: review",
                "Feature: docs"
            ]
        );
        assert_eq!(plan[0].prompt, "Write PLAN.md for rate limits");
        assert!(plan[0].auto_approve);
        assert_eq!(plan[1].prompt, "Implement the api part. {{deps}}");
        assert_eq!(plan[1].depends_on, [0]);
        assert_eq!(plan[2].depends_on, [0]);
        // The step after a matrix waits for all of it.
        assert_eq!(plan[3].depends_on, [1, 2]);
        assert_eq!(plan[3].permission, PermissionMode::Default);
        // `after` reaches back past the previous step.
        assert_eq!(plan[4].depends_on, [0]);
    }

    #[test]
    fn rejects_broken_workflows() {
        assert!(
            parse("name: x\nsteps: []")
                .unwrap_err()
                .contains("at least one step")
        );
        let twice = "name: x\nsteps:\n  - {id: a, prompt: p}\n  - {id: a, prompt: q}";
        assert!(parse(twice).unwrap_err().contains("used twice"));
        let forward = "name: x\nsteps:\n  - {id: a, prompt: p, after: [b]}\n  - {id: b, prompt: q}";
        assert!(
            parse(forward)
                .unwrap_err()
                .contains("isn't an earlier step")
        );
        let typo = "name: x\nsteps:\n  - {id: a, prompt: p, auto_aprove: true}";
        assert!(parse(typo).unwrap_err().contains("unknown field"));
    }
}
