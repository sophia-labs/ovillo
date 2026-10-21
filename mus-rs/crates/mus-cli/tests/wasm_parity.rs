//! The browser engine (`mus-wasm`, run natively here) renders what `mus render`
//! renders: the same WAV digest, the same Ariadne World ledger, the same energy
//! trace, stepped in blocks the way a Worker steps it.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn the_wasm_path_renders_the_tour_as_mus_render_does() {
    let score = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../examples/tab/steel_string_tour.mus");
    let stem = std::env::temp_dir().join(format!("mus-wasm-parity-{}", std::process::id()));
    let (wav, energy) = (
        stem.with_extension("wav"),
        stem.with_extension("energy.json"),
    );
    let out = Command::new(env!("CARGO_BIN_EXE_mus"))
        .args(["render", score.to_str().unwrap(), "-o"])
        .arg(&wav)
        .arg("--energy")
        .arg(&energy)
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let cli: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let cli_energy: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&energy).unwrap()).unwrap();
    let cli_wav = std::fs::read(&wav).unwrap();
    std::fs::remove_file(&wav).unwrap();
    std::fs::remove_file(&energy).unwrap();

    let source = std::fs::read_to_string(&score).unwrap();
    let report = mus_wasm::check(&source);
    assert!(report.clean, "{:#?}", report.diagnostics);
    let mut render = mus_wasm::Render::begin(&source).unwrap_or_else(|e| panic!("{e:?}"));
    // One second of World at a time, as the Worker steps it.
    while !render.step(48_000).unwrap() {}
    let done = render.finish(0.0).unwrap();
    let browser: serde_json::Value = serde_json::from_str(&done.receipt_json()).unwrap();

    assert_eq!(done.wav, cli_wav, "the same WAV bytes");
    assert_eq!(browser["renderDigest"], cli["renderDigest"]);
    assert_eq!(browser["sourceDigest"], cli["sourceDigest"]);
    assert_eq!(
        browser["ariadneWorlds"], cli["ariadneWorlds"],
        "the same Worlds and ledgers"
    );
    for key in [
        "schema",
        "engineVersion",
        "peakDbfs",
        "rmsDbfs",
        "eventCount",
        "voiceCount",
        "durationSeconds",
        "perTrackEvents",
        "badTokens",
        "headSelection",
    ] {
        assert_eq!(browser[key], cli[key], "{key}");
    }
    let world = &browser["ariadneWorlds"][0];
    assert!(
        world["cumulativeClosureResidual"].as_f64().unwrap().abs() < 1e-12,
        "the ledger closes"
    );
    let browser_energy: serde_json::Value = serde_json::from_str(&done.energy_json()).unwrap();
    assert_eq!(browser_energy, cli_energy, "the same energy trace");
    // And the same as before this change (the receipt of `mus render` at origin/main 0fdd48c).
    assert_eq!(
        cli["renderDigest"],
        "sha256:fa3d615fe9dca59ce13c2d7b92c7248ed8f56fe634a33e7734b08f9f5a0ffbfc"
    );
}
