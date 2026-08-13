//! Ariadne's canonical circuit-free metamaterial world.
//!
//! This crate is the sole production-facing home for Ariadne anatomy,
//! persistent state, bodies, spaces, configuration, execution, radiation, and
//! receipts. It contains no host ABI, parser, RDF projection, optimizer,
//! target-audio fitter, circuit model, or post-processing effects chain.
//!
//! ```text
//! WorldSpec + TrajectoryProgram
//!     -> validate and content-address
//!     -> World::try_new + TrajectoryBinding::compile
//!     -> one persistent material-and-space state
//!     -> one frozen stage order
//!     -> radiation + WorldReceipt
//! ```
//!
//! New instruments are configurations and gestures of [`World`], not new
//! renderer structs.

#![forbid(unsafe_code)]

pub mod field;
pub mod ir;
pub mod law;
pub mod performance;
pub mod space;
pub mod trajectory;
pub mod world;

#[cfg(feature = "compat-terminal")]
pub mod compat;

/// Temporary import compatibility for the currently shipped plug-in. The code
/// is quarantined under `compat`; it is not a second canonical architecture.
#[cfg(feature = "compat-terminal")]
pub mod ariadne_terminal {
    pub use crate::compat::terminal::*;
}

pub use ir::{
    ApertureSpec, ConfigurationPath, MetricSpec, RadiationSpec, ReservoirSpec, SpaceSpec,
    TransportSpec, WorldConfiguration, WorldSpec, ACTIVE_RANKS, MODE_COUNT, SPACE_COURSES,
    TRANSPORT_PAIRS,
};
pub use performance::{CostEstimate, RuntimeLayout};
pub use space::{ReciprocalSpace, SpaceControls, SpaceFrame};
pub use trajectory::{TrajectoryBindError, TrajectoryBinding, WorldCoordinates};
pub use world::{
    FrameOutput, ReconfigurationFrame, World, WorldControls, WorldError, WorldReceipt,
};
