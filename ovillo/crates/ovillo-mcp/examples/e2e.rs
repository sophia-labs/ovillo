//! End to end: drive every tool of the real `ovillo-mcp` binary, over stdio
//! JSON-RPC, against a real running Ovillo, and check the instrument did it.
//!
//! ```text
//! ovillo --socket <path> --mute --no-activate &      # an instance of your own
//! cargo build -p ovillo-mcp
//! cargo run -p ovillo-mcp --example e2e -- --socket <path> --out <dir>
//! ```
//! Leaves the instance muted and as it found it, more or less: Wander,
//! default patch, main menu. Writes `<dir>/ovillo-e2e.png` (the screenshot)
//! and `<dir>/e2e-transcript.jsonl` (every request and reply).

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;

struct Mcp {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    id: u64,
    log: std::fs::File,
    checks: usize,
}

impl Mcp {
    fn send(&mut self, v: &Value) {
        writeln!(self.stdin, "{v}").unwrap();
        self.stdin.flush().unwrap();
        let _ = writeln!(self.log, "> {v}");
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.id += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": self.id, "method": method, "params": params}));
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        let v: Value = serde_json::from_str(&line).unwrap_or_else(|e| panic!("not JSON from ovillo-mcp: {e}: {line}"));
        let mut logged = v.clone();
        if let Some(c) = logged.pointer_mut("/result/content/0/data") {
            *c = json!("<png base64 elided>");
        }
        let _ = writeln!(self.log, "< {logged}");
        assert_eq!(v["id"], self.id, "reply out of order");
        v
    }

    /// Call a tool; its parsed JSON result (panics on isError unless `expect_error`).
    fn tool(&mut self, name: &str, args: Value) -> Value {
        let r = self.request("tools/call", json!({"name": name, "arguments": args}));
        let res = &r["result"];
        let text = res["content"][0]["text"].as_str().unwrap_or("").to_string();
        assert_eq!(res["isError"], false, "{name}: {text}");
        println!("  {name:<14} ok  {}", text.chars().take(110).collect::<String>().replace('\n', " "));
        serde_json::from_str(&text).unwrap_or(Value::String(text))
    }

    fn tool_error(&mut self, name: &str, args: Value) -> String {
        let r = self.request("tools/call", json!({"name": name, "arguments": args}));
        assert_eq!(r["result"]["isError"], true, "{name} should have failed: {r}");
        let text = r["result"]["content"][0]["text"].as_str().unwrap_or("").to_string();
        println!("  {name:<14} refused as it should: {text}");
        text
    }

    fn check(&mut self, ok: bool, what: &str) {
        assert!(ok, "FAILED: {what}");
        self.checks += 1;
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let socket = value("--socket").expect("--socket <path of your own Ovillo instance>");
    let out = std::path::PathBuf::from(value("--out").unwrap_or_else(|| ".".into()));
    std::fs::create_dir_all(&out).unwrap();
    let bin = value("--mcp").map(std::path::PathBuf::from).unwrap_or_else(|| {
        let exe = std::env::current_exe().unwrap();
        exe.parent().and_then(|p| p.parent()).unwrap().join("ovillo-mcp")
    });
    let mut child = Command::new(&bin)
        .args(["--socket", &socket])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|e| panic!("{}: {e}", bin.display()));
    let stdin = child.stdin.take().unwrap();
    let stdout = BufReader::new(child.stdout.take().unwrap());
    let log = std::fs::File::create(out.join("e2e-transcript.jsonl")).unwrap();
    let mut m = Mcp { child, stdin, stdout, id: 0, log, checks: 0 };
    let pause = |ms| std::thread::sleep(Duration::from_millis(ms));

    println!("MCP handshake through {}", bin.display());
    let init = m.request("initialize", json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "ovillo-e2e", "version": "0"}}));
    m.check(init["result"]["protocolVersion"] == "2025-06-18", "protocol version");
    m.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    let tools = m.request("tools/list", json!({}));
    let n = tools["result"]["tools"].as_array().map_or(0, Vec::len);
    m.check(n == 18, "eighteen tools");
    println!("{n} tools; driving each against the instrument on {socket}");

    // get_state
    let s = m.tool("get_state", json!({}));
    let pid = s["pid"].as_u64().unwrap();
    m.check(s["output"]["muted"] == true, "the test instance started muted");
    m.check(s["transport"]["composition"] == "wander", "Wander is the default composition");
    m.check(s["ariadne"].as_array().map_or(0, Vec::len) == 20, "twenty parameters");
    m.check(s["menu"]["softkeys"].as_array().map_or(0, Vec::len) == 8, "eight softkeys");
    m.check(s["annunciators"]["CTL"] == true, "CTL lit while we are connected");
    m.check(s["annunciators"]["MUTE"] == true, "MUTE annunciator lit");
    println!("  instance pid {pid}, device {}, {} Hz", s["audio"]["device"], s["audio"]["sample_rate"]);

    // set_output (stay muted: someone may be listening)
    let o = m.tool("set_output", json!({"master_db": -12, "mute": true}));
    m.check(o["master_db"] == -12.0 && o["muted"] == true, "master -12 dB, muted");
    m.tool_error("set_output", json!({"master_db": 6}));

    // press_keys
    let k = m.tool("press_keys", json!({"keys": "5 5 6 6 escape 8"}));
    m.check(k["ignored"].as_array().is_some_and(Vec::is_empty), "all keys known");
    m.check(k["menu"]["id"] == "output", "softkey 8 opened OUTPUT");
    m.check(k["menu"]["softkeys"][1]["lit"] == true, "OUTPUT ▸ MUTE is lit");
    let s = m.tool("get_state", json!({}));
    m.check(s["display"]["lower"] == "tab" && s["display"]["square"] == "chord", "keys set TAB and the CHORD BOX");
    let k = m.tool("press_keys", json!({"keys": ["1", "down", "down", "shift+up", "escape", "plutonium"]}));
    m.check(k["ignored"] == json!(["plutonium"]), "unknown key reported");
    let s = m.tool("get_state", json!({}));
    m.check((s["output"]["master_db"].as_f64().unwrap() - -13.9).abs() < 1e-9, "knob turned the master: -12 -1 -1 +0.1");

    // set_params, get_patch, save/load/default
    let p = m.tool("set_params", json!({"params": {"active_drive": 0.9, "Persistence": 0.66, "material_organization": 0.3, "sustain": 2}}));
    m.check(p["set"]["active_drive"] == 0.9 && p["set"]["space_decay"] == 0.66 && p["set"]["organization"] == 0.3, "params set by id, name and field");
    m.check(p["clamped"][0]["id"] == "sustain" && p["clamped"][0]["set"] == 1.0, "sustain clamped to 1");
    m.tool_error("set_params", json!({"params": {"drve": 1}}));
    let g = m.tool("get_patch", json!({}));
    m.check(g["patch"]["active_drive"] == 0.9 && g["patch"]["space_decay"] == 0.66, "patch reflects the change");
    pause(400);
    let live: Value = serde_json::from_str(&std::fs::read_to_string(g["live_patch"].as_str().unwrap()).unwrap()).unwrap();
    m.check(live["active_drive"] == 0.9, "live.json written");
    let sv = m.tool("save_patch", json!({"slot": "c"}));
    let slot_c = sv["saved"].as_str().unwrap().to_string();
    let d = m.tool("default_patch", json!({}));
    m.check(d["patch"]["active_drive"] == 0.42 && d["patch"]["output_gain"] == 5.0, "default patch");
    let l = m.tool("load_patch", json!({"slot": "c"}));
    m.check(l["patch"]["active_drive"] == 0.9, "slot c loaded back");
    let l = m.tool("load_patch", json!({"patch": {"active_drive": 0.5, "listener_scale": 40.0, "output_gain": 5.0}}));
    m.check(l["patch"]["active_drive"] == 0.5 && l["patch"]["space_decay"] == 0.42, "patch object loaded (unlisted fields at plug-in defaults)");
    m.tool_error("load_patch", json!({"slot": "q"}));
    let _ = std::fs::remove_file(&slot_c);
    m.tool("default_patch", json!({}));

    // set_guitar
    let gt = m.tool("set_guitar", json!({"pickup": "neck", "tone": 0.6, "hum": false}));
    m.check(gt["pickup"] == "neck" && gt["tone"] == 0.6 && gt["hum"] == false, "guitar set");

    // set_display
    let ds = m.tool("set_display", json!({"upper": "scope", "time_div_ms": 4, "amp_div": 0.1, "square": "xy", "menu": "display", "active": "time_div"}));
    m.check(ds["upper"] == "scope" && ds["time_div_ms"] == 5.0 && ds["square"] == "xy", "display set (time/div to the nearest step)");
    m.check(ds["menu"]["id"] == "display" && ds["menu"]["active"]["id"] == "time_div", "menu and knob function");
    m.tool_error("set_display", json!({"square": "circle"}));

    // transport
    let t = m.tool("transport", json!({"composition": "blues", "tempo": 104}));
    m.check(t["composition"] == "blues" && t["tempo"] == 104.0 && t["playing"] == true, "Blues at 104");
    pause(700);
    let s = m.tool("get_state", json!({}));
    m.check(s["transport"]["key"] == "E" || s["transport"]["key"] == "A", "the blues is in E or A");
    m.tool_error("transport", json!({"composition": "polka"}));

    // pluck, strum, mute_strings, reset_world
    let pk = m.tool("pluck", json!({"string": "B", "fret": 8, "velocity": 0.9, "bend": 2, "vibrato": 0.2}));
    m.check(pk["plucked"]["midi"] == 67, "B string fret 8 is G4");
    let st = m.tool("strum", json!({"chord": "Em9", "spread_ms": 90, "velocity": 0.7}));
    m.check(st["spells"].as_str().is_some(), "strum spells a chord");
    m.tool_error("strum", json!({"chord": "H7"}));
    m.tool("mute_strings", json!({"strings": [0, "A"]}));
    m.tool("mute_strings", json!({}));
    m.tool("reset_world", json!({}));

    // record
    let r = m.tool("record", json!({"action": "start"}));
    m.check(r["recording"] == true, "recording");
    pause(1500);
    let r = m.tool("record", json!({"action": "stop"}));
    let take = r["last_take"]["take_dir"].as_str().unwrap().to_string();
    let wet = std::fs::metadata(std::path::Path::new(&take).join("wet.wav")).map_or(0, |m| m.len());
    m.check(r["recording"] == false && wet > 48_000 * 8, "a take of more than a second");
    println!("  take {take} ({wet} bytes wet)");

    // play_score: a one-shot that hands back to the blues
    let score = json!({
        "name": "e2e", "tempo": 120, "then": "resume",
        "events": [
            {"type": "strum", "beat": 0, "chord": "Am7", "spread_ms": 60, "velocity": 0.7},
            {"type": "pluck", "beat": 1, "string": "e", "fret": 8},
            {"type": "bend", "beat": 1.25, "string": "e", "semitones": 2},
            {"type": "vibrato", "beat": 1.75, "string": 5, "depth": 0.2},
            {"type": "strum", "beat": 2, "chord": "x-x-3-2-1-0", "dir": "up"},
            {"type": "mute", "beat": 3.5}
        ]
    });
    let sc = m.tool("play_score", json!({"score": score}));
    m.check(sc["events_written"] == 6 && sc["length_beats"] == 4.0 && sc["seconds"] == 2.0, "score compiled: 6 events, one 2 s bar");
    let s = m.tool("get_state", json!({}));
    m.check(s["transport"]["composition"] == "score", "playing the score");
    pause(2600);
    let s = m.tool("get_state", json!({}));
    m.check(s["transport"]["composition"] == "blues", "the score ended and the blues resumed");
    let e = m.tool_error("play_score", json!({"score": {"events": [{"type": "pluck", "beat": 0, "string": 1, "fret": 40}]}}));
    m.check(e.contains("events[0]") && e.contains("fret 40"), "score errors name the event and field");
    let sc = m.tool("play_score", json!({"score": "{\"loop\": true, \"tempo\": 70, \"events\": [{\"type\": \"pluck\", \"beat\": 0, \"string\": 0}, {\"type\": \"rest\", \"beat\": 0, \"beats\": 8}]}"}));
    m.check(sc["loop"] == true && sc["length_beats"] == 12.0, "a looping score sent as JSON text");

    // A screen worth looking at, then the screenshot.
    m.tool("transport", json!({"composition": "wander", "tempo": 92}));
    m.tool("set_display", json!({"upper": "spectrum", "lower": "tab", "square": "chord", "menu": "output", "active": "master", "hold": true}));
    m.tool("set_params", json!({"params": {"organization": 0.22, "active_drive": 0.7}}));
    pause(2500);
    let r = m.request("tools/call", json!({"name": "screenshot", "arguments": {"path": out.join("ovillo-e2e.png").to_str().unwrap()}}));
    let res = &r["result"];
    m.check(res["isError"] == false, &format!("screenshot: {res}"));
    m.check(res["content"][0]["type"] == "image" && res["content"][0]["mimeType"] == "image/png", "image content");
    let meta: Value = serde_json::from_str(res["content"][1]["text"].as_str().unwrap()).unwrap();
    m.check(meta["pid"] == pid, "the window of the instance on this socket");
    let b64 = res["content"][0]["data"].as_str().unwrap();
    m.check(b64.starts_with("iVBORw0KGgo"), "PNG data");
    m.check(meta["width"].as_u64().unwrap() <= 1240, "downscaled");
    println!("  screenshot     ok  {} ({}x{}, window {})", meta["saved"], meta["width"], meta["height"], meta["window_id"]);

    m.tool_error("no_such_tool", json!({}));
    // Leave it tidy and muted.
    m.tool("set_display", json!({"menu": "main", "active": null, "hold": false}));
    m.tool("set_output", json!({"master_db": 0, "mute": true}));
    m.tool("set_guitar", json!({"pickup": "bridge", "tone": 0.85, "hum": true}));
    drop(m.stdin);
    let _ = m.child.wait();
    println!("all {} checks passed", m.checks);
}
