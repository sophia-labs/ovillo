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
//!     -> World::compile + TrajectoryBinding
//!     -> persistent material state
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

// Preserve the current plug-in import while its source names migrate. This is
// an alias to quarantined compatibility code, not a second canonical runtime.
#[cfg(feature = "compat-terminal")]
pub mod ariadne_terminal {
    pub use crate::compat::terminal::*;
}

pub use ir::{
    ApertureSpec, ConfigurationPath, MetricSpec, RadiationSpec, ReservoirSpec,
    SpaceSpec, TransportSpec, WorldConfiguration, WorldSpec, ACTIVE_RANKS,
    MODE_COUNT, SPACE_COURSES, TRANSPORT_PAIRS,
};
pub use performance::{CostEstimate, RuntimeLayout};
pub use trajectory::{
    CompiledWorldTrajectory, TrajectoryBinding, TrajectoryBindingError,
    WorldTrajectoryFrame, WorldTrajectoryPlayer, WorldTrajectoryTick,
};
pub use world::{
    BlockError, CompileError, ReconfigurationReceipt, World, WorldControls,
    WorldFrame, WorldReceipt, WorldStateSnapshot, EXECUTION_STAGE_ORDER,
    WORLD_RUNTIME_VERSION,
};
