//! The real `ovillo-mcp` binary over real stdio: the MCP handshake, the tool
//! list, and what a tool says when no Ovillo is running on its socket.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn session(lines: &[Value]) -> Vec<Value> {
    let sock = std::env::temp_dir().join(format!("ovillo-mcp-test-{}.sock", std::process::id()));
    let mut child = Command::new(env!("CARGO_BIN_EXE_ovillo-mcp"))
        .args(["--socket", sock.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn ovillo-mcp");
    let mut stdin = child.stdin.take().unwrap();
    for l in lines {
        writeln!(stdin, "{l}").unwrap();
    }
    writeln!(stdin, "this is not json").unwrap();
    drop(stdin);
    let out: Vec<Value> = BufReader::new(child.stdout.take().unwrap())
        .lines()
        .map(|l| serde_json::from_str(&l.unwrap()).unwrap())
        .collect();
    assert!(child.wait().unwrap().success());
    out
}

#[test]
fn the_handshake_the_tools_and_an_absent_instrument() {
    let out = session(&[
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "0"}}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "get_state", "arguments": {}}}),
        json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {"name": "no_such_tool", "arguments": {}}}),
        json!({"jsonrpc": "2.0", "id": 5, "method": "ping"}),
        json!({"jsonrpc": "2.0", "id": 6, "method": "resources/list"}),
        json!({"jsonrpc": "2.0", "id": 7, "method": "initialize", "params": {"protocolVersion": "2099-01-01"}}),
    ]);
    // Seven requests answered (the notification is not), plus the parse error.
    assert_eq!(out.len(), 8, "{out:#?}");
    let init = &out[0]["result"];
    assert_eq!(init["protocolVersion"], "2025-06-18");
    assert_eq!(init["serverInfo"]["name"], "ovillo");
    assert!(init["capabilities"]["tools"].is_object());
    let tools = out[1]["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    for want in [
        "get_state", "press_keys", "set_params", "get_patch", "load_patch", "save_patch", "default_patch", "set_guitar",
        "set_display", "transport", "pluck", "strum", "mute_strings", "reset_world", "set_output", "record", "play_score", "set_input", "screenshot",
    ] {
        assert!(names.contains(&want), "missing tool {want}");
    }
    assert_eq!(names.len(), 19);
    for t in tools {
        assert_eq!(t["inputSchema"]["type"], "object", "{}", t["name"]);
        assert!(t["description"].as_str().unwrap().len() > 60, "{} needs a real description", t["name"]);
    }
    let absent = &out[2]["result"];
    assert_eq!(absent["isError"], true);
    assert!(absent["content"][0]["text"].as_str().unwrap().contains("not running"), "{absent}");
    assert_eq!(out[3]["result"]["isError"], true);
    assert_eq!(out[4]["result"], json!({}));
    assert_eq!(out[5]["error"]["code"], -32601);
    // A version it does not know: it offers its own.
    assert_eq!(out[6]["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(out[7]["error"]["code"], -32700);
}
