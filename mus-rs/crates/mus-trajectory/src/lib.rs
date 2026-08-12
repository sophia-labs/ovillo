//! Authored continuous control paths for persistent physical instrument bodies.
//!
//! `mus-trajectory` owns the direct path layer:
//!
//! ```text
//! authored knots + coordinate contracts
//!     -> validated continuous paths
//!     -> deterministic sample-indexed control frames
//!     -> receipt with bounds, declared metric effort, and content digests
//! ```
//!
//! It deliberately does **not** choose a path from an acoustic target. That is
//! `mus-gesture`'s inverse-compilation job. Keeping the two layers separate lets
//! a musician author a path now, while an agent may later synthesize an
//! equivalent [`TrajectoryProgram`] from a higher-level request.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const TRAJECTORY_SCHEMA_VERSION: &str = "trajectory-program-v1";
pub const TRAJECTORY_COMPILER_VERSION: &str = "mus-trajectory/0.1.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Interpolation {
    Step,
    Linear,
    Smoothstep,
    Smootherstep,
    Cosine,
}

impl Interpolation {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "step" => Self::Step,
            "linear" | "lin" => Self::Linear,
            "smoothstep" | "smooth" => Self::Smoothstep,
            "smootherstep" | "smoother" => Self::Smootherstep,
            "cosine" | "cos" => Self::Cosine,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Step => "step",
            Self::Linear => "linear",
            Self::Smoothstep => "smoothstep",
            Self::Smootherstep => "smootherstep",
            Self::Cosine => "cosine",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisposalPolicy {
    Radiate,
    Dissipate,
    Retire,
    Reject,
}

impl DisposalPolicy {
    pub fn name(self) -> &'static str {
        match self {
            Self::Radiate => "radiate",
            Self::Dissipate => "dissipate",
            Self::Retire => "retire",
            Self::Reject => "reject",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StateSemantics {
    #[default]
    FixedCapacity,
    TopologyEdit {
        disposition: DisposalPolicy,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoordinateContract {
    pub unit: String,
    pub minimum: f64,
    pub maximum: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_rate_per_second: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_acceleration_per_second2: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric_weight: Option<f64>,
    #[serde(default)]
    pub state_semantics: StateSemantics,
}

impl CoordinateContract {
    pub fn fixed(unit: impl Into<String>, minimum: f64, maximum: f64) -> Self {
        Self {
            unit: unit.into(),
            minimum,
            maximum,
            max_rate_per_second: None,
            max_acceleration_per_second2: None,
            metric_weight: None,
            state_semantics: StateSemantics::FixedCapacity,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryKnot {
    pub at_sample: u64,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryTrack {
    pub coordinate: String,
    pub interpolation: Interpolation,
    pub contract: CoordinateContract,
    pub knots: Vec<TrajectoryKnot>,
}

impl TrajectoryTrack {
    pub fn value_at(&self, sample: u64, duration_samples: u64) -> Result<f64, TrajectoryError> {
        if sample > duration_samples {
            return Err(TrajectoryError::for_coordinate(
                TrajectoryErrorKind::SampleOutOfRange,
                &self.coordinate,
                format!("sample {sample} is after the authored duration {duration_samples}"),
            ));
        }
        let search = self
            .knots
            .binary_search_by_key(&sample, |knot| knot.at_sample);
        match search {
            Ok(index) => Ok(self.knots[index].value),
            Err(0) => Ok(self.knots[0].value),
            Err(index) if index >= self.knots.len() => Ok(self.knots[self.knots.len() - 1].value),
            Err(index) => {
                let left = self.knots[index - 1];
                let right = self.knots[index];
                let width = right.at_sample - left.at_sample;
                let position = sample - left.at_sample;
                let phase = position as f64 / width as f64;
                let shaped = match self.interpolation {
                    Interpolation::Step => 0.0,
                    Interpolation::Linear => phase,
                    Interpolation::Smoothstep => phase * phase * (3.0 - 2.0 * phase),
                    Interpolation::Smootherstep => {
                        phase * phase * phase * (phase * (phase * 6.0 - 15.0) + 10.0)
                    }
                    Interpolation::Cosine => 0.5 - 0.5 * (std::f64::consts::PI * phase).cos(),
                };
                Ok(left.value + (right.value - left.value) * shaped)
            }
        }
    }

    fn changed(&self) -> bool {
        self.knots
            .windows(2)
            .any(|pair| pair[0].value.to_bits() != pair[1].value.to_bits())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryProgram {
    pub schema_version: String,
    pub sample_rate_hz: u32,
    pub duration_samples: u64,
    pub control_stride_samples: u32,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    pub tracks: Vec<TrajectoryTrack>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoordinateValue {
    pub coordinate: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryFrame {
    pub at_sample: u64,
    pub sets: Vec<CoordinateValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackReceipt {
    pub coordinate: String,
    pub interpolation: Interpolation,
    pub start_value: f64,
    pub end_value: f64,
    pub max_rate_per_second: Option<f64>,
    pub max_acceleration_per_second2: Option<f64>,
    pub declared_metric_effort: f64,
    pub zero_order_hold_error_bound: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryReceipt {
    pub schema_version: String,
    pub compiler_version: String,
    pub program_digest: String,
    pub frame_digest: String,
    pub sample_rate_hz: u32,
    pub duration_samples: u64,
    pub control_stride_samples: u32,
    pub frame_count: usize,
    pub tracks: Vec<TrackReceipt>,
    pub claim_level: String,
    pub evidence_class: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledTrajectory {
    pub frames: Vec<TrajectoryFrame>,
    pub receipt: TrajectoryReceipt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrajectoryErrorKind {
    UnsupportedSchema,
    InvalidSampleRate,
    InvalidDuration,
    InvalidControlStride,
    EmptyCoordinate,
    DuplicateCoordinate,
    InvalidContract,
    TooFewKnots,
    EndpointMismatch,
    NonIncreasingKnots,
    NonFiniteValue,
    ValueOutOfRange,
    StepRequiresTopology,
    TopologyRequiresStep,
    CertificateUnavailable,
    RateLimitExceeded,
    AccelerationLimitExceeded,
    SampleOutOfRange,
    InvalidArrowSyntax,
    DurationTooShort,
    Serialization,
}

impl TrajectoryErrorKind {
    pub fn code(self) -> &'static str {
        match self {
            Self::UnsupportedSchema => "trajectory-unsupported-schema",
            Self::InvalidSampleRate => "trajectory-invalid-sample-rate",
            Self::InvalidDuration => "trajectory-invalid-duration",
            Self::InvalidControlStride => "trajectory-invalid-control-stride",
            Self::EmptyCoordinate => "trajectory-empty-coordinate",
            Self::DuplicateCoordinate => "trajectory-duplicate-coordinate",
            Self::InvalidContract => "trajectory-invalid-contract",
            Self::TooFewKnots => "trajectory-too-few-knots",
            Self::EndpointMismatch => "trajectory-endpoint-mismatch",
            Self::NonIncreasingKnots => "trajectory-nonincreasing-knots",
            Self::NonFiniteValue => "trajectory-nonfinite-value",
            Self::ValueOutOfRange => "trajectory-value-out-of-range",
            Self::StepRequiresTopology => "trajectory-step-requires-topology",
            Self::TopologyRequiresStep => "trajectory-topology-requires-step",
            Self::CertificateUnavailable => "trajectory-certificate-unavailable",
            Self::RateLimitExceeded => "trajectory-rate-limit-exceeded",
            Self::AccelerationLimitExceeded => "trajectory-acceleration-limit-exceeded",
            Self::SampleOutOfRange => "trajectory-sample-out-of-range",
            Self::InvalidArrowSyntax => "trajectory-invalid-arrow-syntax",
            Self::DurationTooShort => "trajectory-duration-too-short",
            Self::Serialization => "trajectory-serialization",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrajectoryError {
    pub kind: TrajectoryErrorKind,
    pub coordinate: Option<String>,
    pub message: String,
}

impl TrajectoryError {
    fn new(kind: TrajectoryErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            coordinate: None,
            message: message.into(),
        }
    }

    fn for_coordinate(
        kind: TrajectoryErrorKind,
        coordinate: &str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            coordinate: Some(coordinate.to_string()),
            message: message.into(),
        }
    }

    pub fn code(&self) -> &'static str {
        self.kind.code()
    }
}

impl Display for TrajectoryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.coordinate {
            Some(coordinate) => write!(
                formatter,
                "{} ({coordinate}): {}",
                self.kind.code(),
                self.message
            ),
            None => write!(formatter, "{}: {}", self.kind.code(), self.message),
        }
    }
}

impl std::error::Error for TrajectoryError {}

#[derive(Debug, Clone, Copy)]
struct TrackKinematics {
    max_rate: Option<f64>,
    max_acceleration: Option<f64>,
    metric_effort: f64,
}

impl TrajectoryProgram {
    pub fn from_json(source: &str) -> Result<Self, TrajectoryError> {
        let program: Self = serde_json::from_str(source).map_err(|error| {
            TrajectoryError::new(
                TrajectoryErrorKind::Serialization,
                format!("could not decode trajectory JSON: {error}"),
            )
        })?;
        program.validate()?;
        Ok(program)
    }

    pub fn to_pretty_json(&self) -> Result<String, TrajectoryError> {
        serde_json::to_string_pretty(self).map_err(|error| {
            TrajectoryError::new(
                TrajectoryErrorKind::Serialization,
                format!("could not encode trajectory JSON: {error}"),
            )
        })
    }

    pub fn validate(&self) -> Result<(), TrajectoryError> {
        if self.schema_version != TRAJECTORY_SCHEMA_VERSION {
            return Err(TrajectoryError::new(
                TrajectoryErrorKind::UnsupportedSchema,
                format!(
                    "expected {TRAJECTORY_SCHEMA_VERSION}, found {}",
                    self.schema_version
                ),
            ));
        }
        if self.sample_rate_hz == 0 {
            return Err(TrajectoryError::new(
                TrajectoryErrorKind::InvalidSampleRate,
                "sample_rate_hz must be positive",
            ));
        }
        if self.duration_samples == 0 {
            return Err(TrajectoryError::new(
                TrajectoryErrorKind::InvalidDuration,
                "duration_samples must be positive",
            ));
        }
        if self.control_stride_samples == 0 {
            return Err(TrajectoryError::new(
                TrajectoryErrorKind::InvalidControlStride,
                "control_stride_samples must be positive",
            ));
        }

        let mut coordinates = BTreeSet::new();
        for track in &self.tracks {
            self.validate_track(track)?;
            if !coordinates.insert(track.coordinate.as_str()) {
                return Err(TrajectoryError::for_coordinate(
                    TrajectoryErrorKind::DuplicateCoordinate,
                    &track.coordinate,
                    "each coordinate may appear only once",
                ));
            }
        }
        Ok(())
    }

    fn validate_track(&self, track: &TrajectoryTrack) -> Result<(), TrajectoryError> {
        if track.coordinate.trim().is_empty() || track.coordinate.trim() != track.coordinate {
            return Err(TrajectoryError::for_coordinate(
                TrajectoryErrorKind::EmptyCoordinate,
                &track.coordinate,
                "coordinate names must be non-empty and have no surrounding whitespace",
            ));
        }
        let contract = &track.contract;
        if contract.unit.trim().is_empty()
            || !contract.minimum.is_finite()
            || !contract.maximum.is_finite()
            || contract.minimum > contract.maximum
        {
            return Err(TrajectoryError::for_coordinate(
                TrajectoryErrorKind::InvalidContract,
                &track.coordinate,
                "unit must be non-empty and finite minimum must not exceed maximum",
            ));
        }
        for (name, value, positive) in [
            ("max_rate_per_second", contract.max_rate_per_second, true),
            (
                "max_acceleration_per_second2",
                contract.max_acceleration_per_second2,
                true,
            ),
            ("metric_weight", contract.metric_weight, false),
        ] {
            if let Some(value) = value {
                let invalid_sign = if positive { value <= 0.0 } else { value < 0.0 };
                if !value.is_finite() || invalid_sign {
                    return Err(TrajectoryError::for_coordinate(
                        TrajectoryErrorKind::InvalidContract,
                        &track.coordinate,
                        format!("{name} is outside its finite admissible range"),
                    ));
                }
            }
        }
        if track.knots.len() < 2 {
            return Err(TrajectoryError::for_coordinate(
                TrajectoryErrorKind::TooFewKnots,
                &track.coordinate,
                "a track needs at least two knots",
            ));
        }
        if track.knots.first().map(|knot| knot.at_sample) != Some(0)
            || track.knots.last().map(|knot| knot.at_sample) != Some(self.duration_samples)
        {
            return Err(TrajectoryError::for_coordinate(
                TrajectoryErrorKind::EndpointMismatch,
                &track.coordinate,
                format!(
                    "first knot must be at 0 and last knot at {}",
                    self.duration_samples
                ),
            ));
        }
        for pair in track.knots.windows(2) {
            if pair[0].at_sample >= pair[1].at_sample {
                return Err(TrajectoryError::for_coordinate(
                    TrajectoryErrorKind::NonIncreasingKnots,
                    &track.coordinate,
                    "knot samples must be strictly increasing",
                ));
            }
        }
        for knot in &track.knots {
            if !knot.value.is_finite() {
                return Err(TrajectoryError::for_coordinate(
                    TrajectoryErrorKind::NonFiniteValue,
                    &track.coordinate,
                    format!("knot at sample {} is not finite", knot.at_sample),
                ));
            }
            if !(contract.minimum..=contract.maximum).contains(&knot.value) {
                return Err(TrajectoryError::for_coordinate(
                    TrajectoryErrorKind::ValueOutOfRange,
                    &track.coordinate,
                    format!(
                        "{} is outside [{}, {}] at sample {}",
                        knot.value, contract.minimum, contract.maximum, knot.at_sample
                    ),
                ));
            }
        }

        let changed = track.changed();
        match &contract.state_semantics {
            StateSemantics::FixedCapacity => {
                if changed && track.interpolation == Interpolation::Step {
                    return Err(TrajectoryError::for_coordinate(
                        TrajectoryErrorKind::StepRequiresTopology,
                        &track.coordinate,
                        "a discontinuous authored change needs explicit topology-edit semantics",
                    ));
                }
            }
            StateSemantics::TopologyEdit { .. } => {
                if track.interpolation != Interpolation::Step {
                    return Err(TrajectoryError::for_coordinate(
                        TrajectoryErrorKind::TopologyRequiresStep,
                        &track.coordinate,
                        "topology edits must occur at declared knot boundaries",
                    ));
                }
                if contract.metric_weight.unwrap_or(0.0) > 0.0 {
                    return Err(TrajectoryError::for_coordinate(
                        TrajectoryErrorKind::CertificateUnavailable,
                        &track.coordinate,
                        "quadratic path effort is not defined across a topology edit",
                    ));
                }
            }
        }

        let measured = self.track_kinematics(track);
        if let Some(limit) = contract.max_rate_per_second {
            match measured.max_rate {
                Some(value) if value <= limit * (1.0 + 1e-12) => {}
                Some(value) => {
                    return Err(TrajectoryError::for_coordinate(
                        TrajectoryErrorKind::RateLimitExceeded,
                        &track.coordinate,
                        format!("measured rate {value} exceeds declared limit {limit}"),
                    ));
                }
                None if changed => {
                    return Err(TrajectoryError::for_coordinate(
                        TrajectoryErrorKind::CertificateUnavailable,
                        &track.coordinate,
                        "a finite rate certificate is unavailable for this path",
                    ));
                }
                None => {}
            }
        }
        if let Some(limit) = contract.max_acceleration_per_second2 {
            match measured.max_acceleration {
                Some(value) if value <= limit * (1.0 + 1e-12) => {}
                Some(value) => {
                    return Err(TrajectoryError::for_coordinate(
                        TrajectoryErrorKind::AccelerationLimitExceeded,
                        &track.coordinate,
                        format!("measured acceleration {value} exceeds declared limit {limit}"),
                    ));
                }
                None if changed => {
                    return Err(TrajectoryError::for_coordinate(
                        TrajectoryErrorKind::CertificateUnavailable,
                        &track.coordinate,
                        "piecewise-linear or discontinuous corners have no finite acceleration certificate",
                    ));
                }
                None => {}
            }
        }
        Ok(())
    }

    fn track_kinematics(&self, track: &TrajectoryTrack) -> TrackKinematics {
        let sample_rate = f64::from(self.sample_rate_hz);
        let mut max_rate = 0.0_f64;
        let mut max_acceleration = 0.0_f64;
        let mut acceleration_available = true;
        let mut metric_effort = 0.0_f64;
        let metric_weight = track.contract.metric_weight.unwrap_or(0.0);

        for pair in track.knots.windows(2) {
            let delta = pair[1].value - pair[0].value;
            if delta == 0.0 {
                continue;
            }
            let seconds = (pair[1].at_sample - pair[0].at_sample) as f64 / sample_rate;
            let (rate_factor, acceleration_factor, effort_factor) =
                interpolation_factors(track.interpolation);
            if let Some(factor) = rate_factor {
                max_rate = max_rate.max(factor * delta.abs() / seconds);
            }
            match acceleration_factor {
                Some(factor) => {
                    max_acceleration =
                        max_acceleration.max(factor * delta.abs() / (seconds * seconds));
                }
                None => acceleration_available = false,
            }
            if metric_weight > 0.0 {
                if let Some(factor) = effort_factor {
                    metric_effort += metric_weight * factor * delta * delta / seconds;
                }
            }
        }

        TrackKinematics {
            max_rate: if track.changed() && track.interpolation == Interpolation::Step {
                None
            } else {
                Some(max_rate)
            },
            max_acceleration: if acceleration_available {
                Some(max_acceleration)
            } else {
                None
            },
            metric_effort,
        }
    }

    pub fn semantic_digest(&self) -> Result<String, TrajectoryError> {
        self.validate()?;
        Ok(sha256_prefixed(&self.semantic_bytes()))
    }

    fn semantic_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        put_string(&mut bytes, "mus-trajectory-semantic-v1");
        put_string(&mut bytes, &self.schema_version);
        bytes.extend_from_slice(&self.sample_rate_hz.to_be_bytes());
        bytes.extend_from_slice(&self.duration_samples.to_be_bytes());
        bytes.extend_from_slice(&self.control_stride_samples.to_be_bytes());

        put_len(&mut bytes, self.metadata.len());
        for (key, value) in &self.metadata {
            put_string(&mut bytes, key);
            put_string(&mut bytes, value);
        }

        let mut tracks: Vec<&TrajectoryTrack> = self.tracks.iter().collect();
        tracks.sort_by(|left, right| left.coordinate.cmp(&right.coordinate));
        put_len(&mut bytes, tracks.len());
        for track in tracks {
            put_string(&mut bytes, &track.coordinate);
            put_string(&mut bytes, track.interpolation.name());
            put_string(&mut bytes, &track.contract.unit);
            put_f64(&mut bytes, track.contract.minimum);
            put_f64(&mut bytes, track.contract.maximum);
            put_option_f64(&mut bytes, track.contract.max_rate_per_second);
            put_option_f64(&mut bytes, track.contract.max_acceleration_per_second2);
            put_option_f64(&mut bytes, track.contract.metric_weight);
            match &track.contract.state_semantics {
                StateSemantics::FixedCapacity => {
                    put_string(&mut bytes, "fixed_capacity");
                }
                StateSemantics::TopologyEdit { disposition } => {
                    put_string(&mut bytes, "topology_edit");
                    put_string(&mut bytes, disposition.name());
                }
            }
            put_len(&mut bytes, track.knots.len());
            for knot in &track.knots {
                bytes.extend_from_slice(&knot.at_sample.to_be_bytes());
                put_f64(&mut bytes, knot.value);
            }
        }
        bytes
    }

    pub fn compile(&self) -> Result<CompiledTrajectory, TrajectoryError> {
        self.validate()?;

        let mut sample_points = BTreeSet::new();
        sample_points.insert(0_u64);
        sample_points.insert(self.duration_samples);
        for track in &self.tracks {
            sample_points.extend(track.knots.iter().map(|knot| knot.at_sample));
        }
        let stride = u64::from(self.control_stride_samples);
        let mut sample = stride;
        while sample < self.duration_samples {
            sample_points.insert(sample);
            sample = match sample.checked_add(stride) {
                Some(next) => next,
                None => break,
            };
        }

        let mut tracks: Vec<&TrajectoryTrack> = self.tracks.iter().collect();
        tracks.sort_by(|left, right| left.coordinate.cmp(&right.coordinate));
        let mut last_bits: BTreeMap<&str, u64> = BTreeMap::new();
        let mut frames = Vec::with_capacity(sample_points.len());
        for at_sample in sample_points {
            let mut sets = Vec::new();
            for track in &tracks {
                let value = track.value_at(at_sample, self.duration_samples)?;
                let bits = value.to_bits();
                let changed = last_bits
                    .get(track.coordinate.as_str())
                    .is_none_or(|previous| *previous != bits);
                if changed {
                    sets.push(CoordinateValue {
                        coordinate: track.coordinate.clone(),
                        value,
                    });
                    last_bits.insert(track.coordinate.as_str(), bits);
                }
            }
            if !sets.is_empty() {
                frames.push(TrajectoryFrame { at_sample, sets });
            }
        }

        let program_digest = sha256_prefixed(&self.semantic_bytes());
        let frame_digest = sha256_prefixed(&frame_semantic_bytes(&frames));
        let stride_seconds =
            f64::from(self.control_stride_samples) / f64::from(self.sample_rate_hz);
        let track_receipts = tracks
            .iter()
            .map(|track| {
                let measured = self.track_kinematics(track);
                let hold_error = if track.interpolation == Interpolation::Step {
                    Some(0.0)
                } else {
                    measured.max_rate.map(|rate| rate * stride_seconds)
                };
                TrackReceipt {
                    coordinate: track.coordinate.clone(),
                    interpolation: track.interpolation,
                    start_value: track.knots[0].value,
                    end_value: track.knots[track.knots.len() - 1].value,
                    max_rate_per_second: measured.max_rate,
                    max_acceleration_per_second2: measured.max_acceleration,
                    declared_metric_effort: measured.metric_effort,
                    zero_order_hold_error_bound: hold_error,
                }
            })
            .collect();

        Ok(CompiledTrajectory {
            receipt: TrajectoryReceipt {
                schema_version: TRAJECTORY_SCHEMA_VERSION.to_string(),
                compiler_version: TRAJECTORY_COMPILER_VERSION.to_string(),
                program_digest,
                frame_digest,
                sample_rate_hz: self.sample_rate_hz,
                duration_samples: self.duration_samples,
                control_stride_samples: self.control_stride_samples,
                frame_count: frames.len(),
                tracks: track_receipts,
                claim_level: "authored-control-path".to_string(),
                evidence_class: "validated-numerics".to_string(),
            },
            frames,
        })
    }
}

pub fn parse_arrow_values(source: &str) -> Result<Vec<f64>, TrajectoryError> {
    let parts: Vec<&str> = source.split("->").map(str::trim).collect();
    if parts.len() < 2 || parts.iter().any(|part| part.is_empty()) {
        return Err(TrajectoryError::new(
            TrajectoryErrorKind::InvalidArrowSyntax,
            format!("expected at least two finite values separated by '->': {source}"),
        ));
    }
    parts
        .into_iter()
        .map(|part| {
            let value = part.parse::<f64>().map_err(|_| {
                TrajectoryError::new(
                    TrajectoryErrorKind::InvalidArrowSyntax,
                    format!("'{part}' is not a floating-point value"),
                )
            })?;
            if !value.is_finite() {
                return Err(TrajectoryError::new(
                    TrajectoryErrorKind::NonFiniteValue,
                    format!("'{part}' is not finite"),
                ));
            }
            Ok(value)
        })
        .collect()
}

pub fn equally_spaced_track(
    coordinate: impl Into<String>,
    values: &[f64],
    duration_samples: u64,
    interpolation: Interpolation,
    contract: CoordinateContract,
) -> Result<TrajectoryTrack, TrajectoryError> {
    let coordinate = coordinate.into();
    if values.len() < 2 {
        return Err(TrajectoryError::for_coordinate(
            TrajectoryErrorKind::TooFewKnots,
            &coordinate,
            "an arrow trajectory needs at least two values",
        ));
    }
    if duration_samples < (values.len() - 1) as u64 {
        return Err(TrajectoryError::for_coordinate(
            TrajectoryErrorKind::DurationTooShort,
            &coordinate,
            "duration is too short to place strictly increasing knots",
        ));
    }
    let denominator = (values.len() - 1) as u128;
    let knots = values
        .iter()
        .enumerate()
        .map(|(index, value)| TrajectoryKnot {
            at_sample: ((u128::from(duration_samples) * index as u128) / denominator) as u64,
            value: *value,
        })
        .collect();
    Ok(TrajectoryTrack {
        coordinate,
        interpolation,
        contract,
        knots,
    })
}

fn interpolation_factors(interpolation: Interpolation) -> (Option<f64>, Option<f64>, Option<f64>) {
    match interpolation {
        Interpolation::Step => (None, None, None),
        Interpolation::Linear => (Some(1.0), None, Some(1.0)),
        Interpolation::Smoothstep => (Some(1.5), Some(6.0), Some(6.0 / 5.0)),
        Interpolation::Smootherstep => (
            Some(15.0 / 8.0),
            Some(10.0 * 3.0_f64.sqrt() / 3.0),
            Some(10.0 / 7.0),
        ),
        Interpolation::Cosine => (
            Some(std::f64::consts::PI / 2.0),
            Some(std::f64::consts::PI.powi(2) / 2.0),
            Some(std::f64::consts::PI.powi(2) / 8.0),
        ),
    }
}

fn frame_semantic_bytes(frames: &[TrajectoryFrame]) -> Vec<u8> {
    let mut bytes = Vec::new();
    put_string(&mut bytes, "mus-trajectory-frames-v1");
    put_len(&mut bytes, frames.len());
    for frame in frames {
        bytes.extend_from_slice(&frame.at_sample.to_be_bytes());
        let mut sets: Vec<&CoordinateValue> = frame.sets.iter().collect();
        sets.sort_by(|left, right| left.coordinate.cmp(&right.coordinate));
        put_len(&mut bytes, sets.len());
        for set in sets {
            put_string(&mut bytes, &set.coordinate);
            put_f64(&mut bytes, set.value);
        }
    }
    bytes
}

fn sha256_prefixed(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn put_len(bytes: &mut Vec<u8>, length: usize) {
    bytes.extend_from_slice(&(length as u64).to_be_bytes());
}

fn put_string(bytes: &mut Vec<u8>, value: &str) {
    put_len(bytes, value.len());
    bytes.extend_from_slice(value.as_bytes());
}

fn put_f64(bytes: &mut Vec<u8>, value: f64) {
    bytes.extend_from_slice(&value.to_bits().to_be_bytes());
}

fn put_option_f64(bytes: &mut Vec<u8>, value: Option<f64>) {
    match value {
        Some(value) => {
            bytes.push(1);
            put_f64(bytes, value);
        }
        None => bytes.push(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> TrajectoryProgram {
        TrajectoryProgram::from_json(include_str!("../fixtures/trajectory-program-v1.json"))
            .expect("fixture validates")
    }

    #[test]
    fn fixture_compiles_deterministically() {
        let program = fixture();
        let left = program.compile().expect("compile");
        let right = program.compile().expect("compile again");
        assert_eq!(left, right);
        assert!(left.frames.len() > 10);
        assert_eq!(left.frames[0].at_sample, 0);
        assert_eq!(
            left.receipt.program_digest,
            "sha256:f863212b5f40a0158719a4298d766bd4da41ccfdfb8430cbfd7654d03a7efc6d"
        );
        assert_eq!(
            left.receipt.frame_digest,
            "sha256:9604d346ac8ad35e338a3bffef75cd45ee887f57a774536bdc333207b0e23a0e"
        );
        assert_eq!(left.receipt.frame_count, 21);
    }

    #[test]
    fn semantic_digest_ignores_track_order_but_not_values() {
        let program = fixture();
        let digest = program.semantic_digest().expect("digest");
        let mut reordered = program.clone();
        reordered.tracks.reverse();
        assert_eq!(digest, reordered.semantic_digest().expect("digest"));
        reordered.tracks[0].knots[1].value += 0.001;
        assert_ne!(digest, reordered.semantic_digest().expect("digest"));
    }

    #[test]
    fn smooth_paths_report_finite_kinematics_and_effort() {
        let compiled = fixture().compile().expect("compile");
        let couple = compiled
            .receipt
            .tracks
            .iter()
            .find(|track| track.coordinate == "string.couple")
            .expect("couple receipt");
        assert!(couple.max_rate_per_second.is_some_and(|value| value > 0.0));
        assert!(couple
            .max_acceleration_per_second2
            .is_some_and(|value| value > 0.0));
        assert!(couple.declared_metric_effort > 0.0);
        assert!(couple
            .zero_order_hold_error_bound
            .is_some_and(|value| value > 0.0));
    }

    #[test]
    fn fixed_capacity_step_change_is_refused() {
        let mut program = fixture();
        program.tracks[0].interpolation = Interpolation::Step;
        let error = program.validate().expect_err("discontinuous fixed path");
        assert_eq!(error.kind, TrajectoryErrorKind::StepRequiresTopology);
    }

    #[test]
    fn arrow_values_build_exact_endpoints() {
        let values = parse_arrow_values("0.1 -> 0.5 -> -0.2").expect("arrow values");
        let track = equally_spaced_track(
            "string.chirality",
            &values,
            4_800,
            Interpolation::Smoothstep,
            CoordinateContract::fixed("ratio", -1.0, 1.0),
        )
        .expect("track");
        assert_eq!(track.knots[0].at_sample, 0);
        assert_eq!(track.knots[1].at_sample, 2_400);
        assert_eq!(track.knots[2].at_sample, 4_800);
    }
}
