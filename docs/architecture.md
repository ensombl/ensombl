# Architecture

## Crates and shells

All the backend logic lives in one Tauri-free crate. Every way of running Ensombl is a thin shell around it.

```
crates/ensombl-core/     domain, use cases and adapters. No Tauri, no async runtime
crates/ensombl-server/   HTTP + WebSocket shell (axum); library and `ensombl-server` binary
crates/ensombl-mcp/      stdio MCP bridge for lead agents, talking to a server's API
src-tauri/               the desktop shell: Tauri commands, window, embedded local API
src/                     React frontend for the desktop shell
```

CI checks that `ensombl-core` never depends on Tauri, so anything the app does can also run headless.

### Core layers

`ensombl-core` follows Clean Architecture with CQRS. Each use case is either a command, which changes state, or a query, which reads it, and each has exactly one handler. Dependencies point inward only.

```
domain/           entities and pure rules (tasks and their state machine, workflows, ...)
application/
  cqrs.rs         Command/Query and handler traits
  commands/       state-changing use cases
  queries/        read-only use cases
  ports.rs        traits that adapters implement
infrastructure/   adapters: git, sqlite, tree-sitter, PTYs, agent CLIs, ...
app.rs            composition root: Core::build wires adapters into Handlers
```

A shell provides three things:

- `AppPaths`: where data and worktrees live.
- An `EventBus`: where events go. The desktop app emits them to the webview, and the server broadcasts them on its WebSocket.
- `CoreOptions`, for example the MCP bridge that lead agents use.

In return the shell gets back `Handlers`, the command and query handlers it exposes.

### Agent runtimes

Headless runs go through the `AgentRuntime` port. Every CLI's output is normalized into the same `RunEvent`s, so the orchestrator treats them all alike:

- `claude_headless.rs`: Claude Code over `claude -p --input-format stream-json --output-format stream-json`. Permission prompts come back to Ensombl, which shows them so you can answer them.
- `codex_exec.rs`: Codex over `codex exec --json`, one process per turn, resuming the thread for follow-ups. Codex can't ask for permission when it runs headless, so the task's permission mode picks a sandbox instead.

Whether a CLI can lead is declared in `AgentProfile::lead_args_for`. To add a CLI, add an `AgentKind`, a profile, its lead args and an `AgentRuntime`.

### Frontend

```
src/
  app/              providers, layout, top-level screen
  features/<name>/
    queries/        useQuery hooks over backend queries
    commands/       useMutation hooks over backend commands
    model/          client-only UI state
    ui/             feature components (shadcn/ui only)
  shared/
    ipc/client.ts   the only module that talks to Tauri
    bindings/       TS types generated from Rust by ts-rs
```

Rules:

- The UI never calls IPC directly; it goes through a feature's queries or commands.
- Tauri commands contain no logic; they build the message and dispatch it.
- Handlers depend on ports, never on concrete adapters.

## Data

Each instance keeps its state in its data directory. For the desktop app on macOS that's `~/Library/Application Support/io.ensombl.crm/`; for the server it's whatever you pass as `--data-dir`.

| File | What's in it |
| --- | --- |
| `session.json` | Projects, open worktree tabs |
| `orchestrator.db` | Tasks and their runs (SQLite) |
| `runs/<run>.jsonl` | Every event of a run, replayable after a restart |
| `index.db` | Symbol index cache |
| `settings.json` | Editor command, task concurrency |
| `comments.json` | Review comments |
| `patterns.json` | Design pattern decisions |
| `api.json` | Desktop only: port and token of the embedded local API (mode 0600) |
| `mcp.json` | Desktop only: MCP config pointing lead agents at the bridge |

Worktrees go under `~/.crm/worktrees/<project>/` in the desktop app. Task worktrees are named `task-<id>`.

## Events

Everything the core publishes goes out through the `EventBus` as `{ topic, payload }`:

| Topic | Payload |
| --- | --- |
| `index://progress` | Indexing progress for a worktree |
| `index://updated` | The model changed (a delta, or `null` to refetch) |
| `tasks://changed` | A task, after any change |
| `tasks://run` | `{ taskId, runId, event }`, one `RunEvent` of a live run |
