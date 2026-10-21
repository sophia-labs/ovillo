//! Annotated-tab instruments played on one persistent Ariadne world
//! (`SPEC-TAB.md`, `SPEC-ARIADNE-STEEL-STRING.md`).
//!
//! An instrument whose declaration carries `anatomy=` is not a voice. Its
//! dot-voices (`gt.p`, `gt.v`, ...) are hands of one physical instrument. Their
//! tab notes compile into one performance (plucks, hammer-ons, pull-offs and
//! slides on the plan's string bodies) that drives one canonical
//! `mus_ariadne::World`. A stereo listener observes the world, and the
//! observation mixes into the dry bus like any other voice. Nothing here
//! synthesizes a note: every sound is the world's response to a touch.

use std::collections::BTreeMap;
use std::f64::consts::PI;
use std::path::Path;

use mus_ariadne::{
    BodyControls, StereoListener, StereoListenerSpec, World, WorldControls, WorldPlan, MODE_COUNT,
};
use mus_notation::{
    expand_pattern_repetitions, parse_token, split_attached_dynamics, tokenize_bar_content,
};
use mus_text::ProposalInstrument;
use serde::Serialize;

use crate::layout::BarMap;
use crate::mixbus::{db_to_gain, MixBuffers};
use crate::util::{dyn_db, note_to_midi, ART_GAIN};
use crate::{RenderError, RenderStats};

/// `anatomy=steel-string-v0`: the dreadnought compiled by `mus_analysis.steel_string_anatomy`.
const STEEL_STRING_V0: &str =
    include_str!("../../../../profiles/ariadne-steel-string-v0.plan.json");
/// A unit hand gesture reaches the string through the plan's reciprocal contact
/// port (a gentle 80 Hz exchange, so the port does not damp a ringing string);
/// this scales the gesture to a pluck of ordinary loudness.
const PLUCK_SCALE: f64 = 25.0;
/// World observation is in the world's own energy units. This brings a mezzo-forte guitar near the engine's other voices.
const OBSERVATION_GAIN: f64 = 300.0;
/// Strings barely radiate; the box does the speaking.
const STRING_RADIATION: f64 = 2.0e-6;
const BOX_RADIATION: f64 = 0.02;
/// Radiation efficiency grows toward the top's coincidence frequency: a box body
/// whose modes sit higher radiates proportionally faster (relative to 1 kHz).
const RADIATION_REFERENCE_HZ: f64 = 1000.0;
const SLIDE_SECONDS: f64 = 0.06;
/// The fretting finger lands a little before the other hand plucks.
const FRET_LEAD_SECONDS: f64 = 0.008;
const ENERGY_HOP_SECONDS: f64 = 0.01;
/// Contact time of a pluck: a plectrum's, plus the flesh a softer touch adds.
const PULSE_PICK_SECONDS: f64 = 0.00006;
const PULSE_FLESH_SECONDS: f64 = 0.0003;
const STANDARD_TUNING: [f64; 6] = [40.0, 45.0, 50.0, 55.0, 59.0, 64.0];

pub struct TabInstrument {
    pub abbrev: String,
    pub anatomy: String,
    pub hands: Vec<Hand>,
    plan: WorldPlan,
    /// Open pitch of each string, string 1 (highest) first.
    pub(crate) open: Vec<f64>,
    pub(crate) capo: f64,
    gain_db: f64,
    send: f64,
}

pub struct Hand {
    pub abbrev: String,
    pub(crate) pos: f64,
    pub(crate) pick: f64,
    pub(crate) pm: f64,
    gain_db: f64,
}

impl TabInstrument {
    /// The pitch (MIDI, fractional for cents) that `@string:fret` sounds on this
    /// instrument (SPEC-TAB §1): open course + capo + fret, or for a natural
    /// harmonic the harmonic's interval above the open course.
    pub(crate) fn expected_midi(&self, string: usize, fret: u8, harmonic: bool) -> f64 {
        let harmonic = if harmonic {
            harmonic_offset(fret)
        } else {
            None
        };
        self.open[string - 1] + self.capo + harmonic.unwrap_or(fret as f64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TouchKind {
    Pluck,
    HammerOn,
    PullOff,
    Slide,
}

#[derive(Debug, Clone, Copy)]
struct Touch {
    seconds: f64,
    string: usize,
    fret: f64,
    db: f64,
    hand: usize,
    kind: TouchKind,
}

/// What a tab instrument's world did, for the render receipt.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AriadneWorldReceipt {
    pub instrument: String,
    pub anatomy: String,
    pub plan_digest: String,
    pub listener_digest: String,
    pub bodies: Vec<String>,
    pub plucks: usize,
    pub hammer_ons: usize,
    pub pull_offs: usize,
    pub slides: usize,
    pub simulated_seconds: f64,
    /// Which hand each string answered to, and how that hand touched it.
    pub strings: Vec<StringHandReceipt>,
    pub source_work: f64,
    pub control_work: f64,
    pub internal_loss: f64,
    pub radiation: f64,
    pub cumulative_closure_residual: f64,
    pub maximum_frame_residual: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StringHandReceipt {
    pub string: usize,
    pub open: String,
    pub hand: Option<String>,
    pub pos: f64,
    pub pick: f64,
    pub palm_mute: f64,
}

/// Stored energy of every body (and the room) through the render: the visualizer's raw material.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnergyTrace {
    pub instrument: String,
    pub hop_seconds: f64,
    pub start_seconds: f64,
    /// Every body, then the room: stored energy at each hop.
    pub channels: Vec<String>,
    pub frames: Vec<Vec<f32>>,
    /// Box bodies' modes, in plan order (24 per body): energy at each hop.
    pub mode_bodies: Vec<String>,
    pub mode_frequency_hz: Vec<f32>,
    pub mode_frames: Vec<Vec<f32>>,
    /// Energy moved during each hop. Per body: what the hands put in, what it radiated, what
    /// it lost inside. Per bridge edge: what flowed into the string side (negative = into the box).
    pub source_frames: Vec<Vec<f32>>,
    pub radiation_frames: Vec<Vec<f32>>,
    pub loss_frames: Vec<Vec<f32>>,
    pub edges: Vec<String>,
    pub bridge_frames: Vec<Vec<f32>>,
    /// The World's own books at each hop (`World::receipt`, cumulative from the start of the
    /// render, f64), one row per hop in `ledger_columns` order. The last row is the receipt's.
    pub ledger_columns: Vec<&'static str>,
    pub ledger_frames: Vec<[f64; 8]>,
    /// Every touch the score made, as it reached the world.
    pub touches: Vec<TouchRecord>,
}

/// Column names of [`EnergyTrace::ledger_frames`].
pub const LEDGER_COLUMNS: [&str; 8] = [
    "sourceWork",
    "controlWork",
    "internalLoss",
    "radiation",
    "initialEnergy",
    "currentEnergy",
    "cumulativeClosureResidual",
    "maximumFrameResidual",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TouchRecord {
    pub seconds: f64,
    pub string: usize,
    pub fret: f64,
    pub kind: &'static str,
    pub hand: String,
    pub pos: f64,
    pub db: f64,
}

/// The tab instruments in a score, and every abbrev they own (base and hands).
pub fn collect(
    instruments: &[ProposalInstrument],
    score_dir: &Path,
) -> Result<(Vec<TabInstrument>, Vec<String>), RenderError> {
    let mut found = Vec::new();
    let mut owned = Vec::new();
    // A piano's anatomy is not a tab instrument's (`ariadne_piano`).
    for base in instruments.iter().filter(|i| {
        i.params
            .get("anatomy")
            .is_some_and(|a| !crate::ariadne_piano::is_piano_anatomy(a))
    }) {
        let invalid = |msg: String| RenderError::Invalid(format!("{}: {msg}", base.abbrev));
        let anatomy = base.params["anatomy"].trim().to_owned();
        let text = if anatomy == "steel-string-v0" {
            STEEL_STRING_V0.to_owned()
        } else {
            std::fs::read_to_string(score_dir.join(&anatomy))
                .map_err(|e| invalid(format!("anatomy {anatomy}: {e}")))?
        };
        let plan: WorldPlan =
            serde_json::from_str(&text).map_err(|e| invalid(format!("anatomy {anatomy}: {e}")))?;
        let tuning: Vec<f64> = match base.params.get("tuning") {
            Some(t) => t
                .split_whitespace()
                .map(|n| note_to_midi(n).ok_or_else(|| invalid(format!("tuning note {n}"))))
                .collect::<Result<_, _>>()?,
            None => STANDARD_TUNING.to_vec(),
        };
        let string_bodies = plan
            .bodies
            .iter()
            .filter(|b| b.id.starts_with("string/"))
            .count();
        if tuning.len() != string_bodies {
            return Err(invalid(format!(
                "tuning has {} courses, the anatomy has {string_bodies} strings",
                tuning.len()
            )));
        }
        let number = |key: &str,
                      default: f64,
                      params: &BTreeMap<String, String>|
         -> Result<f64, RenderError> {
            params.get(key).map_or(Ok(default), |v| {
                v.trim()
                    .parse::<f64>()
                    .map_err(|_| invalid(format!("{key}={v}")))
            })
        };
        let prefix = format!("{}.", base.abbrev);
        let hands = instruments
            .iter()
            .filter(|i| i.abbrev.starts_with(&prefix))
            .map(|i| {
                Ok(Hand {
                    abbrev: i.abbrev.clone(),
                    pos: number("pos", 0.18, &i.params)?.clamp(0.02, 0.5),
                    pick: number("pick", 0.4, &i.params)?.clamp(0.0, 1.0),
                    pm: number("pm", 0.0, &i.params)?.clamp(0.0, 1.0),
                    gain_db: number("gain", 0.0, &i.params)?,
                })
            })
            .collect::<Result<Vec<_>, RenderError>>()?;
        owned.push(base.abbrev.clone());
        owned.extend(hands.iter().map(|h| h.abbrev.clone()));
        found.push(TabInstrument {
            abbrev: base.abbrev.clone(),
            anatomy,
            capo: number("capo", 0.0, &base.params)?,
            gain_db: number("gain", 0.0, &base.params)?,
            send: number("send", 0.0, &base.params)?,
            open: tuning.iter().rev().copied().collect(),
            plan,
            hands,
        });
    }
    Ok((found, owned))
}

/// SPEC-TAB §5: swing as a piecewise-linear warp of each beat pair. It agrees
/// with `layout::swung` on the grid and carries 16ths proportionally.
pub(crate) fn warp(q: f64, unit: f64, ratio: f64) -> f64 {
    if unit <= 0.0 || ratio <= 0.5 {
        return q;
    }
    let pair = 2.0 * unit;
    let k = (q / pair + 1e-9).floor();
    let local = q - k * pair;
    let warped = if local < unit {
        local * 2.0 * ratio
    } else {
        2.0 * unit * ratio + (local - unit) * (2.0 - 2.0 * ratio)
    };
    k * pair + warped
}

#[allow(clippy::too_many_arguments)]
fn performance(
    inst: &TabInstrument,
    bar_map: &BarMap,
    bar_content: &BTreeMap<u32, BTreeMap<String, String>>,
    n_bars: u32,
    swing_unit_ql: f64,
    swing_r: f64,
    stats: &mut RenderStats,
) -> Vec<Touch> {
    let mut touches = Vec::new();
    for (h, hand) in inst.hands.iter().enumerate() {
        let abbr = hand.abbrev.as_str();
        let mut cur_dyn = "mf".to_string();
        let mut count = 0_u32;
        for b in 1..=n_bars {
            let Some(content) = bar_content.get(&b).and_then(|m| m.get(abbr)) else {
                continue;
            };
            if content.trim().is_empty() || content.trim() == "-" {
                continue;
            }
            let Some(timing) = bar_map.bars.get(&b) else {
                continue;
            };
            let tokens = split_attached_dynamics(&expand_pattern_repetitions(
                &tokenize_bar_content(content),
            ));
            let mut q = 0.0_f64;
            for tk in &tokens {
                let Some(ev) = parse_token(tk) else {
                    if !matches!(tk.trim(), "" | "-") {
                        stats.bad.push(format!("b{b} {abbr}: {tk}"));
                    }
                    continue;
                };
                match ev.kind.as_str() {
                    "dynamic" => {
                        if let Some(v) = ev.dyn_value {
                            cur_dyn = v;
                        }
                        continue;
                    }
                    "note" | "chord" => {}
                    _ => {
                        q += ev.ql;
                        continue;
                    }
                }
                let seconds = timing.start_s + warp(q, swing_unit_ql, swing_r) * timing.spq;
                q += ev.ql;
                if ev.tab.len() != ev.midis.len() {
                    stats.bad.push(format!(
                        "b{b} {abbr}: {tk} (a tab instrument's notes need @string:fret)"
                    ));
                    continue;
                }
                let flag = |f: &str| ev.flags.iter().any(|x| x == f);
                let kind = if flag("ho") {
                    TouchKind::HammerOn
                } else if flag("po") {
                    TouchKind::PullOff
                } else if flag("sl") {
                    TouchKind::Slide
                } else {
                    TouchKind::Pluck
                };
                let art: f64 = ev
                    .flags
                    .iter()
                    .filter_map(|f| ART_GAIN.iter().find(|(n, _)| *n == f).map(|(_, d)| *d))
                    .sum();
                let gain = ev
                    .params
                    .get("gain")
                    .and_then(|g| g.parse::<f64>().ok())
                    .unwrap_or(0.0);
                for (midi, at) in ev.midis.iter().zip(&ev.tab) {
                    let string = at.string as usize;
                    if string == 0 || string > inst.open.len() {
                        stats
                            .bad
                            .push(format!("b{b} {abbr}: {tk} (no string {string})"));
                        continue;
                    }
                    let expected = inst.expected_midi(string, at.fret, flag("harm"));
                    if (midi - expected).abs() > 0.01 {
                        stats.bad.push(format!(
                            "b{b} {abbr}: {tk} (string {string} fret {} sounds midi {expected}, not {midi})",
                            at.fret
                        ));
                    }
                    touches.push(Touch {
                        seconds,
                        string,
                        fret: at.fret as f64,
                        db: dyn_db(&cur_dyn) + art + gain + hand.gain_db,
                        hand: h,
                        kind,
                    });
                    count += 1;
                }
            }
        }
        stats.per_track_events.insert(abbr.to_string(), count);
        stats.notes += count;
    }
    touches.sort_by(|a, b| a.seconds.total_cmp(&b.seconds));
    touches
}

pub(crate) fn midi_name(midi: f64) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B",
    ];
    let m = midi.round() as i64;
    format!(
        "{}{}",
        NAMES[m.rem_euclid(12) as usize],
        m.div_euclid(12) - 1
    )
}

/// The mean mode frequency of body `index` as built.
fn world_plan_band(plan: &WorldPlan, index: usize) -> f64 {
    let f = &plan.bodies[index]
        .body
        .configuration_path
        .left
        .metric
        .frequency_hz;
    f.iter().sum::<f64>() / f.len() as f64
}

fn box_radiation(mean_hz: &f64) -> f64 {
    (BOX_RADIATION * (mean_hz / RADIATION_REFERENCE_HZ).max(1.0)).min(0.5)
}

fn harmonic_offset(fret: u8) -> Option<f64> {
    match fret {
        12 => Some(12.0),
        7 | 19 => Some(19.0),
        5 | 24 => Some(24.0),
        _ => None,
    }
}

/// Fold each string's hand into its body: where it is plucked (the source
/// direction is the modal shape at the pluck point), how hard the contact is,
/// and how much a resting palm damps it. The law matches `steel_string_anatomy.py`.
fn apply_hands(
    plan: &mut WorldPlan,
    inst: &TabInstrument,
    touches: &[Touch],
) -> Vec<StringHandReceipt> {
    let mut receipts = Vec::new();
    for body in plan.bodies.iter_mut() {
        let Some(number) = body
            .id
            .strip_prefix("string/")
            .and_then(|n| n.parse::<usize>().ok())
        else {
            continue;
        };
        let mut votes = vec![0_usize; inst.hands.len()];
        for t in touches.iter().filter(|t| t.string == number) {
            votes[t.hand] += 1;
        }
        let hand = votes
            .iter()
            .enumerate()
            .filter(|(_, &v)| v > 0)
            .max_by_key(|(_, &v)| v)
            .map(|(h, _)| &inst.hands[h]);
        let (pos, pick, pm) = hand.map_or((0.18, 0.4, 0.0), |h| (h.pos, h.pick, h.pm));
        let source: [f64; MODE_COUNT] = std::array::from_fn(|i| {
            let n = i as f64 + 1.0;
            (PI * n * pos).sin() / n
        });
        let norm = source.iter().map(|v| v * v).sum::<f64>().sqrt().max(1e-12);
        body.body.source_direction = source.map(|v| v / norm);
        // retune the course when the score's tuning differs from the anatomy's
        let path = &mut body.body.configuration_path;
        let plan_open = (69.0 + 12.0 * (path.left.metric.frequency_hz[0] / 440.0).log2()).round();
        let ratio = 2f64.powf((inst.open[number - 1] - plan_open) / 12.0);
        for config in [&mut path.left, &mut path.right] {
            config.aperture.natural_frequency_hz = 380.0 + 900.0 * pick;
            config.aperture.damping_per_second = 60.0 - 30.0 * pick;
            for i in 0..MODE_COUNT {
                config.metric.frequency_hz[i] *= ratio;
                if pm > 0.0 {
                    let w = 2.0 * PI * config.metric.frequency_hz[i];
                    let rate = 6.91 / config.metric.t60_seconds[i] + pm * (24.0 + 1.2e-5 * w * w);
                    config.metric.t60_seconds[i] = 6.91 / rate;
                }
            }
        }
        receipts.push(StringHandReceipt {
            string: number,
            open: midi_name(inst.open[number - 1]),
            hand: hand.map(|h| h.abbrev.clone()),
            pos,
            pick,
            palm_mute: pm,
        });
    }
    receipts.sort_by_key(|r| r.string);
    receipts
}

/// One tab instrument's world, set up and ready to be stepped a block of
/// samples at a time (`mus-wasm` renders in chunks so a page can show
/// progress). [`render`] steps every run to the end in one call; a run
/// stepped in blocks of any size produces the same samples, receipt and trace.
pub struct WorldRun {
    abbrev: String,
    anatomy: String,
    capo: f64,
    send: f64,
    gain_db: f64,
    sample_rate: f64,
    world: World,
    listener: StereoListener,
    bodies: Vec<String>,
    string_body: BTreeMap<usize, usize>,
    controls: Vec<BodyControls>,
    strings: Vec<StringHandReceipt>,
    stops: Vec<(usize, usize, f64, usize)>,
    pulses: Vec<(usize, usize, f64, usize)>,
    counts: [usize; 4],
    start: usize,
    n_total: usize,
    k: usize,
    hop: usize,
    box_bodies: Vec<usize>,
    omega: Vec<Vec<f64>>,
    trace: EnergyTrace,
    dt: f64,
    source_acc: Vec<f64>,
    radiation_acc: Vec<f64>,
    loss_acc: Vec<f64>,
    bridge_acc: Vec<f64>,
    left: Vec<f32>,
    right: Vec<f32>,
    current: BTreeMap<usize, f64>,
    ramps: BTreeMap<usize, (f64, f64, usize, usize)>,
    next_stop: usize,
    next_pulse: usize,
    active: Vec<(usize, usize, f64, usize)>,
}

/// Compile every tab instrument's performance and build its world. Instruments
/// whose hands play nothing get no world (and no receipt), as before.
#[allow(clippy::too_many_arguments)]
pub fn prepare(
    instruments: &[TabInstrument],
    bar_map: &BarMap,
    bar_content: &BTreeMap<u32, BTreeMap<String, String>>,
    n_bars: u32,
    swing_unit_ql: f64,
    swing_r: f64,
    sample_rate: f64,
    n_total: usize,
    stats: &mut RenderStats,
) -> Result<Vec<WorldRun>, RenderError> {
    let mut runs = Vec::new();
    for inst in instruments {
        let world_err = |e: String| RenderError::Invalid(format!("{} world: {e}", inst.abbrev));
        let touches = performance(
            inst,
            bar_map,
            bar_content,
            n_bars,
            swing_unit_ql,
            swing_r,
            stats,
        );
        if touches.is_empty() {
            continue;
        }
        let mut plan = inst.plan.clone();
        if (plan.internal_sample_rate_hz - sample_rate).abs() > 1e-6 {
            return Err(world_err(format!(
                "anatomy runs at {} Hz, the engine at {sample_rate} Hz",
                plan.internal_sample_rate_hz
            )));
        }
        let strings = apply_hands(&mut plan, inst, &touches);
        let listener_spec =
            StereoListenerSpec::all_bodies(&plan, 1.0).map_err(|e| world_err(e.to_string()))?;
        let listener =
            StereoListener::bind(&listener_spec, &plan).map_err(|e| world_err(e.to_string()))?;
        let world = World::from_plan(plan).map_err(|e| world_err(e.to_string()))?;
        let bodies: Vec<String> = (0..world.body_count())
            .map(|i| world.body_id(i).unwrap_or("?").to_owned())
            .collect();
        let string_body: BTreeMap<usize, usize> = bodies
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                id.strip_prefix("string/")
                    .and_then(|n| n.parse().ok())
                    .map(|n| (n, i))
            })
            .collect();
        let controls: Vec<BodyControls> = bodies
            .iter()
            .enumerate()
            .map(|(i, id)| BodyControls {
                radiation_fraction: if id.starts_with("string/") {
                    STRING_RADIATION
                } else {
                    box_radiation(&world_plan_band(&inst.plan, i))
                },
                ..BodyControls::default()
            })
            .collect();

        // ---- the performance, as per-sample actions on the world
        let start = ((touches[0].seconds - FRET_LEAD_SECONDS).max(0.0) * sample_rate) as usize;
        let mut amount: BTreeMap<usize, f64> = BTreeMap::new(); // current stop per string, in frets
        let mut stops: Vec<(usize, usize, f64, usize)> = Vec::new(); // (sample, string, target fret, ramp samples)
        let mut pulses: Vec<(usize, usize, f64, usize)> = Vec::new(); // (sample, string, amplitude, width samples)
        let mut counts = [0_usize; 4];
        for t in &touches {
            let at = (t.seconds * sample_rate) as usize;
            let hand = &inst.hands[t.hand];
            // The string slips off a fingertip in a few tenths of a millisecond, off a pick in less.
            let width = ((PULSE_PICK_SECONDS + PULSE_FLESH_SECONDS * (1.0 - hand.pick))
                * sample_rate)
                .max(3.0) as usize;
            let amp = PLUCK_SCALE * db_to_gain(t.db + 11.0); // mezzo-forte is a unit gesture
            let fretted = inst.capo + t.fret;
            let previous = *amount.get(&t.string).unwrap_or(&inst.capo);
            match t.kind {
                TouchKind::Pluck => {
                    if (fretted - previous).abs() > 1e-9 {
                        stops.push((
                            at.saturating_sub((FRET_LEAD_SECONDS * sample_rate) as usize),
                            t.string,
                            fretted,
                            1,
                        ));
                    }
                    pulses.push((at, t.string, amp, width));
                    counts[0] += 1;
                }
                TouchKind::HammerOn => {
                    stops.push((at, t.string, fretted, 1));
                    pulses.push((at, t.string, 0.3 * amp, (0.0003 * sample_rate) as usize));
                    counts[1] += 1;
                }
                TouchKind::PullOff => {
                    stops.push((at, t.string, fretted, 1));
                    pulses.push((at, t.string, 0.5 * amp, width));
                    counts[2] += 1;
                }
                TouchKind::Slide => {
                    stops.push((
                        at,
                        t.string,
                        fretted,
                        (SLIDE_SECONDS * sample_rate) as usize,
                    ));
                    counts[3] += 1;
                }
            }
            amount.insert(t.string, fretted);
        }
        stops.sort_by_key(|s| s.0);
        pulses.sort_by_key(|p| p.0);

        let hop = (ENERGY_HOP_SECONDS * sample_rate) as usize;
        let box_bodies: Vec<usize> = (0..bodies.len())
            .filter(|&i| bodies[i].starts_with("box"))
            .collect();
        let omega: Vec<Vec<f64>> = box_bodies
            .iter()
            .map(|&i| {
                inst.plan.bodies[i]
                    .body
                    .configuration_path
                    .left
                    .metric
                    .frequency_hz
                    .iter()
                    .map(|f| 2.0 * PI * f)
                    .collect()
            })
            .collect();
        let edges = world.coupling_count();
        let trace = EnergyTrace {
            instrument: inst.abbrev.clone(),
            hop_seconds: ENERGY_HOP_SECONDS,
            start_seconds: start as f64 / sample_rate,
            channels: bodies
                .iter()
                .cloned()
                .chain(std::iter::once("room".to_owned()))
                .collect(),
            frames: Vec::new(),
            mode_bodies: box_bodies.iter().map(|&i| bodies[i].clone()).collect(),
            mode_frequency_hz: omega
                .iter()
                .flatten()
                .map(|w| (w / (2.0 * PI)) as f32)
                .collect(),
            mode_frames: Vec::new(),
            source_frames: Vec::new(),
            radiation_frames: Vec::new(),
            loss_frames: Vec::new(),
            edges: (0..edges)
                .map(|e| world.coupling_id(e).unwrap_or("?").to_owned())
                .collect(),
            bridge_frames: Vec::new(),
            ledger_columns: LEDGER_COLUMNS.to_vec(),
            ledger_frames: Vec::new(),
            touches: touches
                .iter()
                .map(|t| TouchRecord {
                    seconds: t.seconds,
                    string: t.string,
                    fret: t.fret,
                    kind: match t.kind {
                        TouchKind::Pluck => "pluck",
                        TouchKind::HammerOn => "hammer-on",
                        TouchKind::PullOff => "pull-off",
                        TouchKind::Slide => "slide",
                    },
                    hand: inst.hands[t.hand].abbrev.clone(),
                    pos: inst.hands[t.hand].pos,
                    db: t.db,
                })
                .collect(),
        };
        let n = bodies.len();
        let samples = n_total.saturating_sub(start);
        runs.push(WorldRun {
            abbrev: inst.abbrev.clone(),
            anatomy: inst.anatomy.clone(),
            capo: inst.capo,
            send: inst.send,
            gain_db: inst.gain_db,
            sample_rate,
            world,
            listener,
            bodies,
            string_body,
            controls,
            strings,
            stops,
            pulses,
            counts,
            start,
            n_total,
            k: start,
            hop,
            box_bodies,
            omega,
            trace,
            dt: 1.0 / sample_rate,
            source_acc: vec![0.0_f64; n],
            radiation_acc: vec![0.0_f64; n],
            loss_acc: vec![0.0_f64; n],
            bridge_acc: vec![0.0_f64; edges],
            left: vec![0.0_f32; samples],
            right: vec![0.0_f32; samples],
            current: BTreeMap::new(),
            ramps: BTreeMap::new(),
            next_stop: 0,
            next_pulse: 0,
            active: Vec::new(),
        });
    }
    Ok(runs)
}

impl WorldRun {
    /// The instrument this world plays.
    pub fn instrument(&self) -> &str {
        &self.abbrev
    }

    /// Samples this world simulates in all (from just before its first touch to the end of the render).
    pub fn samples(&self) -> usize {
        self.n_total.saturating_sub(self.start)
    }

    /// Samples simulated so far.
    pub fn done(&self) -> usize {
        self.k - self.start.min(self.k)
    }

    pub fn is_finished(&self) -> bool {
        self.k >= self.n_total
    }

    /// Simulate up to `max` more samples; returns how many were simulated.
    pub fn step(&mut self, max: usize) -> Result<usize, RenderError> {
        let abbrev = self.abbrev.clone();
        let world_err = move |e: String| RenderError::Invalid(format!("{abbrev} world: {e}"));
        let end = self.n_total.min(self.k.saturating_add(max));
        let from = self.k;
        let start = self.start;
        for k in from..end {
            while self.next_stop < self.stops.len() && self.stops[self.next_stop].0 <= k {
                let (_, string, to, len) = self.stops[self.next_stop];
                let from = *self.current.get(&string).unwrap_or(&self.capo);
                self.ramps.insert(string, (from, to, k, len.max(1)));
                self.next_stop += 1;
            }
            let mut done = Vec::new();
            for (&string, &(from, to, s0, len)) in &self.ramps {
                let x = ((k - s0 + 1) as f64 / len as f64).min(1.0);
                let frets = from + (to - from) * x;
                if let Some(&body) = self.string_body.get(&string) {
                    self.world
                        .set_body_configuration_index(body, (frets / 12.0).clamp(0.0, 1.0))
                        .map_err(|e| world_err(e.to_string()))?;
                }
                self.current.insert(string, frets);
                if x >= 1.0 {
                    done.push(string);
                }
            }
            for s in done {
                self.ramps.remove(&s);
            }
            while self.next_pulse < self.pulses.len() && self.pulses[self.next_pulse].0 <= k {
                self.active.push(self.pulses[self.next_pulse]);
                self.next_pulse += 1;
            }
            for c in self.controls.iter_mut() {
                c.source_wave = 0.0;
            }
            let (string_body, controls) = (&self.string_body, &mut self.controls);
            self.active.retain(|&(s0, string, amp, width)| {
                let i = k - s0;
                if i >= width {
                    return false;
                }
                if let Some(&body) = string_body.get(&string) {
                    controls[body].source_wave +=
                        amp * 0.5 * (1.0 - (2.0 * PI * i as f64 / width as f64).cos());
                }
                true
            });
            let frame = self
                .world
                .process_sample(&self.controls, WorldControls::default())
                .map_err(|e| world_err(e.to_string()))?;
            let heard = self
                .listener
                .observe(&self.world, &frame)
                .map_err(|e| world_err(e.to_string()))?;
            self.left[k - start] = heard.left as f32;
            self.right[k - start] = heard.right as f32;
            let dt = self.dt;
            for i in 0..self.bodies.len() {
                self.source_acc[i] += self.world.body_last_source_work(i).unwrap_or(0.0);
                self.loss_acc[i] += self.world.body_last_internal_loss(i).unwrap_or(0.0);
                let e = self.world.body_emission(i).unwrap_or(0.0);
                self.radiation_acc[i] += 0.5 * dt * e * e;
            }
            for (e, acc) in self.bridge_acc.iter_mut().enumerate() {
                *acc += self
                    .world
                    .coupling_last_exchange_into_left(e)
                    .unwrap_or(0.0);
            }
            if (k - start).is_multiple_of(self.hop) {
                let world = &self.world;
                let mut row: Vec<f32> = (0..self.bodies.len())
                    .map(|i| world.body_material_energy(i).unwrap_or(0.0) as f32)
                    .collect();
                row.push(world.space_stored_energy() as f32);
                self.trace.frames.push(row);
                let mut modes = Vec::with_capacity(24 * self.box_bodies.len());
                for (b, &i) in self.box_bodies.iter().enumerate() {
                    if let Some(state) = world.body_state(i) {
                        for (m, &w) in self.omega[b].iter().enumerate().take(MODE_COUNT) {
                            let (p, x) = (state.momentum[m], w * state.displacement[m]);
                            modes.push((0.5 * dt * (p * p + x * x)) as f32);
                        }
                    }
                }
                self.trace.mode_frames.push(modes);
                let take = |acc: &mut Vec<f64>| -> Vec<f32> {
                    acc.iter_mut().map(|v| std::mem::take(v) as f32).collect()
                };
                self.trace.source_frames.push(take(&mut self.source_acc));
                self.trace
                    .radiation_frames
                    .push(take(&mut self.radiation_acc));
                self.trace.loss_frames.push(take(&mut self.loss_acc));
                self.trace.bridge_frames.push(take(&mut self.bridge_acc));
                let books = world.receipt();
                self.trace.ledger_frames.push([
                    books.source_work,
                    books.control_work,
                    books.internal_loss,
                    books.radiation,
                    books.initial_energy,
                    books.current_energy,
                    books.cumulative_closure_residual,
                    books.maximum_frame_residual,
                ]);
            }
        }
        self.k = end.max(self.k);
        Ok(end.saturating_sub(from))
    }

    /// Mix what the listener heard into the buses and file the world's receipt and energy trace.
    /// Steps whatever is left first.
    pub fn finish(
        mut self,
        buffers: &mut MixBuffers,
        stats: &mut RenderStats,
    ) -> Result<(), RenderError> {
        while !self.is_finished() {
            self.step(usize::MAX)?;
        }
        buffers.mix_one(
            self.start,
            &self.left,
            &self.right,
            self.send,
            OBSERVATION_GAIN * db_to_gain(self.gain_db),
            None,
        );
        let r = self.world.receipt();
        stats
            .ariadne_worlds
            .push(crate::AriadneReceipt::Tab(AriadneWorldReceipt {
                instrument: self.abbrev.clone(),
                anatomy: self.anatomy.clone(),
                plan_digest: self.world.plan_digest().to_owned(),
                listener_digest: self.listener.listener_digest().to_owned(),
                bodies: self.bodies,
                plucks: self.counts[0],
                hammer_ons: self.counts[1],
                pull_offs: self.counts[2],
                slides: self.counts[3],
                simulated_seconds: (self.n_total - self.start) as f64 / self.sample_rate,
                strings: self.strings,
                source_work: r.source_work,
                control_work: r.control_work,
                internal_loss: r.internal_loss,
                radiation: r.radiation,
                cumulative_closure_residual: r.cumulative_closure_residual,
                maximum_frame_residual: r.maximum_frame_residual,
            }));
        stats
            .ariadne_energy
            .push(crate::AriadneTrace::Tab(self.trace));
        Ok(())
    }
}

/// Play every tab instrument and mix what the listener hears into the dry/wet buses.
#[allow(clippy::too_many_arguments)]
pub fn render(
    instruments: &[TabInstrument],
    bar_map: &BarMap,
    bar_content: &BTreeMap<u32, BTreeMap<String, String>>,
    n_bars: u32,
    swing_unit_ql: f64,
    swing_r: f64,
    sample_rate: f64,
    buffers: &mut MixBuffers,
    stats: &mut RenderStats,
) -> Result<(), RenderError> {
    let n_total = buffers.dry[0].len();
    let runs = prepare(
        instruments,
        bar_map,
        bar_content,
        n_bars,
        swing_unit_ql,
        swing_r,
        sample_rate,
        n_total,
        stats,
    )?;
    for run in runs {
        run.finish(buffers, stats)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swing_warp_matches_the_grid_and_carries_sixteenths() {
        let (unit, r) = (0.5, 0.64);
        for (q, want) in [
            (0.0, 0.0),
            (0.5, 0.64),
            (1.0, 1.0),
            (1.5, 1.64),
            (3.5, 3.64),
        ] {
            assert!((warp(q, unit, r) - want).abs() < 1e-12, "{q}");
            assert!((warp(q, unit, r) - crate::layout::swung(q, unit, r)).abs() < 1e-12);
        }
        assert!((warp(3.75, unit, r) - (3.0 + 0.64 + 0.25 * 0.72)).abs() < 1e-12);
        assert_eq!(warp(1.25, 0.0, 0.64), 1.25);
    }
}
