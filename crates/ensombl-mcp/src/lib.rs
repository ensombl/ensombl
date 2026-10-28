//! An MCP server over stdio whose tools drive Ensombl's API: a lead agent
//! uses it to split work into tasks, wait on them, review and merge them.
//! Run as `ensombl-mcp --api-file <api.json>` (or `Ensombl --mcp ...`).

pub mod tools;

use std::io::{BufRead, Write};
use std::path::Path;

use serde_json::{Value, json};

/// Ensombl's API, called by command name.
pub trait Backend {
    fn call(&self, name: &str, input: Value) -> Result<Value, String>;
}

/// The HTTP API of a running Ensombl (desktop or server).
pub struct HttpApi {
    url: String,
    token: Option<String>,
    agent: ureq::Agent,
}

impl HttpApi {
    pub fn new(url: impl Into<String>, token: Option<String>) -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            url: url.into().trim_end_matches('/').to_string(),
            token,
            agent,
        }
    }

    /// From the `api.json` a running app writes.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("can't read {} (is Ensombl running?): {e}", path.display()))?;
        let api: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let url = api["url"].as_str().ok_or("api file has no url")?;
        Ok(Self::new(url, api["token"].as_str().map(String::from)))
    }
}

impl Backend for HttpApi {
    fn call(&self, name: &str, input: Value) -> Result<Value, String> {
        let mut request = self.agent.post(format!("{}/api/{name}", self.url));
        if let Some(token) = &self.token {
            request = request.header("Authorization", &format!("Bearer {token}"));
        }
        let mut response = request
            .send_json(&input)
            .map_err(|e| format!("can't reach Ensombl at {}: {e}", self.url))?;
        let status = response.status();
        let body: Value = response.body_mut().read_json().unwrap_or(Value::Null);
        if status.is_success() {
            Ok(body)
        } else {
            Err(body["error"]
                .as_str()
                .map(String::from)
                .unwrap_or_else(|| format!("Ensombl answered {status}")))
        }
    }
}

const PROTOCOL: &str = "2025-06-18";

/// Answers one JSON-RPC message; `None` for notifications.
pub fn handle(backend: &dyn Backend, message: &Value) -> Option<Value> {
    let id = message.get("id")?.clone();
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    let result = match message["method"].as_str().unwrap_or_default() {
        "initialize" => Ok(json!({
            "protocolVersion": params["protocolVersion"].as_str().unwrap_or(PROTOCOL),
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "ensombl", "version": env!("CARGO_PKG_VERSION") },
            "instructions": tools::INSTRUCTIONS,
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools::definitions() })),
        "tools/call" => {
            let name = params["name"].as_str().unwrap_or_default();
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            Ok(match tools::call(backend, name, &args) {
                Ok(text) => json!({ "content": [{ "type": "text", "text": text }] }),
                Err(error) => {
                    json!({ "content": [{ "type": "text", "text": error }], "isError": true })
                }
            })
        }
        other => Err(json!({ "code": -32601, "message": format!("unknown method {other}") })),
    };
    Some(match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(error) => json!({ "jsonrpc": "2.0", "id": id, "error": error }),
    })
}

/// Serves newline-delimited JSON-RPC until input ends.
pub fn serve(
    backend: &dyn Backend,
    input: impl BufRead,
    mut output: impl Write,
) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(message) => handle(backend, &message),
            Err(e) => Some(
                json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": e.to_string() } }),
            ),
        };
        if let Some(reply) = reply {
            writeln!(output, "{reply}")?;
            output.flush()?;
        }
    }
    Ok(())
}

/// Entry point for `--api-file PATH` or `--api-url URL [--token T]`.
pub fn run(args: &[String]) -> i32 {
    let value = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
    };
    let api = match (value("--api-file"), value("--api-url")) {
        (Some(file), _) => HttpApi::from_file(Path::new(file)),
        (None, Some(url)) => Ok(HttpApi::new(
            url.as_str(),
            value("--token")
                .cloned()
                .or_else(|| std::env::var("ENSOMBL_TOKEN").ok()),
        )),
        _ => Err("usage: ensombl-mcp --api-file PATH | --api-url URL [--token TOKEN]".into()),
    };
    match api {
        Ok(api) => match serve(&api, std::io::stdin().lock(), std::io::stdout().lock()) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("ensombl-mcp: {e}");
                1
            }
        },
        Err(e) => {
            eprintln!("ensombl-mcp: {e}");
            2
        }
    }
}
