//! Ensombl as a service. `POST /api/<name>` runs a command or query with a
//! JSON body; `GET /events` is a WebSocket of everything the core publishes.
//! The desktop app embeds the same router on localhost so local tools (the
//! MCP bridge, scripts) can drive it.

pub mod rpc;

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use ensombl_core::app::Handlers;
use ensombl_core::application::ports::EventBus;
use serde::Serialize;
use serde_json::Value;
use tokio::sync::broadcast;

/// One published event, as sent to WebSocket clients.
#[derive(Debug, Clone, Serialize)]
pub struct BusEvent {
    pub topic: String,
    pub payload: Value,
}

/// [`EventBus`] fanning events out to every connected WebSocket.
#[derive(Clone)]
pub struct BroadcastBus(pub broadcast::Sender<BusEvent>);

impl BroadcastBus {
    pub fn new() -> Self {
        Self(broadcast::channel(1024).0)
    }
}

impl Default for BroadcastBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus for BroadcastBus {
    fn publish(&self, topic: &str, payload: Value) {
        // No subscribers is fine.
        let _ = self.0.send(BusEvent {
            topic: topic.into(),
            payload,
        });
    }
}

#[derive(Clone)]
struct AppState {
    handlers: Arc<Handlers>,
    events: broadcast::Sender<BusEvent>,
    token: Option<Arc<str>>,
}

/// The API. With a token, every request needs `Authorization: Bearer <token>`
/// (or `?token=` for the WebSocket, which browsers can't give headers).
pub fn router(handlers: Arc<Handlers>, bus: &BroadcastBus, token: Option<String>) -> Router {
    let state = AppState {
        handlers,
        events: bus.0.clone(),
        token: token.map(Into::into),
    };
    Router::new()
        .route("/api/{name}", post(call))
        .route("/events", get(events))
        .route_layer(middleware::from_fn_with_state(state.clone(), authorize))
        .route("/health", get(|| async { "ok" }))
        .with_state(state)
}

async fn authorize(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let Some(token) = &state.token else {
        return next.run(request).await;
    };
    let bearer = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let query = request
        .uri()
        .query()
        .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("token=")));
    if bearer.or(query) == Some(&**token) {
        next.run(request).await
    } else {
        (StatusCode::UNAUTHORIZED, "missing or wrong token").into_response()
    }
}

async fn call(
    State(state): State<AppState>,
    Path(name): Path<String>,
    body: Option<Json<Value>>,
) -> Response {
    let input = body.map(|Json(v)| v).unwrap_or(Value::Null);
    let handlers = state.handlers.clone();
    let result = tokio::task::spawn_blocking(move || rpc::call(&handlers, &name, input)).await;
    match result {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(e)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn events(State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    let rx = state.events.subscribe();
    ws.on_upgrade(move |socket| stream(socket, rx))
}

async fn stream(mut socket: WebSocket, mut rx: broadcast::Receiver<BusEvent>) {
    loop {
        tokio::select! {
            event = rx.recv() => match event {
                Ok(event) => {
                    let text = serde_json::to_string(&event).unwrap_or_default();
                    if socket.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                // A slow client missed some; carry on with the newest.
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            incoming = socket.recv() => match incoming {
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                _ => {}
            },
        }
    }
}

/// Where a running instance's API can be reached, written to a file so local
/// tools can find it.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct LocalApi {
    pub url: String,
    pub token: String,
}

impl LocalApi {
    pub fn read(file: &std::path::Path) -> std::io::Result<LocalApi> {
        let text = std::fs::read_to_string(file)?;
        serde_json::from_str(&text).map_err(std::io::Error::other)
    }
}

fn random_token() -> std::io::Result<String> {
    let mut bytes = [0u8; 24];
    getrandom::fill(&mut bytes).map_err(std::io::Error::other)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Serves the API on a free localhost port from its own thread and writes
/// where to `file` (readable only by the user). For the desktop app.
pub fn spawn_local(
    handlers: Arc<Handlers>,
    bus: &BroadcastBus,
    file: &std::path::Path,
) -> std::io::Result<LocalApi> {
    let token = random_token()?;
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let api = LocalApi {
        url: format!("http://{}", listener.local_addr()?),
        token: token.clone(),
    };
    write_private(
        file,
        &serde_json::to_string_pretty(&api).map_err(std::io::Error::other)?,
    )?;
    let app = router(handlers, bus, Some(token));
    std::thread::Builder::new()
        .name("local-api".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => return eprintln!("local API: {e}"),
            };
            runtime.block_on(async move {
                match tokio::net::TcpListener::from_std(listener) {
                    Ok(listener) => {
                        if let Err(e) = axum::serve(listener, app).await {
                            eprintln!("local API: {e}");
                        }
                    }
                    Err(e) => eprintln!("local API: {e}"),
                }
            });
        })?;
    Ok(api)
}

fn write_private(file: &std::path::Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    use std::io::Write;
    options.open(file)?.write_all(text.as_bytes())
}
