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


# ---------------------------------------------------------------------------
# 1. Tighten the body adapter before exposing it to ordinary score events.
# ---------------------------------------------------------------------------
replace_once(
    "mus-rs/crates/mus-dsp/src/trajectory.rs",
    '''use mus_trajectory::{
    Curve, LaneTemplate, TrajectoryError, TrajectoryProgram, TrajectoryReceipt, ValueSpace,
    FORMAT_V1,
};
''',
    '''use mus_trajectory::{
    Curve, LaneTemplate, TrajectoryError, TrajectoryProgram, ValueSpace, FORMAT_V1,
};
pub use mus_trajectory::TrajectoryReceipt;
''',
)
replace_once(
    "mus-rs/crates/mus-dsp/src/trajectory.rs",
    '''    if params
        .get("pitch_ratio")
        .is_some_and(|value| value.contains("->"))
    {
        return Err(StringTrajectoryError::PitchTransportPending);
    }

    let mut static_params = params.clone();
''',
    '''    if params
        .get("pitch_ratio")
        .is_some_and(|value| value.contains("->"))
    {
        return Err(StringTrajectoryError::PitchTransportPending);
    }
    for target in CONSTRUCTION_CAPACITY_TARGETS {
        if params
            .get(*target)
            .is_some_and(|value| value.contains("->"))
        {
            return Err(StringTrajectoryError::ConstructionCapacityPending {
                target: (*target).to_string(),
            });
        }
    }

    let mut static_params = params.clone();
''',
)
replace_once(
    "mus-rs/crates/mus-dsp/src/trajectory.rs",
    '''const STRING_TRAJECTORY_TARGETS: &[&str] = &[
    "couple",
    "chirality",
    "orbit",
    "orbit_depth",
    "curvature",
    "body",
    "buzz",
    "sing",
    "bow",
    "bow_speed",
    "sus",
    "damp",
    "breath",
];
''',
    '''const STRING_TRAJECTORY_TARGETS: &[&str] = &[
    "couple",
    "chirality",
    "orbit",
    "orbit_depth",
    "curvature",
    "body",
    "buzz",
    "bow",
    "bow_speed",
    "damp",
];

/// These controls are runtime-retargetable only when their supporting state
/// was sized or allocated at construction. Until the constructor receives an
/// explicit capacity contract, an arrow could request motion the body cannot
/// actually execute (for example breath 0 -> 1 with no noise channels).
const CONSTRUCTION_CAPACITY_TARGETS: &[&str] = &["sing", "sus", "breath"];
''',
)
replace_once(
    "mus-rs/crates/mus-dsp/src/trajectory.rs",
    '''        "orbit_depth" | "curvature" | "body" | "buzz" | "sing" | "bow" | "bow_speed" | "breath" => {
            ("unitless", 0.0, 1.0)
        }
        "sus" => ("second", 0.05, 20.0),
        "damp" => ("unitless", 0.0, 0.95),
''',
    '''        "orbit_depth" | "curvature" | "body" | "buzz" | "bow" | "bow_speed" => {
            ("unitless", 0.0, 1.0)
        }
        "damp" => ("unitless", 0.0, 0.95),
''',
)
replace_once(
    "mus-rs/crates/mus-dsp/src/trajectory.rs",
    '''    UnsupportedTarget {
        target: String,
    },
    /// The first unified seam deliberately left open. A physical pitch path
''',
    '''    UnsupportedTarget {
        target: String,
    },
    /// The target can be changed after construction, but its supporting state
    /// or render horizon cannot yet be grown honestly during a note.
    ConstructionCapacityPending {
        target: String,
    },
    /// The first unified seam deliberately left open. A physical pitch path
''',
)
replace_once(
    "mus-rs/crates/mus-dsp/src/trajectory.rs",
    '''            Self::UnsupportedTarget { .. } => "string-trajectory-unsupported-target",
            Self::PitchTransportPending => "string-trajectory-pitch-transport-pending",
''',
    '''            Self::UnsupportedTarget { .. } => "string-trajectory-unsupported-target",
            Self::ConstructionCapacityPending { .. } => {
                "string-trajectory-construction-capacity-pending"
            }
            Self::PitchTransportPending => "string-trajectory-pitch-transport-pending",
''',
)
replace_once(
    "mus-rs/crates/mus-dsp/src/trajectory.rs",
    '''            Self::PitchTransportPending => write!(
                formatter,
                "pitch_ratio trajectory refused: lawful delay-state transport is still open"
            ),
''',
    '''            Self::ConstructionCapacityPending { target } => write!(
                formatter,
                "{target} trajectory refused: its supporting state is sized at construction and no capacity contract exists yet"
            ),
            Self::PitchTransportPending => write!(
                formatter,
                "pitch_ratio trajectory refused: lawful delay-state transport is still open"
            ),
''',
)

# ---------------------------------------------------------------------------
# 2. Admit the two score-level directive keys through the synth patch merger.
# ---------------------------------------------------------------------------
replace_once(
    "mus-rs/crates/mus-engine/src/pack.rs",
    '''    "bow_speed",
    "retune",
];
''',
    '''    "bow_speed",
    "retune",
    "trajectory_curve",
    "pitch_ratio",
];
''',
)

# ---------------------------------------------------------------------------
# 3. Route only non-empty programs through the persistent trajectory voice.
#    Empty programs retain the exact historical entry points.
# ---------------------------------------------------------------------------
replace_once(
    "mus-rs/crates/mus-engine/src/source.rs",
    '''use mus_dsp::pluck::{pluck_note, pluck_note_sung, weave_note, weave_note_sung};
''',
    '''use mus_dsp::pluck::{
    pluck_note, pluck_note_sung, weave_note, weave_note_sung, StringNetworkVoice,
};
use mus_dsp::trajectory::{
    compile_string_trajectory, StringTrajectoryVoice, TrajectoryReceipt,
};
''',
)
replace_once(
    "mus-rs/crates/mus-engine/src/source.rs",
    '''pub enum SourceStep {
    Rendered(Vec<f32>),
    /// A synth-track event with no pitch (line 929): `stats["bad"]` gets
''',
    '''pub enum SourceStep {
    Rendered(Vec<f32>),
    RenderedTrajectory {
        samples: Vec<f32>,
        receipt: TrajectoryReceipt,
    },
    /// A synth-track event with no pitch (line 929): `stats["bad"]` gets
''',
)
replace_once(
    "mus-rs/crates/mus-engine/src/source.rs",
    '''    let mut x: Vec<f32>;
    let mut smp_f0_hz: Option<f64> = None;
''',
    '''    let mut x: Vec<f32>;
    let mut trajectory_receipt = None;
    let mut smp_f0_hz: Option<f64> = None;
''',
)
replace_once(
    "mus-rs/crates/mus-engine/src/source.rs",
    '''        let ratio = match ev.gliss_midi {
            Some(g) => midi_to_hz(g, a4) / midi_to_hz(ev.midis[0], a4),
            None => 1.0,
        };
        let freqs: Vec<f64> = ev.midis.iter().map(|&m| midi_to_hz(m, a4)).collect();
        // The pluck and weave voices are POST-PARITY (mus-x): no oracle
        // line to cite. Everything else about the event pipeline treats
        // them exactly like the subtractive synth output. `sing > 0`
        // additionally drives the string network with the instrument's
        // own sample through the shared bridge (cuerdas simpaticas);
        // a missing sample= is a logged refusal, rendered un-sung.
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
''',
    '''        let ratio = match ev.gliss_midi {
            Some(g) => midi_to_hz(g, a4) / midi_to_hz(ev.midis[0], a4),
            None => 1.0,
        };
        let freqs: Vec<f64> = ev.midis.iter().map(|&m| midi_to_hz(m, a4)).collect();
        let synth_name = patch_map
            .get("synth")
            .cloned()
            .unwrap_or_else(|| "osc".to_string());
        let trajectory_plan = if matches!(synth_name.as_str(), "pluck" | "weave") {
            let duration_samples =
                (slot_s.max(0.0) * mus_dsp::SR_F64).round().max(1.0) as u64;
            Some(
                compile_string_trajectory(&patch_map, duration_samples).map_err(|error| {
                    ctx(RenderError::Invalid(format!("{}: {error}", error.code())))
                })?,
            )
        } else {
            None
        };
        let effective_patch = trajectory_plan
            .as_ref()
            .map(|plan| &plan.static_params)
            .unwrap_or(&patch_map);

        // The pluck and weave voices are POST-PARITY (mus-x): no oracle
        // line to cite. Everything else about the event pipeline treats
        // them exactly like the subtractive synth output. A scalar
        // `sing > 0` can coexist with safe geometric trajectories because
        // its external state is still allocated at construction; an arrow
        // on sing itself is refused by the body adapter above.
        let sing = effective_patch
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

        if let Some(plan) = trajectory_plan.filter(|plan| !plan.program.lanes.is_empty()) {
            let receipt = plan.receipt.clone();
            let base_voice = match (synth_name.as_str(), &sung_audio) {
                ("pluck", Some(audio)) => StringNetworkVoice::guitar_sung(
                    &plan.static_params,
                    &freqs,
                    slot_s,
                    ratio,
                    audio.as_slice(),
                ),
                ("weave", Some(audio)) => StringNetworkVoice::weave_sung(
                    &plan.static_params,
                    &freqs,
                    slot_s,
                    ratio,
                    audio.as_slice(),
                ),
                ("pluck", None) => {
                    StringNetworkVoice::guitar(&plan.static_params, &freqs, slot_s, ratio)
                }
                ("weave", None) => {
                    StringNetworkVoice::weave(&plan.static_params, &freqs, slot_s, ratio)
                }
                _ => unreachable!("trajectory plan is created only for pluck/weave"),
            };
            x = StringTrajectoryVoice::new(base_voice, plan.program)
                .map_err(|error| {
                    ctx(RenderError::Invalid(format!("{}: {error}", error.code())))
                })?
                .render();
            trajectory_receipt = Some(receipt);
        } else {
            match (synth_name.as_str(), &sung_audio) {
                ("pluck", Some(audio)) => {
                    x = pluck_note_sung(effective_patch, &freqs, slot_s, ratio, audio.as_slice())
                }
                ("weave", Some(audio)) => {
                    x = weave_note_sung(effective_patch, &freqs, slot_s, ratio, audio.as_slice())
                }
                ("pluck", None) => x = pluck_note(effective_patch, &freqs, slot_s, ratio),
                ("weave", None) => x = weave_note(effective_patch, &freqs, slot_s, ratio),
                _ => {
                    let patch = Patch::from(effective_patch);
                    x = synth_note(&patch, &freqs, slot_s, ratio);
                }
            }
        }
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

# ---------------------------------------------------------------------------
# 4. Join the path receipt to render statistics and CLI receipts.
# ---------------------------------------------------------------------------
replace_once(
    "mus-rs/crates/mus-engine/src/lib.rs",
    '''use sha2::{Digest, Sha256};
''',
    '''use mus_dsp::trajectory::TrajectoryReceipt;
use serde::Serialize;
use sha2::{Digest, Sha256};
''',
)
replace_once(
    "mus-rs/crates/mus-engine/src/lib.rs",
    '''/// Everything the render receipt wants beyond peak/RMS: `mus_audio.py`'s
/// own `stats` dict (notes/skipped/bad, lines 874-1179) extended with the
/// per-track and sidechain counts the WF8 spec asks for.
#[derive(Debug, Clone, Default)]
pub struct RenderStats {
''',
    '''/// One trajectory path executed for one score event. The body-neutral
/// receipt is joined to musical placement here rather than mutating its digest.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EventTrajectoryReceipt {
    pub bar: u32,
    pub track: String,
    pub onset_seconds: f64,
    pub slot_seconds: f64,
    pub receipt: TrajectoryReceipt,
}

/// Everything the render receipt wants beyond peak/RMS: `mus_audio.py`'s
/// own `stats` dict (notes/skipped/bad, lines 874-1179) extended with the
/// per-track and sidechain counts the WF8 spec asks for.
#[derive(Debug, Clone, Default)]
pub struct RenderStats {
''',
)
replace_once(
    "mus-rs/crates/mus-engine/src/lib.rs",
    '''    pub warnings: Vec<String>,
    pub per_track_events: BTreeMap<String, u32>,
''',
    '''    pub warnings: Vec<String>,
    pub trajectory_receipts: Vec<EventTrajectoryReceipt>,
    pub per_track_events: BTreeMap<String, u32>,
''',
)
replace_once(
    "mus-rs/crates/mus-engine/src/events.rs",
    '''use crate::{RenderError, RenderStats};
''',
    '''use crate::{EventTrajectoryReceipt, RenderError, RenderStats};
''',
)
replace_once(
    "mus-rs/crates/mus-engine/src/events.rs",
    '''            let x = match step {
                SourceStep::Rendered(x) => x,
                SourceStep::NeedsPitch => continue,
''',
    '''            let x = match step {
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
replace_once(
    "mus-rs/crates/mus-cli/src/main.rs",
    '''use mus_engine::{render, sha256_bytes, write_wav, RenderStats, ENGINE_VERSION};
''',
    '''use mus_engine::{
    render, sha256_bytes, write_wav, EventTrajectoryReceipt, RenderStats, ENGINE_VERSION,
};
''',
)
replace_once(
    "mus-rs/crates/mus-cli/src/main.rs",
    '''    #[serde(rename = "warnings")]
    warnings: Vec<String>,
    /// `--stems DIR` only: abbrev → written mono stem path (dry,
''',
    '''    #[serde(rename = "warnings")]
    warnings: Vec<String>,
    #[serde(rename = "trajectoryReceipts", skip_serializing_if = "Vec::is_empty")]
    trajectory_receipts: Vec<EventTrajectoryReceipt>,
    /// `--stems DIR` only: abbrev → written mono stem path (dry,
''',
)
replace_once(
    "mus-rs/crates/mus-cli/src/main.rs",
    '''            bad_tokens: stats.bad.clone(),
            warnings: stats.warnings.clone(),
            stems: None,
''',
    '''            bad_tokens: stats.bad.clone(),
            warnings: stats.warnings.clone(),
            trajectory_receipts: stats.trajectory_receipts.clone(),
            stems: None,
''',
)

# ---------------------------------------------------------------------------
# 5. Acceptance tests at the actual score/event boundary.
# ---------------------------------------------------------------------------
test_path = Path("mus-rs/crates/mus-engine/tests/trajectory_source.rs")
if test_path.exists():
    raise SystemExit(f"{test_path}: already exists")
test_path.parent.mkdir(parents=True, exist_ok=True)
test_path.write_text(
    r'''use std::path::Path;

use mus_engine::render;

fn score(event_params: &str) -> String {
    format!(
        r#"# score: trajectory source integration
# tempo: 120
# time: 4/4
# key: C
# bars: 1
# master: rms=-16

# instruments:
#   w = weave (treble, synth=weave, courses=5, couple=0.10, body=0.25, sus=0.4)

bar 1: w=A3q[{event_params}] Rq Rh
"#
    )
}

#[test]
fn ordinary_mus_event_executes_and_receipts_a_safe_trajectory() {
    let source = score(
        "couple=0.10->0.38->0.14,body=0.25->0.70,curvature=0.20->0.82,trajectory_curve=smootherstep",
    );
    let first = render(&source, Path::new(".")).expect("trajectory score renders");
    let second = render(&source, Path::new(".")).expect("trajectory score replays");

    assert_eq!(first.audio.samples, second.audio.samples, "deterministic audio");
    assert_eq!(first.stats.trajectory_receipts, second.stats.trajectory_receipts);
    assert_eq!(first.stats.trajectory_receipts.len(), 1);
    let event = &first.stats.trajectory_receipts[0];
    assert_eq!(event.bar, 1);
    assert_eq!(event.track, "w");
    assert_eq!(event.receipt.lane_count, 3);
    assert_eq!(
        event
            .receipt
            .lanes
            .iter()
            .map(|lane| lane.target.as_str())
            .collect::<Vec<_>>(),
        ["couple", "body", "curvature"]
    );
    assert!(first.audio.samples.iter().any(|sample| *sample != 0.0));
}

#[test]
fn scalar_score_stays_on_the_legacy_path() {
    let outcome = render(&score("couple=0.12,body=0.40"), Path::new("."))
        .expect("scalar score renders");
    assert!(outcome.stats.trajectory_receipts.is_empty());
}

#[test]
fn unrelated_mus_a_arrow_is_not_claimed_as_a_body_trajectory() {
    let outcome = render(&score("pan=-1->1,lpf=800->2400"), Path::new("."))
        .expect("legacy transform arrows render");
    assert!(outcome.stats.trajectory_receipts.is_empty());
}

#[test]
fn pitch_is_refused_at_the_score_boundary() {
    let error = render(&score("pitch_ratio=1->1.0594630943592953"), Path::new("."))
        .expect_err("uncertified delay-state transport must refuse");
    let text = error.to_string();
    assert!(text.contains("string-trajectory-pitch-transport-pending"), "{text}");
}

#[test]
fn construction_sized_control_is_refused_at_the_score_boundary() {
    for params in ["breath=0->0.8", "sing=0->0.7", "sus=0.4->3.0"] {
        let error = render(&score(params), Path::new("."))
            .expect_err("construction-sized motion must refuse");
        let text = error.to_string();
        assert!(
            text.contains("string-trajectory-construction-capacity-pending"),
            "{params}: {text}"
        );
    }
}
''',
    encoding="utf-8",
)

# Add a direct adapter regression for the construction-capacity refusal.
path = "mus-rs/crates/mus-dsp/tests/trajectory_program.rs"
text = read(path)
append = r'''

#[test]
fn construction_sized_controls_refuse_before_voice_construction() {
    for target in ["sing", "breath", "sus"] {
        let mut patch = working_patch();
        patch.insert(target.into(), "0.1->0.8".into());
        let error = compile_string_trajectory(&patch, 4_800).unwrap_err();
        assert_eq!(
            error.code(),
            "string-trajectory-construction-capacity-pending",
            "{target}"
        );
    }
}
'''
if "construction_sized_controls_refuse_before_voice_construction" in text:
    raise SystemExit(f"{path}: acceptance test already exists")
write(path, text + append)
