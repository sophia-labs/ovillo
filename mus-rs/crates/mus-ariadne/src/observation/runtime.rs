use mus_observation::{
    Aggregation, ObservationFrame, ObservationSchema, ObservationValue, ScalarType,
};

use crate::ir::MODE_COUNT;
use crate::plan::WorldPlan;
use crate::shared_world::{World, WorldFrame};

use super::program::{
    ObservationProgramError, ObservationSource, WorldObservationProgram,
};

#[derive(Debug, Clone)]
enum BoundSource {
    Tick,
    WorldStoredEnergy,
    WorldSourceWork,
    WorldControlWork,
    WorldInternalLoss,
    WorldRadiation,
    WorldGlobalClosure,
    WorldMaximumLocalResidual,
    BodyMaterialEnergy(usize),
    BodyReservoirEnergy(usize),
    BodyConfiguration(usize),
    BodyEmission(usize),
    BodySourceWork(usize),
    BodyInternalLoss(usize),
    BodyPosition { body: usize, axis: usize },
    BodyDisplacementProjection {
        body: usize,
        direction: [f64; MODE_COUNT],
    },
    BodyMomentumProjection {
        body: usize,
        direction: [f64; MODE_COUNT],
    },
    SpaceStoredEnergy,
    SpaceEmissionLeft,
    SpaceEmissionRight,
    SpaceExchangeIntoBodies,
    SpaceCourseHead(usize),
    CouplingExchangeIntoLeft(usize),
}

#[derive(Debug, Clone)]
enum ChannelAccumulator {
    F64 {
        aggregation: Aggregation,
        last: f64,
        sum: f64,
        sum_squares: f64,
        peak: f64,
        maximum_absolute: f64,
        count: u64,
    },
    U64 {
        aggregation: Aggregation,
        last: u64,
        peak: u64,
        count: u64,
    },
}

impl ChannelAccumulator {
    fn new(scalar_type: ScalarType, aggregation: Aggregation) -> Self {
        match scalar_type {
            ScalarType::F32 | ScalarType::F64 => Self::F64 {
                aggregation,
                last: 0.0,
                sum: 0.0,
                sum_squares: 0.0,
                peak: f64::NEG_INFINITY,
                maximum_absolute: 0.0,
                count: 0,
            },
            ScalarType::U64 => Self::U64 {
                aggregation,
                last: 0,
                peak: 0,
                count: 0,
            },
        }
    }

    fn push(&mut self, value: RawValue) -> Result<(), ObservationRuntimeError> {
        match (self, value) {
            (
                Self::F64 {
                    last,
                    sum,
                    sum_squares,
                    peak,
                    maximum_absolute,
                    count,
                    ..
                },
                RawValue::F64(value),
            ) => {
                if !value.is_finite() {
                    return Err(ObservationRuntimeError::NonFiniteSource);
                }
                *last = value;
                *sum += value;
                *sum_squares += value * value;
                *peak = peak.max(value);
                *maximum_absolute = maximum_absolute.max(value.abs());
                *count = count.saturating_add(1);
                Ok(())
            }
            (Self::U64 { last, peak, count, .. }, RawValue::U64(value)) => {
                *last = value;
                *peak = (*peak).max(value);
                *count = count.saturating_add(1);
                Ok(())
            }
            _ => Err(ObservationRuntimeError::InternalTypeMismatch),
        }
    }

    fn write_and_reset(
        &mut self,
        scalar_type: ScalarType,
    ) -> Result<ObservationValue, ObservationRuntimeError> {
        let value = match self {
            Self::F64 {
                aggregation,
                last,
                sum,
                sum_squares,
                peak,
                maximum_absolute,
                count,
            } => {
                let n = (*count).max(1) as f64;
                let value = match aggregation {
                    Aggregation::Instant => *last,
                    Aggregation::IntervalDelta => *sum,
                    Aggregation::Mean => *sum / n,
                    Aggregation::Rms => (*sum_squares / n).sqrt(),
                    Aggregation::Peak => {
                        if *count == 0 { 0.0 } else { *peak }
                    }
                    Aggregation::MaximumAbsolute => *maximum_absolute,
                };
                *last = 0.0;
                *sum = 0.0;
                *sum_squares = 0.0;
                *peak = f64::NEG_INFINITY;
                *maximum_absolute = 0.0;
                *count = 0;
                match scalar_type {
                    ScalarType::F32 => ObservationValue::F32(value as f32),
                    ScalarType::F64 => ObservationValue::F64(value),
                    ScalarType::U64 => return Err(ObservationRuntimeError::InternalTypeMismatch),
                }
            }
            Self::U64 {
                aggregation,
                last,
                peak,
                count,
            } => {
                let value = match aggregation {
                    Aggregation::Instant => *last,
                    Aggregation::Peak => *peak,
                    _ => return Err(ObservationRuntimeError::InternalTypeMismatch),
                };
                *last = 0;
                *peak = 0;
                *count = 0;
                ObservationValue::U64(value)
            }
        };
        Ok(value)
    }
}

#[derive(Debug, Clone, Copy)]
enum RawValue {
    F64(f64),
    U64(u64),
}

#[derive(Debug, Clone)]
pub struct BoundObservationProgram {
    schema: ObservationSchema,
    program_digest: String,
    sources: Vec<BoundSource>,
    accumulators: Vec<ChannelAccumulator>,
    next_capture_tick: u64,
    last_tick: u64,
}

impl BoundObservationProgram {
    pub fn compile(
        program: &WorldObservationProgram,
        plan: &WorldPlan,
    ) -> Result<Self, ObservationRuntimeError> {
        program.validate(plan)?;
        let sources = program
            .channels
            .iter()
            .map(|channel| bind_source(&channel.source, plan))
            .collect::<Result<Vec<_>, _>>()?;
        let accumulators = program
            .channels
            .iter()
            .map(|channel| {
                ChannelAccumulator::new(
                    channel.channel.scalar_type,
                    channel.channel.aggregation,
                )
            })
            .collect();
        Ok(Self {
            schema: program.schema()?,
            program_digest: program.digest()?,
            sources,
            accumulators,
            next_capture_tick: program.cadence_ticks,
            last_tick: 0,
        })
    }

    #[must_use]
    pub fn schema(&self) -> &ObservationSchema {
        &self.schema
    }

    #[must_use]
    pub fn program_digest(&self) -> &str {
        &self.program_digest
    }

    #[must_use]
    pub fn channel_count(&self) -> usize {
        self.sources.len()
    }

    pub fn reset(&mut self) {
        self.next_capture_tick = self.schema.cadence_ticks;
        self.last_tick = 0;
        for (accumulator, channel) in self.accumulators.iter_mut().zip(&self.schema.channels) {
            *accumulator = ChannelAccumulator::new(channel.scalar_type, channel.aggregation);
        }
    }

    /// Capture the zero-time state without advancing interval accumulators.
    pub fn snapshot_initial_into(
        &self,
        world: &World,
        output: &mut [ObservationValue],
    ) -> Result<(), ObservationRuntimeError> {
        if output.len() != self.sources.len() {
            return Err(ObservationRuntimeError::OutputSize {
                expected: self.sources.len(),
                actual: output.len(),
            });
        }
        let frame = WorldFrame::default();
        for (index, source) in self.sources.iter().enumerate() {
            let raw = evaluate(source, world, &frame, 0)?;
            output[index] = initial_value(raw, self.schema.channels[index].scalar_type);
        }
        Ok(())
    }

    /// Observe one completed material sample. Returns `true` after writing a
    /// due frame into `output`. No allocation occurs.
    pub fn observe_sample_into(
        &mut self,
        tick: u64,
        world: &World,
        frame: &WorldFrame,
        output: &mut [ObservationValue],
    ) -> Result<bool, ObservationRuntimeError> {
        if tick <= self.last_tick {
            return Err(ObservationRuntimeError::NonMonotoneTick {
                previous: self.last_tick,
                next: tick,
            });
        }
        if output.len() != self.sources.len() {
            return Err(ObservationRuntimeError::OutputSize {
                expected: self.sources.len(),
                actual: output.len(),
            });
        }
        for (source, accumulator) in self.sources.iter().zip(&mut self.accumulators) {
            accumulator.push(evaluate(source, world, frame, tick)?)?;
        }
        self.last_tick = tick;
        if tick < self.next_capture_tick {
            return Ok(false);
        }
        self.write_interval(output)?;
        while self.next_capture_tick <= tick {
            self.next_capture_tick = self
                .next_capture_tick
                .saturating_add(self.schema.cadence_ticks);
        }
        Ok(true)
    }

    /// Flush a partial final interval at the supplied absolute tick.
    pub fn flush_into(
        &mut self,
        tick: u64,
        output: &mut [ObservationValue],
    ) -> Result<ObservationFrame, ObservationRuntimeError> {
        if output.len() != self.sources.len() {
            return Err(ObservationRuntimeError::OutputSize {
                expected: self.sources.len(),
                actual: output.len(),
            });
        }
        self.write_interval(output)?;
        Ok(ObservationFrame::new(tick, output.to_vec()))
    }

    fn write_interval(
        &mut self,
        output: &mut [ObservationValue],
    ) -> Result<(), ObservationRuntimeError> {
        for (index, accumulator) in self.accumulators.iter_mut().enumerate() {
            output[index] = accumulator.write_and_reset(self.schema.channels[index].scalar_type)?;
        }
        Ok(())
    }
}

fn bind_source(
    source: &ObservationSource,
    plan: &WorldPlan,
) -> Result<BoundSource, ObservationRuntimeError> {
    let body = |id: &str| {
        plan.bodies
            .iter()
            .position(|body| body.id == id)
            .ok_or_else(|| ObservationRuntimeError::UnknownBody(id.to_owned()))
    };
    Ok(match source {
        ObservationSource::Tick => BoundSource::Tick,
        ObservationSource::WorldStoredEnergy => BoundSource::WorldStoredEnergy,
        ObservationSource::WorldSourceWork => BoundSource::WorldSourceWork,
        ObservationSource::WorldControlWork => BoundSource::WorldControlWork,
        ObservationSource::WorldInternalLoss => BoundSource::WorldInternalLoss,
        ObservationSource::WorldRadiation => BoundSource::WorldRadiation,
        ObservationSource::WorldGlobalClosure => BoundSource::WorldGlobalClosure,
        ObservationSource::WorldMaximumLocalResidual => BoundSource::WorldMaximumLocalResidual,
        ObservationSource::BodyMaterialEnergy { body_id } => {
            BoundSource::BodyMaterialEnergy(body(body_id)?)
        }
        ObservationSource::BodyReservoirEnergy { body_id } => {
            BoundSource::BodyReservoirEnergy(body(body_id)?)
        }
        ObservationSource::BodyConfiguration { body_id } => {
            BoundSource::BodyConfiguration(body(body_id)?)
        }
        ObservationSource::BodyEmission { body_id } => BoundSource::BodyEmission(body(body_id)?),
        ObservationSource::BodySourceWork { body_id } => {
            BoundSource::BodySourceWork(body(body_id)?)
        }
        ObservationSource::BodyInternalLoss { body_id } => {
            BoundSource::BodyInternalLoss(body(body_id)?)
        }
        ObservationSource::BodyPosition { body_id, axis } => BoundSource::BodyPosition {
            body: body(body_id)?,
            axis: *axis,
        },
        ObservationSource::BodyDisplacementProjection { body_id, direction } => {
            BoundSource::BodyDisplacementProjection {
                body: body(body_id)?,
                direction: normalize(*direction)?,
            }
        }
        ObservationSource::BodyMomentumProjection { body_id, direction } => {
            BoundSource::BodyMomentumProjection {
                body: body(body_id)?,
                direction: normalize(*direction)?,
            }
        }
        ObservationSource::SpaceStoredEnergy => BoundSource::SpaceStoredEnergy,
        ObservationSource::SpaceEmissionLeft => BoundSource::SpaceEmissionLeft,
        ObservationSource::SpaceEmissionRight => BoundSource::SpaceEmissionRight,
        ObservationSource::SpaceExchangeIntoBodies => BoundSource::SpaceExchangeIntoBodies,
        ObservationSource::SpaceCourseHead { course } => BoundSource::SpaceCourseHead(*course),
        ObservationSource::CouplingExchangeIntoLeft { coupling_id } => {
            let index = plan
                .direct_couplings
                .iter()
                .position(|coupling| coupling.id == *coupling_id)
                .ok_or_else(|| ObservationRuntimeError::UnknownCoupling(coupling_id.clone()))?;
            BoundSource::CouplingExchangeIntoLeft(index)
        }
    })
}

fn evaluate(
    source: &BoundSource,
    world: &World,
    frame: &WorldFrame,
    tick: u64,
) -> Result<RawValue, ObservationRuntimeError> {
    let f64_value = match source {
        BoundSource::Tick => return Ok(RawValue::U64(tick)),
        BoundSource::WorldStoredEnergy => world.total_stored_energy(),
        BoundSource::WorldSourceWork => frame.source_work,
        BoundSource::WorldControlWork => frame.control_work,
        BoundSource::WorldInternalLoss => frame.internal_loss,
        BoundSource::WorldRadiation => frame.radiation,
        BoundSource::WorldGlobalClosure => world.global_closure_residual(),
        BoundSource::WorldMaximumLocalResidual => world.maximum_local_residual(),
        BoundSource::BodyMaterialEnergy(body) => world
            .body_material_energy(*body)
            .ok_or(ObservationRuntimeError::BoundIndex)?,
        BoundSource::BodyReservoirEnergy(body) => world
            .body_reservoir_energy(*body)
            .ok_or(ObservationRuntimeError::BoundIndex)?,
        BoundSource::BodyConfiguration(body) => world
            .body_configuration_amount(*body)
            .ok_or(ObservationRuntimeError::BoundIndex)?,
        BoundSource::BodyEmission(body) => world
            .body_emission(*body)
            .ok_or(ObservationRuntimeError::BoundIndex)?,
        BoundSource::BodySourceWork(body) => world
            .body_last_source_work(*body)
            .ok_or(ObservationRuntimeError::BoundIndex)?,
        BoundSource::BodyInternalLoss(body) => world
            .body_last_internal_loss(*body)
            .ok_or(ObservationRuntimeError::BoundIndex)?,
        BoundSource::BodyPosition { body, axis } => world
            .body_pose(*body)
            .and_then(|pose| pose.position.get(*axis).copied())
            .ok_or(ObservationRuntimeError::BoundIndex)?,
        BoundSource::BodyDisplacementProjection { body, direction } => {
            let state = world.body_state(*body).ok_or(ObservationRuntimeError::BoundIndex)?;
            dot(&state.displacement, direction)
        }
        BoundSource::BodyMomentumProjection { body, direction } => {
            let state = world.body_state(*body).ok_or(ObservationRuntimeError::BoundIndex)?;
            dot(&state.momentum, direction)
        }
        BoundSource::SpaceStoredEnergy => world.space_stored_energy(),
        BoundSource::SpaceEmissionLeft => frame.space_emitted_left,
        BoundSource::SpaceEmissionRight => frame.space_emitted_right,
        BoundSource::SpaceExchangeIntoBodies => frame.space_exchange_into_bodies,
        BoundSource::SpaceCourseHead(course) => world
            .space_course_head(*course)
            .ok_or(ObservationRuntimeError::BoundIndex)?,
        BoundSource::CouplingExchangeIntoLeft(coupling) => world
            .coupling_last_exchange_into_left(*coupling)
            .ok_or(ObservationRuntimeError::BoundIndex)?,
    };
    if !f64_value.is_finite() {
        return Err(ObservationRuntimeError::NonFiniteSource);
    }
    Ok(RawValue::F64(f64_value))
}

fn initial_value(raw: RawValue, scalar_type: ScalarType) -> ObservationValue {
    match (raw, scalar_type) {
        (RawValue::F64(value), ScalarType::F32) => ObservationValue::F32(value as f32),
        (RawValue::F64(value), ScalarType::F64) => ObservationValue::F64(value),
        (RawValue::U64(value), ScalarType::U64) => ObservationValue::U64(value),
        _ => ObservationValue::F64(0.0),
    }
}

fn normalize(mut direction: [f64; MODE_COUNT]) -> Result<[f64; MODE_COUNT], ObservationRuntimeError> {
    let norm = direction
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    if !norm.is_finite() || norm <= 1.0e-12 {
        return Err(ObservationRuntimeError::InvalidDirection);
    }
    for value in &mut direction {
        *value /= norm;
    }
    Ok(direction)
}

fn dot<const N: usize>(left: &[f64; N], right: &[f64; N]) -> f64 {
    left.iter().zip(right).map(|(left, right)| left * right).sum()
}

#[derive(Debug, thiserror::Error)]
pub enum ObservationRuntimeError {
    #[error(transparent)]
    Program(#[from] ObservationProgramError),
    #[error("bound observation references unknown body {0:?}")]
    UnknownBody(String),
    #[error("bound observation references unknown coupling {0:?}")]
    UnknownCoupling(String),
    #[error("bound observation direction is non-finite or degenerate")]
    InvalidDirection,
    #[error("bound observation index is no longer valid for this World")]
    BoundIndex,
    #[error("observation source produced a non-finite value")]
    NonFiniteSource,
    #[error("observation scalar type and accumulator disagree")]
    InternalTypeMismatch,
    #[error("observation output has length {actual}; expected {expected}")]
    OutputSize { expected: usize, actual: usize },
    #[error("observation tick moved backward or repeated: {previous} then {next}")]
    NonMonotoneTick { previous: u64, next: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::program::WorldObservationChannel;
    use mus_observation::{
        Aggregation, ChannelSpec, ClaimDomain, Interpolation, ScalarType,
    };
    use crate::body::BodyControls;
    use crate::plan::WorldPlan;

    fn channel(id: &str, source: ObservationSource, aggregation: Aggregation) -> WorldObservationChannel {
        WorldObservationChannel {
            channel: ChannelSpec {
                id: id.to_owned(),
                unit: "normalized".to_owned(),
                scalar_type: source.scalar_type(),
                aggregation,
                interpolation: if matches!(source.scalar_type(), ScalarType::U64) {
                    Interpolation::Step
                } else {
                    Interpolation::Linear
                },
                claim_domain: ClaimDomain::Physical,
            },
            source,
        }
    }

    #[test]
    fn observer_reads_do_not_load_the_world() {
        let plan = WorldPlan::string_brass_v1(48_000.0);
        let program = WorldObservationProgram::new(
            &plan,
            48,
            vec![
                channel("tick", ObservationSource::Tick, Aggregation::Instant),
                channel(
                    "world.source-work",
                    ObservationSource::WorldSourceWork,
                    Aggregation::IntervalDelta,
                ),
                channel(
                    "body.energy",
                    ObservationSource::BodyMaterialEnergy {
                        body_id: "body/0".to_owned(),
                    },
                    Aggregation::Instant,
                ),
            ],
        )
        .unwrap();
        let mut binding = BoundObservationProgram::compile(&program, &plan).unwrap();
        let mut observed = World::from_plan(plan.clone()).unwrap();
        let mut control = World::from_plan(plan).unwrap();
        let mut body_controls = vec![BodyControls::default()];
        let world_controls = crate::shared_world::WorldControls::default();
        let mut values = vec![ObservationValue::U64(0); binding.channel_count()];
        for tick in 1..=2_048 {
            body_controls[0].source_wave = if tick == 1 { 0.7 } else { 0.0 };
            let observed_frame = observed
                .process_sample(&body_controls, world_controls)
                .unwrap();
            control
                .process_sample(&body_controls, world_controls)
                .unwrap();
            let _ = binding
                .observe_sample_into(tick, &observed, &observed_frame, &mut values)
                .unwrap();
        }
        assert_eq!(observed.body_state(0), control.body_state(0));
        assert_eq!(observed.receipt(), control.receipt());
    }
}
