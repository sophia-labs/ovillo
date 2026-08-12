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


def insert_before_once(path: str, marker: str, insertion: str) -> None:
    text = read(path)
    count = text.count(marker)
    if count != 1:
        raise SystemExit(f"{path}: expected one insertion marker, found {count}")
    write(path, text.replace(marker, insertion + marker, 1))


def replace_region(path: str, start: str, end: str, replacement: str) -> None:
    text = read(path)
    start_count = text.count(start)
    end_count = text.count(end)
    if start_count != 1 or end_count != 1:
        raise SystemExit(
            f"{path}: region markers start={start_count} end={end_count}"
        )
    left, rest = text.split(start, 1)
    _middle, right = rest.split(end, 1)
    write(path, left + replacement + end + right)


# ---------------------------------------------------------------------------
# 1. Export the exact registered MUS-T key boundary.
# ---------------------------------------------------------------------------

trajectory_path = "mus-rs/crates/mus-dsp/src/trajectory.rs"
trajectory_source = read(trajectory_path)
if "pub fn is_inline_string_trajectory_key" in trajectory_source:
    raise SystemExit(f"{trajectory_path}: trajectory key boundary already exported")

insert_before_once(
    trajectory_path,
    "\nfn contract_for_coordinate(coordinate: &str) -> Option<CoordinateContract> {",
    r'''
/// Whether an event parameter belongs to the registered inline string
/// trajectory surface.
///
/// Per-coordinate curve selectors are admitted only when their base
/// coordinate is registered. Unrelated MUS-A arrows such as `pan=-1->1` and
/// `lpf=70->900` therefore remain on the historical transform path.
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

# ---------------------------------------------------------------------------
# 2. Route ordinary pluck/weave MUS events through the persistent trajectory
#    adapter, while preserving the exact scalar path by structural absence.
# ---------------------------------------------------------------------------

source_path = "mus-rs/crates/mus-engine/src/source.rs"

replace_once(
    source_path,
    "use mus_dsp::pluck::{pluck_note, pluck_note_sung, weave_note, weave_note_sung};\n",
    r'''use mus_dsp::pluck::{pluck_note, pluck_note_sung, weave_note, weave_note_sung};
use mus_dsp::trajectory::{
    is_inline_string_trajectory_key, program_from_inline_string_params,
    render_guitar_program, render_weave_program, StringTrajectoryReceipt,
};
''',
)

replace_once(
    source_path,
    "    Rendered(Vec<f32>),\n",
    r'''    Rendered(Vec<f32>),
    /// A registered MUS-T program executed inside one persistent string
    /// voice. The receipt is attached to score placement by `events`.
    RenderedTrajectory {
        samples: Vec<f32>,
        receipt: StringTrajectoryReceipt,
    },
''',
)

replace_once(
    source_path,
    "    let mut x: Vec<f32>;\n    let mut smp_f0_hz: Option<f64> = None;\n",
    r'''    let mut x: Vec<f32>;
    let mut trajectory_receipt: Option<StringTrajectoryReceipt> = None;
    let mut smp_f0_hz: Option<f64> = None;
''',
)

replace_region(
    source_path,
    "    if let Some(synth_defaults) = &voice.synth {\n",
    r'''    } else {
        // `int(float(ev.params.get("s", 1))) - 1` (line 941): unguarded —
''',
    r'''    if let Some(synth_defaults) = &voice.synth {
        if ev.midis.is_empty() {
            bad.push(format!("b{bar} {abbr}: synth track needs a pitch"));
            return Ok(SourceStep::NeedsPitch);
        }

        let mut patch_map = synth_defaults.clone();
        for (key, value) in &ev.params {
            if SYNTH_KEYS.contains(&key.as_str()) || is_inline_string_trajectory_key(key) {
                patch_map.insert(key.clone(), value.clone());
            }
        }

        let ratio = match ev.gliss_midi {
            Some(g) => midi_to_hz(g, a4) / midi_to_hz(ev.midis[0], a4),
            None => 1.0,
        };
        let freqs: Vec<f64> = ev.midis.iter().map(|&m| midi_to_hz(m, a4)).collect();
        let (patch_map, trajectory_program) =
            program_from_inline_string_params(&patch_map, slot_s)
                .map_err(|error| ctx(RenderError::Invalid(format!("trajectory: {error}"))))?;

        // `sing` drives the string network from the instrument's sample. The
        // current trajectory adapter owns the persistent string body but not
        // that external source/work ledger, so any trajectory event that
        // needs the sung bridge refuses rather than rendering a counterfeit.
        let sing = patch_map
            .get("sing")
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.0);
        let sung_audio = if sing > 0.0 {
            match voice.samples.first() {
                Some(sample_ref) => Some(cache.load(&sample_ref.path)?),
                None => {
                    bad.push(format!(
                        "b{bar} {abbr}: sing>0 needs a sample= on the instrument; rendering un-sung"
                    ));
                    None
                }
            }
        } else {
            None
        };

        if let Some(program) = trajectory_program.as_ref() {
            let has_sing_lane = program
                .tracks
                .iter()
                .any(|track| track.coordinate == "string.sing");
            if sung_audio.is_some() || has_sing_lane {
                return Err(ctx(RenderError::Invalid(
                    "trajectory execution with an external sing source or string.sing lane is not yet work-accounted"
                        .to_string(),
                )));
            }

            let rendered = match patch_map.get("synth").map(String::as_str) {
                Some("pluck") => {
                    render_guitar_program(&patch_map, &freqs, slot_s, ratio, program, 256)
                }
                Some("weave") => {
                    render_weave_program(&patch_map, &freqs, slot_s, ratio, program, 256)
                }
                other => {
                    return Err(ctx(RenderError::Invalid(format!(
                        "trajectory controls require synth=pluck or synth=weave, got {other:?}"
                    ))));
                }
            }
            .map_err(|error| ctx(RenderError::Invalid(format!("trajectory: {error}"))))?;
            trajectory_receipt = Some(rendered.receipt);
            x = rendered.audio;
        } else {
            // Structural absence law: no registered trajectory means the exact
            // historical render entry points and their byte behavior.
            match (patch_map.get("synth").map(String::as_str), &sung_audio) {
                (Some("pluck"), Some(audio)) => {
                    x = pluck_note_sung(&patch_map, &freqs, slot_s, ratio, audio.as_slice())
                }
                (Some("weave"), Some(audio)) => {
                    x = weave_note_sung(&patch_map, &freqs, slot_s, ratio, audio.as_slice())
                }
                (Some("pluck"), None) => x = pluck_note(&patch_map, &freqs, slot_s, ratio),
                (Some("weave"), None) => x = weave_note(&patch_map, &freqs, slot_s, ratio),
                _ => {
                    let patch = Patch::from(&patch_map);
                    x = synth_note(&patch, &freqs, slot_s, ratio);
                }
            }
        }
''',
)

replace_once(
    source_path,
    "    Ok(SourceStep::Rendered(x))\n}\n",
    r'''    Ok(match trajectory_receipt {
        Some(receipt) => SourceStep::RenderedTrajectory {
            samples: x,
            receipt,
        },
        None => SourceStep::Rendered(x),
    })
}
''',
)

# ---------------------------------------------------------------------------
# 3. Attach body-neutral path identity to musical placement in RenderStats.
# ---------------------------------------------------------------------------

engine_lib = "mus-rs/crates/mus-engine/src/lib.rs"
insert_before_once(
    engine_lib,
    "\nuse sha2::{Digest, Sha256};\n",
    r'''
use mus_dsp::trajectory::StringTrajectoryReceipt;
use serde::Serialize;
''',
)

replace_once(
    engine_lib,
    r'''/// Everything the render receipt wants beyond peak/RMS: `mus_audio.py`'s
/// own `stats` dict (notes/skipped/bad, lines 874-1179) extended with the
/// per-track and sidechain counts the WF8 spec asks for.
#[derive(Debug, Clone, Default)]
pub struct RenderStats {
''',
    r'''/// One canonical trajectory execution joined to its score placement.
///
/// The embedded adapter receipt keeps its own digest unchanged. Bar, track,
/// onset, and slot belong to the score-event envelope rather than the
/// body-neutral path identity.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTrajectoryReceipt {
    pub bar: u32,
    pub track: String,
    pub onset_seconds: f64,
    pub slot_seconds: f64,
    pub receipt: StringTrajectoryReceipt,
}

/// Everything the render receipt wants beyond peak/RMS: `mus_audio.py`'s
/// own `stats` dict (notes/skipped/bad, lines 874-1179) extended with the
/// per-track and sidechain counts the WF8 spec asks for.
#[derive(Debug, Clone, Default)]
pub struct RenderStats {
''',
)

replace_once(
    engine_lib,
    "    pub warnings: Vec<String>,\n    pub per_track_events: BTreeMap<String, u32>,\n",
    r'''    pub warnings: Vec<String>,
    /// Empty for the historical scalar path, and omitted by the CLI receipt.
    pub trajectory_receipts: Vec<EventTrajectoryReceipt>,
    pub per_track_events: BTreeMap<String, u32>,
''',
)

events_path = "mus-rs/crates/mus-engine/src/events.rs"
replace_once(
    events_path,
    "use crate::{RenderError, RenderStats};\n",
    "use crate::{EventTrajectoryReceipt, RenderError, RenderStats};\n",
)

replace_once(
    events_path,
    r'''            let x = match step {
                SourceStep::Rendered(x) => x,
                SourceStep::NeedsPitch => continue,
''',
    r'''            let x = match step {
                SourceStep::Rendered(x) => x,
                SourceStep::RenderedTrajectory { samples, receipt } => {
                    stats.trajectory_receipts.push(EventTrajectoryReceipt {
                        bar: b,
                        track: abbr.to_string(),
                        onset_seconds: onset,
                        slot_seconds: slot,
                        receipt,
                    });
                    samples
                }
                SourceStep::NeedsPitch => continue,
''',
)

# ---------------------------------------------------------------------------
# 4. Carry the event-positioned receipts through ordinary `mus render` JSON.
# ---------------------------------------------------------------------------

cli_path = "mus-rs/crates/mus-cli/src/main.rs"
replace_once(
    cli_path,
    "use mus_engine::{render, sha256_bytes, write_wav, RenderStats, ENGINE_VERSION};\n",
    r'''use mus_engine::{
    render, sha256_bytes, write_wav, EventTrajectoryReceipt, RenderStats, ENGINE_VERSION,
};
''',
)

replace_once(
    cli_path,
    r'''    #[serde(rename = "warnings")]
    warnings: Vec<String>,
''',
    r'''    #[serde(rename = "warnings")]
    warnings: Vec<String>,
    #[serde(rename = "trajectoryReceipts", skip_serializing_if = "Vec::is_empty")]
    trajectory_receipts: Vec<EventTrajectoryReceipt>,
''',
)

replace_once(
    cli_path,
    r'''            bad_tokens: stats.bad.clone(),
            warnings: stats.warnings.clone(),
            stems: None,
''',
    r'''            bad_tokens: stats.bad.clone(),
            warnings: stats.warnings.clone(),
            trajectory_receipts: stats.trajectory_receipts.clone(),
            stems: None,
''',
)

# ---------------------------------------------------------------------------
# 5. End-to-end acceptance at the actual score/CLI boundary.
# ---------------------------------------------------------------------------

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
    let result = Command::new(env!("CARGO_BIN_EXE_mus"))
        .args(["render", input.to_str().expect("score path"), "-o"])
        .arg(&output)
        .arg("--json")
        .output()
        .expect("run mus render");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "rendered wave exists");
    serde_json::from_slice(&result.stdout).expect("receipt json")
}

fn write_score(root: &Path, event_text: &str) {
    fs::write(
        root.join("trajectory.mus"),
        format!(
            "# score: trajectory receipt\n\
# tempo: 120\n\
# time: 4/4\n\
# key: C\n\
# bars: 1\n\
# instruments:\n\
#   w = weave (treble, synth=weave, courses=3, couple=0.10, body=0.25, sus=0.4)\n\
\n\
bar 1: w={event_text}\n"
        ),
    )
    .expect("write score");
}

#[test]
fn ordinary_render_emits_deterministic_trajectory_receipt() {
    let root = TempDir::new();
    write_score(
        root.path(),
        "A3q[couple=0.10@0->0.30@1,traj_curve=smoothstep,traj_hz=200] \
         A3q[pan=-1->1,lpf=800->2400] Rh",
    );

    let first = render(root.path(), "first");
    let second = render(root.path(), "second");
    let first_receipts = first["trajectoryReceipts"]
        .as_array()
        .expect("trajectory receipts array");
    let second_receipts = second["trajectoryReceipts"]
        .as_array()
        .expect("trajectory receipts array");
    assert_eq!(first_receipts.len(), 1, "{first}");
    assert_eq!(
        first_receipts, second_receipts,
        "path and placement receipts are deterministic"
    );

    let event = &first_receipts[0];
    assert_eq!(event["bar"], 1);
    assert_eq!(event["track"], "w");
    assert_eq!(event["onsetSeconds"], 0.0);
    assert_eq!(event["slotSeconds"], 0.5);
    assert_eq!(
        event["receipt"]["adapter_version"],
        "mus-dsp-string-trajectory/0.1.0"
    );
    assert!(
        event["receipt"]["control_frame_count"]
            .as_u64()
            .expect("control frame count")
            > 0
    );
    assert!(
        event["receipt"]["trajectory"]["program_digest"]
            .as_str()
            .expect("program digest")
            .starts_with("sha256:")
    );
    assert_eq!(
        first["eventCount"], 2,
        "the unrelated MUS-A arrow event still renders normally"
    );
}

#[test]
fn scalar_render_omits_the_trajectory_surface() {
    let root = TempDir::new();
    write_score(root.path(), "A3q[couple=0.20] Rq Rh");
    let receipt = render(root.path(), "scalar");
    assert!(
        receipt.get("trajectoryReceipts").is_none(),
        "empty trajectory metadata must be structurally absent: {receipt}"
    );
}

#[test]
fn native_pitch_trajectory_reaches_the_persistent_retune_path() {
    let root = TempDir::new();
    write_score(
        root.path(),
        "A3q[pitch_ratio=1@0->1.0594630943592953@1,traj_hz=200] Rq Rh",
    );
    let receipt = render(root.path(), "pitch");
    let event = &receipt["trajectoryReceipts"][0];
    assert_eq!(
        event["receipt"]["native_pitch_transport"],
        "persistent-linear-f0-to-f1"
    );
    assert_eq!(event["receipt"]["control_frame_count"], 0);
    assert!(
        (event["receipt"]["pitch_end_ratio"]
            .as_f64()
            .expect("pitch end ratio")
            - 1.0594630943592953)
            .abs()
            < 1e-14
    );
}
''',
    encoding="utf-8",
)

print("MUS-T ordinary renderer integration patch applied")
