//! Protocol smoke for `renderScene` (PW7's addition to `mus service`),
//! the same shape as `service_smoke.rs`'s `renderScore` coverage: spawn
//! the real binary, speak NDJSON, hold it to the determinism contract —
//! same `(sceneJson, root)` yields the same `renderDigest` — and check
//! the PCM-envelope response shape matches `renderScore`'s.
//!
//! `ServiceHandle` is duplicated from `service_smoke.rs` rather than
//! shared: each `tests/*.rs` file is its own crate, so there is no
//! `mus-cli`-internal path to reuse it without promoting test-only
//! scaffolding into the binary's own module tree.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct ServiceHandle {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl ServiceHandle {
    fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mus"))
            .arg("service")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn mus service");
        let stdin = child.stdin.take().expect("service stdin");
        let stdout = BufReader::new(child.stdout.take().expect("service stdout"));
        ServiceHandle {
            child,
            stdin,
            stdout,
        }
    }

    fn call(&mut self, id: u64, method: &str, params: serde_json::Value) -> serde_json::Value {
        let request = serde_json::json!({ "id": id, "method": method, "params": params });
        writeln!(self.stdin, "{request}").expect("write request");
        let mut line = String::new();
        self.stdout.read_line(&mut line).expect("read response");
        let response: serde_json::Value = serde_json::from_str(&line).expect("response parses");
        assert_eq!(response["id"], serde_json::json!(id), "response id echoes");
        response
    }
}

impl Drop for ServiceHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/scene")
}

#[test]
fn render_scene_roundtrip_matches_renderscore_envelope_and_is_deterministic() {
    let scene_json =
        std::fs::read_to_string(fixtures_dir().join("static_room.scene.json")).unwrap();
    let root = fixtures_dir();

    let mut service = ServiceHandle::spawn();
    let pong = service.call(1, "ping", serde_json::json!({}));
    assert_eq!(pong["result"]["ok"], serde_json::json!(true));

    let rendered = service.call(
        2,
        "renderScene",
        serde_json::json!({ "sceneJson": scene_json, "root": root.to_string_lossy() }),
    );
    let result = &rendered["result"];
    assert!(result.get("error").is_none(), "{rendered}");

    // Same PCM-envelope shape as `renderScore`.
    let frames = result["frames"].as_u64().expect("frames") as usize;
    let channels = result["channels"].as_u64().expect("channels");
    let sample_rate = result["sampleRate"].as_u64().expect("sampleRate");
    assert_eq!(channels, 2);
    assert_eq!(sample_rate, 48_000);
    assert_eq!(result["layout"], "interleaved-f32le");
    let pcm_path = PathBuf::from(result["pcmPath"].as_str().expect("pcmPath"));
    let bytes = std::fs::metadata(&pcm_path).expect("pcm file exists").len() as usize;
    assert_eq!(bytes, frames * 2 * 4, "interleaved stereo f32le sizing");

    // The attached spatial render receipt.
    assert_eq!(result["receipt"]["renderer"], "mus.spatial-stereo/1");

    // Determinism: identical (sceneJson, root) is a cache hit with the
    // identical digest.
    let digest_a = result["renderDigest"].as_str().expect("digest").to_string();
    let again = service.call(
        3,
        "renderScene",
        serde_json::json!({ "sceneJson": scene_json, "root": root.to_string_lossy() }),
    );
    assert_eq!(
        again["result"]["renderDigest"].as_str().expect("digest"),
        digest_a
    );
}

#[test]
fn render_scene_missing_params_is_a_structured_refusal() {
    let mut service = ServiceHandle::spawn();
    let missing = service.call(1, "renderScene", serde_json::json!({}));
    assert_eq!(missing["error"]["code"], serde_json::json!("bad-request"));
}

#[test]
fn render_scene_unsupported_control_is_a_structured_refusal() {
    let root = fixtures_dir();
    let scene_json = serde_json::json!({
        "sceneId": "svc-refusal-scene",
        "title": "refusal",
        "sampleRate": 48000,
        "durationSeconds": 0.1,
        "objects": [{
            "objectId": "o",
            "label": "o",
            "audio": "stems/sine220.wav",
            "controls": {"pitchSemitones": 2.0},
        }],
    })
    .to_string();

    let mut service = ServiceHandle::spawn();
    let response = service.call(
        1,
        "renderScene",
        serde_json::json!({ "sceneJson": scene_json, "root": root.to_string_lossy() }),
    );
    assert_eq!(
        response["error"]["code"],
        serde_json::json!("unsupported-control"),
        "{response}"
    );
}
