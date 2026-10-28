//! `ensombl-server --data-dir DIR [--worktrees-dir DIR] [--bind ADDR] [--token TOKEN]`
//!
//! Runs Ensombl headless. The token can also come from `ENSOMBL_TOKEN`; without
//! one, only bind to localhost.

use std::path::PathBuf;
use std::sync::Arc;

use ensombl_core::app::{AppPaths, Core};
use ensombl_server::{BroadcastBus, router};

struct Args {
    data_dir: PathBuf,
    worktrees_dir: PathBuf,
    bind: String,
    token: Option<String>,
}

fn usage() -> ! {
    eprintln!(
        "usage: ensombl-server --data-dir DIR [--worktrees-dir DIR] [--bind ADDR] [--token TOKEN]"
    );
    std::process::exit(2)
}

fn args() -> Args {
    let mut data_dir = None;
    let mut worktrees_dir = None;
    let mut bind = "127.0.0.1:7878".to_string();
    let mut token = std::env::var("ENSOMBL_TOKEN")
        .ok()
        .filter(|t| !t.is_empty());
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().unwrap_or_else(|| usage());
        match flag.as_str() {
            "--data-dir" => data_dir = Some(PathBuf::from(value())),
            "--worktrees-dir" => worktrees_dir = Some(PathBuf::from(value())),
            "--bind" => bind = value(),
            "--token" => token = Some(value()),
            "-h" | "--help" => usage(),
            _ => usage(),
        }
    }
    let data_dir = data_dir.unwrap_or_else(|| usage());
    Args {
        worktrees_dir: worktrees_dir.unwrap_or_else(|| data_dir.join("worktrees")),
        data_dir,
        bind,
        token,
    }
}

#[tokio::main]
async fn main() {
    let args = args();
    let bus = BroadcastBus::new();
    let core = Arc::new(
        Core::build(
            AppPaths {
                data_dir: args.data_dir,
                worktrees_dir: args.worktrees_dir,
            },
            Arc::new(bus.clone()),
        )
        .unwrap_or_else(|e| {
            eprintln!("ensombl-server: {e}");
            std::process::exit(1)
        }),
    );
    if args.token.is_none()
        && !args.bind.starts_with("127.0.0.1")
        && !args.bind.starts_with("localhost")
    {
        eprintln!(
            "ensombl-server: refusing to listen on {} without --token",
            args.bind
        );
        std::process::exit(2);
    }
    let app = router(core.handlers.clone(), &bus, args.token);
    let listener = tokio::net::TcpListener::bind(&args.bind)
        .await
        .unwrap_or_else(|e| {
            eprintln!("ensombl-server: can't listen on {}: {e}", args.bind);
            std::process::exit(1)
        });
    eprintln!("ensombl-server listening on http://{}", args.bind);
    let shutdown = {
        let core = core.clone();
        async move {
            let _ = tokio::signal::ctrl_c().await;
            core.shutdown();
        }
    };
    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
    {
        eprintln!("ensombl-server: {e}");
    }
}
