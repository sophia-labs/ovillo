//! Ariadne's canonical circuit-free metamaterial world.
//!
//! This crate is the sole production-facing home for Ariadne anatomy,
//! persistent bodies, shared fields, coupling, configuration, execution,
//! radiation, observers, and receipts. It contains no host ABI, parser, RDF
//! projection, optimizer, target-audio fitter, circuit model, or
//! post-processing effects chain.
//!
//! ```text
//! WorldPlan + intervention/trajectory programs
//!     -> validate and content-address
//!     -> World::from_plan + typed bindings
//!     -> several persistent bodies + shared fields + coupling graph
//!     -> one frozen stage order
//!     -> radiation + non-loading observers + one WorldReceipt
//! ```
//!
//! A one-body [`WorldSpec`] remains an exact compatibility embedding. New
//! instruments are body anatomies, ports, couplings, and gestures inside one
//! [`WorldPlan`], not new renderer structs or private worlds.

#![forbid(unsafe_code)]

pub mod body;
pub mod coupling;
pub mod field;
pub mod ir;
pub mod law;
pub mod performance;
pub mod plan;
pub mod space;
pub mod trajectory;
pub mod world;

#[cfg(feature = "compat-terminal")]
pub mod compat;

/// Temporary import compatibility for historical tools. The code is
/// quarantined under `compat`; it is not a second canonical architecture.
#[cfg(feature = "compat-terminal")]
pub mod ariadne_terminal {
    pub use crate::compat::terminal::*;
}

pub use body::{
    Body, BodyControls, BodyError, BodyLocalFrame, BodyReconfigurationFrame,
};
pub use coupling::{
    CouplingControls, CouplingError, CouplingFrame, CouplingNetwork,
};
pub use ir::{
    ApertureSpec, ConfigurationPath, MetricSpec, RadiationSpec, ReservoirSpec, SpaceSpec,
    TransportSpec, WorldConfiguration, WorldSpec, ACTIVE_RANKS, MODE_COUNT, SPACE_COURSES,
    TRANSPORT_PAIRS,
};
pub use performance::{CostEstimate, RuntimeLayout};
pub use plan::{
    BodyInstanceSpec, BodyPose, BodySpec, DirectCouplingSpec, ObserverBodyTap, ObserverSpec,
    PlanError, SharedSpaceSpec, SpacePortSpec, WorldPlan, WORLD_PLAN_SCHEMA_VERSION,
};
pub use space::{
    ReciprocalSpace, SpaceControls, SpaceError, SpaceExchange, SpaceFrame, SpacePort,
};
pub use trajectory::{
    BodyCoordinates, SharedTrajectoryBinding, SharedWorldCoordinates, TrajectoryBindError,
    TrajectoryBinding, WorldCoordinates,
};
pub use world::{
    BodyReceipt, FrameOutput, ObserverOutput, ReconfigurationFrame, SharedFrameOutput,
    SharedWorldControls, SharedWorldReceipt, World, WorldControls, WorldError, WorldReceipt,
};
