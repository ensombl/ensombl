# Running Ensombl headless

`ensombl-server` runs the same engine as the desktop app, with no window. That includes projects, indexing, tasks, workflows and agent runs. You drive it over HTTP and watch it over a WebSocket.

## Build and run

```sh
cargo build --release -p ensombl-server
./target/release/ensombl-server --data-dir ~/.ensombl-server
```

| Flag | Default | |
| --- | --- | --- |
| `--data-dir DIR` | required | Where state lives (see [architecture.md](architecture.md#data)) |
| `--worktrees-dir DIR` | `<data-dir>/worktrees` | Where task worktrees are created |
| `--bind ADDR` | `127.0.0.1:7878` | Address to listen on |
| `--token TOKEN` | `$ENSOMBL_TOKEN` | Required for every request when set |

The server won't listen on anything but localhost without a token. Agent CLIs (`claude`, `codex`) must be installed and logged in for the user the server runs as.

## API

Every command and query is `POST /api/<name>`. The body is the JSON input, and the response is the JSON output. The names and shapes are the same ones the desktop app uses; their TypeScript types are in `src/shared/bindings/`.

```sh
T=secret
curl -s localhost:7878/api/add_project -H "Authorization: Bearer $T" \
  -d '{"path": "/srv/repos/my-app"}'
curl -s localhost:7878/api/get_session -H "Authorization: Bearer $T" -d '{}'
curl -s localhost:7878/api/create_task -H "Authorization: Bearer $T" -d '{
  "projectId": "…", "title": "Add a health endpoint",
  "prompt": "Add GET /health returning 200 ok, with a test.",
  "agent": "claude", "baseBranch": "main", "dependsOn": [],
  "permission": "acceptEdits", "queue": true
}'
curl -s localhost:7878/api/approve_task -H "Authorization: Bearer $T" -d '{"taskId": "…"}'
```

Errors come back as `400` with `{"error": "…"}`, and a missing or wrong token as `401`. `GET /health` answers `ok` without a token.

Useful names:

- Tasks: `list_tasks`, `get_task`, `get_run_log`, `get_task_diff`, `create_task`, `queue_task`, `cancel_task`, `retry_task`, `respond_to_task`, `send_follow_up`, `approve_task`, `delete_task`, `get_coordination`, `update_task_branch`, `get_usage`, `stop_all_tasks`
- Workflows: `list_workflows`, `start_workflow`
- Projects: `get_session`, `add_project`, `list_worktrees`, `list_branches`

The full list is `NAMES` in `crates/ensombl-server/src/rpc.rs`.

## Events

`GET /events` is a WebSocket that streams every event as JSON `{ "topic": …, "payload": … }` (topics are listed in [architecture.md](architecture.md#events)). Browsers can't set headers on a WebSocket, so pass the token as `?token=`:

```sh
websocat "ws://localhost:7878/events?token=$T"
```

## A lead agent against a server

`ensombl-mcp` is a stdio MCP server whose tools call a server's API:

- `list_projects`, `create_task`, `list_tasks`, `get_task`, `wait_for_task`
- `get_task_activity`, `get_task_diff`
- `send_follow_up`, `answer_task`, `approve_task`
- `list_workflows`, `start_workflow`

Point any MCP-capable agent at it:

```sh
cargo build --release -p ensombl-mcp
```

```json
{
  "mcpServers": {
    "ensombl": {
      "command": "/path/to/ensombl-mcp",
      "args": ["--api-url", "http://localhost:7878", "--token", "secret"]
    }
  }
}
```

```sh
claude --mcp-config ensombl-mcp.json --allowedTools mcp__ensombl
```

The desktop app does the same thing for you. It serves this API on localhost, writes the port and token to `api.json`, and starts leads with `Ensombl --mcp --api-file <api.json>` as their MCP server.
