//! Typed, construction-time binding from `mus-trajectory` into Ariadne.
//!
//! Coordinate strings are resolved once while compiling. The runtime advances
//! a cursor through typed updates; it performs no hashing, parsing, allocation,
//! or string lookup on the audio thread.

use mus_trajectory::{TrajectoryError, TrajectoryProgram, TrajectoryReceipt};

use crate::world::WorldControls;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldCoordinates {
    /// Position on the world's declared configuration path.
    pub material_organization: f64,
    pub controls: WorldControls,
}

impl Default for WorldCoordinates {
    fn default() -> Self {
        Self {
            material_organization: 0.0,
            controls: WorldControls::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoordinateId {
    MaterialOrganization,
    SourceWave,
    WorkInput,
    ActiveCoupling,
    ActiveDrive,
    ActiveHeadroom,
    ActiveSustain,
    ActiveSag,
    ActiveBloom,
    ActiveFeedback,
    ActiveSpace,
    TransportInterlace,
    TransportMemory,
    SpaceCoupling,
    SpaceDecay,
    SpaceRadiation,
    RadiationFraction,
    ObservationScale,
}

impl CoordinateId {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "material_organization" => Self::MaterialOrganization,
            "source_wave" => Self::SourceWave,
            "work_input" => Self::WorkInput,
            "active.coupling" => Self::ActiveCoupling,
            "active.drive" => Self::ActiveDrive,
            "active.headroom" => Self::ActiveHeadroom,
            "active.sustain" => Self::ActiveSustain,
            "active.sag" => Self::ActiveSag,
            "active.bloom" => Self::ActiveBloom,
            "active.feedback" => Self::ActiveFeedback,
            "active.space" => Self::ActiveSpace,
            "transport.interlace" => Self::TransportInterlace,
            "transport.memory" => Self::TransportMemory,
            "space.coupling" => Self::SpaceCoupling,
            "space.decay" => Self::SpaceDecay,
            "space.radiation" => Self::SpaceRadiation,
            "radiation.fraction" => Self::RadiationFraction,
            "observation.scale" => Self::ObservationScale,
            _ => return None,
        })
    }

    fn apply(self, coordinates: &mut WorldCoordinates, value: f64) {
        let value = finite(value);
        match self {
            Self::MaterialOrganization => {
                coordinates.material_organization = value.clamp(0.0, 1.0)
            }
            Self::SourceWave => coordinates.controls.source_wave = value,
            Self::WorkInput => coordinates.controls.work_input = value.clamp(0.0, 1.0),
            Self::ActiveCoupling => {
                coordinates.controls.active_coupling = value.clamp(0.0, 1.0)
            }
            Self::ActiveDrive => coordinates.controls.active.drive = value.clamp(0.0, 1.5),
            Self::ActiveHeadroom => {
                coordinates.controls.active.headroom = value.clamp(0.02, 1.0)
            }
            Self::ActiveSustain => {
                coordinates.controls.active.sustain = value.clamp(0.0, 1.0)
            }
            Self::ActiveSag => coordinates.controls.active.sag = value.clamp(0.0, 1.0),
            Self::ActiveBloom => coordinates.controls.active.bloom = value.clamp(0.0, 1.0),
            Self::ActiveFeedback => {
                coordinates.controls.active.feedback = value.clamp(0.0, 1.0)
            }
            Self::ActiveSpace => coordinates.controls.active.space = value.clamp(0.0, 1.0),
            Self::TransportInterlace => {
                coordinates.controls.transport.interlace = value.clamp(0.0, 1.0)
            }
            Self::TransportMemory => {
                coordinates.controls.transport.memory = value.clamp(0.0, 1.0)
            }
            Self::SpaceCoupling => {
                coordinates.controls.space.coupling = value.clamp(0.0, 1.0)
            }
            Self::SpaceDecay => coordinates.controls.space.decay = value.clamp(0.0, 1.0),
            Self::SpaceRadiation => {
                coordinates.controls.space.radiation = value.clamp(0.0, 1.0)
            }
            Self::RadiationFraction => {
                coordinates.controls.radiation_fraction = value.clamp(0.0, 1.0)
            }
            Self::ObservationScale => {
                coordinates.controls.observation_scale = value.clamp(0.01, 64.0)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct BoundUpdate {
    coordinate: CoordinateId,
    value: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct BoundFrame {
    at_sample: u64,
    updates: Vec<BoundUpdate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrajectoryBindError {
    Compile(String),
    UnknownCoordinate(String),
    SampleMovedBackward { current: u64, requested: u64 },
}

impl std::fmt::Display for TrajectoryBindError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compile(message) => write!(formatter, "trajectory compile failed: {message}"),
            Self::UnknownCoordinate(name) => write!(formatter, "unknown Ariadne coordinate: {name}"),
            Self::SampleMovedBackward { current, requested } => write!(
                formatter,
                "trajectory cursor cannot move backward from {current} to {requested}"
            ),
        }
    }
}

impl std::error::Error for TrajectoryBindError {}

impl From<TrajectoryError> for TrajectoryBindError {
    fn from(error: TrajectoryError) -> Self {
        Self::Compile(error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrajectoryBinding {
    frames: Vec<BoundFrame>,
    cursor: usize,
    current_sample: u64,
    receipt: TrajectoryReceipt,
}

impl TrajectoryBinding {
    pub fn compile(program: &TrajectoryProgram) -> Result<Self, TrajectoryBindError> {
        let compiled = program.compile()?;
        let mut frames = Vec::with_capacity(compiled.frames.len());
        for frame in compiled.frames {
            let mut updates = Vec::with_capacity(frame.sets.len());
            for set in frame.sets {
                let coordinate = CoordinateId::parse(&set.coordinate)
                    .ok_or_else(|| TrajectoryBindError::UnknownCoordinate(set.coordinate.clone()))?;
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
            frames,
            cursor: 0,
            current_sample: 0,
            receipt: compiled.receipt,
        })
    }

    #[must_use]
    pub fn receipt(&self) -> &TrajectoryReceipt {
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
    ) -> Result<(), TrajectoryBindError> {
        if sample < self.current_sample {
            return Err(TrajectoryBindError::SampleMovedBackward {
                current: self.current_sample,
                requested: sample,
            });
        }
        while self.cursor < self.frames.len() && self.frames[self.cursor].at_sample <= sample {
            for update in &self.frames[self.cursor].updates {
                update.coordinate.apply(coordinates, update.value);
            }
            self.cursor += 1;
        }
        self.current_sample = sample;
        Ok(())
    }
}

#[inline]
fn finite(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mus_trajectory::{
        CoordinateContract, Interpolation, TrajectoryKnot, TrajectoryTrack,
        TRAJECTORY_SCHEMA_VERSION,
    };
    use std::collections::BTreeMap;

    #[test]
    fn strings_are_resolved_before_runtime() {
        let program = TrajectoryProgram {
            schema_version: TRAJECTORY_SCHEMA_VERSION.to_owned(),
            sample_rate_hz: 48_000,
            duration_samples: 480,
            control_stride_samples: 48,
            metadata: BTreeMap::new(),
            tracks: vec![TrajectoryTrack {
                coordinate: "material_organization".to_owned(),
                interpolation: Interpolation::Linear,
                contract: CoordinateContract::fixed("normalized", 0.0, 1.0),
                knots: vec![
                    TrajectoryKnot {
                        at_sample: 0,
                        value: 0.0,
                    },
                    TrajectoryKnot {
                        at_sample: 480,
                        value: 1.0,
                    },
                ],
            }],
        };
        let mut binding = TrajectoryBinding::compile(&program).unwrap();
        let mut coordinates = WorldCoordinates::default();
        binding.advance_to(240, &mut coordinates).unwrap();
        assert!((coordinates.material_organization - 0.5).abs() < 1.0e-12);
    }

    #[test]
    fn unknown_coordinates_are_rejected_at_compile_time() {
        let program = TrajectoryProgram {
            schema_version: TRAJECTORY_SCHEMA_VERSION.to_owned(),
            sample_rate_hz: 48_000,
            duration_samples: 480,
            control_stride_samples: 48,
            metadata: BTreeMap::new(),
            tracks: vec![TrajectoryTrack {
                coordinate: "mystery_knob".to_owned(),
                interpolation: Interpolation::Linear,
                contract: CoordinateContract::fixed("normalized", 0.0, 1.0),
                knots: vec![
                    TrajectoryKnot {
                        at_sample: 0,
                        value: 0.0,
                    },
                    TrajectoryKnot {
                        at_sample: 480,
                        value: 1.0,
                    },
                ],
            }],
        };
        assert!(matches!(
            TrajectoryBinding::compile(&program),
            Err(TrajectoryBindError::UnknownCoordinate(_))
        ));
    }
}
