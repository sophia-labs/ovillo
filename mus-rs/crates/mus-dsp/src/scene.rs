//! The spatial scene renderer: a port of the offline half of the MUS
//! spatial-canvas contract — `mus_analysis/spatial_render.py`
//! (`render_stereo`, `render_foa`) over `mus_analysis/spatial_scene.py`
//! (`SpatialScene`/`SpatialObject`/`Position`/`PositionKeyframe`/`Room`/
//! `Listener`) — to sample parity.
//!
//! Coordinates follow Web Audio's default listener convention: `+X`
//! right, `+Y` up, `-Z` front. The scene stores mono object streams plus
//! metadata; stereo and Ambisonic renders are derived, never authoring
//! authority.
//!
//! # Precision policy — this module is `f64`, not `f32`
//!
//! Unlike the rest of `mus-dsp` (built around `mus_audio.py`'s `f32`
//! engine), the oracle here reads object audio as `float64`
//! (`sf.read(path, always_2d=True, dtype="float64")`) and stays `float64`
//! through the whole per-object pipeline — resample, controls, gain,
//! spatialization, room, mix. Only the very last step narrows to `f32`
//! (`mix.T.astype(np.float32)`, the fixture/WAV dtype). This module
//! mirrors that: every object buffer and the mix/room accumulators are
//! `Vec<f64>`/`[f64; N]`, narrowing to `f32` exactly once, at the public
//! `render_stereo`/`render_foa` boundary. The one deliberate exception is
//! noted at [`load_object_audio`] (the soxr resample branch, reused from
//! [`crate::pitch`] at its native `f32`, and untested by the current
//! fixtures — every fixture stem is already scene-rate-native).
//!
//! # Reuse
//!
//! - Object load + resample: [`crate::pitch::load_decode_native`] /
//!   [`crate::pitch::resample_soxr`] (same PCM_16/float32 decode, same
//!   soxr HQ recipe the rest of the engine uses).
//! - The room's Butterworth damping filter and the air-absorption
//!   low-pass: [`crate::filters::butter_sos`] / [`crate::filters::
//!   sosfilt_f64`] (scipy-parity SOS design/filtering, `f64`-native —
//!   see that function's doc for why a sibling of [`crate::filters::
//!   sosfilt`] rather than the `f32` original).
//! - The room impulse's seeded noise: [`np_rand::Pcg64`] — bit-faithful
//!   to `np.random.default_rng(seed).standard_normal(n)`.
//!
//! # What's deliberately not implemented
//!
//! [`PsychoacousticControls`] only *executes* the all-defaults path the
//! fixture scenes exercise (which reduces to a single peak-safety clamp —
//! see [`apply_controls_or_refuse`]'s doc comment for why that's the
//! whole story at defaults). Any object whose controls carry a single
//! non-default field refuses the render — [`SceneError::UnsupportedControl`],
//! naming the field — rather than silently skipping or guessing at
//! pitch-shift/HPSS/shelf-EQ/AM/LUFS ports that have no fixture coverage
//! and no oracle-diffed golden vectors to verify against. `object_from_
//! analysis`/`make_scene` (scene-*authoring* helpers, not renderers) are
//! not ported: nothing in the render path calls them, and no deliverable
//! asks for scene authoring.
//!
//! Field-level input validation (`SpatialObject`/`SpatialScene`/`Room`'s
//! `__post_init__` guards — non-empty ids, `spread`/`reverb_send` in
//! `[0,1]`, trajectory time-ordering, etc.) is also not re-implemented;
//! this module's job is rendering a well-formed scene, and every fixture
//! scene is well-formed. A malformed scene either fails to parse (serde's
//! own type errors) or produces a numerically-different-but-not-oracle-
//! validated render — documented here rather than silently assumed.
//!
//! `Position`'s partial-object JSON parsing (e.g. `"up": {"y": 2.0}` with
//! `x`/`z` omitted) falls back to `Position`'s own default `(0, 0, -1)`
//! for the missing axes, rather than the *contextual* default the
//! oracle's `_position_from_dict(row, default=...)` would use for that
//! specific field (listener `up`'s contextual default is `(0, 1, 0)`).
//! Every position object in every fixture scene is always fully
//! specified (`x`, `y`, `z` all present), so this never fires there —
//! flagged rather than silently accepted, per this stage's tolerance/
//! scope-honesty rule applied to a structural corner instead of a
//! numeric one.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{filters, pitch};

const BLOCK_SIZE: usize = 256;

// ============================================================== errors

/// Every refusal this module can produce, typed rather than stringly —
/// `code()` gives the machine-readable discriminant the CLI/service
/// error envelopes want (mirroring `mus-cli`'s existing `ErrorBody{code,
/// message}` convention); `Display` gives the human line.
#[derive(Debug)]
pub enum SceneError {
    Io(String),
    Json(String),
    Wav(String),
    PathEscape {
        object_id: String,
        audio: String,
    },
    EmptySignal {
        object_id: String,
    },
    NonFiniteSignal {
        object_id: String,
    },
    DegenerateListener(String),
    UnsupportedControl {
        object_id: String,
        control: &'static str,
        value: String,
    },
    Invalid(String),
}

impl std::fmt::Display for SceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneError::Io(m) => write!(f, "I/O error: {m}"),
            SceneError::Json(m) => write!(f, "scene JSON error: {m}"),
            SceneError::Wav(m) => write!(f, "audio file error: {m}"),
            SceneError::PathEscape { object_id, audio } => write!(
                f,
                "object {object_id:?}: audio path escapes scene root: {audio}"
            ),
            SceneError::EmptySignal { object_id } => {
                write!(f, "object {object_id:?}: signal is empty")
            }
            SceneError::NonFiniteSignal { object_id } => write!(
                f,
                "object {object_id:?}: signal contains NaN or infinite samples"
            ),
            SceneError::DegenerateListener(m) => write!(f, "listener error: {m}"),
            SceneError::UnsupportedControl {
                object_id,
                control,
                value,
            } => write!(
                f,
                "unsupported-control: object {object_id:?} sets {control}={value}, \
                 which PW7 does not implement — typed refusal, not a silent skip"
            ),
            SceneError::Invalid(m) => write!(f, "invalid scene: {m}"),
        }
    }
}

impl std::error::Error for SceneError {}

impl SceneError {
    /// Machine-readable discriminant, for JSON error envelopes.
    pub fn code(&self) -> &'static str {
        match self {
            SceneError::Io(_) => "io-error",
            SceneError::Json(_) => "bad-scene-json",
            SceneError::Wav(_) => "audio-file-error",
            SceneError::PathEscape { .. } => "path-escape",
            SceneError::EmptySignal { .. } => "empty-signal",
            SceneError::NonFiniteSignal { .. } => "non-finite-signal",
            SceneError::DegenerateListener(_) => "degenerate-listener",
            SceneError::UnsupportedControl { .. } => "unsupported-control",
            SceneError::Invalid(_) => "invalid-scene",
        }
    }
}

// ============================================================ geometry

/// `spatial_scene.Position`: `+X` right, `+Y` up, `-Z` front. The default
/// `(0, 0, -1)` — one meter directly ahead — is also the JSON-parse
/// fallback for any *individually missing* `x`/`y`/`z` key, matching the
/// oracle's `_position_from_dict`'s per-key `default.x`/`.y`/`.z`
/// fallback for [`SpatialObject::position`] and trajectory keyframes (see
/// this module's doc comment for where that fallback is *not* contextual
/// enough to match [`Listener`]'s three differently-defaulted fields).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default = "Position::default_z")]
    pub z: f64,
}

impl Default for Position {
    fn default() -> Self {
        Position {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        }
    }
}

impl Position {
    fn default_z() -> f64 {
        -1.0
    }

    /// Listener `position`'s contextual default: the origin.
    pub fn origin() -> Self {
        Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    /// Listener `up`'s contextual default: `+Y`.
    pub fn listener_up() -> Self {
        Position {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        }
    }

    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Position { x, y, z }
    }

    pub fn distance(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Front (`-Z`) is zero; positive azimuth points right.
    pub fn azimuth_radians(&self) -> f64 {
        self.x.atan2(-self.z)
    }

    pub fn elevation_radians(&self) -> f64 {
        let horizontal = (self.x * self.x + self.z * self.z).sqrt();
        self.y.atan2(horizontal)
    }

    /// `Position.spherical(azimuth_degrees, elevation_degrees, distance)`.
    pub fn spherical(
        azimuth_degrees: f64,
        elevation_degrees: f64,
        distance: f64,
    ) -> Result<Self, SceneError> {
        if distance < 0.0 {
            return Err(SceneError::Invalid("distance cannot be negative".into()));
        }
        let azimuth = azimuth_degrees.to_radians();
        let elevation = elevation_degrees.to_radians();
        let horizontal = distance * elevation.cos();
        Ok(Position {
            x: horizontal * azimuth.sin(),
            y: distance * elevation.sin(),
            z: -horizontal * azimuth.cos(),
        })
    }
}

/// `spatial_scene.PositionKeyframe`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionKeyframe {
    pub time_seconds: f64,
    #[serde(default)]
    pub position: Position,
}

/// `spatial_scene.Listener`. Each field's default is *its own* — not
/// [`Position`]'s — see that type's doc comment.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Listener {
    #[serde(default = "Position::origin")]
    pub position: Position,
    #[serde(default)]
    pub forward: Position,
    #[serde(default = "Position::listener_up")]
    pub up: Position,
}

impl Default for Listener {
    fn default() -> Self {
        Listener {
            position: Position::origin(),
            forward: Position::default(),
            up: Position::listener_up(),
        }
    }
}

/// `spatial_scene.Room`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    #[serde(default = "Room::default_enabled")]
    pub enabled: bool,
    #[serde(default = "Room::default_decay_seconds")]
    pub decay_seconds: f64,
    #[serde(default = "Room::default_wet")]
    pub wet: f64,
    #[serde(default = "Room::default_damping")]
    pub damping: f64,
    #[serde(default = "Room::default_seed")]
    pub seed: i64,
}

impl Room {
    fn default_enabled() -> bool {
        true
    }
    fn default_decay_seconds() -> f64 {
        1.6
    }
    fn default_wet() -> f64 {
        0.16
    }
    fn default_damping() -> f64 {
        0.45
    }
    fn default_seed() -> i64 {
        17
    }
}

impl Default for Room {
    fn default() -> Self {
        Room {
            enabled: Room::default_enabled(),
            decay_seconds: Room::default_decay_seconds(),
            wet: Room::default_wet(),
            damping: Room::default_damping(),
            seed: Room::default_seed(),
        }
    }
}

// ================================================ psychoacoustic controls

/// `psychoacoustic_controls.PsychoacousticControls`. `deny_unknown_fields`
/// mirrors `PsychoacousticControls.from_mapping`'s explicit `raise
/// ValueError(f"unknown psychoacoustic controls: ...")` on any
/// unrecognized key — the one place in this module's scene-JSON parsing
/// that's strict rather than permissive (see the module doc's note on
/// [`SpatialScene`]/[`SpatialObject`] tolerating-and-ignoring unknown
/// top-level keys instead, matching `from_dict`'s plain `.get(...)`
/// reads).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PsychoacousticControls {
    #[serde(default)]
    pub gain_db: f64,
    #[serde(default)]
    pub target_lufs: Option<f64>,
    #[serde(default)]
    pub brightness_db: f64,
    #[serde(default = "PsychoacousticControls::default_brightness_hz")]
    pub brightness_hz: f64,
    #[serde(default)]
    pub roughness_depth: f64,
    #[serde(default = "PsychoacousticControls::default_roughness_rate_hz")]
    pub roughness_rate_hz: f64,
    #[serde(default)]
    pub fluctuation_depth: f64,
    #[serde(default = "PsychoacousticControls::default_fluctuation_rate_hz")]
    pub fluctuation_rate_hz: f64,
    #[serde(default)]
    pub attack_seconds: Option<f64>,
    #[serde(default)]
    pub pitch_semitones: f64,
    #[serde(default)]
    pub tonal_focus: f64,
    #[serde(default = "PsychoacousticControls::default_safety_peak")]
    pub safety_peak: f64,
}

impl PsychoacousticControls {
    fn default_brightness_hz() -> f64 {
        2500.0
    }
    fn default_roughness_rate_hz() -> f64 {
        70.0
    }
    fn default_fluctuation_rate_hz() -> f64 {
        4.0
    }
    fn default_safety_peak() -> f64 {
        0.98
    }

    /// The first field that differs from its default, as `(camelCase
    /// name, formatted value)` — enough to build [`SceneError::
    /// UnsupportedControl`]'s message. `None` means every field is at its
    /// oracle default, the only case [`apply_controls_or_refuse`] executes.
    fn first_non_default(&self) -> Option<(&'static str, String)> {
        let d = PsychoacousticControls::default();
        if self.gain_db != d.gain_db {
            return Some(("gainDb", self.gain_db.to_string()));
        }
        if self.target_lufs != d.target_lufs {
            return Some(("targetLufs", format!("{:?}", self.target_lufs)));
        }
        if self.brightness_db != d.brightness_db {
            return Some(("brightnessDb", self.brightness_db.to_string()));
        }
        if self.brightness_hz != d.brightness_hz {
            return Some(("brightnessHz", self.brightness_hz.to_string()));
        }
        if self.roughness_depth != d.roughness_depth {
            return Some(("roughnessDepth", self.roughness_depth.to_string()));
        }
        if self.roughness_rate_hz != d.roughness_rate_hz {
            return Some(("roughnessRateHz", self.roughness_rate_hz.to_string()));
        }
        if self.fluctuation_depth != d.fluctuation_depth {
            return Some(("fluctuationDepth", self.fluctuation_depth.to_string()));
        }
        if self.fluctuation_rate_hz != d.fluctuation_rate_hz {
            return Some(("fluctuationRateHz", self.fluctuation_rate_hz.to_string()));
        }
        if self.attack_seconds != d.attack_seconds {
            return Some(("attackSeconds", format!("{:?}", self.attack_seconds)));
        }
        if self.pitch_semitones != d.pitch_semitones {
            return Some(("pitchSemitones", self.pitch_semitones.to_string()));
        }
        if self.tonal_focus != d.tonal_focus {
            return Some(("tonalFocus", self.tonal_focus.to_string()));
        }
        if self.safety_peak != d.safety_peak {
            return Some(("safetyPeak", self.safety_peak.to_string()));
        }
        None
    }
}

impl Default for PsychoacousticControls {
    fn default() -> Self {
        PsychoacousticControls {
            gain_db: 0.0,
            target_lufs: None,
            brightness_db: 0.0,
            brightness_hz: PsychoacousticControls::default_brightness_hz(),
            roughness_depth: 0.0,
            roughness_rate_hz: PsychoacousticControls::default_roughness_rate_hz(),
            fluctuation_depth: 0.0,
            fluctuation_rate_hz: PsychoacousticControls::default_fluctuation_rate_hz(),
            attack_seconds: None,
            pitch_semitones: 0.0,
            tonal_focus: 0.0,
            safety_peak: PsychoacousticControls::default_safety_peak(),
        }
    }
}

/// `psychoacoustic_controls.PsychoacousticControls.to_dict()`'s exact key
/// casing when embedded in a [`ManipulationReceipt`]-shaped JSON blob:
/// **snake_case**, not the camelCase [`PsychoacousticControls`] itself
/// parses/emits as scene-JSON. This is a genuine oracle inconsistency
/// (`psychoacoustic_controls.py`'s own `_jsonable` does not camelize,
/// unlike `spatial_scene.py`'s `_camelize`, which is why `SpatialObject.
/// controls` in a *scene* document is camelCase while the *same*
/// dataclass under a render receipt's `manipulation.controls` is not) —
/// verified directly against the oracle (`PsychoacousticControls().
/// to_dict()`), not assumed. Mirrored exactly rather than "fixed", since
/// deliverable 1 asks for "a receipt mirroring `RenderReceipt`".
fn controls_receipt_json(c: &PsychoacousticControls) -> Value {
    json!({
        "gain_db": c.gain_db,
        "target_lufs": c.target_lufs,
        "brightness_db": c.brightness_db,
        "brightness_hz": c.brightness_hz,
        "roughness_depth": c.roughness_depth,
        "roughness_rate_hz": c.roughness_rate_hz,
        "fluctuation_depth": c.fluctuation_depth,
        "fluctuation_rate_hz": c.fluctuation_rate_hz,
        "attack_seconds": c.attack_seconds,
        "pitch_semitones": c.pitch_semitones,
        "tonal_focus": c.tonal_focus,
        "safety_peak": c.safety_peak,
    })
}

/// `psychoacoustic_controls.apply_controls`, restricted to the subset the
/// fixture scenes exercise: every field at its oracle default.
///
/// Tracing `apply_controls` at all-default `controls` (`gain_db=0`,
/// `target_lufs=None`, `brightness_db=0`, `roughness_depth=0`,
/// `fluctuation_depth=0`, `attack_seconds=None`, `pitch_semitones=0`,
/// `tonal_focus=0`) shows every one of its seven conditional operator
/// stages (pitch-shift, HPSS mix, high-shelf, attack envelope, roughness
/// AM, fluctuation AM, gain) takes its `if`'s *false* branch and is
/// skipped — that's not this port cutting corners, it's what the oracle
/// itself does at these values. What's left, and *not* conditioned on
/// any control value, is the trailing peak-safety clamp: `if peak >
/// controls.safety_peak: scale down to safety_peak`. That clamp still
/// runs — and can still fire, since it depends on the *signal's* peak,
/// not on any control setting — so it's what this function actually
/// computes.
///
/// Any field that isn't at its default refuses instead
/// ([`SceneError::UnsupportedControl`]) — seven unverified DSP ports is
/// not a trade this stage makes silently.
///
/// Returns `(signal, input_peak, output_peak, safety_gain_db,
/// operations)` — the pieces [`ManipulationReceipt`]-shaped JSON needs.
fn apply_controls_or_refuse(
    y: &[f64],
    controls: &PsychoacousticControls,
    object_id: &str,
) -> Result<(Vec<f64>, f64, f64, f64, Vec<Value>), SceneError> {
    if let Some((field, value)) = controls.first_non_default() {
        return Err(SceneError::UnsupportedControl {
            object_id: object_id.to_string(),
            control: field,
            value,
        });
    }
    if y.is_empty() {
        return Err(SceneError::EmptySignal {
            object_id: object_id.to_string(),
        });
    }
    if !y.iter().all(|v| v.is_finite()) {
        return Err(SceneError::NonFiniteSignal {
            object_id: object_id.to_string(),
        });
    }

    let input_peak = y.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let mut out = y.to_vec();
    let mut safety_gain_db = 0.0f64;
    let mut operations = Vec::new();
    if input_peak > controls.safety_peak {
        let scale = controls.safety_peak / input_peak;
        for v in out.iter_mut() {
            *v *= scale;
        }
        safety_gain_db = 20.0 * scale.log10();
        operations.push(json!({
            "operator": "mus.safety-peak-scale/1",
            "targetCue": "digital-integrity",
            "parameters": {"maximumPeak": controls.safety_peak, "gainDb": safety_gain_db},
            "claim": "Linear safety attenuation; no hidden nonlinear limiter.",
        }));
    }
    let output_peak = out.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    Ok((out, input_peak, output_peak, safety_gain_db, operations))
}

// =============================================================== scene

/// `spatial_scene.SpatialObject`. Unknown top-level JSON keys are
/// tolerated and ignored (matching `_object_from_dict`'s plain
/// `row.get(...)` reads — never an error), the opposite policy from
/// [`PsychoacousticControls`]'s `deny_unknown_fields`; see this module's
/// doc comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialObject {
    pub object_id: String,
    pub label: String,
    pub audio: String,
    #[serde(default)]
    pub start_seconds: f64,
    #[serde(default)]
    pub duration_seconds: Option<f64>,
    #[serde(default = "SpatialObject::default_kind")]
    pub kind: String,
    #[serde(default)]
    pub position: Position,
    #[serde(default)]
    pub trajectory: Vec<PositionKeyframe>,
    #[serde(default)]
    pub gain_db: f64,
    #[serde(default)]
    pub spread: f64,
    #[serde(default)]
    pub reverb_send: f64,
    #[serde(default, rename = "loop")]
    pub r#loop: bool,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub controls: PsychoacousticControls,
    #[serde(default)]
    pub analysis: Value,
    #[serde(default)]
    pub source_region: Option<Value>,
    #[serde(default = "SpatialObject::default_provenance")]
    pub placement_provenance: String,
    #[serde(default)]
    pub metadata: Value,
}

impl SpatialObject {
    fn default_kind() -> String {
        "point".into()
    }
    fn default_provenance() -> String {
        "composition-authored".into()
    }

    fn is_diffuse_like(&self) -> bool {
        matches!(self.kind.as_str(), "diffuse" | "ambient")
    }
}

/// `spatial_scene.position_at`: linear interpolation of an object's
/// trajectory in scene time. No trajectory → the object's static
/// position; before the first / after the last keyframe → clamp to that
/// keyframe.
pub fn position_at(obj: &SpatialObject, scene_time_seconds: f64) -> Position {
    if obj.trajectory.is_empty() {
        return obj.position;
    }
    let frames = &obj.trajectory;
    if scene_time_seconds <= frames[0].time_seconds {
        return frames[0].position;
    }
    let last = frames.len() - 1;
    if scene_time_seconds >= frames[last].time_seconds {
        return frames[last].position;
    }
    for pair in frames.windows(2) {
        let (left, right) = (&pair[0], &pair[1]);
        if left.time_seconds <= scene_time_seconds && scene_time_seconds <= right.time_seconds {
            let span = right.time_seconds - left.time_seconds;
            let alpha = if span <= 0.0 {
                0.0
            } else {
                (scene_time_seconds - left.time_seconds) / span
            };
            return Position::new(
                left.position.x + alpha * (right.position.x - left.position.x),
                left.position.y + alpha * (right.position.y - left.position.y),
                left.position.z + alpha * (right.position.z - left.position.z),
            );
        }
    }
    obj.position
}

/// `spatial_scene.SpatialScene`. Unknown top-level keys are tolerated and
/// ignored — see [`SpatialObject`]'s doc comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialScene {
    pub scene_id: String,
    pub title: String,
    pub sample_rate: u32,
    pub duration_seconds: f64,
    pub objects: Vec<SpatialObject>,
    #[serde(default)]
    pub listener: Listener,
    #[serde(default)]
    pub room: Room,
    #[serde(default = "SpatialScene::default_coordinate_system")]
    pub coordinate_system: String,
    #[serde(default)]
    pub metadata: Value,
    #[serde(default = "SpatialScene::default_schema")]
    pub schema: String,
}

impl SpatialScene {
    fn default_coordinate_system() -> String {
        "+X right, +Y up, -Z front".into()
    }
    fn default_schema() -> String {
        "https://sophia-labs.ai/schemas/mus-spatial-scene/1".into()
    }

    /// `SpatialScene.read(path)`.
    pub fn read(path: &Path) -> Result<Self, SceneError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| SceneError::Io(format!("{}: {e}", path.display())))?;
        serde_json::from_str(&text)
            .map_err(|e| SceneError::Json(format!("{}: {e}", path.display())))
    }
}

// ============================================================= receipts

/// `spatial_render.RenderReceipt`. Field names are deliberately
/// **snake_case** at this top level, verbatim — not a `rename_all`
/// oversight. `RenderReceipt.to_dict()` is `_jsonable(asdict(self))`
/// (`spatial_render.py`'s own local, non-camelizing `_jsonable`), so the
/// oracle's own render-receipt JSON has snake_case top-level keys; only
/// `object_receipts`' entries (hand-typed dict literals in the oracle,
/// reproduced here via [`serde_json::json!`]) are camelCase. Verified
/// directly against the oracle, not assumed — see [`controls_receipt_json`]'s
/// doc comment for the same finding on the nested manipulation receipt.
#[derive(Debug, Clone, Serialize)]
pub struct RenderReceipt {
    pub scene_id: String,
    pub target: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub duration_seconds: f64,
    pub peak_before_safety: f64,
    pub safety_gain_db: f64,
    pub output_peak: f64,
    pub channel_order: Vec<&'static str>,
    pub renderer: String,
    pub object_receipts: Vec<Value>,
    pub metadata: Value,
    pub schema: String,
}

// =========================================================== dsp: geometry

/// `spatial_render._listener_relative`: world-space position transformed
/// into Web-Audio-style listener space (right/up/-forward basis).
fn listener_relative(position: Position, listener: &Listener) -> Result<Position, SceneError> {
    let origin = [
        listener.position.x,
        listener.position.y,
        listener.position.z,
    ];
    let forward_raw = [listener.forward.x, listener.forward.y, listener.forward.z];
    let up_raw = [listener.up.x, listener.up.y, listener.up.z];

    let norm = |v: [f64; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let scale = |v: [f64; 3], s: f64| [v[0] * s, v[1] * s, v[2] * s];
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];

    let forward_norm = norm(forward_raw);
    let up_norm = norm(up_raw);
    if forward_norm < 1e-12 || up_norm < 1e-12 {
        return Err(SceneError::DegenerateListener(
            "listener forward and up vectors must be non-zero".into(),
        ));
    }
    let forward = scale(forward_raw, 1.0 / forward_norm);
    let up0 = scale(up_raw, 1.0 / up_norm);
    let right_raw = cross(forward, up0);
    let right_norm = norm(right_raw);
    if right_norm < 1e-12 {
        return Err(SceneError::DegenerateListener(
            "listener forward and up vectors must not be collinear".into(),
        ));
    }
    let right = scale(right_raw, 1.0 / right_norm);
    let up = cross(right, forward);

    let delta = [
        position.x - origin[0],
        position.y - origin[1],
        position.z - origin[2],
    ];
    Ok(Position::new(
        dot(delta, right),
        dot(delta, up),
        -dot(delta, forward),
    ))
}

/// `spatial_render._inverse_distance_gain` (`ref_distance=1.0,
/// rolloff=0.72`, the only values `mus_analysis` ever calls it with).
fn inverse_distance_gain(distance: &[f64]) -> Vec<f64> {
    const REF: f64 = 1.0;
    const ROLLOFF: f64 = 0.72;
    distance
        .iter()
        .map(|&d| {
            let d = d.max(REF);
            REF / (REF + ROLLOFF * (d - REF))
        })
        .collect()
}

/// `spatial_render._distance_air_cutoff`.
fn distance_air_cutoff(distance: f64) -> f64 {
    let value = 20000.0 * (-0.035 * (distance - 1.0).max(0.0)).exp();
    value.min(20000.0).max(2200.0)
}

/// `spatial_render._air_absorption`: a 2nd-order Butterworth low-pass at
/// `cutoff_hz`, or a no-op above `0.47 * sample_rate`.
fn air_absorption(y: &[f64], sample_rate: u32, cutoff_hz: f64) -> Vec<f64> {
    if cutoff_hz >= sample_rate as f64 * 0.47 {
        return y.to_vec();
    }
    let sos = filters::butter_sos(2, cutoff_hz / (sample_rate as f64 / 2.0), "low");
    filters::sosfilt_f64(&sos, y, None).0
}

/// `np.interp(np.arange(n), xp, fp)` specialized to this module's actual
/// shape: `xp` a strictly-increasing block-spaced index list ending
/// exactly at `n - 1` (both `_spatialize_stereo_object` and `_foa_
/// coefficients` build `xp` that way — `np.append(control_times, len(y)
/// - 1)` — so the query grid `0..n` never actually extrapolates past
/// `xp`'s span). One forward sweep since both the query grid and `xp`
/// are sorted; the boundary clamps below still match numpy's `interp`
/// exactly regardless.
fn interp_from_controls(xp: &[usize], fp: &[f64], n: usize) -> Vec<f64> {
    let m = xp.len();
    let mut out = Vec::with_capacity(n);
    let mut seg = 0usize;
    for i in 0..n {
        if i <= xp[0] {
            out.push(fp[0]);
            continue;
        }
        if i >= xp[m - 1] {
            out.push(fp[m - 1]);
            continue;
        }
        while seg + 1 < m && xp[seg + 1] < i {
            seg += 1;
        }
        let (x0, x1) = (xp[seg] as f64, xp[seg + 1] as f64);
        let t = (i as f64 - x0) / (x1 - x0);
        out.push(fp[seg] + t * (fp[seg + 1] - fp[seg]));
    }
    out
}

/// `arange(0, n, max(1, block_size))`, appending `n - 1` if it isn't
/// already the last control point — the shared control-time construction
/// `_spatialize_stereo_object` and `_foa_coefficients` both open with.
fn block_control_indices(n: usize, block_size: usize) -> Vec<usize> {
    let step = block_size.max(1);
    let mut control: Vec<usize> = (0..n).step_by(step).collect();
    if control.is_empty() || *control.last().unwrap() != n - 1 {
        control.push(n - 1);
    }
    control
}

fn median(values: &[f64]) -> f64 {
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).expect("median: NaN distance"));
    let n = v.len();
    if n == 0 {
        return f64::NAN;
    }
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// `spatial_render._seed_from_id`: first 4 big-endian bytes of the
/// object id's SHA-256, as an unsigned 32-bit int (`int.from_bytes(...,
/// "big")` is unsigned by default) — always non-negative, so the
/// `(seed + channel + index) % 5` arithmetic in [`decorrelate_stereo`]
/// never needs a euclidean-remainder guard.
fn seed_from_id(value: &str) -> u32 {
    let digest = Sha256::digest(value.as_bytes());
    u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]])
}

/// A single Schroeder all-pass delay stage: `scipy.signal.lfilter(b, a,
/// x)` specialized to the sparse `b = [-gain, 0, .., 0, 1]`, `a = [1, 0,
/// .., 0, -gain]` (length `delay + 1`) that [`decorrelate_stereo`] builds
/// — algebraically `y[n] = -gain*x[n] + x[n-delay] + gain*y[n-delay]`
/// (zero for negative indices, matching scipy's zero-initial-condition
/// default). Derived from, and equivalent to, scipy's direct-form-II-
/// transposed state recursion for this exact sparse `(b, a)` shape: every
/// interior state slot has zero contribution each step (both `b[i]` and
/// `a[i]` are zero there) and so just shifts down the line — i.e. *is* a
/// `delay`-sample delay buffer — which this computes directly in O(1)
/// per sample rather than walking a `delay`-long state array on every
/// sample the way a generic `lfilter` would.
fn allpass_delay(x: &[f64], delay: usize, gain: f64) -> Vec<f64> {
    let n = x.len();
    let mut y = vec![0.0f64; n];
    for i in 0..n {
        let xd = if i >= delay { x[i - delay] } else { 0.0 };
        let yd = if i >= delay { y[i - delay] } else { 0.0 };
        y[i] = -gain * x[i] + xd + gain * yd;
    }
    y
}

/// `spatial_render._decorrelate_stereo`: two channels of cascaded
/// Schroeder all-pass filters (three delay taps each, different delay
/// sets per channel), RMS-matched back to the input's level. Returns
/// `(left, right)`.
fn decorrelate_stereo(y: &[f64], sample_rate: u32, seed: u32) -> (Vec<f64>, Vec<f64>) {
    const DELAY_SETS: [[f64; 3]; 2] = [[0.0037, 0.0113, 0.0179], [0.0053, 0.0137, 0.0197]];
    let mut channels: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
    for (channel, delays) in DELAY_SETS.iter().enumerate() {
        let mut out = y.to_vec();
        for (index, &delay_seconds) in delays.iter().enumerate() {
            let delay = ((delay_seconds * sample_rate as f64).round() as i64).max(1) as usize;
            let gain = 0.55 + 0.03 * ((seed as u64 + channel as u64 + index as u64) % 5) as f64;
            out = allpass_delay(&out, delay, gain);
        }
        channels[channel] = out;
    }

    let input_rms = (y.iter().map(|v| v * v).sum::<f64>() / y.len().max(1) as f64).sqrt();
    let total: f64 = channels[0].iter().map(|v| v * v).sum::<f64>()
        + channels[1].iter().map(|v| v * v).sum::<f64>();
    let output_rms = (total / (2 * y.len().max(1)) as f64).sqrt();
    if output_rms > 1e-15 {
        let scale = input_rms / output_rms;
        for ch in channels.iter_mut() {
            for v in ch.iter_mut() {
                *v *= scale;
            }
        }
    }
    let [left, right] = channels;
    (left, right)
}

// ============================================================ FFT convolve

/// `scipy.signal.fftconvolve(x, h, mode="full")`: zero-padded circular
/// convolution at a transform size `>= len(x) + len(h) - 1` is
/// mathematically identical to linear "full" convolution (no aliasing),
/// whatever that size's factorization — so this doesn't need to match
/// scipy's specific 5-smooth `next_fast_len` choice, only to be `>=` the
/// full length. `next_power_of_two` is always even (`>= 2`), which is
/// [`realfft`]'s only length constraint, and gives a fast transform
/// besides. `f64` throughout: this module's [`synthetic_room`] is the
/// only caller, and its inputs (the seeded impulse, the room bus) are
/// already `f64`.
fn fft_convolve_full(x: &[f64], h: &[f64]) -> Vec<f64> {
    let full_len = x.len() + h.len() - 1;
    let n = full_len.max(2).next_power_of_two();

    let mut planner = realfft::RealFftPlanner::<f64>::new();
    let r2c = planner.plan_fft_forward(n);
    let c2r = planner.plan_fft_inverse(n);

    let mut xa = r2c.make_input_vec();
    xa[..x.len()].copy_from_slice(x);
    let mut ha = r2c.make_input_vec();
    ha[..h.len()].copy_from_slice(h);

    let mut xs = r2c.make_output_vec();
    let mut hs = r2c.make_output_vec();
    r2c.process(&mut xa, &mut xs)
        .expect("fft_convolve_full: forward transform of x");
    r2c.process(&mut ha, &mut hs)
        .expect("fft_convolve_full: forward transform of h");

    let mut product: Vec<realfft::num_complex::Complex<f64>> =
        xs.iter().zip(hs.iter()).map(|(&a, &b)| a * b).collect();
    let mut y = c2r.make_output_vec();
    c2r.process(&mut product, &mut y)
        .expect("fft_convolve_full: inverse transform");

    // realfft's inverse is unnormalized (see `pitch::istft`'s doc comment
    // for the same convention) — scipy's own ifft folds the 1/n scaling
    // in, so it's applied here explicitly.
    let scale = 1.0 / n as f64;
    y.truncate(full_len);
    for v in y.iter_mut() {
        *v *= scale;
    }
    y
}

/// `spatial_render._synthetic_room`: a seeded, damped noise-burst "room"
/// impulse per channel, convolved against the room bus. Returns
/// `(left, right)`, each `len(y) + impulse_len - 1` long ("full" mode).
fn synthetic_room(
    y: &[f64],
    sample_rate: u32,
    decay_seconds: f64,
    damping: f64,
    seed: i64,
) -> (Vec<f64>, Vec<f64>) {
    let length = ((decay_seconds * sample_rate as f64).round() as i64).max(64) as usize;
    let envelope: Vec<f64> = (0..length)
        .map(|i| {
            let t = i as f64 / sample_rate as f64;
            (-6.91 * t / decay_seconds.max(1e-3)).exp()
        })
        .collect();

    let cutoff = 18000.0 - 13000.0 * damping;
    let sos = if cutoff < sample_rate as f64 * 0.47 {
        Some(filters::butter_sos(
            2,
            cutoff / (sample_rate as f64 / 2.0),
            "low",
        ))
    } else {
        None
    };

    let mut channels: [Vec<f64>; 2] = [Vec::new(), Vec::new()];
    for channel in 0..2usize {
        let channel_seed = seed.wrapping_add(channel as i64 * 7919);
        // `np.random.default_rng` seeds are non-negative; every `Room.seed`
        // this module ever sees comes from JSON (never negative in
        // practice) or the default `17`, so the `as u64` cast below is
        // exact for every real input.
        let mut rng = np_rand::Pcg64::seeded(channel_seed as u64);
        let mut impulse = rng.standard_normal_vec(length);
        for i in 0..length {
            impulse[i] *= envelope[i];
        }
        impulse[0] += 1.0;
        if let Some(sos) = &sos {
            impulse = filters::sosfilt_f64(sos, &impulse, None).0;
        }
        let energy = impulse.iter().map(|v| v * v).sum::<f64>().sqrt().max(1e-15);
        for v in impulse.iter_mut() {
            *v /= energy;
        }
        channels[channel] = fft_convolve_full(y, &impulse);
    }
    let [left, right] = channels;
    (left, right)
}

// ============================================================ object load

/// `spatial_render._load_object_audio` + the caller's resample branch:
/// decode `root/relative` to mono at its native rate (via [`pitch::
/// load_decode_native`], the same PCM_16/float32 path the rest of the
/// engine uses), resampling to `target_sample_rate` with soxr HQ
/// ([`pitch::resample_soxr`]) only when the file's own rate differs.
///
/// **The one precision approximation in this module**: the shared soxr
/// wrapper is `f32`-native (the rest of `mus-dsp` is built around it), so
/// a resample here round-trips `f64 -> f32 -> soxr -> f64` rather than
/// staying `f64` throughout the way `librosa.resample` does on the
/// oracle's `float64` array. Every fixture stem is already the scene's
/// native sample rate, so this branch has **no golden-fixture coverage**
/// — flagged per this stage's tolerance-honesty rule rather than left
/// implicit. The round-trip's own error is bounded by `f32` epsilon
/// (~6e-8 relative) against a signal already well under the fixtures'
/// `2e-5` absolute tolerance's headroom, so it is expected to be
/// immaterial if this branch is ever exercised for real, but "expected"
/// is not "measured".
///
/// Root-escape guard: `root` is resolved once; the joined path must
/// canonicalize to somewhere under it, mirroring `_load_object_audio`'s
/// `path.relative_to(root)` check (`ValueError` there,
/// [`SceneError::PathEscape`] here).
fn load_object_audio(
    root: &Path,
    relative: &str,
    target_sample_rate: u32,
) -> Result<Vec<f64>, SceneError> {
    let root_resolved = root
        .canonicalize()
        .map_err(|e| SceneError::Io(format!("{}: {e}", root.display())))?;
    let candidate = root_resolved.join(relative);
    let resolved = candidate
        .canonicalize()
        .map_err(|e| SceneError::Io(format!("{}: {e}", candidate.display())))?;
    if !resolved.starts_with(&root_resolved) {
        return Err(SceneError::PathEscape {
            object_id: String::new(),
            audio: relative.to_string(),
        });
    }

    let (mono_f32, source_sr) = pitch::load_decode_native(&resolved)
        .map_err(|e| SceneError::Wav(format!("{}: {e}", resolved.display())))?;

    if source_sr == target_sample_rate {
        Ok(mono_f32.iter().map(|&v| v as f64).collect())
    } else {
        let resampled =
            pitch::resample_soxr(&mono_f32, source_sr as f64, target_sample_rate as f64);
        Ok(resampled.iter().map(|&v| v as f64).collect())
    }
}

// =============================================================== stereo

/// `spatial_render._spatialize_stereo_object`. Returns `(spatial, dry,
/// meta)` — `spatial` is the per-sample stereo pair (point-panned mixed
/// with decorrelated diffuse per `effective_spread`), `dry` is the mono
/// air-absorbed/distance-attenuated signal the room send is built from,
/// `meta` is the `object_receipts[i].spatial` JSON blob.
fn spatialize_stereo_object(
    y: &[f64],
    sample_rate: u32,
    obj: &SpatialObject,
    listener: &Listener,
    block_size: usize,
) -> Result<(Vec<[f64; 2]>, Vec<f64>, Value), SceneError> {
    let n = y.len();
    let control_times = block_control_indices(n, block_size);

    let mut pans = Vec::with_capacity(control_times.len());
    let mut distances = Vec::with_capacity(control_times.len());
    for &idx in &control_times {
        let t = obj.start_seconds + idx as f64 / sample_rate as f64;
        let world = position_at(obj, t);
        let rel = listener_relative(world, listener)?;
        pans.push(rel.azimuth_radians().sin());
        distances.push(rel.distance().max(0.001));
    }
    let pan = interp_from_controls(&control_times, &pans, n);
    let distance = interp_from_controls(&control_times, &distances, n);

    let mut gain_left = vec![0.0f64; n];
    let mut gain_right = vec![0.0f64; n];
    for i in 0..n {
        let x = (pan[i].clamp(-1.0, 1.0) + 1.0) / 2.0;
        gain_left[i] = (x * std::f64::consts::FRAC_PI_2).cos();
        gain_right[i] = (x * std::f64::consts::FRAC_PI_2).sin();
    }

    let distance_gain = inverse_distance_gain(&distance);
    let median_distance = median(&distance);
    let min_distance = distance.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_distance = distance.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let air_cutoff = distance_air_cutoff(median_distance);

    let absorbed = air_absorption(y, sample_rate, air_cutoff);
    let dry: Vec<f64> = absorbed
        .iter()
        .zip(&distance_gain)
        .map(|(&d, &g)| d * g)
        .collect();

    let mut point = vec![[0.0f64; 2]; n];
    for i in 0..n {
        point[i] = [dry[i] * gain_left[i], dry[i] * gain_right[i]];
    }

    let effective_spread = obj
        .spread
        .max(if obj.is_diffuse_like() { 0.78 } else { 0.0 });
    let spatial = if effective_spread > 0.0 {
        let seed = seed_from_id(&obj.object_id);
        let (dl, dr) = decorrelate_stereo(&dry, sample_rate, seed);
        (0..n)
            .map(|i| {
                [
                    (1.0 - effective_spread) * point[i][0] + effective_spread * dl[i],
                    (1.0 - effective_spread) * point[i][1] + effective_spread * dr[i],
                ]
            })
            .collect()
    } else {
        point
    };

    let meta = json!({
        "panning": "equal-power",
        "distanceModel": "inverse",
        "medianDistance": median_distance,
        "minimumDistance": min_distance,
        "maximumDistance": max_distance,
        "airAbsorptionCutoffHz": air_cutoff,
        "effectiveSpread": effective_spread,
    });
    Ok((spatial, dry, meta))
}

/// `obj.duration_seconds` trim + `obj.start_seconds` sample offset,
/// shared by [`render_stereo`] and [`render_foa`]. `round()` is Python's
/// round-half-to-even, matching [`pitch::py_round`] (reused rather than
/// Rust's round-half-away-from-zero `f64::round`, which can disagree
/// exactly at a `.5` boundary — see that function's doc comment).
fn trim_to_duration(y: Vec<f64>, duration_seconds: Option<f64>, sample_rate: u32) -> Vec<f64> {
    match duration_seconds {
        None => y,
        Some(dur) => {
            let n = (pitch::py_round(dur * sample_rate as f64) as i64).max(1) as usize;
            let take = n.min(y.len());
            y[..take].to_vec()
        }
    }
}

fn start_sample(start_seconds: f64, sample_rate: u32) -> usize {
    (pitch::py_round(start_seconds * sample_rate as f64).max(0.0)) as usize
}

/// `spatial_render.render_stereo`. `block_size` is always [`BLOCK_SIZE`]
/// (256, the oracle's default and the only value any real caller in this
/// port ever needs — see the module doc). Returns interleaved `f32`
/// stereo PCM (`[l0, r0, l1, r1, ...]`, matching this crate's usual audio
/// convention) plus the render receipt.
pub fn render_stereo(
    scene: &SpatialScene,
    root: &Path,
) -> Result<(Vec<f32>, RenderReceipt), SceneError> {
    let sample_rate = scene.sample_rate;
    let length = ((scene.duration_seconds * sample_rate as f64).ceil() as i64).max(1) as usize;
    let mut mix = vec![[0.0f64; 2]; length];
    let mut room_bus = vec![0.0f64; length];
    let mut object_receipts = Vec::new();

    for obj in &scene.objects {
        if obj.muted {
            continue;
        }
        let raw = load_object_audio(root, &obj.audio, sample_rate).map_err(|e| {
            if let SceneError::PathEscape { audio, .. } = e {
                SceneError::PathEscape {
                    object_id: obj.object_id.clone(),
                    audio,
                }
            } else {
                e
            }
        })?;
        let y = trim_to_duration(raw, obj.duration_seconds, sample_rate);

        let (controlled, input_peak, output_peak, safety_gain_db, operations) =
            apply_controls_or_refuse(&y, &obj.controls, &obj.object_id)?;
        let object_gain = 10f64.powf(obj.gain_db / 20.0);
        let manipulated: Vec<f64> = controlled.iter().map(|&v| v * object_gain).collect();

        let (spatial, send, spatial_meta) =
            spatialize_stereo_object(&manipulated, sample_rate, obj, &scene.listener, BLOCK_SIZE)?;

        let start = start_sample(obj.start_seconds, sample_rate);
        if start >= length {
            continue;
        }
        let available = spatial.len().min(length - start);
        for i in 0..available {
            mix[start + i][0] += spatial[i][0];
            mix[start + i][1] += spatial[i][1];
            room_bus[start + i] += send[i] * obj.reverb_send;
        }

        let manipulation = json!({
            "controls": controls_receipt_json(&obj.controls),
            "operations": operations,
            "input_peak": input_peak,
            "output_peak": output_peak,
            "safety_gain_db": safety_gain_db,
            "schema": "https://sophia-labs.ai/schemas/mus-psychoacoustic-manipulation/1",
        });
        object_receipts.push(json!({
            "objectId": obj.object_id,
            "audio": obj.audio,
            "startSeconds": obj.start_seconds,
            "samplesRendered": available,
            "gainDb": obj.gain_db,
            "spread": obj.spread,
            "reverbSend": obj.reverb_send,
            "manipulation": manipulation,
            "spatial": spatial_meta,
        }));
    }

    if scene.room.enabled && room_bus.iter().any(|&v| v != 0.0) {
        let (left, right) = synthetic_room(
            &room_bus,
            sample_rate,
            scene.room.decay_seconds,
            scene.room.damping,
            scene.room.seed,
        );
        let wet = scene.room.wet;
        let take = left.len().min(length);
        for i in 0..take {
            mix[i][0] += wet * left[i];
            mix[i][1] += wet * right[i];
        }
    }

    let peak_before = mix
        .iter()
        .flat_map(|f| f.iter())
        .fold(0.0f64, |m, v| m.max(v.abs()));
    let mut safety_gain_db = 0.0f64;
    if peak_before > 0.98 {
        let scale = 0.98 / peak_before;
        for frame in mix.iter_mut() {
            frame[0] *= scale;
            frame[1] *= scale;
        }
        safety_gain_db = 20.0 * scale.log10();
    }
    let output_peak = mix
        .iter()
        .flat_map(|f| f.iter())
        .fold(0.0f64, |m, v| m.max(v.abs()));

    let mut interleaved = Vec::with_capacity(length * 2);
    for frame in &mix {
        interleaved.push(frame[0] as f32);
        interleaved.push(frame[1] as f32);
    }

    let receipt = RenderReceipt {
        scene_id: scene.scene_id.clone(),
        target: "stereo-equalpower-plus-diffuse".into(),
        sample_rate,
        channels: 2,
        duration_seconds: length as f64 / sample_rate as f64,
        peak_before_safety: peak_before,
        safety_gain_db,
        output_peak,
        channel_order: vec!["left", "right"],
        renderer: "mus.spatial-stereo/1".into(),
        object_receipts,
        metadata: json!({
            "coordinateSystem": scene.coordinate_system,
            "claim": "Deterministic production render from authored object metadata; not an HRTF binaural render.",
            "listenerPoseHonored": true,
        }),
        schema: "https://sophia-labs.ai/schemas/mus-spatial-render-receipt/1".into(),
    };
    Ok((interleaved, receipt))
}

// ================================================================== foa

/// `spatial_render._foa_coefficients`: block-interpolated ACN/SN3D
/// `[W, Y, Z, X]` encoding coefficients, plus the same distance-gain and
/// air-cutoff computation the stereo path uses.
fn foa_coefficients(
    obj: &SpatialObject,
    listener: &Listener,
    sample_rate: u32,
    length: usize,
    block_size: usize,
) -> Result<(Vec<[f64; 4]>, Vec<f64>, f64), SceneError> {
    let control = block_control_indices(length, block_size);
    let directional_gain = 1.0
        - obj
            .spread
            .max(if obj.is_diffuse_like() { 0.9 } else { 0.0 });

    let mut rows = Vec::with_capacity(control.len());
    let mut distances = Vec::with_capacity(control.len());
    for &idx in &control {
        let t = obj.start_seconds + idx as f64 / sample_rate as f64;
        let world = position_at(obj, t);
        let rel = listener_relative(world, listener)?;
        let az = rel.azimuth_radians();
        let el = rel.elevation_radians();
        rows.push([
            1.0,
            directional_gain * el.cos() * az.sin(),
            directional_gain * el.sin(),
            directional_gain * el.cos() * az.cos(),
        ]);
        distances.push(rel.distance().max(0.001));
    }

    let mut coefficients = vec![[0.0f64; 4]; length];
    for channel in 0..4 {
        let column: Vec<f64> = rows.iter().map(|r| r[channel]).collect();
        let interpolated = interp_from_controls(&control, &column, length);
        for i in 0..length {
            coefficients[i][channel] = interpolated[i];
        }
    }
    let distance = interp_from_controls(&control, &distances, length);
    let distance_gain = inverse_distance_gain(&distance);
    let air_cutoff = distance_air_cutoff(median(&distance));
    Ok((coefficients, distance_gain, air_cutoff))
}

/// `spatial_render.render_foa`: first-order Ambisonics, ACN channel
/// numbering / SN3D normalization, channel order `[W, Y, Z, X]`. Returns
/// interleaved `f32` 4-channel PCM plus the render receipt.
pub fn render_foa(
    scene: &SpatialScene,
    root: &Path,
) -> Result<(Vec<f32>, RenderReceipt), SceneError> {
    let sample_rate = scene.sample_rate;
    let length = ((scene.duration_seconds * sample_rate as f64).ceil() as i64).max(1) as usize;
    let mut mix = vec![[0.0f64; 4]; length];
    let mut room_bus = vec![0.0f64; length];
    let mut object_receipts = Vec::new();

    for obj in &scene.objects {
        if obj.muted {
            continue;
        }
        let raw = load_object_audio(root, &obj.audio, sample_rate).map_err(|e| {
            if let SceneError::PathEscape { audio, .. } = e {
                SceneError::PathEscape {
                    object_id: obj.object_id.clone(),
                    audio,
                }
            } else {
                e
            }
        })?;
        let y = trim_to_duration(raw, obj.duration_seconds, sample_rate);

        let (controlled, input_peak, output_peak, safety_gain_db, operations) =
            apply_controls_or_refuse(&y, &obj.controls, &obj.object_id)?;
        let object_gain = 10f64.powf(obj.gain_db / 20.0);
        let gained: Vec<f64> = controlled.iter().map(|&v| v * object_gain).collect();

        let (coefficients, distance_gain, air_cutoff) =
            foa_coefficients(obj, &scene.listener, sample_rate, gained.len(), BLOCK_SIZE)?;
        let absorbed = air_absorption(&gained, sample_rate, air_cutoff);

        let start = start_sample(obj.start_seconds, sample_rate);
        if start >= length {
            continue;
        }
        let available = absorbed.len().min(length - start);
        for i in 0..available {
            let g = distance_gain[i];
            for ch in 0..4 {
                mix[start + i][ch] += absorbed[i] * coefficients[i][ch] * g;
            }
            room_bus[start + i] += absorbed[i] * g * obj.reverb_send;
        }

        let manipulation = json!({
            "controls": controls_receipt_json(&obj.controls),
            "operations": operations,
            "input_peak": input_peak,
            "output_peak": output_peak,
            "safety_gain_db": safety_gain_db,
            "schema": "https://sophia-labs.ai/schemas/mus-psychoacoustic-manipulation/1",
        });
        object_receipts.push(json!({
            "objectId": obj.object_id,
            "samplesRendered": available,
            "manipulation": manipulation,
            "spatial": {
                "format": "ACN/SN3D",
                "channelOrder": ["W", "Y", "Z", "X"],
                "spread": obj.spread,
                "airAbsorptionCutoffHz": air_cutoff,
            },
        }));
    }

    if scene.room.enabled && room_bus.iter().any(|&v| v != 0.0) {
        let (left, right) = synthetic_room(
            &room_bus,
            sample_rate,
            scene.room.decay_seconds,
            scene.room.damping,
            scene.room.seed,
        );
        let wet = scene.room.wet;
        let take = left.len().min(right.len()).min(length);
        for i in 0..take {
            let omni = (left[i] + right[i]) / 2.0;
            mix[i][0] += wet * omni;
        }
    }

    let peak_before = mix
        .iter()
        .flat_map(|f| f.iter())
        .fold(0.0f64, |m, v| m.max(v.abs()));
    let mut safety_gain_db = 0.0f64;
    if peak_before > 0.98 {
        let scale = 0.98 / peak_before;
        for frame in mix.iter_mut() {
            for v in frame.iter_mut() {
                *v *= scale;
            }
        }
        safety_gain_db = 20.0 * scale.log10();
    }
    let output_peak = mix
        .iter()
        .flat_map(|f| f.iter())
        .fold(0.0f64, |m, v| m.max(v.abs()));

    let mut interleaved = Vec::with_capacity(length * 4);
    for frame in &mix {
        for &v in frame {
            interleaved.push(v as f32);
        }
    }

    let receipt = RenderReceipt {
        scene_id: scene.scene_id.clone(),
        target: "foa-ambix".into(),
        sample_rate,
        channels: 4,
        duration_seconds: length as f64 / sample_rate as f64,
        peak_before_safety: peak_before,
        safety_gain_db,
        output_peak,
        channel_order: vec!["W", "Y", "Z", "X"],
        renderer: "mus.foa-ambix-encoder/1".into(),
        object_receipts,
        metadata: json!({
            "normalization": "SN3D",
            "channelNumbering": "ACN",
            "coordinateSystem": scene.coordinate_system,
            "claim": "Portable plane-wave object encoding; binaural or loudspeaker decoding is downstream.",
            "listenerPoseHonored": true,
        }),
        schema: "https://sophia-labs.ai/schemas/mus-spatial-render-receipt/1".into(),
    };
    Ok((interleaved, receipt))
}

// ================================================================== I/O

/// `sf.write(path, audio, sample_rate, subtype="FLOAT")`: interleaved
/// `f32` samples, any channel count.
pub fn write_wav_f32(
    interleaved: &[f32],
    channels: u16,
    sample_rate: u32,
    path: &Path,
) -> Result<(), SceneError> {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|e| SceneError::Wav(format!("{}: {e}", path.display())))?;
    for &s in interleaved {
        writer
            .write_sample(s)
            .map_err(|e| SceneError::Wav(e.to_string()))?;
    }
    writer
        .finalize()
        .map_err(|e| SceneError::Wav(e.to_string()))?;
    Ok(())
}
