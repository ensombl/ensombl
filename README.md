# CRM: Clanker Resource Management

CRM is an architecture-first desktop editor for coding agents (clankers). It keeps a live UML diagram of the whole codebase on screen next to the agent you're driving (`claude`, `codex` or any CLI). You can expand any component to see its signatures, and you can `@`-tag classes, methods and detected design patterns in your prompts the way you tag files today.

You need no API keys: CRM runs the real agent CLIs in an embedded terminal, so they use your existing logins.

## Stack

- **Tauri 2**: Rust backend (indexing, PTYs, file watching) with a React + TypeScript frontend
- **tree-sitter** for TS/JS symbol extraction, **React Flow + ELK** for the diagram, **xterm.js** for the agents

## Develop

```sh
pnpm install
pnpm tauri dev
```

Checks:

```sh
pnpm lint && pnpm test          # frontend
pnpm test:rust                  # backend (also regenerates src/bindings)
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Shared types live in `src-tauri/src/model.rs`. ts-rs exports them to `src/bindings/` when you run `cargo test`, so commit the regenerated files.
