# Ensombl

Where clankers come together.

Ensombl is a desktop app for working with coding agents (clankers). It shows a live architecture view of your codebase beside the agents you run (`claude`, `codex` or a shell), and it orchestrates many agents at once: you hand out tasks, each runs on its own branch, and you review and merge the results.

You need no API keys. Ensombl runs the real agent CLIs, so they use your existing logins.

## What's in it

- **Projects, worktrees and tabs.** A project is a git repository, and each tab is one of its worktrees, grouped by project in the title bar. Each worktree has its own index, agents and file watcher.
- **Architecture views.** Components and how they talk to each other, a class diagram, and a Code tab with an editor and review comments.
- **Agents.** Interactive CLI terminals in the worktree. Start one with **+** in the Agents pane.
- **Tasks.** The Tasks tab hands work to headless agents:
  - Each task runs alone in its own worktree and branch, and waits in review until you approve it. Approving merges it.
  - Tasks can wait on other tasks, and a task that waits on others starts with what they reported.
  - Ensombl warns when tasks touch the same files or would conflict with their base.
  - It caps how many run at once, shows what they've cost today, and can stop them all.
  - You can take over any task in a terminal.
  - Tasks run on Claude Code or Codex.
- **Lead agent.** In the Agents pane, choose **+ → Start a lead** and pick the CLI that leads. The lead splits your request into tasks, watches them, asks for changes and merges the work, all through Ensombl's MCP tools.
- **Workflows.** Pipelines kept in your repo as `.ensombl/workflows/*.yml`, with steps, parallel matrix steps and steps that merge themselves. See [docs/workflows.md](docs/workflows.md).
- **Headless.** The same engine runs without a window as `ensombl-server`, with an HTTP API and a live event stream. See [docs/headless.md](docs/headless.md).
- **Updates.** The app checks for new releases and offers **Restart to update**. You can also check from the version badge in the title bar.

## Install

Download the latest build from [Releases](https://github.com/ensombl/ensombl/releases):

- macOS (Apple Silicon or Intel): `.dmg`
- Linux: `.AppImage` or `.deb`
- Windows: `.msi`

After you install it, the app updates itself. Builds aren't notarized yet, so the first time you open it on macOS, right-click the app and choose **Open**.

## Develop

```sh
pnpm install
pnpm tauri dev
```

Checks, which CI runs too:

```sh
pnpm lint && pnpm test                                 # frontend
cargo test --workspace                                 # backend; regenerates src/shared/bindings
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

Rust types shared with the frontend are exported to `src/shared/bindings/` by ts-rs when you run `cargo test -p ensombl-core`, so commit the regenerated files. CI fails if they're stale.

How the code is laid out, including the core crate, the shells and the data each instance keeps, is in [docs/architecture.md](docs/architecture.md).

## Release

1. Bump `version` in `package.json`, `src-tauri/tauri.conf.json` and the crates' `Cargo.toml` files.
2. Push a tag, for example `git tag v0.2.0 && git push origin v0.2.0`.
3. The Release workflow builds every platform and signs the update bundles with the `TAURI_SIGNING_PRIVATE_KEY` secret. It publishes the release together with the `latest.json` that installed apps poll.
