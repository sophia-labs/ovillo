use mus_trajectory::TrajectoryReceipt;

use crate::body::BodyControls;
use crate::plan::WorldPlan;
use crate::shared_world::SharedWorldControls;

#[derive(Debug, Clone, PartialEq)]
pub struct SharedCoordinates {
    pub body_configurations: Vec<f64>,
    pub bodies: Vec<BodyControls>,
    pub world: SharedWorldControls,
}

impl SharedCoordinates {
    #[must_use]
    pub fn for_plan(plan: &WorldPlan) -> Self {
        Self {
            body_configurations: plan
                .bodies
                .iter()
                .map(|body| body.body.configuration_path.initial_amount)
                .collect(),
            bodies: vec![BodyControls::default(); plan.bodies.len()],
            world: SharedWorldControls::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BodyCoordinate {
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
    RadiationFraction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorldCoordinate {
    SpaceCoupling,
    SpaceDecay,
    SpaceRadiation,
    CouplingScale,
    ObservationScale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CoordinateId {
    Body {
        body: usize,
        coordinate: BodyCoordinate,
    },
    World(WorldCoordinate),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct BoundUpdate {
    pub(super) coordinate: CoordinateId,
    pub(super) value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct BoundFrame {
    pub(super) at_sample: u64,
    pub(super) updates: Vec<BoundUpdate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SharedTrajectoryBindError {
    Compile(String),
    ClockMismatch {
        program_hz: u32,
        world_hz: u32,
    },
    UnknownCoordinate(String),
    UnknownBody(String),
    CoordinateStateMismatch {
        expected_bodies: usize,
        actual_bodies: usize,
    },
    SampleMovedBackward {
        current: u64,
        requested: u64,
    },
    World(String),
}

impl std::fmt::Display for SharedTrajectoryBindError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compile(message) => {
                write!(formatter, "trajectory compile failed: {message}")
            }
            Self::ClockMismatch {
                program_hz,
                world_hz,
            } => write!(
                formatter,
                "trajectory clock {program_hz} Hz does not match Shared World clock {world_hz} Hz"
            ),
            Self::UnknownCoordinate(name) => {
                write!(formatter, "unknown Shared World coordinate: {name}")
            }
            Self::UnknownBody(id) => {
                write!(formatter, "unknown Shared World body: {id}")
            }
            Self::CoordinateStateMismatch {
                expected_bodies,
                actual_bodies,
            } => write!(
                formatter,
                "Shared World coordinates expected {expected_bodies} bodies, received {actual_bodies}"
            ),
            Self::SampleMovedBackward { current, requested } => write!(
                formatter,
                "trajectory cursor cannot move backward from {current} to {requested}"
            ),
            Self::World(message) => {
                write!(formatter, "Shared World rejected trajectory update: {message}")
            }
        }
    }
}

impl std::error::Error for SharedTrajectoryBindError {}

#[derive(Debug, Clone, PartialEq)]
pub struct SharedTrajectoryBinding {
    pub(super) body_count: usize,
    pub(super) frames: Vec<BoundFrame>,
    pub(super) cursor: usize,
    pub(super) current_sample: u64,
    pub(super) receipt: TrajectoryReceipt,
}
