//! The bow on the Ariadne steel-string, from a score (SPEC-TAB.md §4 `arco`,
//! SPEC-ARIADNE-STEEL-STRING.md "The bow"): the two demo scores render,
//! their books close with the bow's work inside, the trace carries the
//! stick-slip signature, and an unbowed score is untouched.

use std::path::{Path, PathBuf};
use std::process::Command;

fn render(score: &Path, tag: &str) -> (serde_json::Value, serde_json::Value) {
    let root = std::env::temp_dir().join(format!("mus-ariadne-bow-{tag}-{}", std::process::id()));
    let (wav, energy) = (
        root.with_extension("wav"),
        root.with_extension("energy.json"),
    );
    let result = Command::new(env!("CARGO_BIN_EXE_mus"))
        .args(["render", score.to_str().unwrap(), "-o"])
        .arg(&wav)
        .arg("--energy")
        .arg(&energy)
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let receipt = serde_json::from_slice(&result.stdout).unwrap();
    let trace = serde_json::from_slice(&std::fs::read(&energy).unwrap()).unwrap();
    std::fs::remove_file(wav).unwrap();
    std::fs::remove_file(energy).unwrap();
    (receipt, trace)
}

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../../examples/tab/{name}.mus"))
}

/// Releases per second between `from` and `to` seconds of the render, from the
/// trace's per-hop bow columns, and the stuck fraction over the same span.
fn regime(trace: &serde_json::Value, from: f64, to: f64) -> (f64, f64) {
    let world = &trace["worlds"][0];
    let hop = world["hopSeconds"].as_f64().unwrap();
    let start = world["startSeconds"].as_f64().unwrap();
    let columns: Vec<&str> = world["bowColumns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    let (stuck, releases) = (
        columns.iter().position(|c| *c == "stuckFraction").unwrap(),
        columns.iter().position(|c| *c == "releases").unwrap(),
    );
    let rows: Vec<&Vec<serde_json::Value>> = world["bowFrames"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            let t = start + *i as f64 * hop;
            t >= from && t < to
        })
        .map(|(_, r)| r.as_array().unwrap())
        .collect();
    let n = rows.len() as f64;
    (
        rows.iter().map(|r| r[stuck].as_f64().unwrap()).sum::<f64>() / n,
        rows.iter()
            .map(|r| r[releases].as_f64().unwrap())
            .sum::<f64>()
            / (n * hop),
    )
}

#[test]
fn a_bowed_note_holds_and_its_work_is_in_the_books() {
    let (receipt, trace) = render(&example("bowed-note"), "note");
    assert_eq!(receipt["badTokens"], serde_json::json!([]));
    let world = &receipt["ariadneWorlds"][0];
    assert_eq!(world["plucks"], 0);
    let bow = &world["bow"];
    assert_eq!(bow["bowedNotes"], 1);
    assert_eq!(bow["strings"][0]["string"], 3);
    assert_eq!(bow["strings"][0]["hand"], "gt.b");
    assert_eq!(bow["strings"][0]["pos"], 0.12);
    assert_eq!(bow["bowedSeconds"], 6.0);
    let (source, friction) = (
        bow["sourceWork"].as_f64().unwrap(),
        bow["frictionLoss"].as_f64().unwrap(),
    );
    assert!(source > 0.0 && friction > 0.0 && friction < source);
    // The arm is the world's only source (the silent contact port returns a
    // little); its heat is inside the world's loss.
    let world_source = world["sourceWork"].as_f64().unwrap();
    assert!(
        (world_source - source).abs() < 1.0e-4 * source,
        "{world_source} vs {source}"
    );
    assert!(world["internalLoss"].as_f64().unwrap() > friction);
    assert!(
        world["cumulativeClosureResidual"].as_f64().unwrap().abs() < 1.0e-15,
        "the ledger closes: {}",
        world["cumulativeClosureResidual"]
    );
    assert!(world["maximumFrameResidual"].as_f64().unwrap() < 1.0e-18);
    // Helmholtz motion once the attack has settled: stuck most of the period,
    // one release per period of G3 (196 Hz).
    let (stuck, releases) = regime(&trace, 1.0, 6.0);
    assert!((0.8..0.9).contains(&stuck), "{stuck}");
    assert!((releases / 195.998 - 1.0).abs() < 0.03, "{releases}/s");
    let (lifted, _) = regime(&trace, 7.0, 10.0);
    assert_eq!(lifted, 0.0, "the bow is up");
    let touches = trace["worlds"][0]["touches"].as_array().unwrap();
    assert_eq!(touches[0]["kind"], "bow");
    assert_eq!(touches[0]["bow"]["direction"], "down");
}

#[test]
fn a_swell_rises_and_falls_with_the_bow() {
    let (receipt, trace) = render(&example("bowed-swell"), "swell");
    assert_eq!(receipt["badTokens"], serde_json::json!([]));
    let world = &receipt["ariadneWorlds"][0];
    let bow = &world["bow"];
    assert_eq!(bow["bowedNotes"], 2);
    assert_eq!(bow["bowedSeconds"], 16.0);
    assert!(world["cumulativeClosureResidual"].as_f64().unwrap().abs() < 1.0e-15);
    let touches = trace["worlds"][0]["touches"].as_array().unwrap();
    assert_eq!(touches[0]["bow"]["direction"], "down");
    assert_eq!(touches[0]["bow"]["speed"], serde_json::json!([0.2, 1.0]));
    assert_eq!(touches[1]["bow"]["direction"], "up");
    assert_eq!(touches[1]["bow"]["speed"], serde_json::json!([1.0, 0.2]));
    // The string's stored energy follows the bow: up through the down-bow,
    // down through the up-bow (string/4 is the bowed body).
    let w = &trace["worlds"][0];
    let channel = w["channels"]
        .as_array()
        .unwrap()
        .iter()
        .position(|c| c == "string/4")
        .unwrap();
    let energy_at = |t: f64| -> f64 {
        let hop = w["hopSeconds"].as_f64().unwrap();
        let i = ((t - w["startSeconds"].as_f64().unwrap()) / hop) as usize;
        w["frames"][i][channel].as_f64().unwrap()
    };
    let (e2, e5, e8, e11, e14) = (
        energy_at(2.0),
        energy_at(5.0),
        energy_at(8.0),
        energy_at(11.0),
        energy_at(14.0),
    );
    assert!(e2 < e5 && e5 < e8, "rising: {e2:e} {e5:e} {e8:e}");
    assert!(e8 > e11 && e11 > e14, "falling: {e8:e} {e11:e} {e14:e}");
    // One release per period of D3 (146.8 Hz) through the middle of each bow.
    for (from, to) in [(2.0, 7.5), (8.5, 14.0)] {
        let (stuck, releases) = regime(&trace, from, to);
        assert!((0.8..0.9).contains(&stuck), "{stuck}");
        assert!(
            (releases / 146.832 - 1.0).abs() < 0.03,
            "{from}-{to} s: {releases}/s"
        );
    }
}

#[test]
fn an_unbowed_score_is_untouched() {
    let (receipt, trace) = render(&example("steel_string_tour"), "tour");
    assert_eq!(
        receipt["renderDigest"],
        "sha256:fa3d615fe9dca59ce13c2d7b92c7248ed8f56fe634a33e7734b08f9f5a0ffbfc",
        "the tour's bytes before the bow existed (SPEC-ARIADNE-PIANO.md §9.1)"
    );
    let world = &receipt["ariadneWorlds"][0];
    assert!(world.get("bow").is_none(), "no bow, no bow entry");
    assert!(trace["worlds"][0].get("bows").is_none());
    assert!(trace["worlds"][0].get("bowFrames").is_none());
}
