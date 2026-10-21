use std::collections::{BTreeMap, BTreeSet};

use mus_ariadne::{ObservationSource, WorldObservationChannel, WorldObservationProgram, WorldPlan};
use mus_observation::{
    Aggregation, ChannelSpec, ClaimDomain, Interpolation, ObservationSchema, ScalarType,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const OPTICAL_CONSTITUTION_VERSION: &str = "ariadne-optical-constitution-v1";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpticalBounds2 {
    pub minimum: [f32; 2],
    pub maximum: [f32; 2],
}

impl OpticalBounds2 {
    #[must_use]
    pub fn from_world(plan: &WorldPlan) -> Self {
        let mut minimum = [f64::INFINITY; 2];
        let mut maximum = [f64::NEG_INFINITY; 2];
        for body in &plan.bodies {
            for axis in 0..2 {
                minimum[axis] = minimum[axis].min(body.pose.position[axis]);
                maximum[axis] = maximum[axis].max(body.pose.position[axis]);
            }
        }
        for axis in 0..2 {
            if !minimum[axis].is_finite() || !maximum[axis].is_finite() {
                minimum[axis] = -1.0;
                maximum[axis] = 1.0;
            }
            let extent = maximum[axis] - minimum[axis];
            if extent.abs() < 1.0e-9 {
                minimum[axis] -= 0.5;
                maximum[axis] += 0.5;
            } else {
                let padding = 0.15 * extent.abs();
                minimum[axis] -= padding;
                maximum[axis] += padding;
            }
        }
        Self {
            minimum: [minimum[0] as f32, minimum[1] as f32],
            maximum: [maximum[0] as f32, maximum[1] as f32],
        }
    }

    pub fn validate(self) -> Result<(), OpticalPlanError> {
        if self
            .minimum
            .iter()
            .chain(self.maximum.iter())
            .any(|value| !value.is_finite())
            || self.maximum[0] <= self.minimum[0]
            || self.maximum[1] <= self.minimum[1]
        {
            return Err(OpticalPlanError::InvalidBounds);
        }
        Ok(())
    }

    #[must_use]
    pub fn normalize(self, position: [f64; 2]) -> [f32; 2] {
        [
            ((position[0] as f32 - self.minimum[0]) / (self.maximum[0] - self.minimum[0]))
                .clamp(0.0, 1.0),
            ((position[1] as f32 - self.minimum[1]) / (self.maximum[1] - self.minimum[1]))
                .clamp(0.0, 1.0),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyOpticalSpec {
    pub body_id: String,
    pub radius: f32,
    pub color: [f32; 3],
    pub material_energy_channel: String,
    pub reservoir_energy_channel: String,
    pub emission_channel: String,
    pub source_work_channel: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CouplingOpticalSpec {
    pub coupling_id: String,
    pub radius: f32,
    pub color: [f32; 3],
    pub exchange_channel: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedFieldOpticalSpec {
    pub position: [f32; 2],
    pub radius: f32,
    pub color: [f32; 3],
    pub stored_energy_channel: String,
    pub emitted_left_channel: String,
    pub emitted_right_channel: String,
    pub exchange_channel: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpticalSurfaceSpec {
    pub width: u32,
    pub height: u32,
    pub maximum_impulses: usize,
    pub fixed_rate_hz: u32,
    pub maximum_catch_up_steps: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpticalConstitutionPlan {
    pub schema_version: String,
    pub physical_world_digest: String,
    pub observation_schema_digest: String,
    pub bounds: OpticalBounds2,
    pub surface: OpticalSurfaceSpec,
    pub bodies: Vec<BodyOpticalSpec>,
    pub shared_field: SharedFieldOpticalSpec,
    pub couplings: Vec<CouplingOpticalSpec>,
}

impl OpticalConstitutionPlan {
    pub fn canonical(
        world: &WorldPlan,
        observation: &WorldObservationProgram,
    ) -> Result<Self, OpticalPlanError> {
        observation.validate(world)?;
        let schema = observation.schema()?;
        let bounds = OpticalBounds2::from_world(world);
        let bodies = world
            .bodies
            .iter()
            .enumerate()
            .map(|(index, body)| BodyOpticalSpec {
                body_id: body.id.clone(),
                radius: 0.035 + 0.004 * (index % 4) as f32,
                color: palette(index),
                material_energy_channel: body_channel(&body.id, "material-energy"),
                reservoir_energy_channel: body_channel(&body.id, "reservoir-energy"),
                emission_channel: body_channel(&body.id, "emission-rms"),
                source_work_channel: body_channel(&body.id, "source-work"),
            })
            .collect::<Vec<_>>();
        let couplings = world
            .direct_couplings
            .iter()
            .enumerate()
            .map(|(index, coupling)| CouplingOpticalSpec {
                coupling_id: coupling.id.clone(),
                radius: 0.018,
                color: palette(index + world.bodies.len()),
                exchange_channel: coupling_channel(&coupling.id),
            })
            .collect::<Vec<_>>();
        let maximum_impulses = bodies.len() + couplings.len() + 1;
        let result = Self {
            schema_version: OPTICAL_CONSTITUTION_VERSION.to_owned(),
            physical_world_digest: world.digest()?,
            observation_schema_digest: schema.digest()?,
            bounds,
            surface: OpticalSurfaceSpec {
                width: 512,
                height: 512,
                maximum_impulses: maximum_impulses.max(1),
                fixed_rate_hz: 120,
                maximum_catch_up_steps: 32,
            },
            bodies,
            shared_field: SharedFieldOpticalSpec {
                position: [0.5, 0.5],
                radius: 0.12,
                color: [0.18, 0.42, 1.0],
                stored_energy_channel: "space/stored-energy".to_owned(),
                emitted_left_channel: "space/emitted-left-rms".to_owned(),
                emitted_right_channel: "space/emitted-right-rms".to_owned(),
                exchange_channel: "space/exchange".to_owned(),
            },
            couplings,
        };
        result.validate(world, &schema)?;
        Ok(result)
    }

    pub fn validate(
        &self,
        world: &WorldPlan,
        schema: &ObservationSchema,
    ) -> Result<(), OpticalPlanError> {
        world.validate()?;
        schema.validate()?;
        if self.schema_version != OPTICAL_CONSTITUTION_VERSION {
            return Err(OpticalPlanError::UnsupportedVersion(
                self.schema_version.clone(),
            ));
        }
        let world_digest = world.digest()?;
        if self.physical_world_digest != world_digest
            || schema.physical_world_digest != world_digest
        {
            return Err(OpticalPlanError::WorldDigestMismatch);
        }
        if self.observation_schema_digest != schema.digest()? {
            return Err(OpticalPlanError::ObservationDigestMismatch);
        }
        self.bounds.validate()?;
        if self.surface.width == 0
            || self.surface.height == 0
            || self.surface.maximum_impulses == 0
            || self.surface.fixed_rate_hz == 0
            || self.surface.maximum_catch_up_steps == 0
        {
            return Err(OpticalPlanError::InvalidSurface);
        }
        let channels = schema
            .channels
            .iter()
            .map(|channel| channel.id.as_str())
            .collect::<BTreeSet<_>>();
        let body_ids = world
            .bodies
            .iter()
            .map(|body| body.id.as_str())
            .collect::<BTreeSet<_>>();
        for body in &self.bodies {
            if !body_ids.contains(body.body_id.as_str()) {
                return Err(OpticalPlanError::UnknownBody(body.body_id.clone()));
            }
            validate_visual(body.radius, body.color)?;
            for channel in [
                &body.material_energy_channel,
                &body.reservoir_energy_channel,
                &body.emission_channel,
                &body.source_work_channel,
            ] {
                require_channel(channel, &channels)?;
            }
        }
        validate_visual(self.shared_field.radius, self.shared_field.color)?;
        for channel in [
            &self.shared_field.stored_energy_channel,
            &self.shared_field.emitted_left_channel,
            &self.shared_field.emitted_right_channel,
            &self.shared_field.exchange_channel,
        ] {
            require_channel(channel, &channels)?;
        }
        let coupling_ids = world
            .direct_couplings
            .iter()
            .map(|coupling| coupling.id.as_str())
            .collect::<BTreeSet<_>>();
        for coupling in &self.couplings {
            if !coupling_ids.contains(coupling.coupling_id.as_str()) {
                return Err(OpticalPlanError::UnknownCoupling(
                    coupling.coupling_id.clone(),
                ));
            }
            validate_visual(coupling.radius, coupling.color)?;
            require_channel(&coupling.exchange_channel, &channels)?;
        }
        if self.surface.maximum_impulses < self.bodies.len() + self.couplings.len() + 1 {
            return Err(OpticalPlanError::InsufficientImpulseCapacity);
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String, OpticalPlanError> {
        let canonical = serde_json::to_vec(self)
            .map_err(|error| OpticalPlanError::Serialization(error.to_string()))?;
        Ok(format!("sha256:{:x}", Sha256::digest(canonical)))
    }
}

/// Canonical observation program used by the default optical constitution.
pub fn canonical_optical_observation_program(
    world: &WorldPlan,
    cadence_ticks: u64,
) -> Result<WorldObservationProgram, OpticalPlanError> {
    let mut channels = vec![
        observed(
            "world/tick",
            "material-tick",
            ScalarType::U64,
            Aggregation::Instant,
            Interpolation::Step,
            ObservationSource::Tick,
        ),
        observed(
            "world/stored-energy",
            "normalized-joule",
            ScalarType::F64,
            Aggregation::Instant,
            Interpolation::Linear,
            ObservationSource::WorldStoredEnergy,
        ),
        observed(
            "world/source-work",
            "normalized-joule",
            ScalarType::F64,
            Aggregation::IntervalDelta,
            Interpolation::Linear,
            ObservationSource::WorldSourceWork,
        ),
        observed(
            "world/internal-loss",
            "normalized-joule",
            ScalarType::F64,
            Aggregation::IntervalDelta,
            Interpolation::Linear,
            ObservationSource::WorldInternalLoss,
        ),
        observed(
            "world/radiation",
            "normalized-joule",
            ScalarType::F64,
            Aggregation::IntervalDelta,
            Interpolation::Linear,
            ObservationSource::WorldRadiation,
        ),
        observed(
            "world/closure",
            "normalized-joule",
            ScalarType::F64,
            Aggregation::MaximumAbsolute,
            Interpolation::Linear,
            ObservationSource::WorldGlobalClosure,
        ),
        observed(
            "space/stored-energy",
            "normalized-joule",
            ScalarType::F32,
            Aggregation::Instant,
            Interpolation::Linear,
            ObservationSource::SpaceStoredEnergy,
        ),
        observed(
            "space/emitted-left-rms",
            "normalized-wave",
            ScalarType::F32,
            Aggregation::Rms,
            Interpolation::Linear,
            ObservationSource::SpaceEmissionLeft,
        ),
        observed(
            "space/emitted-right-rms",
            "normalized-wave",
            ScalarType::F32,
            Aggregation::Rms,
            Interpolation::Linear,
            ObservationSource::SpaceEmissionRight,
        ),
        observed(
            "space/exchange",
            "normalized-joule",
            ScalarType::F32,
            Aggregation::IntervalDelta,
            Interpolation::Linear,
            ObservationSource::SpaceExchangeIntoBodies,
        ),
    ];
    for body in &world.bodies {
        channels.extend([
            observed(
                &body_channel(&body.id, "material-energy"),
                "normalized-joule",
                ScalarType::F32,
                Aggregation::Instant,
                Interpolation::Linear,
                ObservationSource::BodyMaterialEnergy {
                    body_id: body.id.clone(),
                },
            ),
            observed(
                &body_channel(&body.id, "reservoir-energy"),
                "normalized-joule",
                ScalarType::F32,
                Aggregation::Instant,
                Interpolation::Linear,
                ObservationSource::BodyReservoirEnergy {
                    body_id: body.id.clone(),
                },
            ),
            observed(
                &body_channel(&body.id, "emission-rms"),
                "normalized-wave",
                ScalarType::F32,
                Aggregation::Rms,
                Interpolation::Linear,
                ObservationSource::BodyEmission {
                    body_id: body.id.clone(),
                },
            ),
            observed(
                &body_channel(&body.id, "source-work"),
                "normalized-joule",
                ScalarType::F32,
                Aggregation::IntervalDelta,
                Interpolation::Linear,
                ObservationSource::BodySourceWork {
                    body_id: body.id.clone(),
                },
            ),
        ]);
    }
    for coupling in &world.direct_couplings {
        channels.push(observed(
            &coupling_channel(&coupling.id),
            "normalized-joule",
            ScalarType::F32,
            Aggregation::IntervalDelta,
            Interpolation::Linear,
            ObservationSource::CouplingExchangeIntoLeft {
                coupling_id: coupling.id.clone(),
            },
        ));
    }
    Ok(WorldObservationProgram::new(
        world,
        cadence_ticks,
        channels,
    )?)
}

fn observed(
    id: &str,
    unit: &str,
    scalar_type: ScalarType,
    aggregation: Aggregation,
    interpolation: Interpolation,
    source: ObservationSource,
) -> WorldObservationChannel {
    WorldObservationChannel {
        channel: ChannelSpec {
            id: id.to_owned(),
            unit: unit.to_owned(),
            scalar_type,
            aggregation,
            interpolation,
            claim_domain: ClaimDomain::Physical,
        },
        source,
    }
}

fn body_channel(body_id: &str, suffix: &str) -> String {
    format!("body:{body_id}/{suffix}")
}

fn coupling_channel(coupling_id: &str) -> String {
    format!("coupling:{coupling_id}/exchange-into-left")
}

fn palette(index: usize) -> [f32; 3] {
    const COLORS: [[f32; 3]; 8] = [
        [0.20, 0.78, 1.00],
        [1.00, 0.36, 0.68],
        [0.38, 1.00, 0.62],
        [1.00, 0.72, 0.20],
        [0.72, 0.38, 1.00],
        [0.16, 0.92, 0.86],
        [1.00, 0.48, 0.20],
        [0.68, 0.82, 1.00],
    ];
    COLORS[index % COLORS.len()]
}

fn require_channel(channel: &str, channels: &BTreeSet<&str>) -> Result<(), OpticalPlanError> {
    if channels.contains(channel) {
        Ok(())
    } else {
        Err(OpticalPlanError::UnknownChannel(channel.to_owned()))
    }
}

fn validate_visual(radius: f32, color: [f32; 3]) -> Result<(), OpticalPlanError> {
    if !radius.is_finite()
        || radius <= 0.0
        || color.iter().any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err(OpticalPlanError::InvalidVisualSpec);
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum OpticalPlanError {
    #[error(transparent)]
    WorldPlan(#[from] mus_ariadne::PlanError),
    #[error(transparent)]
    ObservationProgram(#[from] mus_ariadne::ObservationProgramError),
    #[error(transparent)]
    ObservationSchema(#[from] mus_observation::SchemaError),
    #[error("unsupported optical constitution version {0:?}")]
    UnsupportedVersion(String),
    #[error("optical constitution and observation schema must bind the same physical world")]
    WorldDigestMismatch,
    #[error("optical constitution observation-schema digest mismatch")]
    ObservationDigestMismatch,
    #[error("optical world bounds are invalid")]
    InvalidBounds,
    #[error("optical surface dimensions, cadence, and capacities must be positive")]
    InvalidSurface,
    #[error("optical constitution references unknown body {0:?}")]
    UnknownBody(String),
    #[error("optical constitution references unknown coupling {0:?}")]
    UnknownCoupling(String),
    #[error("optical constitution references unknown channel {0:?}")]
    UnknownChannel(String),
    #[error("optical visual radius or color is invalid")]
    InvalidVisualSpec,
    #[error("optical surface does not have enough impulse capacity")]
    InsufficientImpulseCapacity,
    #[error("optical constitution serialization failed: {0}")]
    Serialization(String),
}
