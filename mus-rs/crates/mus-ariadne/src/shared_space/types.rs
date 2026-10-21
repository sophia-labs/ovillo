#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SharedSpaceControls {
    pub coupling: f64,
    pub decay: f64,
    pub radiation: f64,
}

impl Default for SharedSpaceControls {
    fn default() -> Self {
        Self {
            coupling: 0.18,
            decay: 0.42,
            radiation: 0.16,
        }
    }
}

impl SharedSpaceControls {
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            coupling: finite(self.coupling).clamp(0.0, 1.0),
            decay: finite(self.decay).clamp(0.0, 1.0),
            radiation: finite(self.radiation).clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SharedSpaceExchange {
    pub signed_into_body: f64,
    pub metric_residual: f64,
    pub maximum_abs_angle: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SharedSpaceFrame {
    pub emitted_left: f64,
    pub emitted_right: f64,
    pub exchange_into_bodies: f64,
    pub internal_loss: f64,
    pub radiation: f64,
    pub coupling_metric_residual: f64,
    pub closure_residual: f64,
    pub activity_before: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SharedSpaceError {
    InvalidSampleRate,
    InvalidDelay { course: usize },
    InvalidDirection { name: &'static str, index: usize },
    FrameAlreadyOpen,
    FrameNotOpen,
    NonFiniteState,
}

impl std::fmt::Display for SharedSpaceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSampleRate => formatter.write_str("invalid shared-space sample rate"),
            Self::InvalidDelay { course } => {
                write!(formatter, "invalid delay for shared-space course {course}")
            }
            Self::InvalidDirection { name, index } => {
                write!(formatter, "invalid {name} direction at index {index}")
            }
            Self::FrameAlreadyOpen => formatter.write_str("shared-space frame is already open"),
            Self::FrameNotOpen => formatter.write_str("shared-space frame is not open"),
            Self::NonFiniteState => formatter.write_str("shared-space state became non-finite"),
        }
    }
}

impl std::error::Error for SharedSpaceError {}

#[inline]
pub(super) fn finite(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}
