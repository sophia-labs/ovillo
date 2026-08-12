//! Validated continuous control paths for persistent instrument bodies.
//!
//! A [`TrajectoryProgram`] is deliberately lower-level than an inverse
//! gesture request. It says *which physical coordinate moves, through which
//! values, on which sample clock*. An inverse compiler may produce one, but a
//! composer can author the same value directly. The program is independent of
//! any particular body: Weave, GARGANTA, Ariadne, and learned Extension
//! gestures bind target names at their own adapter boundary.
//!
//! The v1 contract is intentionally small:
//!
//! - fixed-capacity scalar control lanes only;
//! - explicit sample rate and duration;
//! - piecewise step, linear, C1 smoothstep, or C2 smootherstep interpolation;
//! - linear or logarithmic (`log2`) value space;
//! - typed refusal for malformed paths, range violations, and rate violations;
//! - deterministic JSON and SHA-256 receipts;
//! - no allocation while sampling a validated program.
//!
//! Topology edits, stochastic lanes, and automatic return-to-baseline are not
//! smuggled into v1. They need their own state-disposal and supply-law
//! contracts.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Stable format identifier carried inside every program.
pub const FORMAT_V1: &str = "mus-trajectory/v1";
/// JSON Schema identifier used by the language-neutral artifact.
pub const SCHEMA_V1: &str =
    "https://sophia.systems/schemas/mus-trajectory-program-v1.json";

/// Interpolation law between consecutive knots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Curve {
    /// Hold the left knot until the next knot's exact sample.
    Step,
    /// Affine interpolation. Continuous, with velocity jumps at knots.
    #[default]
    Linear,
    /// `3t^2 - 2t^3`; zero velocity at both ends of every segment.
    Smoothstep,
    /// `6t^5 - 15t^4 + 10t^3`; zero velocity and acceleration at ends.
    Smootherstep,
}

impl Curve {
    /// Parse the score/API spelling of a curve.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "step" => Self::Step,
            "linear" | "lin" => Self::Linear,
            "smoothstep" | "ease" | "ease_in_out" => Self::Smoothstep,
            "smootherstep" | "c2" => Self::Smootherstep,
            _ => return None,
        })
    }

    /// Stable spelling emitted into receipts and diagnostics.
    pub fn name(self) -> &'static str {
        match self {
            Self::Step => "step",
            Self::Linear => "linear",
            Self::Smoothstep => "smoothstep",
            Self::Smootherstep => "smootherstep",
        }
    }

    fn weight(self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Step => 0.0,
            Self::Linear => t,
            Self::Smoothstep => t * t * (3.0 - 2.0 * t),
            Self::Smootherstep => t * t * t * (t * (6.0 * t - 15.0) + 10.0),
        }
    }

    fn peak_slope_multiplier(self) -> Option<f64> {
        match self {
            Self::Step => None,
            Self::Linear => Some(1.0),
            Self::Smoothstep => Some(1.5),
            Self::Smootherstep => Some(1.875),
        }
    }

    /// Continuity class guaranteed at interior knots.
    pub fn continuity(self) -> Continuity {
        match self {
            Self::Step => Continuity::Discontinuous,
            Self::Linear => Continuity::C0,
            Self::Smoothstep => Continuity::C1,
            Self::Smootherstep => Continuity::C2,
        }
    }
}

/// Coordinate in which interpolation and rate limits are evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ValueSpace {
    /// Interpolate the authored values directly.
    #[default]
    Linear,
    /// Interpolate `log2(value)` and exponentiate. Useful for ratios.
    Log2,
}

impl ValueSpace {
    fn encode(self, target: &str, value: f64) -> Result<f64, TrajectoryError> {
        match self {
            Self::Linear => Ok(value),
            Self::Log2 if value > 0.0 => Ok(value.log2()),
            Self::Log2 => Err(TrajectoryError::NonPositiveLogValue {
                target: target.to_string(),
                value,
            }),
        }
    }

    fn decode(self, value: f64) -> f64 {
        match self {
            Self::Linear => value,
            Self::Log2 => value.exp2(),
        }
    }
}

/// Declared smoothness of a lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Continuity {
    Discontinuous,
    C0,
    C1,
    C2,
}

/// One authored value at one sample of the program clock.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Knot {
    pub at_sample: u64,
    pub value: f64,
}

/// One scalar path through one named body coordinate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lane {
    pub target: String,
    pub unit: String,
    #[serde(default)]
    pub curve: Curve,
    #[serde(default)]
    pub value_space: ValueSpace,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f64>,
    /// Bound in the lane's declared `value_space` units per second.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_rate_per_second: Option<f64>,
    pub knots: Vec<Knot>,
}

impl Lane {
    /// Evaluate this lane at one sample. The program validator guarantees
    /// that knots are non-empty, ordered, finite, and begin at sample zero.
    pub fn value_at(&self, sample: u64) -> f64 {
        let first = &self.knots[0];
        if sample <= first.at_sample {
            return first.value;
        }
        let last = self.knots.last().expect("validated non-empty lane");
        if sample >= last.at_sample {
            return last.value;
        }

        let right = self
            .knots
            .partition_point(|knot| knot.at_sample <= sample);
        let left = right - 1;
        let start = &self.knots[left];
        let end = &self.knots[right];
        let span = end.at_sample - start.at_sample;
        let t = (sample - start.at_sample) as f64 / span as f64;
        let weight = self.curve.weight(t);
        let a = match self.value_space {
            ValueSpace::Linear => start.value,
            ValueSpace::Log2 => start.value.log2(),
        };
        let b = match self.value_space {
            ValueSpace::Linear => end.value,
            ValueSpace::Log2 => end.value.log2(),
        };
        self.value_space.decode(a + (b - a) * weight)
    }

    fn rate_observation(&self, sample_rate_hz: u32) -> Result<RateObservation, TrajectoryError> {
        let mut maximum = 0.0_f64;
        let mut discontinuous = false;
        for pair in self.knots.windows(2) {
            let left = &pair[0];
            let right = &pair[1];
            let a = self.value_space.encode(&self.target, left.value)?;
            let b = self.value_space.encode(&self.target, right.value)?;
            if a.to_bits() == b.to_bits() {
                continue;
            }
            let delta_samples = right.at_sample - left.at_sample;
            let seconds = delta_samples as f64 / sample_rate_hz as f64;
            match self.curve.peak_slope_multiplier() {
                Some(multiplier) => {
                    maximum = maximum.max(multiplier * (b - a).abs() / seconds);
                }
                None => discontinuous = true,
            }
        }
        Ok(RateObservation {
            finite_maximum: (!discontinuous).then_some(maximum),
            discontinuous,
        })
    }
}

/// Complete direct-control artifact. Lanes are applied in authored order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryProgram {
    pub format: String,
    pub sample_rate_hz: u32,
    /// Number of samples in the authored path horizon. Valid knot indices are
    /// `0..duration_samples`; after the horizon the final values are held.
    pub duration_samples: u64,
    /// V1 requires true. Returning to a baseline needs an explicit baseline
    /// and work/disposal semantics, so it is not inferred here.
    pub hold_final: bool,
    #[serde(default)]
    pub lanes: Vec<Lane>,
}

impl TrajectoryProgram {
    /// Construct an empty, byte-inert program with a valid clock.
    pub fn empty(sample_rate_hz: u32, duration_samples: u64) -> Self {
        Self {
            format: FORMAT_V1.to_string(),
            sample_rate_hz,
            duration_samples,
            hold_final: true,
            lanes: Vec::new(),
        }
    }

    /// Validate every structural, range, and declared rate obligation.
    pub fn validate(&self) -> Result<TrajectorySummary, TrajectoryError> {
        if self.format != FORMAT_V1 {
            return Err(TrajectoryError::InvalidFormat {
                found: self.format.clone(),
            });
        }
        if self.sample_rate_hz == 0 {
            return Err(TrajectoryError::InvalidSampleRate);
        }
        if self.duration_samples == 0 {
            return Err(TrajectoryError::InvalidDuration);
        }
        if !self.hold_final {
            return Err(TrajectoryError::ReleasePolicyUnsupported);
        }

        let mut targets = BTreeSet::new();
        let mut lanes = Vec::with_capacity(self.lanes.len());
        let mut program_zero_motion = true;
        for (lane_index, lane) in self.lanes.iter().enumerate() {
            if lane.target.trim().is_empty() {
                return Err(TrajectoryError::EmptyTarget { lane_index });
            }
            if !targets.insert(lane.target.clone()) {
                return Err(TrajectoryError::DuplicateTarget {
                    target: lane.target.clone(),
                });
            }
            if lane.knots.is_empty() {
                return Err(TrajectoryError::EmptyLane {
                    target: lane.target.clone(),
                });
            }
            if lane.knots[0].at_sample != 0 {
                return Err(TrajectoryError::FirstKnotNotZero {
                    target: lane.target.clone(),
                    at_sample: lane.knots[0].at_sample,
                });
            }
            if let (Some(minimum), Some(maximum)) = (lane.min_value, lane.max_value) {
                if !minimum.is_finite() || !maximum.is_finite() || minimum > maximum {
                    return Err(TrajectoryError::InvalidBounds {
                        target: lane.target.clone(),
                    });
                }
            }
            if lane.min_value.is_some_and(|value| !value.is_finite())
                || lane.max_value.is_some_and(|value| !value.is_finite())
            {
                return Err(TrajectoryError::InvalidBounds {
                    target: lane.target.clone(),
                });
            }
            if let Some(limit) = lane.max_rate_per_second {
                if !limit.is_finite() || limit < 0.0 {
                    return Err(TrajectoryError::InvalidRateBound {
                        target: lane.target.clone(),
                        limit,
                    });
                }
            }

            let mut previous = None;
            let first_bits = lane.knots[0].value.to_bits();
            let mut zero_motion = true;
            let mut minimum_seen = f64::INFINITY;
            let mut maximum_seen = f64::NEG_INFINITY;
            for knot in &lane.knots {
                if let Some(previous) = previous {
                    if knot.at_sample <= previous {
                        return Err(TrajectoryError::KnotOrder {
                            target: lane.target.clone(),
                            previous,
                            current: knot.at_sample,
                        });
                    }
                }
                previous = Some(knot.at_sample);
                if knot.at_sample >= self.duration_samples {
                    return Err(TrajectoryError::KnotOutsideDuration {
                        target: lane.target.clone(),
                        at_sample: knot.at_sample,
                        duration_samples: self.duration_samples,
                    });
                }
                if !knot.value.is_finite() {
                    return Err(TrajectoryError::NonFiniteValue {
                        target: lane.target.clone(),
                        at_sample: knot.at_sample,
                    });
                }
                lane.value_space.encode(&lane.target, knot.value)?;
                if lane.min_value.is_some_and(|minimum| knot.value < minimum) {
                    return Err(TrajectoryError::BelowMinimum {
                        target: lane.target.clone(),
                        value: knot.value,
                        minimum: lane.min_value.expect("checked Some"),
                    });
                }
                if lane.max_value.is_some_and(|maximum| knot.value > maximum) {
                    return Err(TrajectoryError::AboveMaximum {
                        target: lane.target.clone(),
                        value: knot.value,
                        maximum: lane.max_value.expect("checked Some"),
                    });
                }
                minimum_seen = minimum_seen.min(knot.value);
                maximum_seen = maximum_seen.max(knot.value);
                zero_motion &= knot.value.to_bits() == first_bits;
            }

            let observed = lane.rate_observation(self.sample_rate_hz)?;
            if let Some(limit) = lane.max_rate_per_second {
                match observed.finite_maximum {
                    Some(rate) if rate <= limit => {}
                    Some(rate) => {
                        return Err(TrajectoryError::RateExceeded {
                            target: lane.target.clone(),
                            observed: Some(rate),
                            limit,
                        });
                    }
                    None => {
                        return Err(TrajectoryError::RateExceeded {
                            target: lane.target.clone(),
                            observed: None,
                            limit,
                        });
                    }
                }
            }
            program_zero_motion &= zero_motion;
            lanes.push(LaneSummary {
                target: lane.target.clone(),
                unit: lane.unit.clone(),
                curve: lane.curve,
                continuity: lane.curve.continuity(),
                value_space: lane.value_space,
                knot_count: lane.knots.len(),
                zero_motion,
                minimum: minimum_seen,
                maximum: maximum_seen,
                observed_max_rate_per_second: observed.finite_maximum,
                discontinuous: observed.discontinuous,
            });
        }

        Ok(TrajectorySummary {
            sample_rate_hz: self.sample_rate_hz,
            duration_samples: self.duration_samples,
            lane_count: self.lanes.len(),
            zero_motion: program_zero_motion,
            lanes,
        })
    }

    /// Fill a caller-owned lane buffer for one sample. A sample after the
    /// authored horizon receives the final value of every lane.
    pub fn write_values(
        &self,
        sample: u64,
        output: &mut [f64],
    ) -> Result<(), TrajectoryError> {
        if output.len() != self.lanes.len() {
            return Err(TrajectoryError::OutputLength {
                expected: self.lanes.len(),
                actual: output.len(),
            });
        }
        let sample = sample.min(self.duration_samples - 1);
        for (slot, lane) in output.iter_mut().zip(&self.lanes) {
            *slot = lane.value_at(sample);
        }
        Ok(())
    }

    /// Deterministic JSON bytes after validation.
    pub fn canonical_json(&self) -> Result<Vec<u8>, TrajectoryError> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|error| TrajectoryError::Serialization {
            message: error.to_string(),
        })
    }

    /// SHA-256 of [`Self::canonical_json`].
    pub fn digest_hex(&self) -> Result<String, TrajectoryError> {
        let digest = Sha256::digest(self.canonical_json()?);
        Ok(format!("{digest:x}"))
    }

    /// Content-addressed validation receipt.
    pub fn receipt(&self) -> Result<TrajectoryReceipt, TrajectoryError> {
        let summary = self.validate()?;
        Ok(TrajectoryReceipt {
            format: FORMAT_V1.to_string(),
            program_sha256: self.digest_hex()?,
            sample_rate_hz: summary.sample_rate_hz,
            duration_samples: summary.duration_samples,
            lane_count: summary.lane_count,
            zero_motion: summary.zero_motion,
            lanes: summary.lanes,
        })
    }
}

/// Reusable lane policy supplied by a body adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct LaneTemplate {
    pub target: String,
    pub unit: String,
    pub curve: Curve,
    pub value_space: ValueSpace,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub max_rate_per_second: Option<f64>,
}

impl LaneTemplate {
    /// Compile an arrow expression (`a->b->c`) into equally spaced knots.
    /// Returns `Ok(None)` when the value contains no arrow.
    pub fn compile_arrow(
        &self,
        raw: &str,
        duration_samples: u64,
    ) -> Result<Option<Lane>, TrajectoryError> {
        let Some(values) = parse_arrow_values(&self.target, raw)? else {
            return Ok(None);
        };
        if duration_samples < values.len() as u64 {
            return Err(TrajectoryError::TooManyKnots {
                target: self.target.clone(),
                knot_count: values.len(),
                duration_samples,
            });
        }
        let denominator = values.len() as u64 - 1;
        let horizon = duration_samples - 1;
        let knots = values
            .into_iter()
            .enumerate()
            .map(|(index, value)| Knot {
                at_sample: (index as u64 * horizon + denominator / 2) / denominator,
                value,
            })
            .collect();
        Ok(Some(Lane {
            target: self.target.clone(),
            unit: self.unit.clone(),
            curve: self.curve,
            value_space: self.value_space,
            min_value: self.min_value,
            max_value: self.max_value,
            max_rate_per_second: self.max_rate_per_second,
            knots,
        }))
    }
}

/// Parse an arrow expression. Ordinary scalar values are not trajectories.
pub fn parse_arrow_values(
    target: &str,
    raw: &str,
) -> Result<Option<Vec<f64>>, TrajectoryError> {
    if !raw.contains("->") {
        return Ok(None);
    }
    let tokens: Vec<&str> = raw.split("->").map(str::trim).collect();
    if tokens.len() < 2 || tokens.iter().any(|token| token.is_empty()) {
        return Err(TrajectoryError::InvalidArrow {
            target: target.to_string(),
            raw: raw.to_string(),
        });
    }
    let values = tokens
        .into_iter()
        .map(|token| {
            token
                .parse::<f64>()
                .map_err(|_| TrajectoryError::InvalidArrowNumber {
                    target: target.to_string(),
                    token: token.to_string(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(values))
}

#[derive(Debug, Clone, Copy)]
struct RateObservation {
    finite_maximum: Option<f64>,
    discontinuous: bool,
}

/// Receipt-ready summary of one lane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaneSummary {
    pub target: String,
    pub unit: String,
    pub curve: Curve,
    pub continuity: Continuity,
    pub value_space: ValueSpace,
    pub knot_count: usize,
    pub zero_motion: bool,
    pub minimum: f64,
    pub maximum: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_max_rate_per_second: Option<f64>,
    pub discontinuous: bool,
}

/// Validation result without a digest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectorySummary {
    pub sample_rate_hz: u32,
    pub duration_samples: u64,
    pub lane_count: usize,
    pub zero_motion: bool,
    pub lanes: Vec<LaneSummary>,
}

/// Immutable content-addressed program receipt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryReceipt {
    pub format: String,
    pub program_sha256: String,
    pub sample_rate_hz: u32,
    pub duration_samples: u64,
    pub lane_count: usize,
    pub zero_motion: bool,
    pub lanes: Vec<LaneSummary>,
}

/// Typed trajectory refusal. The stable [`Self::code`] is suitable for a
/// service or score-compiler receipt.
#[derive(Debug, Clone, PartialEq)]
pub enum TrajectoryError {
    InvalidFormat {
        found: String,
    },
    InvalidSampleRate,
    InvalidDuration,
    ReleasePolicyUnsupported,
    EmptyTarget {
        lane_index: usize,
    },
    DuplicateTarget {
        target: String,
    },
    EmptyLane {
        target: String,
    },
    FirstKnotNotZero {
        target: String,
        at_sample: u64,
    },
    KnotOrder {
        target: String,
        previous: u64,
        current: u64,
    },
    KnotOutsideDuration {
        target: String,
        at_sample: u64,
        duration_samples: u64,
    },
    NonFiniteValue {
        target: String,
        at_sample: u64,
    },
    InvalidBounds {
        target: String,
    },
    BelowMinimum {
        target: String,
        value: f64,
        minimum: f64,
    },
    AboveMaximum {
        target: String,
        value: f64,
        maximum: f64,
    },
    NonPositiveLogValue {
        target: String,
        value: f64,
    },
    InvalidRateBound {
        target: String,
        limit: f64,
    },
    RateExceeded {
        target: String,
        observed: Option<f64>,
        limit: f64,
    },
    InvalidArrow {
        target: String,
        raw: String,
    },
    InvalidArrowNumber {
        target: String,
        token: String,
    },
    TooManyKnots {
        target: String,
        knot_count: usize,
        duration_samples: u64,
    },
    OutputLength {
        expected: usize,
        actual: usize,
    },
    Serialization {
        message: String,
    },
}

impl TrajectoryError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidFormat { .. } => "trajectory-invalid-format",
            Self::InvalidSampleRate => "trajectory-invalid-sample-rate",
            Self::InvalidDuration => "trajectory-invalid-duration",
            Self::ReleasePolicyUnsupported => "trajectory-release-policy-unsupported",
            Self::EmptyTarget { .. } => "trajectory-empty-target",
            Self::DuplicateTarget { .. } => "trajectory-duplicate-target",
            Self::EmptyLane { .. } => "trajectory-empty-lane",
            Self::FirstKnotNotZero { .. } => "trajectory-first-knot-not-zero",
            Self::KnotOrder { .. } => "trajectory-knot-order",
            Self::KnotOutsideDuration { .. } => "trajectory-knot-outside-duration",
            Self::NonFiniteValue { .. } => "trajectory-nonfinite-value",
            Self::InvalidBounds { .. } => "trajectory-invalid-bounds",
            Self::BelowMinimum { .. } => "trajectory-below-minimum",
            Self::AboveMaximum { .. } => "trajectory-above-maximum",
            Self::NonPositiveLogValue { .. } => "trajectory-log-value-nonpositive",
            Self::InvalidRateBound { .. } => "trajectory-invalid-rate-bound",
            Self::RateExceeded { .. } => "trajectory-rate-exceeded",
            Self::InvalidArrow { .. } => "trajectory-invalid-arrow",
            Self::InvalidArrowNumber { .. } => "trajectory-invalid-arrow-number",
            Self::TooManyKnots { .. } => "trajectory-too-many-knots",
            Self::OutputLength { .. } => "trajectory-output-length",
            Self::Serialization { .. } => "trajectory-serialization",
        }
    }
}

impl fmt::Display for TrajectoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat { found } => {
                write!(formatter, "trajectory format {found:?} is not {FORMAT_V1:?}")
            }
            Self::InvalidSampleRate => write!(formatter, "trajectory sample rate must be positive"),
            Self::InvalidDuration => write!(formatter, "trajectory duration must be positive"),
            Self::ReleasePolicyUnsupported => write!(
                formatter,
                "trajectory v1 only supports hold_final=true; baseline return needs an explicit law"
            ),
            Self::EmptyTarget { lane_index } => {
                write!(formatter, "trajectory lane {lane_index} has an empty target")
            }
            Self::DuplicateTarget { target } => {
                write!(formatter, "trajectory target {target:?} appears more than once")
            }
            Self::EmptyLane { target } => {
                write!(formatter, "trajectory target {target:?} has no knots")
            }
            Self::FirstKnotNotZero { target, at_sample } => write!(
                formatter,
                "trajectory target {target:?} begins at sample {at_sample}, not sample 0"
            ),
            Self::KnotOrder {
                target,
                previous,
                current,
            } => write!(
                formatter,
                "trajectory target {target:?} has non-increasing knots {previous} then {current}"
            ),
            Self::KnotOutsideDuration {
                target,
                at_sample,
                duration_samples,
            } => write!(
                formatter,
                "trajectory target {target:?} knot {at_sample} is outside duration {duration_samples}"
            ),
            Self::NonFiniteValue { target, at_sample } => write!(
                formatter,
                "trajectory target {target:?} has a non-finite value at sample {at_sample}"
            ),
            Self::InvalidBounds { target } => {
                write!(formatter, "trajectory target {target:?} has invalid bounds")
            }
            Self::BelowMinimum {
                target,
                value,
                minimum,
            } => write!(
                formatter,
                "trajectory target {target:?} value {value} is below minimum {minimum}"
            ),
            Self::AboveMaximum {
                target,
                value,
                maximum,
            } => write!(
                formatter,
                "trajectory target {target:?} value {value} is above maximum {maximum}"
            ),
            Self::NonPositiveLogValue { target, value } => write!(
                formatter,
                "trajectory target {target:?} uses log2 space but value {value} is not positive"
            ),
            Self::InvalidRateBound { target, limit } => write!(
                formatter,
                "trajectory target {target:?} has invalid rate bound {limit}"
            ),
            Self::RateExceeded {
                target,
                observed,
                limit,
            } => match observed {
                Some(observed) => write!(
                    formatter,
                    "trajectory target {target:?} rate {observed} exceeds {limit} per second"
                ),
                None => write!(
                    formatter,
                    "trajectory target {target:?} is discontinuous and cannot satisfy finite rate {limit}"
                ),
            },
            Self::InvalidArrow { target, raw } => write!(
                formatter,
                "trajectory target {target:?} has malformed arrow expression {raw:?}"
            ),
            Self::InvalidArrowNumber { target, token } => write!(
                formatter,
                "trajectory target {target:?} arrow token {token:?} is not a number"
            ),
            Self::TooManyKnots {
                target,
                knot_count,
                duration_samples,
            } => write!(
                formatter,
                "trajectory target {target:?} has {knot_count} knots but only {duration_samples} samples"
            ),
            Self::OutputLength { expected, actual } => write!(
                formatter,
                "trajectory output buffer has length {actual}; expected {expected}"
            ),
            Self::Serialization { message } => {
                write!(formatter, "trajectory serialization failed: {message}")
            }
        }
    }
}

impl std::error::Error for TrajectoryError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn lane(curve: Curve, value_space: ValueSpace, values: &[(u64, f64)]) -> Lane {
        Lane {
            target: "x".to_string(),
            unit: "unitless".to_string(),
            curve,
            value_space,
            min_value: None,
            max_value: None,
            max_rate_per_second: None,
            knots: values
                .iter()
                .map(|&(at_sample, value)| Knot { at_sample, value })
                .collect(),
        }
    }

    fn program(lane: Lane) -> TrajectoryProgram {
        TrajectoryProgram {
            format: FORMAT_V1.to_string(),
            sample_rate_hz: 10,
            duration_samples: 11,
            hold_final: true,
            lanes: vec![lane],
        }
    }

    #[test]
    fn linear_endpoints_and_midpoint_are_exact() {
        let program = program(lane(Curve::Linear, ValueSpace::Linear, &[(0, 2.0), (10, 6.0)]));
        program.validate().unwrap();
        assert_eq!(program.lanes[0].value_at(0), 2.0);
        assert_eq!(program.lanes[0].value_at(5), 4.0);
        assert_eq!(program.lanes[0].value_at(10), 6.0);
        assert_eq!(program.lanes[0].value_at(1_000), 6.0);
    }

    #[test]
    fn log2_midpoint_is_the_geometric_mean() {
        let program = program(lane(Curve::Linear, ValueSpace::Log2, &[(0, 1.0), (10, 4.0)]));
        program.validate().unwrap();
        assert!((program.lanes[0].value_at(5) - 2.0).abs() < 1e-14);
    }

    #[test]
    fn smoothness_and_rate_receipts_are_derived() {
        let mut lane = lane(Curve::Smoothstep, ValueSpace::Linear, &[(0, 0.0), (10, 1.0)]);
        lane.max_rate_per_second = Some(1.51);
        let summary = program(lane).validate().unwrap();
        assert_eq!(summary.lanes[0].continuity, Continuity::C1);
        assert!((summary.lanes[0].observed_max_rate_per_second.unwrap() - 1.5).abs() < 1e-14);
    }

    #[test]
    fn discontinuous_lane_refuses_a_finite_rate_claim() {
        let mut lane = lane(Curve::Step, ValueSpace::Linear, &[(0, 0.0), (10, 1.0)]);
        lane.max_rate_per_second = Some(10_000.0);
        let error = program(lane).validate().unwrap_err();
        assert_eq!(error.code(), "trajectory-rate-exceeded");
    }

    #[test]
    fn arrow_compilation_places_knots_on_the_full_horizon() {
        let template = LaneTemplate {
            target: "body".to_string(),
            unit: "unitless".to_string(),
            curve: Curve::Smootherstep,
            value_space: ValueSpace::Linear,
            min_value: Some(0.0),
            max_value: Some(1.0),
            max_rate_per_second: None,
        };
        let lane = template
            .compile_arrow("0.2 -> 0.8 -> 0.3", 101)
            .unwrap()
            .unwrap();
        assert_eq!(lane.knots[0].at_sample, 0);
        assert_eq!(lane.knots[1].at_sample, 50);
        assert_eq!(lane.knots[2].at_sample, 100);
    }

    #[test]
    fn receipt_digest_survives_json_roundtrip() {
        let program = program(lane(Curve::Smootherstep, ValueSpace::Linear, &[(0, -1.0), (10, 1.0)]));
        let before = program.digest_hex().unwrap();
        let encoded = serde_json::to_vec(&program).unwrap();
        let decoded: TrajectoryProgram = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded.digest_hex().unwrap(), before);
        assert_eq!(decoded.receipt().unwrap().program_sha256, before);
    }

    #[test]
    fn empty_program_is_a_valid_zero_motion_program() {
        let program = TrajectoryProgram::empty(48_000, 64);
        let summary = program.validate().unwrap();
        assert_eq!(summary.lane_count, 0);
        assert!(summary.zero_motion);
        let mut output = [];
        program.write_values(19, &mut output).unwrap();
    }
}
