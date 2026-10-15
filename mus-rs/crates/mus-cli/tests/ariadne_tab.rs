//! Annotated tab played on the Ariadne steel-string (SPEC-TAB.md, SPEC-ARIADNE-STEEL-STRING.md).

use std::path::{Path, PathBuf};
use std::process::Command;

fn render(score: &Path, tag: &str) -> (serde_json::Value, serde_json::Value) {
    let root = std::env::temp_dir().join(format!("mus-ariadne-tab-{tag}-{}", std::process::id()));
    let (wav, energy) = (root.with_extension("wav"), root.with_extension("energy.json"));
    let result = Command::new(env!("CARGO_BIN_EXE_mus"))
        .args(["render", score.to_str().unwrap(), "-o"])
        .arg(&wav)
        .arg("--energy")
        .arg(&energy)
        .arg("--json")
        .output()
        .unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    let receipt = serde_json::from_slice(&result.stdout).unwrap();
    let trace = serde_json::from_slice(&std::fs::read(&energy).unwrap()).unwrap();
    std::fs::remove_file(wav).unwrap();
    std::fs::remove_file(energy).unwrap();
    (receipt, trace)
}

#[test]
fn the_tour_plays_every_touch_on_one_closed_world() {
    let score = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../examples/tab/steel_string_tour.mus");
    let (receipt, trace) = render(&score, "tour");
    assert_eq!(receipt["badTokens"], serde_json::json!([]), "every pitch agrees with its string and fret");
    let world = &receipt["ariadneWorlds"][0];
    assert_eq!(world["anatomy"], "steel-string-v0");
    assert_eq!((world["hammerOns"].as_u64(), world["pullOffs"].as_u64(), world["slides"].as_u64()), (Some(2), Some(2), Some(1)));
    assert!(world["plucks"].as_u64().unwrap() > 30);
    assert!(world["cumulativeClosureResidual"].as_f64().unwrap().abs() < 1e-12, "the ledger closes");
    assert!(world["radiation"].as_f64().unwrap() > 0.0 && world["sourceWork"].as_f64().unwrap() > 0.0);
    let strings = world["strings"].as_array().unwrap();
    assert_eq!(strings[0]["hand"], "gt.v");
    assert_eq!(strings[0]["pos"], 0.2);
    assert_eq!(strings[5]["hand"], "gt.p");
    assert_eq!(strings[5]["palmMute"], 0.45);
    assert!(receipt["peakDbfs"].as_f64().unwrap().is_finite());
    let worlds = &trace["worlds"][0];
    let bodies = world["bodies"].as_array().unwrap();
    assert!(bodies.iter().filter(|b| b.as_str().unwrap().starts_with("string/")).count() == 6);
    assert_eq!(worlds["channels"].as_array().unwrap().len(), bodies.len() + 1, "every body, then the room");
    assert!(worlds["frames"].as_array().unwrap().len() > 1000);
}

#[test]
fn a_fret_that_disagrees_with_its_pitch_is_reported() {
    let dir = std::env::temp_dir().join(format!("mus-ariadne-tab-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let score = dir.join("bad.mus");
    std::fs::write(
        &score,
        "# score: wrong fret\n# tempo: 120    time: 4/4    bars: 1\n# instruments:\n\
         #   gt = steel-string guitar (treble, -12, anatomy=steel-string-v0)\n\
         #   gt.v = fingers (treble, pos=0.2)\nbar 1: gt.v=A3q@3:3 Rh.\n",
    )
    .unwrap();
    let (receipt, _) = render(&score, "bad");
    let bad = receipt["badTokens"].as_array().unwrap();
    assert_eq!(bad.len(), 1);
    assert!(bad[0].as_str().unwrap().contains("string 3 fret 3"), "{bad:?}");
    std::fs::remove_dir_all(dir).unwrap();
}
