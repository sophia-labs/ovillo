use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use mus_observation::{Aggregation, ChannelSpec, ObservationSchema, ScalarType, SchemaError};

use crate::ir::{MODE_COUNT, SPACE_COURSES};
use crate::plan::{PlanError, WorldPlan};

pub const WORLD_OBSERVATION_PROGRAM_VERSION: &str = "ariadne-world-observation-v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ObservationSource {
    Tick,
    WorldStoredEnergy,
    WorldSourceWork,
    WorldControlWork,
    WorldInternalLoss,
    WorldRadiation,
    WorldGlobalClosure,
    WorldMaximumLocalResidual,
    BodyMaterialEnergy {
        body_id: String,
    },
    BodyReservoirEnergy {
        body_id: String,
    },
    BodyConfiguration {
        body_id: String,
    },
    BodyEmission {
        body_id: String,
    },
    BodySourceWork {
        body_id: String,
    },
    BodyInternalLoss {
        body_id: String,
    },
    BodyPosition {
        body_id: String,
        axis: usize,
    },
    BodyDisplacementProjection {
        body_id: String,
        direction: [f64; MODE_COUNT],
    },
    BodyMomentumProjection {
        body_id: String,
        direction: [f64; MODE_COUNT],
    },
    SpaceStoredEnergy,
    SpaceEmissionLeft,
    SpaceEmissionRight,
    SpaceExchangeIntoBodies,
    SpaceCourseHead {
        course: usize,
    },
    CouplingExchangeIntoLeft {
        coupling_id: String,
    },
}

impl ObservationSource {
    #[must_use]
    pub const fn scalar_type(&self) -> ScalarType {
        match self {
            Self::Tick => ScalarType::U64,
            _ => ScalarType::F64,
        }
    }

    #[must_use]
    pub const fn is_increment(&self) -> bool {
        matches!(
            self,
            Self::WorldSourceWork
                | Self::WorldControlWork
                | Self::WorldInternalLoss
                | Self::WorldRadiation
                | Self::BodySourceWork { .. }
                | Self::BodyInternalLoss { .. }
                | Self::SpaceExchangeIntoBodies
                | Self::CouplingExchangeIntoLeft { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldObservationChannel {
    pub channel: ChannelSpec,
    pub source: ObservationSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldObservationProgram {
    pub schema_version: String,
    pub physical_world_digest: String,
    pub sample_rate_hz: u32,
    pub cadence_ticks: u64,
    pub channels: Vec<WorldObservationChannel>,
}

impl WorldObservationProgram {
    pub fn new(
        plan: &WorldPlan,
        cadence_ticks: u64,
        channels: Vec<WorldObservationChannel>,
    ) -> Result<Self, ObservationProgramError> {
        let sample_rate_hz = integer_clock(plan.internal_sample_rate_hz)?;
        let result = Self {
            schema_version: WORLD_OBSERVATION_PROGRAM_VERSION.to_owned(),
            physical_world_digest: plan.digest()?,
            sample_rate_hz,
            cadence_ticks,
            channels,
        };
        result.validate(plan)?;
        Ok(result)
    }

    pub fn validate(&self, plan: &WorldPlan) -> Result<(), ObservationProgramError> {
        plan.validate()?;
        if self.schema_version != WORLD_OBSERVATION_PROGRAM_VERSION {
            return Err(ObservationProgramError::UnsupportedVersion(
                self.schema_version.clone(),
            ));
        }
        let expected_digest = plan.digest()?;
        if self.physical_world_digest != expected_digest {
            return Err(ObservationProgramError::WorldDigestMismatch {
                expected: expected_digest,
                actual: self.physical_world_digest.clone(),
            });
        }
        let expected_rate = integer_clock(plan.internal_sample_rate_hz)?;
        if self.sample_rate_hz != expected_rate {
            return Err(ObservationProgramError::ClockMismatch {
                expected: expected_rate,
                actual: self.sample_rate_hz,
            });
        }
        if self.cadence_ticks == 0 {
            return Err(ObservationProgramError::InvalidCadence);
        }
        self.schema()?.validate()?;
        for (index, observed) in self.channels.iter().enumerate() {
            if observed.channel.scalar_type != observed.source.scalar_type() {
                return Err(ObservationProgramError::ScalarTypeMismatch { index });
            }
            if matches!(observed.channel.aggregation, Aggregation::IntervalDelta)
                && !observed.source.is_increment()
            {
                return Err(ObservationProgramError::InvalidAggregation {
                    channel: observed.channel.id.clone(),
                    reason: "interval-delta requires a per-sample increment source",
                });
            }
            if matches!(observed.source, ObservationSource::Tick)
                && !matches!(
                    observed.channel.aggregation,
                    Aggregation::Instant | Aggregation::Peak
                )
            {
                return Err(ObservationProgramError::InvalidAggregation {
                    channel: observed.channel.id.clone(),
                    reason: "tick supports instant or peak aggregation",
                });
            }
            validate_source(&observed.source, plan)?;
        }
        Ok(())
    }

    pub fn schema(&self) -> Result<ObservationSchema, ObservationProgramError> {
        let schema = ObservationSchema::new(
            self.physical_world_digest.clone(),
            self.sample_rate_hz,
            self.cadence_ticks,
            self.channels
                .iter()
                .map(|channel| channel.channel.clone())
                .collect(),
        );
        schema.validate()?;
        Ok(schema)
    }

    pub fn digest(&self) -> Result<String, ObservationProgramError> {
        let canonical = serde_json::to_vec(self)
            .map_err(|error| ObservationProgramError::Serialization(error.to_string()))?;
        Ok(format!("sha256:{:x}", Sha256::digest(canonical)))
    }
}

fn validate_source(
    source: &ObservationSource,
    plan: &WorldPlan,
) -> Result<(), ObservationProgramError> {
    let body_exists = |id: &str| plan.bodies.iter().any(|body| body.id == id);
    match source {
        ObservationSource::BodyMaterialEnergy { body_id }
        | ObservationSource::BodyReservoirEnergy { body_id }
        | ObservationSource::BodyConfiguration { body_id }
        | ObservationSource::BodyEmission { body_id }
        | ObservationSource::BodySourceWork { body_id }
        | ObservationSource::BodyInternalLoss { body_id } => {
            if !body_exists(body_id) {
                return Err(ObservationProgramError::UnknownBody(body_id.clone()));
            }
        }
        ObservationSource::BodyPosition { body_id, axis } => {
            if !body_exists(body_id) {
                return Err(ObservationProgramError::UnknownBody(body_id.clone()));
            }
            if *axis >= 3 {
                return Err(ObservationProgramError::InvalidAxis(*axis));
            }
        }
        ObservationSource::BodyDisplacementProjection { body_id, direction }
        | ObservationSource::BodyMomentumProjection { body_id, direction } => {
            if !body_exists(body_id) {
                return Err(ObservationProgramError::UnknownBody(body_id.clone()));
            }
            validate_direction(direction)?;
        }
        ObservationSource::SpaceCourseHead { course } => {
            if *course >= SPACE_COURSES {
                return Err(ObservationProgramError::InvalidSpaceCourse(*course));
            }
        }
        ObservationSource::CouplingExchangeIntoLeft { coupling_id }
            if !plan
                .direct_couplings
                .iter()
                .any(|coupling| coupling.id == *coupling_id) =>
        {
            return Err(ObservationProgramError::UnknownCoupling(
                coupling_id.clone(),
            ));
        }
        _ => {}
    }
    Ok(())
}

fn validate_direction(direction: &[f64; MODE_COUNT]) -> Result<(), ObservationProgramError> {
    let norm = direction
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    if direction.iter().any(|value| !value.is_finite()) || norm <= 1.0e-12 {
        return Err(ObservationProgramError::InvalidDirection);
    }
    Ok(())
}

fn integer_clock(sample_rate_hz: f64) -> Result<u32, ObservationProgramError> {
    let rounded = sample_rate_hz.round();
    if !sample_rate_hz.is_finite()
        || (sample_rate_hz - rounded).abs() > 1.0e-9
        || rounded <= 0.0
        || rounded > u32::MAX as f64
    {
        return Err(ObservationProgramError::UnrepresentableClock(
            sample_rate_hz,
        ));
    }
    Ok(rounded as u32)
}

#[derive(Debug, thiserror::Error)]
pub enum ObservationProgramError {
    #[error(transparent)]
    Plan(#[from] PlanError),
    #[error(transparent)]
    Schema(#[from] SchemaError),
    #[error("unsupported World observation program version {0:?}")]
    UnsupportedVersion(String),
    #[error("observation program world digest mismatch: expected {expected}, found {actual}")]
    WorldDigestMismatch { expected: String, actual: String },
    #[error("observation program clock mismatch: expected {expected}, found {actual}")]
    ClockMismatch { expected: u32, actual: u32 },
    #[error("World material clock {0} cannot be represented by the observation format")]
    UnrepresentableClock(f64),
    #[error("observation cadence must be at least one material tick")]
    InvalidCadence,
    #[error("observation channel {index} scalar type does not match its source")]
    ScalarTypeMismatch { index: usize },
    #[error("invalid aggregation for {channel:?}: {reason}")]
    InvalidAggregation {
        channel: String,
        reason: &'static str,
    },
    #[error("observation references unknown body {0:?}")]
    UnknownBody(String),
    #[error("observation references unknown coupling {0:?}")]
    UnknownCoupling(String),
    #[error("observation body-position axis {0} is outside 0..3")]
    InvalidAxis(usize),
    #[error("observation shared-space course {0} is out of range")]
    InvalidSpaceCourse(usize),
    #[error("observation projection direction is non-finite or degenerate")]
    InvalidDirection,
    #[error("observation program serialization failed: {0}")]
    Serialization(String),
}
