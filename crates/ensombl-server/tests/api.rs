use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use ensombl_core::app::{AppPaths, Core};
use ensombl_core::application::ports::EventBus;
use ensombl_server::{BroadcastBus, router};
use futures_util::StreamExt;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

fn core(dir: &std::path::Path, bus: &BroadcastBus) -> Arc<Core> {
    Arc::new(
        Core::build(
            AppPaths {
                data_dir: dir.join("data"),
                worktrees_dir: dir.join("worktrees"),
            },
            Arc::new(bus.clone()),
        )
        .unwrap(),
    )
}

async fn post(
    app: axum::Router,
    path: &str,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut req = Request::post(path).header("content-type", "application/json");
    if let Some(t) = token {
        req = req.header("authorization", format!("Bearer {t}"));
    }
    let res = app
        .oneshot(req.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn runs_commands_and_queries_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let bus = BroadcastBus::new();
    let core = core(dir.path(), &bus);
    let app = router(core.handlers.clone(), &bus, Some("s3cret".into()));

    let (status, _) = post(app.clone(), "/api/list_tasks", None, json!({})).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, info) = post(
        app.clone(),
        "/api/get_app_info",
        Some("s3cret"),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(info["name"], "Ensombl");

    let (status, tasks) = post(app.clone(), "/api/list_tasks", Some("s3cret"), json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(tasks, json!([]));

    let (status, err) = post(
        app.clone(),
        "/api/get_task",
        Some("s3cret"),
        json!({ "taskId": "nope" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(err["error"], "no task nope");

    let (status, _) = post(app, "/api/rm_rf", Some("s3cret"), json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    core.shutdown();
}

#[tokio::test]
async fn streams_events_over_websocket() {
    let dir = tempfile::tempdir().unwrap();
    let bus = BroadcastBus::new();
    let core = core(dir.path(), &bus);
    let app = router(core.handlers.clone(), &bus, Some("t".into()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/events?token=t"))
        .await
        .unwrap();
    // Give the server a moment to subscribe before publishing.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    bus.publish("tasks://changed", json!({ "id": "k1" }));
    let msg = tokio::time::timeout(std::time::Duration::from_secs(5), ws.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let event: Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
    assert_eq!(
        event,
        json!({ "topic": "tasks://changed", "payload": { "id": "k1" } })
    );

    assert!(
        tokio_tungstenite::connect_async(format!("ws://{addr}/events"))
            .await
            .is_err()
    );
    core.shutdown();
}
