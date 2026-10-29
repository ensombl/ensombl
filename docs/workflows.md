# Workflows

A workflow is a pipeline of steps kept in the repo under `.ensombl/workflows/*.yml`. Running one, either with **Run workflow** on the Tasks board or through the `start_workflow` API or MCP tool, expands it into tasks that wait on each other.

```yaml
name: Feature
description: Plan, build in parallel, then review
steps:
  - id: plan
    prompt: Write PLAN.md for {{input}}
    auto_approve: true
  - id: build
    title: Build {{item}}
    prompt: Implement the {{item}} part of PLAN.md. {{deps}}
    matrix: [api, web]
  - id: review
    prompt: Review the changes and fix anything broken.
    agent: codex
```

## Steps

| Field | |
| --- | --- |
| `id` | Required and unique |
| `prompt` | Required. What the task's agent is told |
| `title` | The task's title; defaults to `<workflow>: <id>` |
| `agent` | `claude` (default) or `codex` |
| `permission` | `acceptEdits` (default), `default`, `bypassPermissions` or `plan` |
| `auto_approve` | Merge as soon as it finishes, so the next steps start from its work |
| `matrix` | One task per entry, run in parallel |
| `after` | Step ids to wait for; by default a step waits for the step before it |

A step that comes after a matrix step waits for every task in the matrix.

## Placeholders

| | |
| --- | --- |
| `{{input}}` | What you typed when you started the run |
| `{{item}}` | The matrix entry |
| `{{deps}}` | What the tasks this one waits on reported, filled in when it starts |

Steps that aren't auto-approved stop in review until you approve them, so the steps after them wait too. Mistakes such as unknown fields, repeated ids, or `after` naming a later step are reported when you open **Run workflow**.
