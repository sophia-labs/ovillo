from __future__ import annotations

from pathlib import Path


def read(path: str) -> str:
    return Path(path).read_text(encoding="utf-8")


def write(path: str, text: str) -> None:
    Path(path).write_text(text, encoding="utf-8")


def replace_once(path: str, old: str, new: str) -> None:
    text = read(path)
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one replacement target, found {count}")
    write(path, text.replace(old, new, 1))


# 1. Make the registered MUS-T key boundary available to the ordinary event
# renderer.  This is the guard that keeps MUS-A arrows such as pan/lpf out of
# the trajectory parser.
replace_once(
    "mus-rs/crates/mus-dsp/src/trajectory.rs",
    '''fn coordinate_for_inline_key(key: &str) -> Option<&'static str> {
    Some(match key {
        "couple" => "string.couple",
        "chirality" => "string.chirality",
        "orbit" => "string.orbit_hz",
        "orbit_depth" => "string.orbit_depth",
        "curvature" => "string.curvature",
        "body" => "string.body",
        "buzz" => "string.buzz",
        "sing" => "string.sing",
        "bow" => "string.bow",
        "bow_speed" => "string.bow_speed",
        "sus" => "string.sus",
        "damp" => "string.damp",
        "breath" => "string.breath",
        "pitch_ratio" => "string.pitch_ratio",
        _ => return None,
    })
}
''',
    '''fn coordinate_for_inline_key(key: &str) -> Option<&'static str> {
    Some(match key {
        "couple" => "string.couple",
        "chirality" => "string.chirality",
        "orbit" => "string.orbit_hz",
        "orbit_depth" => "string.orbit_depth",
        "curvature" => "string.curvature",
        "body" => "string.body",
        "buzz" => "string.buzz",
        "sing" => "string.sing",
        "bow" => "string.bow",
        "bow_speed" => "string.bow_speed",
        "sus" => "string.sus",
        "damp" => "string.damp",
        "breath" => "string.breath",
        "pitch_ratio" => "string.pitch_ratio",
        _ => return None,
    })
}

/// Whether an event parameter belongs to the registered inline string
/// trajectory surface.  Per-coordinate curve selectors are admitted only when
/// their base coordinate is registered; unrelated MUS-A arrows remain on the
/// historical transform path.
pub fn is_inline_string_trajectory_key(key: &str) -> bool {
    if matches!(key, "traj_curve" | "traj_hz" | "traj_strict") {
        return true;
    }
    if coordinate_for_inline_key(key).is_some() {
        return true;
    }
    key.strip_suffix("_curve")
        .is_some_and(|base| coordinate_for_inline_key(base).is_some())
}
''',
)

# 2. Route ordinary pluck/weave events through the live adapter when a
# registered trajectory is present.  No-trajectory events retain the exact
# historical path.  External sung-source execution is refused because the
# current adapter does not yet carry that source/work ledger.
replace_once(
    "mus-rs/crates/mus-engine/src/source.rs",
    '''use mus_dsp::pluck::{guitar_note_sung, pluck_note, weave_note, weave_note_sung};
''',
    '''use mus_dsp::pluck::{guitar_note_sung, pluck_note, weave_note, weave_note_sung};
use mus_dsp::trajectory::{
    is_inline_string_trajectory_key, program_from_inline_string_params,
    render_guitar_program, render_weave_program, StringTrajectoryReceipt,
};
''',
)
replace_once(
    "mus-rs/crates/mus-engine/src/source.rs",
    '''pub enum SourceStep {
    Rendered(Vec<f32>),
    NeedsPitch,
    TooShort,
}
''',
    '''pub enum SourceStep {
    Rendered(Vec<f32>),
    RenderedTrajectory {
        samples: Vec<f32>,
        receipt: StringTrajectoryReceipt,
    },
    NeedsPitch,
    TooShort,
}
''',
)
replace_once(
    "mus-rs/crates/mus-engine/src/source.rs",
    '''        if SYNTH_KEYS.contains(&key.as_str()) {
            patch_map.insert(key.clone(), value.clone());
        }
''',
    '''        if SYNTH_KEYS.contains(&key.as_str()) || is_inline_string_trajectory_key(key) {
            patch_map.insert(key.clone(), value.clone());
        }
''',
)
replace_once(
    "mus-rs/crates/mus-engine/src/source.rs",
    '''    let synth_name = patch_map
        .get("synth")
        .map_or("osc", String::as_str)
        .to_ascii_lowercase();
    let sing = float_param(&patch_map, "sing", 0.0);
''',
    '''    let (patch_map, trajectory_program) =
        program_from_inline_string_params(&patch_map, slot_s)
            .map_err(|error| ctx(RenderError::Invalid(format!("trajectory: {error}"))))?;

    let synth_name = patch_map
        .get("synth")
        .map_or("osc", String::as_str)
        .to_ascii_lowercase();
    let sing = float_param(&patch_map, "sing", 0.0);
''',
)
replace_once(
    "mus-rs/crates/mus-engine/src/source.rs",
    '''    let mut x = match (synth_name.as_str(), singing) {
        ("pluck", true) => guitar_note_sung(
            &patch_map,
            &freqs,
            slot_s,
            1.0,
            sung_audio.as_deref().expect("sing source loaded"),
            sing,
        ),
        ("pluck", false) => pluck_note(&patch_map, &freqs, slot_s, 1.0),
        ("weave", true) => weave_note_sung(
            &patch_map,
            &freqs,
            slot_s,
            1.0,
            sung_audio.as_deref().expect("sing source loaded"),
            sing,
        ),
        ("weave", false) => weave_note(&patch_map, &freqs, slot_s, 1.0),
        ("pluck" | "weave", _) => unreachable!(),
        _ => synth_note(&freqs, slot_s, &patch_map),
    };
''',
    '''    let mut trajectory_receipt = None;
    let mut x = if let Some(program) = trajectory_program.as_ref() {
        if singing || patch_map.contains_key("sing_audio") {
            return Err(ctx(RenderError::Invalid(
                "trajectory execution with an external sung source is not yet work-accounted"
                    .to_string(),
            )));
        }
        let rendered = match synth_name.as_str() {
            "pluck" => render_guitar_program(&patch_map, &freqs, slot_s, 1.0, program, 256),
            "weave" => render_weave_program(&patch_map, &freqs, slot_s, 1.0, program, 256),
            other => {
                return Err(ctx(RenderError::Invalid(format!(
                    "trajectory controls require synth=pluck or synth=weave, got {other:?}"
                ))));
            }
        }
        .map_err(|error| ctx(RenderError::Invalid(format!("trajectory: {error}"))))?;
        trajectory_receipt = Some(rendered.receipt);
        rendered.audio
    } else {
        match (synth_name.as_str(), singing) {
            ("pluck", true) => guitar_note_sung(
                &patch_map,
                &freqs,
                slot_s,
                1.0,
                sung_audio.as_deref().expect("sing source loaded"),
                sing,
            ),
            ("pluck", false) => pluck_note(&patch_map, &freqs, slot_s, 1.0),
            ("weave", true) => weave_note_sung(
                &patch_map,
                &freqs,
                slot_s,
                1.0,
                sung_audio.as_deref().expect("sing source loaded"),
                sing,
            ),
            ("weave", false) => weave_note(&patch_map, &freqs, slot_s, 1.0),
            ("pluck" | "weave", _) => unreachable!(),
            _ => synth_note(&freqs, slot_s, &patch_map),
        }
    };
''',
)
replace_once(
    "mus-rs/crates/mus-engine/src/source.rs",
    '''    Ok(SourceStep::Rendered(x))
}
''',
    '''    Ok(match trajectory_receipt {
        Some(receipt) => SourceStep::RenderedTrajectory {
            samples: x,
            receipt,
        },
        None => SourceStep::Rendered(x),
    })
}
''',
)

# 3. Make trajectory execution a first-class event receipt on ordinary render
# statistics.  This metadata is omitted from CLI JSON when empty, preserving
# existing no-trajectory receipts.
replace_once(
    "mus-rs/crates/mus-engine/src/lib.rs",
    '''use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use mus_notation::parser;
''',
    '''use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use mus_dsp::trajectory::StringTrajectoryReceipt;
use mus_notation::parser;
use serde::Serialize;
''',
)
replace_once(
    "mus-rs/crates/mus-engine/src/lib.rs",
    '''#[derive(Debug, Clone, Default)]
pub struct RenderStats {
    pub events: Vec<EventStats>,
''',
    '''#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTrajectoryReceipt {
    pub bar: u32,
    pub track: String,
    pub onset_seconds: f64,
    pub slot_seconds: f64,
    pub receipt: StringTrajectoryReceipt,
}

#[derive(Debug, Clone, Default)]
pub struct RenderStats {
    pub events: Vec<EventStats>,
    pub trajectory_receipts: Vec<EventTrajectoryReceipt>,
''',
)
replace_once(
    "mus-rs/crates/mus-engine/src/events.rs",
    '''use crate::{
    analysis_root, bar_times, end_bar_time, pack, source, EventStats, InstrumentDef,
    ParsedScore, RenderError, RenderStats, SAMPLE_RATE,
};
''',
    '''use crate::{
    analysis_root, bar_times, end_bar_time, pack, source, EventStats, EventTrajectoryReceipt,
    InstrumentDef, ParsedScore, RenderError, RenderStats, SAMPLE_RATE,
};
''',
)
replace_once(
    "mus-rs/crates/mus-engine/src/events.rs",
    '''        let mut x = match step {
            source::SourceStep::Rendered(x) => x,
            source::SourceStep::NeedsPitch => {
''',
    '''        let mut x = match step {
            source::SourceStep::Rendered(x) => x,
            source::SourceStep::RenderedTrajectory { samples, receipt } => {
                stats.trajectory_receipts.push(EventTrajectoryReceipt {
                    bar: *b,
                    track: abbr.clone(),
                    onset_seconds: onset,
                    slot_seconds: slot,
                    receipt,
                });
                samples
            }
            source::SourceStep::NeedsPitch => {
''',
)

# 4. Join event receipts to the ordinary `mus render --receipt` JSON surface.
replace_once(
    "mus-rs/crates/mus-cli/src/main.rs",
    '''use mus_engine::{parse_and_render, RenderError, RenderStats, SAMPLE_RATE};
''',
    '''use mus_engine::{
    parse_and_render, EventTrajectoryReceipt, RenderError, RenderStats, SAMPLE_RATE,
};
''',
)
replace_once(
    "mus-rs/crates/mus-cli/src/main.rs",
    '''    #[serde(rename = "eventCount")]
    event_count: usize,
''',
    '''    #[serde(rename = "eventCount")]
    event_count: usize,
    #[serde(rename = "trajectoryReceipts", skip_serializing_if = "Vec::is_empty")]
    trajectory_receipts: Vec<EventTrajectoryReceipt>,
''',
)
replace_once(
    "mus-rs/crates/mus-cli/src/main.rs",
    '''            event_count: stats.events.len(),
            seed: stats.seed,
''',
    '''            event_count: stats.events.len(),
            trajectory_receipts: stats.trajectory_receipts.clone(),
            seed: stats.seed,
''',
)

# 5. End-to-end CLI acceptance.  It renders the same score twice, proves the
# new field is present only for the trajectory event, and checks receipt
# identity rather than merely audio existence.
test_path = Path("mus-rs/crates/mus-cli/tests/trajectory_receipt.rs")
if test_path.exists():
    raise SystemExit(f"{test_path}: already exists")
test_path.write_text(
    r'''use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        path.push(format!(
            "mus-cli-trajectory-receipt-{}-{nanos}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp directory");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn render(root: &Path, stem: &str) -> serde_json::Value {
    let input = root.join("trajectory.mus");
    let output = root.join(format!("{stem}.wav"));
    let receipt = root.join(format!("{stem}.receipt.json"));
    let status = Command::new(env!("CARGO_BIN_EXE_mus"))
        .arg("render")
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .arg("--receipt")
        .arg(&receipt)
        .status()
        .expect("run mus render");
    assert!(status.success(), "trajectory render should succeed");
    assert!(output.exists(), "rendered wave exists");
    serde_json::from_slice(&fs::read(receipt).expect("read receipt")).expect("receipt json")
}

#[test]
fn ordinary_render_emits_deterministic_trajectory_receipt() {
    let root = TempDir::new();
    fs::write(
        root.path().join("trajectory.mus"),
        r#"# score: trajectory receipt
# tempo: 120
# time: 4/4
# key: C
# bars: 1
# instruments:
#   w = weave (treble, synth=weave, courses=3, couple=0.10, body=0.25, sus=0.4)

bar 1: w=A3q[couple=0.10@0->0.30@1,traj_curve=smoothstep,traj_hz=200] Rq Rh
"#,
    )
    .expect("write score");

    let first = render(root.path(), "first");
    let second = render(root.path(), "second");
    let first_receipts = first["trajectoryReceipts"]
        .as_array()
        .expect("trajectory receipts array");
    let second_receipts = second["trajectoryReceipts"]
        .as_array()
        .expect("trajectory receipts array");
    assert_eq!(first_receipts.len(), 1);
    assert_eq!(first_receipts, second_receipts, "receipt identity is deterministic");

    let event = &first_receipts[0];
    assert_eq!(event["bar"], 1);
    assert_eq!(event["track"], "w");
    assert_eq!(event["onsetSeconds"], 0.0);
    assert_eq!(event["slotSeconds"], 0.5);
    assert_eq!(event["receipt"]["adapter_version"], "mus-string-trajectory/v1");
    assert!(event["receipt"]["control_frame_count"]
        .as_u64()
        .expect("control frame count")
        > 0);
    assert!(event["receipt"]["trajectory"]["program_digest"]
        .as_str()
        .expect("program digest")
        .starts_with("sha256:"));
}
''',
    encoding="utf-8",
)

print("MUS-T ordinary renderer integration patch applied")
