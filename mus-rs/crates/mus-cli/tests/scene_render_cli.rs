//! CLI smoke test for `mus scene-render` (PW7), the same shape as
//! `render_receipt.rs`'s coverage of `mus render`: spawn the real binary,
//! point it at a golden fixture scene, and check the printed receipt and
//! the written WAV file — not a second copy of `mus-dsp`'s own golden
//! numeric-parity check (that's `mus-dsp/tests/golden_scene.rs`'s job),
//! but proof the CLI wiring (arg parsing, `--root`, `-o`, `--foa`, the
//! receipt-JSON-on-stdout contract, refusal-as-JSON) actually works end
//! to end.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/scene")
}

#[test]
fn scene_render_writes_wav_and_prints_receipt_matching_the_manifest() {
    let scene = fixtures_dir().join("static_room.scene.json");
    let root = fixtures_dir();
    let output = std::env::temp_dir().join(format!(
        "mus-pw7-cli-static-room-{}.wav",
        std::process::id()
    ));

    let result = Command::new(env!("CARGO_BIN_EXE_mus"))
        .arg("scene-render")
        .arg(&scene)
        .arg("--root")
        .arg(&root)
        .arg("-o")
        .arg(&output)
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let receipt: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(receipt["renderer"], "mus.spatial-stereo/1");
    assert_eq!(receipt["target"], "stereo-equalpower-plus-diffuse");
    assert_eq!(receipt["channels"], 2);
    assert_eq!(receipt["sample_rate"], 48_000);
    // safety_gain_db, snake_case — see `scene.rs`'s `RenderReceipt` doc
    // comment for why this top-level receipt is snake_case while
    // `object_receipts` entries inside it are camelCase.
    assert!((receipt["safety_gain_db"].as_f64().unwrap() - 0.0).abs() < 1e-6);
    assert_eq!(receipt["object_receipts"].as_array().unwrap().len(), 3);

    let manifest_text =
        std::fs::read_to_string(fixtures_dir().join("manifest.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest_text).unwrap();
    let entry = manifest
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == "static_room")
        .unwrap();
    assert_eq!(receipt["scene_id"], "fx-static");
    assert_eq!(receipt["renderer"], entry["renderer"]);

    let wav_bytes = std::fs::metadata(&output).expect("wav file exists").len();
    // 44-ish byte header (hound's float WAV) + frames*channels*4 data
    // bytes; a loose floor is enough to prove *something* real was
    // written, not an empty/truncated file.
    let frames = entry["frames"].as_u64().unwrap();
    assert!(
        wav_bytes >= frames * 2 * 4,
        "wav file too small: {wav_bytes} bytes for {frames} stereo frames"
    );

    std::fs::remove_file(&output).ok();
}

#[test]
fn scene_render_foa_writes_four_channel_wav() {
    let scene = fixtures_dir().join("static_room.scene.json");
    let root = fixtures_dir();
    let output = std::env::temp_dir().join(format!(
        "mus-pw7-cli-static-room-foa-{}.wav",
        std::process::id()
    ));

    let result = Command::new(env!("CARGO_BIN_EXE_mus"))
        .arg("scene-render")
        .arg(&scene)
        .arg("--root")
        .arg(&root)
        .arg("-o")
        .arg(&output)
        .arg("--foa")
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(receipt["renderer"], "mus.foa-ambix-encoder/1");
    assert_eq!(receipt["channels"], 4);
    assert_eq!(
        receipt["channel_order"],
        serde_json::json!(["W", "Y", "Z", "X"])
    );

    std::fs::remove_file(&output).ok();
}

/// Missing `--root` is a usage error (nonzero exit), not a panic — PW7's
/// CLI treats it as required, matching the spec's bracket-free `--root
/// <dir>` in `mus scene-render <scene.json> --root <dir> [-o out.wav]
/// [--foa]`.
#[test]
fn scene_render_without_root_is_a_usage_error() {
    let scene = fixtures_dir().join("static_room.scene.json");
    let result = Command::new(env!("CARGO_BIN_EXE_mus"))
        .arg("scene-render")
        .arg(&scene)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("--root"));
}

/// An unsupported (non-default) psychoacoustic control refuses with a
/// structured JSON error on stdout, matching `render`'s existing
/// refusal-as-JSON contract — never a bare panic/crash.
#[test]
fn scene_render_refuses_unsupported_control_as_structured_json() {
    let dir = std::env::temp_dir().join(format!(
        "mus-pw7-cli-refusal-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    write_tiny_wav(&dir.join("tone.wav"), &[1000i16; 4800]);

    let scene_json = serde_json::json!({
        "sceneId": "cli-refusal-scene",
        "title": "refusal",
        "sampleRate": 48000,
        "durationSeconds": 0.1,
        "objects": [{
            "objectId": "o",
            "label": "o",
            "audio": "tone.wav",
            "controls": {"brightnessDb": 3.0},
        }],
    });
    let scene_path = dir.join("scene.json");
    std::fs::write(&scene_path, serde_json::to_string(&scene_json).unwrap()).unwrap();
    let output = dir.join("out.wav");

    let result = Command::new(env!("CARGO_BIN_EXE_mus"))
        .arg("scene-render")
        .arg(&scene_path)
        .arg("--root")
        .arg(&dir)
        .arg("-o")
        .arg(&output)
        .arg("--json")
        .output()
        .unwrap();
    assert!(!result.status.success());
    let body: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(body["schema"], "mus.spatial-render-error.v1");
    assert!(
        body["error"].as_str().unwrap().contains("unsupported-control"),
        "{body}"
    );
    assert!(!output.exists(), "refused render must not write a WAV file");

    std::fs::remove_dir_all(&dir).ok();
}

/// Minimal hand-written mono 16-bit PCM WAV — same helper shape as
/// `render_receipt.rs`'s `write_tiny_wav`, duplicated locally rather than
/// shared across integration-test binaries (each `tests/*.rs` file is
/// its own crate).
fn write_tiny_wav(path: &std::path::Path, samples: &[i16]) {
    let data_bytes = samples.len() * 2;
    let mut buf = Vec::with_capacity(44 + data_bytes);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&((36 + data_bytes) as u32).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&48_000u32.to_le_bytes());
    buf.extend_from_slice(&(48_000u32 * 2).to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes());
    buf.extend_from_slice(&16u16.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&(data_bytes as u32).to_le_bytes());
    for &s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, buf).unwrap();
}
