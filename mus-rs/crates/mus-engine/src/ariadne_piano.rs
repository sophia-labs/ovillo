//! A grand piano played on one persistent Ariadne world
//! (`SPEC-ARIADNE-PIANO.md`).
//!
//! An instrument whose declaration carries `anatomy=piano-v1` is not a voice.
//! Its dot-voices (`pn.r`, `pn.l`, ...) are staves of one physical piano; a
//! dot-voice declared `pedal=sustain` is its sustain pedal. Every note is a
//! key: the key lifts its strings' damper, throws its hammer at the velocity
//! of its dynamic, and lets the damper fall again when it is released, unless
//! the pedal holds the dampers up. For each score the world is composed from
//! `profiles/ariadne-piano-v1.anatomy.json`: the soundboard, and the strings
//! and hammer of every key the score plays. A stereo listener observes the
//! world, and the observation mixes into the dry bus like any other voice.
//! Nothing here synthesizes a note: every sound is the world's answer to a
//! hammer.

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::TAU;
use std::path::Path;

use mus_ariadne::{
    BodyConstitutionSpec, BodyControls, BodyPose, ConfigurationCoordinate, ConfigurationMapSpec,
    ConfigurationSpaceSpec, ConnectionPolicy, ConstitutiveBodyControls, ConstitutiveBodyInstanceSpec,
    ConstitutiveWorldPlan, DamperSpec, DirectCouplingSpec, HammerContactSpec, HammerSpec, MetricSpec,
    ReservoirSpec, SharedSpaceSpec, SpaceBodyControls, SpaceBodyLaws, SpaceBodySpec, SpacePortSpec,
    StereoBodyTap, StereoListener, StereoListenerSpec, StiffWireSpec, World, WorldConfiguration,
    WorldControls, ApertureSpec, MODE_COUNT, SPACE_COURSES, WORLD_PLAN_V3_SCHEMA_VERSION,
};
use mus_notation::{expand_pattern_repetitions, parse_token, split_attached_dynamics, tokenize_bar_content};
use mus_text::ProposalInstrument;
use serde::{Deserialize, Serialize};

use crate::layout::BarMap;
use crate::mixbus::{db_to_gain, MixBuffers};
use crate::util::{ART_GAIN, ART_SHORTEN};
use crate::{RenderError, RenderStats};

/// `anatomy=piano-v1`: the grand compiled by `mus_analysis.piano_anatomy`.
const PIANO_V1: &str = include_str!("../../../../profiles/ariadne-piano-v1.anatomy.json");
/// Hammer velocity at the string for each written dynamic (m/s). Chabassier,
/// Chaigne & Joly (2016) play p at 0.5 m/s and ff at 4 to 4.5 m/s; Bank (2000)
/// puts the largest a piano reaches near 6 to 7 m/s. Steps of x1.47 between
/// marks (design-hypothesis).
const VELOCITY: &[(&str, f64)] = &[
    ("ppp", 0.34),
    ("pp", 0.5),
    ("p", 0.74),
    ("mp", 1.09),
    ("mf", 1.6),
    ("f", 2.35),
    ("ff", 3.45),
    ("fff", 5.1),
    ("sf", 3.45),
    ("sfz", 3.8),
];
/// An accent (or a `gain=` in dB) scales the hammer velocity: +12 dB doubles it.
const ARTICULATION_DB_PER_DOUBLING: f64 = 12.0;
/// The key lifts its damper this long before the hammer reaches the strings
/// (the damper rises with the key, halfway through its travel), and the
/// felt leaves the wire over `DAMPER_LIFT_SECONDS`. After release the damper
/// falls for `DAMPER_FALL_SECONDS` and its felt settles over
/// `DAMPER_LAND_SECONDS` (design-hypothesis).
const DAMPER_LEAD_SECONDS: f64 = 0.015;
const DAMPER_LIFT_SECONDS: f64 = 0.004;
const DAMPER_FALL_SECONDS: f64 = 0.008;
const DAMPER_LAND_SECONDS: f64 = 0.012;
/// A pedal mark goes down this long after its written onset (the foot waits for
/// the new harmony's keys, then catches them: syncopated pedalling).
const PEDAL_LAG_SECONDS: f64 = 0.08;
/// The tuning pin's range (tension ratio).
const TENSION_RANGE: (f64, f64) = (0.98, 1.02);
/// World observation is in the world's own units (sqrt(W), see `field.rs::radiate`).
/// This brings a mezzo-forte piano near the engine's other voices.
const OBSERVATION_GAIN: f64 = 40.0;
const ENERGY_HOP_SECONDS: f64 = 0.01;

pub fn is_piano_anatomy(value: &str) -> bool {
    let value = value.trim();
    value.starts_with("piano-") || value.ends_with(".anatomy.json")
}

// ---------------------------------------------------------------- the anatomy (data)

#[derive(Debug, Clone, Deserialize)]
struct Anatomy {
    schema: String,
    sample_rate_hz: f64,
    wire: WireMaterial,
    notes: Vec<KeyAnatomy>,
    board: BoardAnatomy,
    room: SharedSpaceSpec,
}

#[derive(Debug, Clone, Deserialize)]
struct WireMaterial {
    youngs_modulus_pa: f64,
    decay_constant_per_s: f64,
    decay_quadratic_s: f64,
    band_limit_hz: f64,
    air_density_kg_m3: f64,
    speed_of_sound_m_s: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct KeyAnatomy {
    key: usize,
    name: String,
    speaking_length_m: f64,
    strike_fraction: f64,
    core_diameter_m: f64,
    outer_diameter_m: f64,
    linear_density_kg_m: f64,
    hammer: HammerAnatomy,
    damper: Option<DamperSpec>,
    wires: Vec<WireAnatomy>,
}

#[derive(Debug, Clone, Deserialize)]
struct HammerAnatomy {
    mass_kg: f64,
    felt_stiffness: f64,
    felt_exponent: f64,
    felt_relaxation_s: f64,
    contact_width_m: f64,
    letoff_m: f64,
    catch_m: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct WireAnatomy {
    tension_n: f64,
    board_shapes: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, Deserialize)]
struct BoardAnatomy {
    bodies: Vec<BoardBody>,
    room_weight: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct BoardBody {
    id: String,
    frequency_hz: Vec<f64>,
    t60_seconds: Vec<f64>,
    radiation_direction: Vec<f64>,
    radiation_fraction: f64,
}

// ---------------------------------------------------------------- the score side

pub struct PianoInstrument {
    pub abbrev: String,
    pub anatomy_name: String,
    anatomy: Anatomy,
    staves: Vec<Staff>,
    pedal: Option<Pedal>,
    gain_db: f64,
    send: f64,
}

struct Staff {
    abbrev: String,
    gain_db: f64,
}

struct Pedal {
    abbrev: String,
    lag_s: f64,
}

/// One key going down: when the hammer meets the strings, how hard, and when the key is let go.
#[derive(Debug, Clone, Copy)]
struct KeyPress {
    onset_s: f64,
    key: usize,
    velocity: f64,
    release_s: f64,
    staff: usize,
}

/// What a piano's world did, for the render receipt.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PianoWorldReceipt {
    pub instrument: String,
    pub anatomy: String,
    pub plan_digest: String,
    pub listener_digest: String,
    pub bodies: Vec<String>,
    /// Every key the score plays: its strings and what the wire law made of them.
    pub keys: Vec<KeyReceipt>,
    pub strikes: usize,
    /// Throws the World refused because the felt still pressed on the strings.
    pub refused_strikes: usize,
    pub damper_falls: usize,
    pub pedal_spans: usize,
    pub simulated_seconds: f64,
    /// Felt contact of each strike (ms), shortest and longest.
    pub contact_ms_min: f64,
    pub contact_ms_max: f64,
    pub source_work: f64,
    pub control_work: f64,
    pub internal_loss: f64,
    pub radiation: f64,
    /// Radiation by the soundboard and by the strings themselves (J).
    pub board_radiation: f64,
    pub string_radiation: f64,
    pub cumulative_closure_residual: f64,
    pub maximum_frame_residual: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyReceipt {
    pub key: usize,
    pub name: String,
    pub strings: usize,
    pub retained_partials: usize,
    pub fundamental_hz: f64,
    pub inharmonicity: f64,
    pub strike_fraction: f64,
    pub hammer_mass_kg: f64,
    pub damped: bool,
}

/// Stored energy of every body (and the room) through the render.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PianoEnergyTrace {
    pub instrument: String,
    pub hop_seconds: f64,
    pub start_seconds: f64,
    /// Every body (soundboard, strings, hammers), then the room: stored energy at each hop.
    pub channels: Vec<String>,
    pub frames: Vec<Vec<f32>>,
    /// Energy moved during each hop, per body: put in, radiated, lost inside.
    pub source_frames: Vec<Vec<f32>>,
    pub radiation_frames: Vec<Vec<f32>>,
    pub loss_frames: Vec<Vec<f32>>,
    /// Per string body: energy the bridge moved into it during each hop
    /// (negative: into the soundboard).
    pub strings: Vec<String>,
    pub bridge_frames: Vec<Vec<f32>>,
    /// Every key the score pressed, as it reached the world.
    pub strikes: Vec<StrikeRecord>,
    pub pedal: Vec<[f64; 2]>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrikeRecord {
    pub seconds: f64,
    pub midi: usize,
    pub name: String,
    /// The hammer's velocity at escapement (m/s).
    pub hammer_velocity: f64,
    pub release_seconds: f64,
    pub staff: String,
    /// Felt contact (ms) and peak force (N) the World reported.
    pub contact_ms: f64,
    pub peak_force_n: f64,
    pub refused: bool,
}

/// A piano part rendered to the world's own observation (no bus, no master).
pub struct PianoPerformance {
    pub start: usize,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
    pub receipt: PianoWorldReceipt,
    pub trace: PianoEnergyTrace,
    pub send: f64,
    pub gain: f64,
}

/// The pianos in a score, and every abbrev they own (base, staves, pedal).
pub fn collect(
    instruments: &[ProposalInstrument],
    score_dir: &Path,
) -> Result<(Vec<PianoInstrument>, Vec<String>), RenderError> {
    let mut found = Vec::new();
    let mut owned = Vec::new();
    for base in instruments.iter().filter(|i| i.params.get("anatomy").is_some_and(|a| is_piano_anatomy(a))) {
        let invalid = |msg: String| RenderError::Invalid(format!("{}: {msg}", base.abbrev));
        let anatomy_name = base.params["anatomy"].trim().to_owned();
        let text = if anatomy_name == "piano-v1" {
            PIANO_V1.to_owned()
        } else {
            std::fs::read_to_string(score_dir.join(&anatomy_name)).map_err(|e| invalid(format!("anatomy {anatomy_name}: {e}")))?
        };
        let anatomy: Anatomy =
            serde_json::from_str(&text).map_err(|e| invalid(format!("anatomy {anatomy_name}: {e}")))?;
        if anatomy.schema != "mus.ariadne.piano-anatomy.v1" {
            return Err(invalid(format!("anatomy {anatomy_name} has schema {}", anatomy.schema)));
        }
        let number = |key: &str, default: f64, params: &BTreeMap<String, String>| -> Result<f64, RenderError> {
            params.get(key).map_or(Ok(default), |v| v.trim().parse::<f64>().map_err(|_| invalid(format!("{key}={v}"))))
        };
        let prefix = format!("{}.", base.abbrev);
        let mut staves = Vec::new();
        let mut pedal = None;
        for voice in instruments.iter().filter(|i| i.abbrev.starts_with(&prefix)) {
            match voice.params.get("pedal").map(|p| p.trim()) {
                Some("sustain") => {
                    pedal = Some(Pedal { abbrev: voice.abbrev.clone(), lag_s: number("lag", PEDAL_LAG_SECONDS, &voice.params)? })
                }
                Some(other) => return Err(invalid(format!("{}: pedal={other} (only pedal=sustain)", voice.abbrev))),
                None => staves.push(Staff { abbrev: voice.abbrev.clone(), gain_db: number("gain", 0.0, &voice.params)? }),
            }
        }
        owned.push(base.abbrev.clone());
        owned.extend(staves.iter().map(|s| s.abbrev.clone()));
        owned.extend(pedal.iter().map(|p| p.abbrev.clone()));
        found.push(PianoInstrument {
            abbrev: base.abbrev.clone(),
            anatomy_name,
            gain_db: number("gain", 0.0, &base.params)?,
            send: number("send", 0.0, &base.params)?,
            anatomy,
            staves,
            pedal,
        });
    }
    Ok((found, owned))
}

/// SPEC-TAB §5's swing warp, as the tab instruments use it.
fn warp(q: f64, unit: f64, ratio: f64) -> f64 {
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

fn velocity_of(dynamic: &str) -> f64 {
    VELOCITY.iter().find(|(name, _)| *name == dynamic).map_or(1.6, |(_, v)| *v)
}

#[allow(clippy::too_many_arguments)]
fn performance(
    inst: &PianoInstrument,
    bar_map: &BarMap,
    bar_content: &BTreeMap<u32, BTreeMap<String, String>>,
    n_bars: u32,
    swing_unit_ql: f64,
    swing_r: f64,
    stats: &mut RenderStats,
) -> (Vec<KeyPress>, Vec<[f64; 2]>) {
    let mut presses = Vec::new();
    let keys: BTreeSet<usize> = inst.anatomy.notes.iter().map(|n| n.key).collect();
    for (s, staff) in inst.staves.iter().enumerate() {
        let abbr = staff.abbrev.as_str();
        let mut cur_dyn = "mf".to_string();
        let mut count = 0_u32;
        for b in 1..=n_bars {
            let Some(content) = bar_content.get(&b).and_then(|m| m.get(abbr)) else { continue };
            if content.trim().is_empty() || content.trim() == "-" {
                continue;
            }
            let Some(timing) = bar_map.bars.get(&b) else { continue };
            let tokens = split_attached_dynamics(&expand_pattern_repetitions(&tokenize_bar_content(content)));
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
                let onset = timing.start_s + warp(q, swing_unit_ql, swing_r) * timing.spq;
                let end = timing.start_s + warp(q + ev.ql, swing_unit_ql, swing_r) * timing.spq;
                q += ev.ql;
                if !ev.tab.is_empty() {
                    stats.bad.push(format!("b{b} {abbr}: {tk} (a piano takes pitches, not @string:fret)"));
                    continue;
                }
                let shorten = ev
                    .flags
                    .iter()
                    .filter_map(|f| ART_SHORTEN.iter().find(|(n, _)| *n == f).map(|(_, r)| *r))
                    .fold(1.0_f64, f64::min);
                let art: f64 = ev.flags.iter().filter_map(|f| ART_GAIN.iter().find(|(n, _)| *n == f).map(|(_, d)| *d)).sum();
                let gain = ev.params.get("gain").and_then(|g| g.parse::<f64>().ok()).unwrap_or(0.0);
                let velocity = velocity_of(&cur_dyn) * 2f64.powf((art + gain + staff.gain_db) / ARTICULATION_DB_PER_DOUBLING);
                for midi in &ev.midis {
                    let key = (midi.round() as i64) - 20;
                    if (midi - midi.round()).abs() > 1e-9 || key < 1 || !keys.contains(&(key as usize)) {
                        stats.bad.push(format!("b{b} {abbr}: {tk} (midi {midi} is not a key of this piano)"));
                        continue;
                    }
                    presses.push(KeyPress {
                        onset_s: onset,
                        key: key as usize,
                        velocity,
                        release_s: onset + (end - onset) * shorten,
                        staff: s,
                    });
                    count += 1;
                }
            }
        }
        stats.per_track_events.insert(abbr.to_string(), count);
        stats.notes += count;
    }
    let mut pedal = Vec::new();
    if let Some(p) = &inst.pedal {
        let abbr = p.abbrev.as_str();
        let mut count = 0_u32;
        for b in 1..=n_bars {
            let Some(content) = bar_content.get(&b).and_then(|m| m.get(abbr)) else { continue };
            let Some(timing) = bar_map.bars.get(&b) else { continue };
            let mut q = 0.0_f64;
            for tk in &tokenize_bar_content(content) {
                let Some(ev) = parse_token(tk) else {
                    if !matches!(tk.trim(), "" | "-") {
                        stats.bad.push(format!("b{b} {abbr}: {tk}"));
                    }
                    continue;
                };
                let onset = timing.start_s + warp(q, swing_unit_ql, swing_r) * timing.spq;
                let end = timing.start_s + warp(q + ev.ql, swing_unit_ql, swing_r) * timing.spq;
                q += ev.ql;
                match ev.kind.as_str() {
                    // `X` (or any note) holds the pedal down; a rest lets it up.
                    "unpitched" | "note" | "chord" => {
                        let down = (onset + p.lag_s).min(end);
                        pedal.push([down, end]);
                        count += 1;
                    }
                    "rest" | "dynamic" | "hairpin" => {}
                    _ => stats.bad.push(format!("b{b} {abbr}: {tk} (the pedal takes X<duration> and rests)")),
                }
            }
        }
        stats.per_track_events.insert(abbr.to_string(), count);
    }
    presses.sort_by(|a, b| a.onset_s.total_cmp(&b.onset_s).then(a.key.cmp(&b.key)));
    pedal.sort_by(|a, b| a[0].total_cmp(&b[0]));
    (presses, pedal)
}

// ---------------------------------------------------------------- the world

fn unit(values: &[f64; MODE_COUNT]) -> [f64; MODE_COUNT] {
    let norm = values.iter().map(|v| v * v).sum::<f64>().sqrt();
    values.map(|v| v / norm)
}

fn inert_port() -> SpacePortSpec {
    SpacePortSpec {
        body_directions: [[1.0 / (MODE_COUNT as f64).sqrt(); MODE_COUNT]; SPACE_COURSES],
        coupling_weights: [0.0; SPACE_COURSES],
    }
}

/// The steel-string's port into the room (`steel_string_anatomy.py` `_space_port`).
fn room_port(direction: &[f64; MODE_COUNT], weight: f64) -> SpacePortSpec {
    let body_directions = std::array::from_fn(|course| {
        let sign = if course % 2 == 0 { 1.0 } else { -1.0 };
        let rolled: [f64; MODE_COUNT] =
            std::array::from_fn(|i| sign * direction[(i + MODE_COUNT - course % MODE_COUNT) % MODE_COUNT] + 0.15 * direction[i]);
        unit(&rolled)
    });
    SpacePortSpec { body_directions, coupling_weights: std::array::from_fn(|c| weight * 0.9_f64.powi(c as i32)) }
}

fn inert_aperture() -> ApertureSpec {
    ApertureSpec {
        natural_frequency_hz: 500.0,
        quartic_stiffness: 0.0,
        asymmetry: 0.0,
        equilibrium_displacement: 0.0,
        opening_bias: 0.0,
        damping_per_second: 50.0,
        source_to_aperture_rate_hz: 0.0,
        source_to_field_rate_hz: 0.0,
        sweep_rate_hz: 0.0,
        active_rate_per_second: 0.0,
        active_target_momentum: 0.25,
    }
}

fn wire_spec(anatomy: &Anatomy, key: &KeyAnatomy, wire: &WireAnatomy) -> StiffWireSpec {
    StiffWireSpec {
        speaking_length_m: key.speaking_length_m,
        core_diameter_m: key.core_diameter_m,
        linear_density_kg_m: key.linear_density_kg_m,
        youngs_modulus_pa: anatomy.wire.youngs_modulus_pa,
        tension_n: wire.tension_n,
        tension_coordinate: 0,
        band_limit_hz: anatomy.wire.band_limit_hz,
        decay_constant_per_s: anatomy.wire.decay_constant_per_s,
        decay_quadratic_s: anatomy.wire.decay_quadratic_s,
    }
}

fn space_body(anatomy_name: String, map: ConfigurationMapSpec, coordinate: ConfigurationCoordinate, damper: Option<DamperSpec>) -> BodyConstitutionSpec {
    BodyConstitutionSpec::ModalSpace {
        body: SpaceBodySpec {
            anatomy_name,
            configuration_space: ConfigurationSpaceSpec {
                coordinates: vec![coordinate],
                map,
                connection: ConnectionPolicy::ModalCoordinates,
            },
            laws: SpaceBodyLaws::Passive { damper },
            reservoir: ReservoirSpec { capacity: 1.0e-6 },
        },
    }
}

/// Where each key's strings and hammer sit in the composed world.
struct KeyBodies {
    strings: Vec<usize>,
    hammer: usize,
    damped: bool,
}

struct Composition {
    plan: ConstitutiveWorldPlan,
    listener: StereoListenerSpec,
    radiation_fraction: Vec<f64>,
    keys: BTreeMap<usize, KeyBodies>,
    board_bodies: Vec<usize>,
    string_bodies: Vec<usize>,
    receipts: Vec<KeyReceipt>,
}

/// One world for the keys a score plays: the soundboard bodies, each key's
/// strings (stiff wires with their dampers) and hammer, one bridge edge per
/// string and board body, and the room.
fn compose(anatomy: &Anatomy, keys: &BTreeSet<usize>) -> Result<Composition, String> {
    let fs = anatomy.sample_rate_hz;
    let mut bodies = Vec::new();
    let mut radiation_fraction = Vec::new();
    let mut taps = Vec::new();
    let static_axis = ConfigurationCoordinate { name: "static".into(), lower: 0.0, upper: 1.0, initial: 0.0 };
    let mut board_bodies = Vec::new();
    for board in &anatomy.board.bodies {
        let direction: [f64; MODE_COUNT] = board.radiation_direction.as_slice().try_into().map_err(|_| "board direction size")?;
        let configuration = WorldConfiguration {
            metric: MetricSpec {
                frequency_hz: board.frequency_hz.as_slice().try_into().map_err(|_| "board frequency size")?,
                t60_seconds: board.t60_seconds.as_slice().try_into().map_err(|_| "board t60 size")?,
            },
            aperture: inert_aperture(),
            radiation_direction: direction,
        };
        board_bodies.push(bodies.len());
        bodies.push(ConstitutiveBodyInstanceSpec {
            id: board.id.clone(),
            constitution: space_body(
                format!("piano-soundboard-{}", board.id.replace('/', "-")),
                ConfigurationMapSpec::Multilinear { axes: vec![vec![0.0, 1.0]], anchors: vec![configuration, configuration] },
                static_axis.clone(),
                None,
            ),
            space_port: room_port(&direction.map(f64::abs), anatomy.board.room_weight),
            pose: BodyPose::default(),
        });
        radiation_fraction.push(board.radiation_fraction);
        taps.push(StereoBodyTap { body_id: board.id.clone(), left_projection: 0.5_f64.sqrt(), right_projection: 0.5_f64.sqrt() });
    }
    let mut edges = Vec::new();
    let mut layout = BTreeMap::new();
    let mut string_bodies = Vec::new();
    let mut receipts = Vec::new();
    let tension_axis = ConfigurationCoordinate { name: "tension_ratio".into(), lower: TENSION_RANGE.0, upper: TENSION_RANGE.1, initial: 1.0 };
    for &k in keys {
        let key = anatomy.notes.iter().find(|n| n.key == k).ok_or_else(|| format!("no key {k}"))?;
        let mut strings = Vec::new();
        let mut contacts = Vec::new();
        let pan = 0.25 + 0.5 * (k as f64 - 1.0) / 87.0;
        for (s, wire) in key.wires.iter().enumerate() {
            let spec = wire_spec(anatomy, key, wire);
            let retained = spec.retained_partials(TENSION_RANGE.1);
            let id = format!("string/{}/{s}", key.name);
            // The wire's own line-dipole radiation, as a projection fraction.
            let sigma = spec.dipole_radiation_sigma(1.0, retained, key.outer_diameter_m, anatomy.wire.air_density_kg_m3, anatomy.wire.speed_of_sound_m_s);
            radiation_fraction.push(4.0 * sigma.iter().sum::<f64>() / fs);
            // The bridge: partial n against board mode m exchange at omega_n G_n Phi_m(pin).
            let weights = spec.moving_end_weights(1.0, retained);
            let weight_norm = weights.iter().map(|v| v * v).sum::<f64>().sqrt();
            for (b, shape) in wire.board_shapes.iter().enumerate() {
                let shape: [f64; MODE_COUNT] = shape.as_slice().try_into().map_err(|_| "board shape size")?;
                let shape_norm = shape.iter().map(|v| v * v).sum::<f64>().sqrt();
                if shape_norm < 1.0e-9 {
                    continue;
                }
                edges.push(DirectCouplingSpec {
                    id: format!("bridge/{}/{s}/{b}", key.name),
                    left_body: id.clone(),
                    right_body: anatomy.board.bodies[b].id.clone(),
                    left_direction: unit(&weights),
                    right_direction: shape.map(|v| v / shape_norm),
                    rate_hz: weight_norm * shape_norm / TAU,
                    weight: 1.0,
                });
            }
            strings.push(bodies.len());
            string_bodies.push(bodies.len());
            contacts.push(HammerContactSpec { body: id.clone(), strike_fraction: key.strike_fraction });
            bodies.push(ConstitutiveBodyInstanceSpec {
                id: id.clone(),
                constitution: space_body(
                    format!("piano-wire-{}-{s}", key.name),
                    ConfigurationMapSpec::StiffWire { wire: spec.clone() },
                    tension_axis.clone(),
                    key.damper,
                ),
                space_port: inert_port(),
                pose: BodyPose::default(),
            });
            taps.push(StereoBodyTap { body_id: id, left_projection: (1.0 - pan).sqrt(), right_projection: pan.sqrt() });
            if s == 0 {
                receipts.push(KeyReceipt {
                    key: k,
                    name: key.name.clone(),
                    strings: key.wires.len(),
                    retained_partials: retained,
                    fundamental_hz: spec.flexible_fundamental_hz(1.0),
                    inharmonicity: spec.inharmonicity(1.0),
                    strike_fraction: key.strike_fraction,
                    hammer_mass_kg: key.hammer.mass_kg,
                    damped: key.damper.is_some(),
                });
            }
        }
        let h = &key.hammer;
        let hammer = bodies.len();
        bodies.push(ConstitutiveBodyInstanceSpec {
            id: format!("hammer/{}", key.name),
            constitution: BodyConstitutionSpec::Hammer {
                hammer: HammerSpec {
                    mass_kg: h.mass_kg,
                    felt_stiffness: h.felt_stiffness,
                    felt_exponent: h.felt_exponent,
                    felt_relaxation_s: h.felt_relaxation_s,
                    contact_width_m: h.contact_width_m,
                    letoff_m: h.letoff_m,
                    catch_m: h.catch_m,
                    wires: contacts,
                },
            },
            space_port: inert_port(),
            pose: BodyPose::default(),
        });
        radiation_fraction.push(0.0);
        layout.insert(k, KeyBodies { strings, hammer, damped: key.damper.is_some() });
    }
    let plan = ConstitutiveWorldPlan {
        schema_version: WORLD_PLAN_V3_SCHEMA_VERSION.to_owned(),
        world_name: "piano-parlour-v1".to_owned(),
        internal_sample_rate_hz: fs,
        bodies,
        shared_space: anatomy.room,
        direct_couplings: edges,
    };
    plan.validate().map_err(|e| e.to_string())?;
    let listener = StereoListenerSpec::for_constitutive(&plan, taps, 1.0).map_err(|e| e.to_string())?;
    Ok(Composition { plan, listener, radiation_fraction, keys: layout, board_bodies, string_bodies, receipts })
}

/// A key's damper: resting (1) unless the key or the pedal holds it up.
struct DamperState {
    engagement: f64,
    falling_since: Option<usize>,
    /// Sample spans `[a, b)` during which the damper is held up, merged and in order.
    spans: Vec<(usize, usize)>,
    cursor: usize,
}

impl DamperState {
    /// Held up at sample `k` (samples are visited in order).
    fn lifted(&mut self, k: usize) -> bool {
        while self.cursor < self.spans.len() && self.spans[self.cursor].1 <= k {
            self.cursor += 1;
        }
        self.cursor < self.spans.len() && self.spans[self.cursor].0 <= k
    }
}

/// Play one piano part on its world. Returns the world's own observation.
#[allow(clippy::too_many_arguments)]
fn play(
    inst: &PianoInstrument,
    bar_map: &BarMap,
    bar_content: &BTreeMap<u32, BTreeMap<String, String>>,
    n_bars: u32,
    swing_unit_ql: f64,
    swing_r: f64,
    sample_rate: f64,
    n_total: usize,
    stats: &mut RenderStats,
) -> Result<Option<PianoPerformance>, RenderError> {
    let world_err = |e: String| RenderError::Invalid(format!("{} world: {e}", inst.abbrev));
    let (presses, pedal) = performance(inst, bar_map, bar_content, n_bars, swing_unit_ql, swing_r, stats);
    if presses.is_empty() {
        return Ok(None);
    }
    if (inst.anatomy.sample_rate_hz - sample_rate).abs() > 1e-6 {
        return Err(world_err(format!("anatomy runs at {} Hz, the engine at {sample_rate} Hz", inst.anatomy.sample_rate_hz)));
    }
    let used: BTreeSet<usize> = presses.iter().map(|p| p.key).collect();
    let composition = compose(&inst.anatomy, &used).map_err(world_err)?;
    let listener = StereoListener::bind_constitutive(&composition.listener, &composition.plan).map_err(|e| world_err(e.to_string()))?;
    let mut world = World::from_constitutive_plan(composition.plan.clone()).map_err(|e| world_err(e.to_string()))?;
    let bodies: Vec<String> = (0..world.body_count()).map(|i| world.body_id(i).unwrap_or("?").to_owned()).collect();
    let controls: Vec<ConstitutiveBodyControls> = composition
        .radiation_fraction
        .iter()
        .map(|&fraction| {
            ConstitutiveBodyControls::ModalSpace(SpaceBodyControls {
                body: BodyControls { radiation_fraction: fraction, ..BodyControls::default() },
                ..SpaceBodyControls::default()
            })
        })
        .collect();

    // ---- the performance, as per-sample actions on the world
    let dt = 1.0 / sample_rate;
    let at = |seconds: f64| (seconds.max(0.0) * sample_rate).round() as usize;
    let key_of = |k: usize| inst.anatomy.notes.iter().find(|n| n.key == k).expect("composed key");
    // The hammer leaves the escapement letoff / v before it meets the strings.
    let mut launches: Vec<(usize, usize, f64, usize)> = presses
        .iter()
        .enumerate()
        .map(|(i, p)| (at(p.onset_s - key_of(p.key).hammer.letoff_m / p.velocity), p.key, p.velocity, i))
        .collect();
    launches.sort_by_key(|l| (l.0, l.1));
    let first = launches[0].0.min(at(presses[0].onset_s - DAMPER_LEAD_SECONDS));
    let start = first.saturating_sub(at(0.005));
    // Each damper is held up while its key is down (from DAMPER_LEAD_SECONDS before the
    // hammer arrives to the release) or the pedal is: merged sample spans per key.
    let pedal_spans: Vec<(usize, usize)> = pedal.iter().map(|span| (at(span[0]), at(span[1]))).collect();
    let mut dampers: BTreeMap<usize, DamperState> = composition
        .keys
        .iter()
        .filter(|(_, k)| k.damped)
        .map(|(&key, _)| {
            let mut spans: Vec<(usize, usize)> = presses
                .iter()
                .filter(|p| p.key == key)
                .map(|p| (at(p.onset_s - DAMPER_LEAD_SECONDS), at(p.release_s)))
                .chain(pedal_spans.iter().copied())
                .collect();
            spans.sort_unstable();
            let mut merged: Vec<(usize, usize)> = Vec::with_capacity(spans.len());
            for (a, b) in spans {
                match merged.last_mut() {
                    Some(last) if a <= last.1 => last.1 = last.1.max(b),
                    _ => merged.push((a, b)),
                }
            }
            let mut state = DamperState { engagement: 1.0, falling_since: None, spans: merged, cursor: 0 };
            // A damper rests at the start unless its key (or the pedal) is already down.
            if state.lifted(start) {
                state.engagement = 0.0;
            }
            (key, state)
        })
        .collect();
    for (key, state) in &dampers {
        for &body in &composition.keys[key].strings {
            world.set_body_damper(body, state.engagement).map_err(|e| world_err(e.to_string()))?;
        }
    }
    let hop = (ENERGY_HOP_SECONDS * sample_rate) as usize;
    let edges = world.coupling_count();
    let edge_string: Vec<Option<usize>> = (0..edges)
        .map(|e| {
            let id = world.coupling_id(e).unwrap_or("");
            let rest = id.strip_prefix("bridge/")?;
            let (prefix, _) = rest.rsplit_once('/')?;
            composition.string_bodies.iter().position(|&b| bodies[b] == format!("string/{prefix}"))
        })
        .collect();
    let mut trace = PianoEnergyTrace {
        instrument: inst.abbrev.clone(),
        hop_seconds: ENERGY_HOP_SECONDS,
        start_seconds: start as f64 / sample_rate,
        channels: bodies.iter().cloned().chain(std::iter::once("room".to_owned())).collect(),
        frames: Vec::new(),
        source_frames: Vec::new(),
        radiation_frames: Vec::new(),
        loss_frames: Vec::new(),
        strings: composition.string_bodies.iter().map(|&b| bodies[b].clone()).collect(),
        bridge_frames: Vec::new(),
        strikes: Vec::new(),
        pedal: pedal.clone(),
    };
    let n = bodies.len();
    let (mut source_acc, mut radiation_acc, mut loss_acc) = (vec![0.0_f64; n], vec![0.0_f64; n], vec![0.0_f64; n]);
    let mut bridge_acc = vec![0.0_f64; composition.string_bodies.len()];
    let mut left = vec![0.0_f32; n_total.saturating_sub(start)];
    let mut right = vec![0.0_f32; left.len()];
    let mut next_launch = 0;
    let mut pending_records: Vec<(usize, usize, StrikeRecord)> = Vec::new(); // (hammer body, strikes count at launch, record)
    let mut damper_falls = 0_usize;
    let (mut board_radiation, mut string_radiation) = (0.0_f64, 0.0_f64);
    let is_board: Vec<bool> = (0..n).map(|i| composition.board_bodies.contains(&i)).collect();
    let is_string: Vec<bool> = (0..n).map(|i| composition.string_bodies.contains(&i)).collect();
    let mut refused = 0_usize;
    for k in start..n_total {
        while next_launch < launches.len() && launches[next_launch].0 <= k {
            let (_, key, velocity, press) = launches[next_launch];
            let hammer = composition.keys[&key].hammer;
            let accepted = world.strike_hammer(hammer, velocity).map_err(|e| world_err(e.to_string()))?;
            let p = &presses[press];
            let record = StrikeRecord {
                seconds: p.onset_s,
                midi: key + 20,
                name: key_of(key).name.clone(),
                hammer_velocity: velocity,
                release_seconds: p.release_s,
                staff: inst.staves[p.staff].abbrev.clone(),
                contact_ms: 0.0,
                peak_force_n: 0.0,
                refused: !accepted,
            };
            if accepted {
                let strikes = world.hammer_totals(hammer).map_or(0, |t| t.strikes);
                pending_records.push((hammer, strikes, record));
            } else {
                refused += 1;
                trace.strikes.push(record);
            }
            next_launch += 1;
        }
        // A strike's record is complete once its hammer rests or is thrown again.
        pending_records.retain(|(hammer, strikes, record)| {
            let totals = world.hammer_totals(*hammer).expect("hammer");
            if world.hammer_active(*hammer) == Some(true) && totals.strikes == *strikes {
                return true;
            }
            let mut record = record.clone();
            if totals.strikes == *strikes {
                record.contact_ms = totals.last_contact_samples as f64 / sample_rate * 1e3;
                record.peak_force_n = totals.last_peak_force_n;
            }
            trace.strikes.push(record);
            false
        });
        for (key, state) in dampers.iter_mut() {
            let lifted = state.lifted(k);
            let before = state.engagement;
            if lifted {
                state.falling_since = None;
                state.engagement = (state.engagement - dt / DAMPER_LIFT_SECONDS).max(0.0);
            } else {
                let since = *state.falling_since.get_or_insert(k);
                if k == since && before < 1.0 {
                    damper_falls += 1;
                }
                if (k - since) as f64 * dt >= DAMPER_FALL_SECONDS {
                    state.engagement = (state.engagement + dt / DAMPER_LAND_SECONDS).min(1.0);
                }
            }
            if state.engagement.to_bits() != before.to_bits() {
                for &body in &composition.keys[key].strings {
                    world.set_body_damper(body, state.engagement).map_err(|e| world_err(e.to_string()))?;
                }
            }
        }
        let frame = world.process_constitutive_sample(&controls, WorldControls::default()).map_err(|e| world_err(e.to_string()))?;
        let heard = listener.observe(&world, &frame).map_err(|e| world_err(e.to_string()))?;
        left[k - start] = heard.left as f32;
        right[k - start] = heard.right as f32;
        for i in 0..n {
            source_acc[i] += world.body_last_source_work(i).unwrap_or(0.0);
            loss_acc[i] += world.body_last_internal_loss(i).unwrap_or(0.0);
            let e = world.body_emission(i).unwrap_or(0.0);
            let radiated = 0.5 * dt * e * e;
            radiation_acc[i] += radiated;
            if is_board[i] {
                board_radiation += radiated;
            } else if is_string[i] {
                string_radiation += radiated;
            }
        }
        for (e, string) in edge_string.iter().enumerate() {
            if let Some(s) = string {
                bridge_acc[*s] += world.coupling_last_exchange_into_left(e).unwrap_or(0.0);
            }
        }
        if (k - start) % hop == 0 {
            let mut row: Vec<f32> = (0..n).map(|i| world.body_material_energy(i).unwrap_or(0.0) as f32).collect();
            row.push(world.space_stored_energy() as f32);
            trace.frames.push(row);
            let take = |acc: &mut Vec<f64>| -> Vec<f32> { acc.iter_mut().map(|v| std::mem::take(v) as f32).collect() };
            trace.source_frames.push(take(&mut source_acc));
            trace.radiation_frames.push(take(&mut radiation_acc));
            trace.loss_frames.push(take(&mut loss_acc));
            trace.bridge_frames.push(take(&mut bridge_acc));
        }
    }
    for (hammer, strikes, mut record) in pending_records {
        let totals = world.hammer_totals(hammer).expect("hammer");
        if totals.strikes == strikes {
            record.contact_ms = totals.last_contact_samples as f64 / sample_rate * 1e3;
            record.peak_force_n = totals.last_peak_force_n;
        }
        trace.strikes.push(record);
    }
    trace.strikes.sort_by(|a, b| a.seconds.total_cmp(&b.seconds).then(a.midi.cmp(&b.midi)));
    let contacts: Vec<f64> = trace.strikes.iter().filter(|s| !s.refused && s.contact_ms > 0.0).map(|s| s.contact_ms).collect();
    let r = world.receipt();
    let receipt = PianoWorldReceipt {
        instrument: inst.abbrev.clone(),
        anatomy: inst.anatomy_name.clone(),
        plan_digest: world.plan_digest().to_owned(),
        listener_digest: listener.listener_digest().to_owned(),
        bodies,
        keys: composition.receipts,
        strikes: presses.len() - refused,
        refused_strikes: refused,
        damper_falls,
        pedal_spans: pedal.len(),
        simulated_seconds: (n_total - start) as f64 / sample_rate,
        contact_ms_min: contacts.iter().copied().fold(f64::INFINITY, f64::min),
        contact_ms_max: contacts.iter().copied().fold(0.0, f64::max),
        source_work: r.source_work,
        control_work: r.control_work,
        internal_loss: r.internal_loss,
        radiation: r.radiation,
        board_radiation,
        string_radiation,
        cumulative_closure_residual: r.cumulative_closure_residual,
        maximum_frame_residual: r.maximum_frame_residual,
    };
    Ok(Some(PianoPerformance {
        start,
        left,
        right,
        receipt,
        trace,
        send: inst.send,
        gain: OBSERVATION_GAIN * db_to_gain(inst.gain_db),
    }))
}

/// Play every piano and mix what its listener hears into the dry/wet buses.
#[allow(clippy::too_many_arguments)]
pub fn render(
    instruments: &[PianoInstrument],
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
    for inst in instruments {
        let Some(performance) = play(inst, bar_map, bar_content, n_bars, swing_unit_ql, swing_r, sample_rate, n_total, stats)? else {
            continue;
        };
        buffers.mix_one(performance.start, &performance.left, &performance.right, performance.send, performance.gain, None);
        stats.ariadne_worlds.push(crate::AriadneReceipt::Piano(performance.receipt));
        stats.ariadne_energy.push(crate::AriadneTrace::Piano(performance.trace));
    }
    Ok(())
}

/// The pianos of a score played on their worlds, without the engine's bus:
/// each part's own observation (left/right, gain applied, from sample
/// `start`), its receipt and its energy trace. For measuring the World
/// itself; `mus render` mixes the same samples into the bus and master.
pub fn play_score(source: &str, score_dir: &Path) -> Result<Vec<PianoPerformance>, RenderError> {
    let (proposal, _diags) = mus_text::parse_score_lossy(source);
    let headers = &proposal.headers;
    let (swing_unit_ql, swing_r) = crate::layout::parse_swing(headers.get("swing"));
    let (pianos, _) = collect(&proposal.instruments, score_dir)?;
    let abbrevs: Vec<String> = proposal.instruments.iter().map(|i| i.abbrev.clone()).collect();
    let raw = crate::rawbars::scan(source, &abbrevs);
    let n_bars = headers
        .get("bars")
        .and_then(|v| v.trim().parse::<u32>().ok())
        .filter(|&n| n != 0)
        .unwrap_or(raw.max_bar);
    let bar_map = crate::layout::build_bar_map(headers.get("tempo"), headers.get("time"), &raw.inline_tempo, &raw.inline_time, n_bars)?;
    let n_total = (bar_map.total_s * mus_dsp::SR_F64) as usize;
    let mut stats = RenderStats::default();
    let mut out = Vec::new();
    for inst in &pianos {
        if let Some(p) = play(inst, &bar_map, &raw.bar_content, n_bars, swing_unit_ql, swing_r, mus_dsp::SR_F64, n_total, &mut stats)? {
            out.push(p);
        }
    }
    if !stats.bad.is_empty() {
        return Err(RenderError::Invalid(format!("bad tokens: {:?}", stats.bad)));
    }
    Ok(out)
}
