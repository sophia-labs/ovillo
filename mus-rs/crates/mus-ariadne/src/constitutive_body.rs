//! Runtime adapters that let authored modal bodies and learned local media share
//! one Ariadne `World`, material clock, delayed field, coupling graph, and ledger.

use crate::body::{Body, BodyControls, BodyError, BodyLocalFrame, BodyReconfigurationFrame};
use crate::field::MetricFieldState;
use crate::ir::MODE_COUNT;
use crate::law::medium::{
    LocalConstitutiveMedium, LocalMediumError, LocalMediumInputFrame,
    LocalMediumSourceDrive, LocalMediumState,
};
use crate::plan::{BodyConstitutionSpec, LocalMediumBoundarySpec, PlanError};

#[derive(Debug, Clone, PartialEq)]
pub struct LocalMediumBodyControls {
    pub input: LocalMediumInputFrame,
    /// External work supplied to the finite boundary-transducer reservoir per second.
    pub boundary_work_input: f64,
    /// Participation of reservoir-paid medium-to-boundary transduction.
    pub output_scale: f64,
    pub radiation_fraction: f64,
}

impl Default for LocalMediumBodyControls {
    fn default() -> Self {
        Self {
            input: LocalMediumInputFrame::default(),
            boundary_work_input: 0.0,
            output_scale: 1.0,
            radiation_fraction: 0.004,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstitutiveBodyControls {
    Modal(BodyControls),
    LocalMedium(LocalMediumBodyControls),
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct WorldBodyLocalFrame {
    pub source_work: f64,
    pub internal_loss: f64,
    pub closure_residual: f64,
    pub local_residual: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct WorldBodyRadiationFrame {
    pub emitted: Vec<f64>,
    pub radiation: f64,
    pub residual: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstitutiveBodyError {
    Modal(String),
    LocalMedium(String),
    ControlKindMismatch,
    UnsupportedReconfiguration,
    BoundaryReservoirExhausted,
    BoundaryReservoirOverflow,
    NonFiniteState,
}

impl std::fmt::Display for ConstitutiveBodyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Modal(message) => write!(formatter, "modal body: {message}"),
            Self::LocalMedium(message) => write!(formatter, "local-medium body: {message}"),
            Self::ControlKindMismatch => {
                formatter.write_str("body controls do not match the body constitution")
            }
            Self::UnsupportedReconfiguration => formatter.write_str(
                "this body constitution does not expose the legacy scalar reconfiguration coordinate",
            ),
            Self::BoundaryReservoirExhausted => formatter.write_str(
                "local-medium boundary reservoir cannot pay the requested transduction work",
            ),
            Self::BoundaryReservoirOverflow => formatter.write_str(
                "local-medium boundary reservoir cannot accept returned transduction work",
            ),
            Self::NonFiniteState => {
                formatter.write_str("constitutive body entered a non-finite state")
            }
        }
    }
}

impl std::error::Error for ConstitutiveBodyError {}

impl From<BodyError> for ConstitutiveBodyError {
    fn from(error: BodyError) -> Self {
        Self::Modal(error.to_string())
    }
}

impl From<LocalMediumError> for ConstitutiveBodyError {
    fn from(error: LocalMediumError) -> Self {
        Self::LocalMedium(error.to_string())
    }
}

#[allow(clippy::large_enum_variant)]
pub(crate) enum WorldBody {
    Modal(Body),
    LocalMedium(LocalMediumWorldBody),
}

#[derive(Debug, Clone)]
struct CompiledInputBinding {
    source: usize,
    boundary_direction: [f64; MODE_COUNT],
    wave_gain: f64,
    evidence_gain: f64,
}

#[derive(Debug, Clone)]
struct CompiledOutputBinding {
    wave_weights: Vec<f64>,
    evidence_weights: Vec<f64>,
    reservoir_weights: Vec<f64>,
    boundary_direction: [f64; MODE_COUNT],
    gain_per_second: f64,
}

pub(crate) struct LocalMediumWorldBody {
    spec: LocalMediumBoundarySpec,
    medium: LocalConstitutiveMedium,
    dt: f64,
    boundary_momentum: [f64; MODE_COUNT],
    boundary_reservoir: f64,
    radiation_direction: [f64; MODE_COUNT],
    inputs: Vec<CompiledInputBinding>,
    outputs: Vec<CompiledOutputBinding>,
}


include!("constitutive_body/local_medium.rs");
include!("constitutive_body/world_body.rs");
include!("constitutive_body/helpers.rs");
