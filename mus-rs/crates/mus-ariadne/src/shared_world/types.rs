use crate::coupling::CouplingControls;
use crate::shared_space::SharedSpaceControls;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SharedWorldControls {
    pub space: SharedSpaceControls,
    pub direct_coupling: CouplingControls,
    /// Presentation-only scale applied after all physical radiation.
    pub observation_scale: f64,
}

impl Default for SharedWorldControls {
    fn default() -> Self {
        Self {
            space: SharedSpaceControls::default(),
            direct_coupling: CouplingControls::default(),
            observation_scale: 1.0,
        }
    }
}

impl SharedWorldControls {
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            space: self.space.sanitized(),
            direct_coupling: self.direct_coupling.sanitized(),
            observation_scale: finite(self.observation_scale).clamp(0.0, 16.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObserverOutput {
    pub id: String,
    pub left: f64,
    pub right: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SharedFrameOutput {
    pub left: f64,
    pub right: f64,
    pub source_work: f64,
    pub control_work: f64,
    pub internal_loss: f64,
    pub radiation: f64,
    pub direct_exchange_into_left: f64,
    pub space_exchange_into_bodies: f64,
    pub local_residual: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SharedWorldReceipt {
    pub plan_digest: String,
    pub sample_frames: u64,
    pub body_count: usize,
    pub direct_coupling_count: usize,
    pub observer_count: usize,
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
pub enum SharedWorldError {
    Plan(String),
    Body { id: String, message: String },
    Coupling(String),
    Space(String),
    UnknownBody(String),
    BodyControlCount { expected: usize, actual: usize },
    NonFiniteState,
}

impl std::fmt::Display for SharedWorldError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plan(message) => {
                write!(formatter, "invalid Shared World plan: {message}")
            }
            Self::Body { id, message } => {
                write!(formatter, "Ariadne body {id:?}: {message}")
            }
            Self::Coupling(message) => {
                write!(formatter, "Shared World coupling: {message}")
            }
            Self::Space(message) => {
                write!(formatter, "Shared World space: {message}")
            }
            Self::UnknownBody(id) => {
                write!(formatter, "unknown Shared World body {id:?}")
            }
            Self::BodyControlCount { expected, actual } => write!(
                formatter,
                "Shared World expected {expected} body-control frames, received {actual}"
            ),
            Self::NonFiniteState => {
                formatter.write_str("Shared World entered a non-finite state")
            }
        }
    }
}

impl std::error::Error for SharedWorldError {}

#[inline]
fn finite(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}
