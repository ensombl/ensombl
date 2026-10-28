# CRM: Clanker Resource Management

CRM is an architecture-first desktop editor for coding agents (clankers). It keeps a live UML diagram of the whole codebase on screen next to the agent you're driving (`claude`, `codex` or any CLI). You can expand any component to see its signatures, and you can `@`-tag classes, methods and detected design patterns in your prompts the way you tag files today.

You need no API keys: CRM runs the real agent CLIs in an embedded terminal, so they use your existing logins.

## Stack

- **Tauri 2**: Rust backend (indexing, PTYs, file watching) with a React + TypeScript frontend
- **tree-sitter** for TS/JS symbol extraction, **React Flow + ELK** for the diagram, **xterm.js** for the agents

## Architecture

CRM follows **Clean Architecture** with **CQRS**: every use case is either a *command*, which changes state, or a *query*, which reads state, and each has exactly one handler. Dependencies point inward only.

```
src-tauri/src/
  domain/           entities, value objects, pure rules (no IO)
  application/
    cqrs.rs         Command/Query traits + handler traits
    commands/       state-changing use cases
    queries/        read-only use cases
    ports.rs        traits that infrastructure implements
  infrastructure/   adapters: tree-sitter, sqlite, fs watcher, pty, ...
  interface/        thin #[tauri::command]s that dispatch to handlers
  lib.rs            composition root: wires adapters into handlers

src/
  app/              composition: providers, layout, top-level screen
  features/<name>/
    queries/        useQuery hooks over backend queries
    commands/       useMutation hooks over backend commands
    ui/             feature components (shadcn/ui only)
  shared/
    ipc/client.ts   the only module that talks to Tauri (`query` / `command`)
    bindings/       TS types generated from Rust by ts-rs
    ui/             shared shadcn compositions
  components/ui/    shadcn/ui primitives (generated)
```

Rules:
- UI never calls IPC directly; it goes through a feature's query or command.
- Interface-layer Tauri commands contain no logic; they build the message and dispatch it.
- Handlers depend on ports (traits), never on concrete adapters.

## Develop

```sh
pnpm install
pnpm tauri dev
```

Checks:

```sh
pnpm lint && pnpm test          # frontend
pnpm test:rust                  # backend (also regenerates src/shared/bindings)
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Shared types live in `src-tauri/src/model.rs`. ts-rs exports them to `src/shared/bindings/` when you run `cargo test`, so commit the regenerated files.
