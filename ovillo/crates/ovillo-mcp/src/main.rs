//! `ovillo-mcp`: an MCP server on stdio that lets a Claude session drive a
//! running Ovillo through its control socket.
//!
//! ```text
//! ovillo-mcp [--socket <path>]
//! ovillo-mcp --shot <out.png> [--pid <pid>]   # one picture of an Ovillo window, then exit
//! ```
//!
//! JSON-RPC 2.0, one message per line, written by hand: `initialize`,
//! `notifications/initialized` (and any other notification, ignored),
//! `ping`, `tools/list`, `tools/call`. Each tool but `screenshot` is one op
//! on the app's socket (`{"id", "op", "args"}` → `{"id", "ok", "result" |
//! "error"}`); errors from the instrument come back as tool results with
//! `isError`, so the model reads them. The socket is `--socket`, else
//! `$OVILLO_SOCKET`, else `ovillo.sock` in the per-user temporary directory
//! (the app's default too).

mod link;
mod shot;
mod tools;

use link::{Link, LinkError};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::time::Duration;

/// Protocol versions this server speaks; the first is preferred.
const VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

fn text_result(v: &Value, is_error: bool) -> Value {
    let text = match v {
        Value::String(s) => s.clone(),
        other => serde_json::to_string_pretty(other).unwrap_or_default(),
    };
    json!({"content": [{"type": "text", "text": text}], "isError": is_error})
}

fn call_tool(link: &mut Link, name: &str, args: Value) -> Value {
    let args = if args.is_null() { json!({}) } else { args };
    if name == "screenshot" {
        return screenshot(link, &args);
    }
    let known = tools::list().as_array().is_some_and(|t| t.iter().any(|t| t["name"] == name));
    if !known {
        return text_result(&json!(format!("unknown tool \"{name}\"")), true);
    }
    match link.call(name, args) {
        Ok(v) => text_result(&v, false),
        Err(e) => text_result(&json!(e.text()), true),
    }
}

fn screenshot(link: &mut Link, args: &Value) -> Value {
    let max_width = args.get("max_width").and_then(Value::as_u64).unwrap_or(1240).clamp(320, 4000) as u32;
    let save = args.get("path").and_then(Value::as_str).map(PathBuf::from);
    let allow_stale = args.get("allow_stale").and_then(Value::as_bool).unwrap_or(false);
    let ping = match link.call("ping", json!({})) {
        Ok(v) => v,
        Err(e @ (LinkError::NotRunning(_) | LinkError::Refused(_))) => return text_result(&json!(e.text()), true),
    };
    let Some(pid) = ping.get("pid").and_then(Value::as_u64).map(|p| p as u32) else {
        return text_result(&json!("the instrument did not say its pid"), true);
    };
    let visible = ping.pointer("/window/visible").and_then(Value::as_bool).unwrap_or(true);
    let drawn_ms = ping.pointer("/window/last_drawn_ms_ago").and_then(Value::as_u64);
    if !visible && !allow_stale {
        return text_result(
            &json!(format!(
                "the Ovillo window is hidden (fully covered by other windows, minimized, or on another Space), so macOS is not drawing it: \
                 a picture now would show the last frame drawn{}, not the current state. Bring the window into view and try again, \
                 or pass allow_stale: true for the stale picture. get_state reads the current state either way.",
                drawn_ms.map_or(String::new(), |ms| format!(" {:.1} s ago", ms as f64 / 1000.0))
            )),
            true,
        );
    }
    // Let the last changes reach the screen.
    std::thread::sleep(Duration::from_millis(150));
    match shot::capture(pid, max_width, save.as_deref()) {
        Ok(s) => json!({
            "content": [
                {"type": "image", "data": shot::base64(&s.png), "mimeType": "image/png"},
                {"type": "text", "text": json!({
                    "pid": pid, "window_id": s.window.id, "width": s.width, "height": s.height,
                    "saved": s.saved.map(|p| p.display().to_string()),
                    "window_visible": visible, "last_drawn_ms_ago": drawn_ms, "stale": !visible,
                }).to_string()}
            ],
            "isError": false
        }),
        Err(e) => text_result(&json!(e), true),
    }
}

/// Answer one JSON-RPC message; `None` for notifications.
fn handle(link: &mut Link, msg: &Value) -> Option<Value> {
    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let Some(id) = id else {
        // A notification (notifications/initialized, cancelled, …): nothing to say.
        return None;
    };
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    let result = match method {
        "initialize" => {
            let asked = params.get("protocolVersion").and_then(Value::as_str).unwrap_or(VERSIONS[0]);
            let version = if VERSIONS.contains(&asked) { asked } else { VERSIONS[0] };
            Ok(json!({
                "protocolVersion": version,
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": "ovillo", "title": "Ovillo", "version": env!("CARGO_PKG_VERSION")},
                "instructions": tools::INSTRUCTIONS,
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools::list()})),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);
            Ok(call_tool(link, name, args))
        }
        "" => Err((-32600, "invalid request: no method".to_string())),
        other => Err((-32601, format!("method not found: {other}"))),
    };
    Some(match result {
        Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
        Err((code, message)) => json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}),
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("ovillo-mcp [--socket <path>]  (MCP server on stdio for a running Ovillo; default socket {})", link::socket_path(None).display());
        return;
    }
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).map(String::as_str);
    let socket = value("--socket");
    let mut link = Link::new(link::socket_path(socket));
    if let Some(out) = value("--shot") {
        // A picture without being a client (the pid given), or via the socket's ping.
        let pid = match value("--pid").and_then(|p| p.parse().ok()) {
            Some(p) => p,
            None => match link.call("ping", json!({})) {
                Ok(v) => v.get("pid").and_then(Value::as_u64).unwrap_or(0) as u32,
                Err(e) => {
                    eprintln!("{}", e.text());
                    std::process::exit(1);
                }
            },
        };
        match shot::capture(pid, 1240, Some(std::path::Path::new(out))) {
            Ok(s) => println!("{} {}x{} window {} pid {pid}", out, s.width, s.height, s.window.id),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        return;
    }
    eprintln!("ovillo-mcp: socket {}", link.path.display());
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let replies: Vec<Value> = match serde_json::from_str::<Value>(&line) {
            Err(e) => vec![json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": format!("parse error: {e}")}})],
            Ok(Value::Array(batch)) => batch.iter().filter_map(|m| handle(&mut link, m)).collect(),
            Ok(msg) => handle(&mut link, &msg).into_iter().collect(),
        };
        for r in replies {
            if writeln!(out, "{r}").and_then(|()| out.flush()).is_err() {
                return;
            }
        }
    }
}
