use crate::coupling::CouplingControls;
use crate::shared_space::SharedSpaceControls;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WorldControls {
    pub space: SharedSpaceControls,
    pub direct_coupling: CouplingControls,
}

impl WorldControls {
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            space: self.space.sanitized(),
            direct_coupling: self.direct_coupling.sanitized(),
        }
    }
}

/// Physical emissions and accounting from one material-clock step.
///
/// Left/right listening coordinates are deliberately absent. A listening or
/// optical observation program consumes the body and shared-field emissions
/// without loading the world.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WorldFrame {
    pub source_work: f64,
    pub control_work: f64,
    pub internal_loss: f64,
    pub radiation: f64,
    pub space_emitted_left: f64,
    pub space_emitted_right: f64,
    pub direct_exchange_into_left: f64,
    pub space_exchange_into_bodies: f64,
    pub local_residual: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldReceipt {
    pub plan_digest: String,
    pub sample_frames: u64,
    pub body_count: usize,
    pub direct_coupling_count: usize,
    pub source_work: f64,
    pub control_work: f64,
    pub internal_loss: f64,
    pub radiation: f64,
    pub initial_energy: f64,
    pub current_energy: f64,
    pub global_closure_residual: f64,
    pub cumulative_closure_residual: f64,
    pub maximum_frame_residual: f64,
    pub maximum_local_residual: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldError {
    Plan(String),
    Body { id: String, message: String },
    Coupling(String),
    Space(String),
    UnknownBody(String),
    BodyControlCount { expected: usize, actual: usize },
    NonFiniteState,
}

impl std::fmt::Display for WorldError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plan(message) => write!(formatter, "invalid Ariadne World plan: {message}"),
            Self::Body { id, message } => write!(formatter, "Ariadne body {id:?}: {message}"),
            Self::Coupling(message) => write!(formatter, "Ariadne World coupling: {message}"),
            Self::Space(message) => write!(formatter, "Ariadne shared space: {message}"),
            Self::UnknownBody(id) => write!(formatter, "unknown Ariadne body {id:?}"),
            Self::BodyControlCount { expected, actual } => write!(
                formatter,
                "Ariadne World expected {expected} body-control frames, received {actual}"
            ),
            Self::NonFiniteState => formatter.write_str("Ariadne World entered a non-finite state"),
        }
    }
}

impl std::error::Error for WorldError {}
