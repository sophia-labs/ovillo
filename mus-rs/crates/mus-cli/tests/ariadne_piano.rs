//! A score played on the Ariadne piano (SPEC-ARIADNE-PIANO.md): one closed world,
//! the same bytes twice, and honest bad tokens.

use std::path::{Path, PathBuf};
use std::process::Command;

fn render(score: &Path, tag: &str) -> (serde_json::Value, serde_json::Value, Vec<u8>) {
    let root = std::env::temp_dir().join(format!("mus-ariadne-piano-{tag}-{}", std::process::id()));
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
    let bytes = std::fs::read(&wav).unwrap();
    std::fs::remove_file(wav).unwrap();
    std::fs::remove_file(energy).unwrap();
    (receipt, trace, bytes)
}

fn score(dir: &Path, name: &str, body: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(name);
    std::fs::write(
        &path,
        format!(
            "# score: piano test\n# tempo: 240    time: 4/4    bars: 1\n# instruments:\n\
             #   pn = grand piano (treble, anatomy=piano-v1)\n#   pn.r = right hand (treble)\n\
             #   pn.ped = sustain pedal (perc, pedal=sustain)\n{body}\n"
        ),
    )
    .unwrap();
    path
}

#[test]
fn a_piano_score_plays_on_one_closed_world_and_the_same_bytes_twice() {
    let dir = std::env::temp_dir().join(format!("mus-ariadne-piano-play-{}", std::process::id()));
    let path = score(
        &dir,
        "play.mus",
        "bar 1: pn.r={f} C4e E4e {pp} <C4 G4>q[stac] Rh  pn.ped=Rq Xq Rh",
    );
    let (receipt, trace, bytes) = render(&path, "a");
    assert_eq!(receipt["badTokens"], serde_json::json!([]));
    let world = &receipt["ariadneWorlds"][0];
    assert_eq!(world["anatomy"], "piano-v1");
    assert_eq!(world["strikes"].as_u64(), Some(4));
    assert_eq!(world["refusedStrikes"].as_u64(), Some(0));
    assert_eq!(world["pedalSpans"].as_u64(), Some(1));
    let keys: Vec<&str> = world["keys"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k["name"].as_str().unwrap())
        .collect();
    assert_eq!(keys, ["C4", "E4", "G4"]);
    assert!(world["keys"]
        .as_array()
        .unwrap()
        .iter()
        .all(|k| k["strings"] == 3 && k["damped"] == true));
    assert!(
        world["cumulativeClosureResidual"].as_f64().unwrap().abs() < 1e-12,
        "the ledger closes"
    );
    assert!(world["maximumFrameResidual"].as_f64().unwrap() < 1e-15);
    assert!(
        world["boardRadiation"].as_f64().unwrap()
            > 100.0 * world["stringRadiation"].as_f64().unwrap(),
        "the board speaks"
    );
    assert!(
        world["contactMsMin"].as_f64().unwrap() > 0.5
            && world["contactMsMax"].as_f64().unwrap() < 5.0
    );
    let worlds = &trace["worlds"][0];
    let bodies = world["bodies"].as_array().unwrap();
    assert_eq!(
        worlds["channels"].as_array().unwrap().len(),
        bodies.len() + 1,
        "every body, then the room"
    );
    let strikes = worlds["strikes"].as_array().unwrap();
    assert_eq!(strikes.len(), 4);
    let (loud, soft) = (&strikes[0], &strikes[2]);
    assert!(loud["hammerVelocity"].as_f64().unwrap() > soft["hammerVelocity"].as_f64().unwrap());
    assert!(
        loud["contactMs"].as_f64().unwrap() < soft["contactMs"].as_f64().unwrap(),
        "a harder blow touches for less time"
    );
    // The same score twice: the same bytes.
    let (again, _, bytes_again) = render(&path, "b");
    assert_eq!(receipt["renderDigest"], again["renderDigest"]);
    assert_eq!(bytes, bytes_again);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_pitch_the_piano_has_no_key_for_is_reported() {
    let dir = std::env::temp_dir().join(format!("mus-ariadne-piano-bad-{}", std::process::id()));
    let path = score(&dir, "bad.mus", "bar 1: pn.r=C4q C9q A3q@3:2 Rq  pn.ped=-");
    let (receipt, _, _) = render(&path, "bad");
    let bad = receipt["badTokens"].as_array().unwrap();
    assert_eq!(bad.len(), 2, "{bad:?}");
    assert!(
        bad[0].as_str().unwrap().contains("not a key of this piano"),
        "{bad:?}"
    );
    assert!(
        bad[1].as_str().unwrap().contains("not @string:fret"),
        "{bad:?}"
    );
    assert_eq!(receipt["ariadneWorlds"][0]["strikes"].as_u64(), Some(1));
    std::fs::remove_dir_all(dir).unwrap();
}
