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
