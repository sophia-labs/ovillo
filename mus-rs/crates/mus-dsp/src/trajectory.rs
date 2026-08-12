//! Direct authored trajectories lowered into the existing string-network voice.
//!
//! This is the first production client of `mus-trajectory`. It does not add a
//! second automation executor: non-pitch coordinates become the existing
//! [`crate::pluck::ControlFrame`] values, while the narrow, currently lawful
//! full-note linear pitch subset lowers to the voice's native persistent
//! retuning path (`f0 -> f1`). Richer pitch paths refuse rather than falling
//! through to a post-effect or oscillator sidecar.
//!
//! The MUS-facing inline surface reuses registered parameter arrows only:
//!
//! ```text
//! couple=0.12@0->0.42@0.6->0.18@1
//! body=0.25->0.70->0.35
//! pitch_ratio=1->1.059463094359
//! traj_curve=smoothstep
//! traj_hz=200
//! traj_strict=0
//! ```
//!
//! Unregistered arrows such as `pan=-1->1` remain untouched.

use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

use mus_trajectory::{
    equally_spaced_track, CoordinateContract, Interpolation, StateSemantics, TrajectoryError,
    TrajectoryProgram, TrajectoryReceipt, TrajectoryTrack, TRAJECTORY_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};

use crate::pluck::{ControlFrame, ControlTarget, StringNetworkVoice, WeaveTrace};
use crate::{SR, SR_F64};

pub const STRING_TRAJECTORY_ADAPTER_VERSION: &str = "mus-dsp-string-trajectory/0.1.0";
pub const DEFAULT_CONTROL_RATE_HZ: f64 = 200.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringTrajectoryErrorKind {
    Core,
    InvalidDuration,
    SampleRateMismatch,
    DurationMismatch,
    UnsupportedCoordinate,
    EngineDomainMismatch,
    PitchPathUnsupported,
    PitchConflict,
    TransportUncertified,
    InvalidInlineDirective,
    SampleIndexOverflow,
}

impl StringTrajectoryErrorKind {
    pub fn code(self) -> &'static str {
        match self {
            Self::Core => "string-trajectory-core-refusal",
            Self::InvalidDuration => "string-trajectory-invalid-duration",
            Self::SampleRateMismatch => "string-trajectory-sample-rate-mismatch",
            Self::DurationMismatch => "string-trajectory-duration-mismatch",
            Self::UnsupportedCoordinate => "string-trajectory-unsupported-coordinate",
            Self::EngineDomainMismatch => "string-trajectory-engine-domain-mismatch",
            Self::PitchPathUnsupported => "string-trajectory-pitch-path-unsupported",
            Self::PitchConflict => "string-trajectory-pitch-conflict",
            Self::TransportUncertified => "string-trajectory-transport-uncertified",
            Self::InvalidInlineDirective => "string-trajectory-invalid-inline-directive",
            Self::SampleIndexOverflow => "string-trajectory-sample-index-overflow",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StringTrajectoryError {
    pub kind: StringTrajectoryErrorKind,
    pub coordinate: Option<String>,
    pub message: String,
    pub core_code: Option<String>,
}

impl StringTrajectoryError {
    fn new(kind: StringTrajectoryErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            coordinate: None,
            message: message.into(),
            core_code: None,
        }
    }

    fn for_coordinate(
        kind: StringTrajectoryErrorKind,
        coordinate: &str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            coordinate: Some(coordinate.to_string()),
            message: message.into(),
            core_code: None,
        }
    }

    pub fn code(&self) -> &'static str {
        self.kind.code()
    }
}

impl From<TrajectoryError> for StringTrajectoryError {
    fn from(error: TrajectoryError) -> Self {
        Self {
            kind: StringTrajectoryErrorKind::Core,
            coordinate: error.coordinate,
            message: error.message,
            core_code: Some(error.code().to_string()),
        }
    }
}

impl Display for StringTrajectoryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let code = self.core_code.as_deref().unwrap_or(self.code());
        match &self.coordinate {
            Some(coordinate) => write!(formatter, "{code} ({coordinate}): {}", self.message),
            None => write!(formatter, "{code}: {}", self.message),
        }
    }
}

impl std::error::Error for StringTrajectoryError {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StringTrajectoryReceipt {
    pub adapter_version: String,
    pub trajectory: TrajectoryReceipt,
    pub native_pitch_transport: String,
    pub pitch_start_ratio: f64,
    pub pitch_end_ratio: f64,
    pub control_frame_count: usize,
    pub unresolved_obligations: Vec<String>,
    pub strict_transport: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoweredStringTrajectory {
    pub frequencies_hz: Vec<f64>,
    pub native_gliss_ratio: f64,
    pub control_frames: Vec<ControlFrame>,
    pub receipt: StringTrajectoryReceipt,
}

pub struct StringTrajectoryRender {
    pub audio: Vec<f32>,
    pub trace: WeaveTrace,
    pub receipt: StringTrajectoryReceipt,
}

/// Split registered arrow-valued physical controls from a patch.
///
/// The returned static map carries each trajectory's initial value. The input
/// map is never mutated, and unrelated arrows remain byte-for-byte strings in
/// the static map. `None` means the historical render path should be used.
pub fn program_from_inline_string_params(
    params: &BTreeMap<String, String>,
    duration_seconds: f64,
) -> Result<(BTreeMap<String, String>, Option<TrajectoryProgram>), StringTrajectoryError> {
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return Err(StringTrajectoryError::new(
            StringTrajectoryErrorKind::InvalidDuration,
            format!("duration {duration_seconds} must be finite and positive"),
        ));
    }
    let duration_samples = (duration_seconds * SR_F64).round() as u64;
    if duration_samples == 0 {
        return Err(StringTrajectoryError::new(
            StringTrajectoryErrorKind::InvalidDuration,
            "duration rounds to zero samples",
        ));
    }

    let mut candidates: Vec<(&str, &str)> = params
        .iter()
        .filter_map(|(key, value)| {
            (value.contains("->") && coordinate_for_inline_key(key).is_some())
                .then_some((key.as_str(), value.as_str()))
        })
        .collect();
    candidates.sort_by_key(|(key, _)| *key);
    if candidates.is_empty() {
        return Ok((params.clone(), None));
    }

    let control_rate_hz = params
        .get("traj_hz")
        .map(|value| parse_positive_f64("traj_hz", value))
        .transpose()?
        .unwrap_or(DEFAULT_CONTROL_RATE_HZ);
    if control_rate_hz > SR_F64 {
        return Err(StringTrajectoryError::new(
            StringTrajectoryErrorKind::InvalidInlineDirective,
            format!("traj_hz={control_rate_hz} exceeds the {SR} Hz audio rate"),
        ));
    }
    let control_stride_samples = (SR_F64 / control_rate_hz).round().max(1.0) as u32;
    let global_curve = params
        .get("traj_curve")
        .map(|value| parse_interpolation("traj_curve", value))
        .transpose()?
        .unwrap_or(Interpolation::Smoothstep);
    let strict_transport = params
        .get("traj_strict")
        .map(|value| parse_bool("traj_strict", value))
        .transpose()?
        .unwrap_or(false);

    let mut tracks = Vec::with_capacity(candidates.len());
    let mut static_params = params.clone();
    for (key, expression) in candidates {
        let coordinate = coordinate_for_inline_key(key).expect("candidate key was registered");
        let curve_key = format!("{key}_curve");
        let interpolation = match params.get(&curve_key) {
            Some(value) => parse_interpolation(&curve_key, value)?,
            None if coordinate == "string.pitch_ratio" => Interpolation::Linear,
            None => global_curve,
        };
        let contract = contract_for_coordinate(coordinate).expect("registered contract");
        let track = parse_inline_track(
            coordinate,
            key,
            expression,
            duration_samples,
            interpolation,
            contract,
        )?;
        static_params.insert(key.to_string(), track.knots[0].value.to_string());
        static_params.remove(&curve_key);
        tracks.push(track);
    }
    static_params.remove("traj_curve");
    static_params.remove("traj_hz");
    static_params.remove("traj_strict");

    let mut metadata = BTreeMap::new();
    metadata.insert("surface".to_string(), "mus-inline-string-v1".to_string());
    metadata.insert("strict_transport".to_string(), strict_transport.to_string());
    let program = TrajectoryProgram {
        schema_version: TRAJECTORY_SCHEMA_VERSION.to_string(),
        sample_rate_hz: SR as u32,
        duration_samples,
        control_stride_samples,
        metadata,
        tracks,
    };
    program.validate()?;
    Ok((static_params, Some(program)))
}

pub fn lower_string_trajectory(
    program: &TrajectoryProgram,
    frequencies_hz: &[f64],
    duration_seconds: f64,
    score_gliss_ratio: f64,
) -> Result<LoweredStringTrajectory, StringTrajectoryError> {
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return Err(StringTrajectoryError::new(
            StringTrajectoryErrorKind::InvalidDuration,
            format!("duration {duration_seconds} must be finite and positive"),
        ));
    }
    if program.sample_rate_hz != SR as u32 {
        return Err(StringTrajectoryError::new(
            StringTrajectoryErrorKind::SampleRateMismatch,
            format!(
                "program is {} Hz but the string engine is {SR} Hz",
                program.sample_rate_hz
            ),
        ));
    }
    let expected_duration = (duration_seconds * SR_F64).round() as u64;
    if program.duration_samples != expected_duration {
        return Err(StringTrajectoryError::new(
            StringTrajectoryErrorKind::DurationMismatch,
            format!(
                "program duration {} samples does not match event duration {expected_duration}",
                program.duration_samples
            ),
        ));
    }
    if !score_gliss_ratio.is_finite() || score_gliss_ratio <= 0.0 {
        return Err(StringTrajectoryError::new(
            StringTrajectoryErrorKind::PitchConflict,
            format!("score gliss ratio {score_gliss_ratio} is not positive and finite"),
        ));
    }

    let compiled = program.compile()?;
    let strict_transport = program
        .metadata
        .get("strict_transport")
        .is_some_and(|value| value == "true" || value == "1");
    let mut unresolved = Vec::new();
    for track in &program.tracks {
        let (minimum, maximum) = engine_bounds(&track.coordinate).ok_or_else(|| {
            StringTrajectoryError::for_coordinate(
                StringTrajectoryErrorKind::UnsupportedCoordinate,
                &track.coordinate,
                "the string adapter has no lowering for this coordinate",
            )
        })?;
        for knot in &track.knots {
            if !(minimum..=maximum).contains(&knot.value) {
                return Err(StringTrajectoryError::for_coordinate(
                    StringTrajectoryErrorKind::EngineDomainMismatch,
                    &track.coordinate,
                    format!(
                        "{} is outside the production engine domain [{minimum}, {maximum}]",
                        knot.value
                    ),
                ));
            }
        }
        unresolved.extend(
            obligations_for_coordinate(&track.coordinate)
                .iter()
                .map(|obligation| (*obligation).to_string()),
        );
    }
    unresolved.sort_unstable();
    unresolved.dedup();
    if strict_transport && !unresolved.is_empty() {
        return Err(StringTrajectoryError::new(
            StringTrajectoryErrorKind::TransportUncertified,
            format!(
                "strict transport requested but obligations remain: {}",
                unresolved.join(", ")
            ),
        ));
    }

    let pitch_track = program
        .tracks
        .iter()
        .find(|track| track.coordinate == "string.pitch_ratio");
    let (pitch_start_ratio, pitch_end_ratio, native_gliss_ratio) = match pitch_track {
        Some(track) => {
            if track.interpolation != Interpolation::Linear || track.knots.len() != 2 {
                return Err(StringTrajectoryError::for_coordinate(
                    StringTrajectoryErrorKind::PitchPathUnsupported,
                    &track.coordinate,
                    "v1 native pitch transport accepts exactly two linear full-event knots",
                ));
            }
            if (score_gliss_ratio - 1.0).abs() > 1e-12 {
                return Err(StringTrajectoryError::for_coordinate(
                    StringTrajectoryErrorKind::PitchConflict,
                    &track.coordinate,
                    "score gliss and a pitch-ratio trajectory cannot both own the same retune path",
                ));
            }
            let start = track.knots[0].value;
            let end = track.knots[1].value;
            (start, end, end / start)
        }
        None => (1.0, score_gliss_ratio, score_gliss_ratio),
    };
    let scaled_frequencies: Vec<f64> = frequencies_hz
        .iter()
        .map(|frequency| frequency * pitch_start_ratio)
        .collect();

    let mut control_frames = Vec::new();
    for frame in &compiled.frames {
        let at_sample = usize::try_from(frame.at_sample).map_err(|_| {
            StringTrajectoryError::new(
                StringTrajectoryErrorKind::SampleIndexOverflow,
                format!("sample {} does not fit usize", frame.at_sample),
            )
        })?;
        let mut sets = Vec::new();
        for set in &frame.sets {
            if set.coordinate == "string.pitch_ratio" {
                continue;
            }
            let target = target_for_coordinate(&set.coordinate).ok_or_else(|| {
                StringTrajectoryError::for_coordinate(
                    StringTrajectoryErrorKind::UnsupportedCoordinate,
                    &set.coordinate,
                    "the string adapter has no ControlTarget for this coordinate",
                )
            })?;
            sets.push((target, set.value));
        }
        if !sets.is_empty() {
            control_frames.push(ControlFrame { at_sample, sets });
        }
    }

    Ok(LoweredStringTrajectory {
        frequencies_hz: scaled_frequencies,
        native_gliss_ratio,
        receipt: StringTrajectoryReceipt {
            adapter_version: STRING_TRAJECTORY_ADAPTER_VERSION.to_string(),
            trajectory: compiled.receipt,
            native_pitch_transport: if pitch_track.is_some() {
                "persistent-linear-f0-to-f1".to_string()
            } else {
                "score-gliss-or-static".to_string()
            },
            pitch_start_ratio,
            pitch_end_ratio,
            control_frame_count: control_frames.len(),
            unresolved_obligations: unresolved,
            strict_transport,
        },
        control_frames,
    })
}

pub fn render_weave_program(
    params: &BTreeMap<String, String>,
    frequencies_hz: &[f64],
    duration_seconds: f64,
    score_gliss_ratio: f64,
    program: &TrajectoryProgram,
    trace_stride: usize,
) -> Result<StringTrajectoryRender, StringTrajectoryError> {
    let lowered =
        lower_string_trajectory(program, frequencies_hz, duration_seconds, score_gliss_ratio)?;
    let voice = StringNetworkVoice::weave(
        params,
        &lowered.frequencies_hz,
        duration_seconds,
        lowered.native_gliss_ratio,
    );
    let (audio, trace) = voice.render_traced_with(trace_stride, &lowered.control_frames);
    Ok(StringTrajectoryRender {
        audio,
        trace,
        receipt: lowered.receipt,
    })
}

pub fn render_guitar_program(
    params: &BTreeMap<String, String>,
    frequencies_hz: &[f64],
    duration_seconds: f64,
    score_gliss_ratio: f64,
    program: &TrajectoryProgram,
    trace_stride: usize,
) -> Result<StringTrajectoryRender, StringTrajectoryError> {
    let lowered =
        lower_string_trajectory(program, frequencies_hz, duration_seconds, score_gliss_ratio)?;
    let voice = StringNetworkVoice::guitar(
        params,
        &lowered.frequencies_hz,
        duration_seconds,
        lowered.native_gliss_ratio,
    );
    let (audio, trace) = voice.render_traced_with(trace_stride, &lowered.control_frames);
    Ok(StringTrajectoryRender {
        audio,
        trace,
        receipt: lowered.receipt,
    })
}

fn parse_inline_track(
    coordinate: &str,
    source_key: &str,
    expression: &str,
    duration_samples: u64,
    interpolation: Interpolation,
    contract: CoordinateContract,
) -> Result<TrajectoryTrack, StringTrajectoryError> {
    let terms: Vec<&str> = expression.split("->").map(str::trim).collect();
    if terms.len() < 2 || terms.iter().any(|term| term.is_empty()) {
        return Err(StringTrajectoryError::for_coordinate(
            StringTrajectoryErrorKind::InvalidInlineDirective,
            coordinate,
            format!("{source_key} needs at least two non-empty arrow terms"),
        ));
    }
    let parsed: Vec<(f64, Option<f64>)> = terms
        .iter()
        .enumerate()
        .map(|(index, term)| parse_inline_term(source_key, index, term))
        .collect::<Result<_, _>>()?;
    let timed = parsed.iter().filter(|(_, time)| time.is_some()).count();
    if timed != 0 && timed != parsed.len() {
        return Err(StringTrajectoryError::for_coordinate(
            StringTrajectoryErrorKind::InvalidInlineDirective,
            coordinate,
            format!("{source_key}: every term must carry @time or none may"),
        ));
    }
    let values: Vec<f64> = parsed.iter().map(|(value, _)| *value).collect();
    if timed == 0 {
        return equally_spaced_track(
            coordinate,
            &values,
            duration_samples,
            interpolation,
            contract,
        )
        .map_err(Into::into);
    }

    let knots = parsed
        .into_iter()
        .map(|(value, normalized)| {
            let normalized = normalized.expect("all terms were timed");
            if !(0.0..=1.0).contains(&normalized) {
                return Err(StringTrajectoryError::for_coordinate(
                    StringTrajectoryErrorKind::InvalidInlineDirective,
                    coordinate,
                    format!("normalized time {normalized} is outside [0, 1]"),
                ));
            }
            Ok(mus_trajectory::TrajectoryKnot {
                at_sample: (normalized * duration_samples as f64).round() as u64,
                value,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let track = TrajectoryTrack {
        coordinate: coordinate.to_string(),
        interpolation,
        contract,
        knots,
    };
    Ok(track)
}

fn parse_inline_term(
    key: &str,
    index: usize,
    term: &str,
) -> Result<(f64, Option<f64>), StringTrajectoryError> {
    let (value_source, time_source) = match term.rsplit_once('@') {
        Some((value, time)) => (value.trim(), Some(time.trim())),
        None => (term.trim(), None),
    };
    let value = value_source.parse::<f64>().map_err(|_| {
        StringTrajectoryError::new(
            StringTrajectoryErrorKind::InvalidInlineDirective,
            format!("{key}[{index}] value {value_source:?} is not numeric"),
        )
    })?;
    if !value.is_finite() {
        return Err(StringTrajectoryError::new(
            StringTrajectoryErrorKind::InvalidInlineDirective,
            format!("{key}[{index}] value is not finite"),
        ));
    }
    let time = time_source
        .map(|source| {
            let parsed = source.parse::<f64>().map_err(|_| {
                StringTrajectoryError::new(
                    StringTrajectoryErrorKind::InvalidInlineDirective,
                    format!("{key}[{index}] time {source:?} is not numeric"),
                )
            })?;
            if !parsed.is_finite() {
                return Err(StringTrajectoryError::new(
                    StringTrajectoryErrorKind::InvalidInlineDirective,
                    format!("{key}[{index}] time is not finite"),
                ));
            }
            Ok(parsed)
        })
        .transpose()?;
    Ok((value, time))
}

fn parse_positive_f64(key: &str, source: &str) -> Result<f64, StringTrajectoryError> {
    let value = source.parse::<f64>().map_err(|_| {
        StringTrajectoryError::new(
            StringTrajectoryErrorKind::InvalidInlineDirective,
            format!("{key}={source:?} is not numeric"),
        )
    })?;
    if !value.is_finite() || value <= 0.0 {
        return Err(StringTrajectoryError::new(
            StringTrajectoryErrorKind::InvalidInlineDirective,
            format!("{key} must be finite and positive"),
        ));
    }
    Ok(value)
}

fn parse_interpolation(key: &str, source: &str) -> Result<Interpolation, StringTrajectoryError> {
    Interpolation::from_name(source).ok_or_else(|| {
        StringTrajectoryError::new(
            StringTrajectoryErrorKind::InvalidInlineDirective,
            format!("{key}={source:?} is not a known interpolation"),
        )
    })
}

fn parse_bool(key: &str, source: &str) -> Result<bool, StringTrajectoryError> {
    match source {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(StringTrajectoryError::new(
            StringTrajectoryErrorKind::InvalidInlineDirective,
            format!("{key}={source:?} is not a boolean"),
        )),
    }
}

fn coordinate_for_inline_key(key: &str) -> Option<&'static str> {
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

fn contract_for_coordinate(coordinate: &str) -> Option<CoordinateContract> {
    let (unit, minimum, maximum, metric_weight) = match coordinate {
        "string.couple" => ("radian", 0.0, 0.45, 1.0),
        "string.chirality" => ("ratio", -1.0, 1.0, 0.5),
        "string.orbit_hz" => ("hertz", 0.0, 20.0, 0.05),
        "string.orbit_depth" => ("ratio", 0.0, 1.0, 0.5),
        "string.curvature" => ("ratio", 0.0, 1.0, 0.5),
        "string.body" => ("ratio", 0.0, 1.0, 1.0),
        "string.buzz" => ("ratio", 0.0, 1.0, 0.5),
        "string.sing" => ("ratio", 0.0, 1.0, 0.0),
        "string.bow" => ("ratio", 0.0, 1.0, 0.0),
        "string.bow_speed" => ("ratio", 0.0, 1.0, 0.0),
        "string.sus" => ("second", 0.05, 20.0, 0.1),
        "string.damp" => ("ratio", 0.0, 0.95, 0.5),
        "string.breath" => ("ratio", 0.0, 1.0, 0.0),
        "string.pitch_ratio" => ("ratio", 0.8, 1.2, 1.0),
        _ => return None,
    };
    Some(CoordinateContract {
        unit: unit.to_string(),
        minimum,
        maximum,
        max_rate_per_second: None,
        max_acceleration_per_second2: None,
        metric_weight: Some(metric_weight),
        state_semantics: StateSemantics::FixedCapacity,
    })
}

fn engine_bounds(coordinate: &str) -> Option<(f64, f64)> {
    let contract = contract_for_coordinate(coordinate)?;
    Some((contract.minimum, contract.maximum))
}

fn target_for_coordinate(coordinate: &str) -> Option<ControlTarget> {
    Some(match coordinate {
        "string.couple" => ControlTarget::Couple,
        "string.chirality" => ControlTarget::Chirality,
        "string.orbit_hz" => ControlTarget::OrbitHz,
        "string.orbit_depth" => ControlTarget::OrbitDepth,
        "string.curvature" => ControlTarget::Curvature,
        "string.body" => ControlTarget::Body,
        "string.buzz" => ControlTarget::Buzz,
        "string.sing" => ControlTarget::Sing,
        "string.bow" => ControlTarget::Bow,
        "string.bow_speed" => ControlTarget::BowSpeed,
        "string.sus" => ControlTarget::Sus,
        "string.damp" => ControlTarget::Damp,
        "string.breath" => ControlTarget::Breath,
        _ => return None,
    })
}

fn obligations_for_coordinate(coordinate: &str) -> &'static [&'static str] {
    match coordinate {
        "string.pitch_ratio" => &[
            "integer-delay-state-transport",
            "pitch-sideband-certificate",
        ],
        "string.bow" | "string.bow_speed" => &["bow-contact-supply-law"],
        "string.breath" => &["stochastic-source-ledger"],
        "string.sing" => &["external-source-work-ledger"],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn inline_parser_leaves_unrelated_arrows_alone() {
        let source = params(&[
            ("couple", "0.12@0->0.42@0.6->0.18@1"),
            ("body", "0.25->0.70->0.35"),
            ("pan", "-1->1"),
            ("lpf", "70->900"),
            ("traj_curve", "smoothstep"),
            ("traj_hz", "100"),
        ]);
        let (static_params, program) =
            program_from_inline_string_params(&source, 1.0).expect("inline program");
        let program = program.expect("registered arrows found");
        assert_eq!(static_params["pan"], "-1->1");
        assert_eq!(static_params["lpf"], "70->900");
        assert_eq!(static_params["couple"], "0.12");
        assert_eq!(program.control_stride_samples, 480);
        assert_eq!(program.tracks.len(), 2);
    }

    #[test]
    fn native_pitch_subset_scales_start_and_uses_existing_gliss_path() {
        let source = params(&[("pitch_ratio", "1->1.0594630943592953")]);
        let (_, program) = program_from_inline_string_params(&source, 0.5).expect("inline program");
        let lowered =
            lower_string_trajectory(&program.expect("program"), &[110.0, 165.0], 0.5, 1.0)
                .expect("native pitch lowering");
        assert_eq!(lowered.frequencies_hz, vec![110.0, 165.0]);
        assert!((lowered.native_gliss_ratio - 1.0594630943592953).abs() < 1e-15);
        assert!(lowered.control_frames.is_empty());
        assert!(lowered
            .receipt
            .unresolved_obligations
            .contains(&"integer-delay-state-transport".to_string()));
    }

    #[test]
    fn nonlinear_pitch_refuses_instead_of_becoming_framewise_fake_pitch() {
        let source = params(&[
            ("pitch_ratio", "1->1.05->1.1"),
            ("pitch_ratio_curve", "smoothstep"),
        ]);
        let (_, program) = program_from_inline_string_params(&source, 0.5).expect("inline program");
        let error = lower_string_trajectory(&program.expect("program"), &[110.0], 0.5, 1.0)
            .expect_err("unsupported pitch path");
        assert_eq!(error.kind, StringTrajectoryErrorKind::PitchPathUnsupported);
    }

    #[test]
    fn strict_transport_refuses_open_supply_and_pitch_obligations() {
        let source = params(&[
            ("pitch_ratio", "1->1.02"),
            ("bow", "0.1->0.5"),
            ("traj_strict", "1"),
        ]);
        let (_, program) = program_from_inline_string_params(&source, 0.5).expect("inline program");
        let error = lower_string_trajectory(&program.expect("program"), &[110.0], 0.5, 1.0)
            .expect_err("strict refusal");
        assert_eq!(error.kind, StringTrajectoryErrorKind::TransportUncertified);
    }
}
