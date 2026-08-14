use mus_trajectory::TrajectoryProgram;

use crate::plan::WorldPlan;
use crate::shared_world::World;

use super::types::{
    BodyCoordinate, BoundFrame, BoundUpdate, CoordinateId, TrajectoryBindError,
    TrajectoryBinding, WorldCoordinate, WorldCoordinates,
};

impl TrajectoryBinding {
    pub fn compile(
        program: &TrajectoryProgram,
        plan: &WorldPlan,
    ) -> Result<Self, TrajectoryBindError> {
        plan.validate()
            .map_err(|error| TrajectoryBindError::World(error.to_string()))?;
        let world_hz_f64 = plan.internal_sample_rate_hz.round();
        if (plan.internal_sample_rate_hz - world_hz_f64).abs() > 1.0e-9
            || world_hz_f64 > u32::MAX as f64
        {
            return Err(TrajectoryBindError::World(
                "World clock is not representable by TrajectoryProgram v1".to_owned(),
            ));
        }
        let world_hz = world_hz_f64 as u32;
        if program.sample_rate_hz != world_hz {
            return Err(TrajectoryBindError::ClockMismatch {
                program_hz: program.sample_rate_hz,
                world_hz,
            });
        }
        let compiled = program
            .compile()
            .map_err(|error| TrajectoryBindError::Compile(error.to_string()))?;
        let mut frames = Vec::with_capacity(compiled.frames.len());
        for frame in compiled.frames {
            let mut updates = Vec::with_capacity(frame.sets.len());
            for set in frame.sets {
                let coordinate = parse_coordinate(&set.coordinate, plan)?;
                updates.push(BoundUpdate {
                    coordinate,
                    value: set.value,
                });
            }
            frames.push(BoundFrame {
                at_sample: frame.at_sample,
                updates,
            });
        }
        Ok(Self {
            body_count: plan.bodies.len(),
            frames,
            cursor: 0,
            current_sample: 0,
            receipt: compiled.receipt,
        })
    }

    #[must_use]
    pub fn receipt(&self) -> &mus_trajectory::TrajectoryReceipt {
        &self.receipt
    }

    #[must_use]
    pub fn current_sample(&self) -> u64 {
        self.current_sample
    }

    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.cursor >= self.frames.len()
    }

    pub fn reset(&mut self) {
        self.cursor = 0;
        self.current_sample = 0;
    }

    /// Apply every typed update due at or before `sample`.
    pub fn advance_to(
        &mut self,
        sample: u64,
        coordinates: &mut WorldCoordinates,
        world: &mut World,
    ) -> Result<(), TrajectoryBindError> {
        if sample < self.current_sample {
            return Err(TrajectoryBindError::SampleMovedBackward {
                current: self.current_sample,
                requested: sample,
            });
        }
        if coordinates.bodies.len() != self.body_count
            || coordinates.body_configurations.len() != self.body_count
        {
            return Err(TrajectoryBindError::CoordinateStateMismatch {
                expected_bodies: self.body_count,
                actual_bodies: coordinates.bodies.len(),
            });
        }
        while self.cursor < self.frames.len() && self.frames[self.cursor].at_sample <= sample {
            let frame = &self.frames[self.cursor];
            for update in &frame.updates {
                apply_update(*update, coordinates, world)?;
            }
            self.cursor += 1;
        }
        self.current_sample = sample;
        Ok(())
    }
}

fn parse_coordinate(name: &str, plan: &WorldPlan) -> Result<CoordinateId, TrajectoryBindError> {
    if let Some(world) = parse_world_coordinate(name) {
        return Ok(CoordinateId::World(world));
    }
    let Some(rest) = name.strip_prefix("body:") else {
        return Err(TrajectoryBindError::UnknownCoordinate(name.to_owned()));
    };
    let mut best: Option<(usize, usize, &str)> = None;
    for (index, instance) in plan.bodies.iter().enumerate() {
        let prefix_len = instance.id.len();
        if rest.len() > prefix_len
            && rest.starts_with(instance.id.as_str())
            && rest.as_bytes()[prefix_len] == b'/'
            && best.is_none_or(|(_, length, _)| prefix_len > length)
        {
            best = Some((index, prefix_len, &rest[prefix_len + 1..]));
        }
    }
    let Some((body, _, suffix)) = best else {
        return Err(TrajectoryBindError::UnknownBody(rest.to_owned()));
    };
    let coordinate = parse_body_coordinate(suffix)
        .ok_or_else(|| TrajectoryBindError::UnknownCoordinate(name.to_owned()))?;
    Ok(CoordinateId::Body { body, coordinate })
}

fn parse_body_coordinate(name: &str) -> Option<BodyCoordinate> {
    Some(match name {
        "material_organization" => BodyCoordinate::MaterialOrganization,
        "source_wave" => BodyCoordinate::SourceWave,
        "work_input" => BodyCoordinate::WorkInput,
        "active.coupling" => BodyCoordinate::ActiveCoupling,
        "active.drive" => BodyCoordinate::ActiveDrive,
        "active.headroom" => BodyCoordinate::ActiveHeadroom,
        "active.sustain" => BodyCoordinate::ActiveSustain,
        "active.sag" => BodyCoordinate::ActiveSag,
        "active.bloom" => BodyCoordinate::ActiveBloom,
        "active.feedback" => BodyCoordinate::ActiveFeedback,
        "active.space" => BodyCoordinate::ActiveSpace,
        "transport.interlace" => BodyCoordinate::TransportInterlace,
        "transport.memory" => BodyCoordinate::TransportMemory,
        "radiation.fraction" => BodyCoordinate::RadiationFraction,
        _ => return None,
    })
}

fn parse_world_coordinate(name: &str) -> Option<WorldCoordinate> {
    Some(match name {
        "space.coupling" => WorldCoordinate::SpaceCoupling,
        "space.decay" => WorldCoordinate::SpaceDecay,
        "space.radiation" => WorldCoordinate::SpaceRadiation,
        "coupling.scale" => WorldCoordinate::CouplingScale,
        _ => return None,
    })
}

fn apply_update(
    update: BoundUpdate,
    coordinates: &mut WorldCoordinates,
    world: &mut World,
) -> Result<(), TrajectoryBindError> {
    let value = finite(update.value);
    match update.coordinate {
        CoordinateId::Body { body, coordinate } => {
            let controls = coordinates.bodies.get_mut(body).ok_or(
                TrajectoryBindError::CoordinateStateMismatch {
                    expected_bodies: world.body_count(),
                    actual_bodies: coordinates.bodies.len(),
                },
            )?;
            match coordinate {
                BodyCoordinate::MaterialOrganization => {
                    let amount = value.clamp(0.0, 1.0);
                    world
                        .set_body_configuration_index(body, amount)
                        .map_err(|error| TrajectoryBindError::World(error.to_string()))?;
                    coordinates.body_configurations[body] = amount;
                }
                BodyCoordinate::SourceWave => controls.source_wave = value,
                BodyCoordinate::WorkInput => controls.work_input = value.clamp(0.0, 1.0),
                BodyCoordinate::ActiveCoupling => {
                    controls.active_coupling = value.clamp(0.0, 1.0)
                }
                BodyCoordinate::ActiveDrive => controls.active.drive = value.clamp(0.0, 1.5),
                BodyCoordinate::ActiveHeadroom => {
                    controls.active.headroom = value.clamp(0.02, 1.0)
                }
                BodyCoordinate::ActiveSustain => {
                    controls.active.sustain = value.clamp(0.0, 1.0)
                }
                BodyCoordinate::ActiveSag => controls.active.sag = value.clamp(0.0, 1.0),
                BodyCoordinate::ActiveBloom => controls.active.bloom = value.clamp(0.0, 1.0),
                BodyCoordinate::ActiveFeedback => {
                    controls.active.feedback = value.clamp(0.0, 1.0)
                }
                BodyCoordinate::ActiveSpace => controls.active.space = value.clamp(0.0, 1.0),
                BodyCoordinate::TransportInterlace => {
                    controls.transport.interlace = value.clamp(0.0, 1.0)
                }
                BodyCoordinate::TransportMemory => {
                    controls.transport.memory = value.clamp(0.0, 1.0)
                }
                BodyCoordinate::RadiationFraction => {
                    controls.radiation_fraction = value.clamp(0.0, 1.0)
                }
            }
        }
        CoordinateId::World(coordinate) => match coordinate {
            WorldCoordinate::SpaceCoupling => {
                coordinates.world.space.coupling = value.clamp(0.0, 1.0)
            }
            WorldCoordinate::SpaceDecay => {
                coordinates.world.space.decay = value.clamp(0.0, 1.0)
            }
            WorldCoordinate::SpaceRadiation => {
                coordinates.world.space.radiation = value.clamp(0.0, 1.0)
            }
            WorldCoordinate::CouplingScale => {
                coordinates.world.direct_coupling.scale = value.clamp(0.0, 1.0)
            }
        },
    }
    Ok(())
}

#[inline]
fn finite(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}
