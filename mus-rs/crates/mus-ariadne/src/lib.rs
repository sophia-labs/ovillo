//! Ariadne's canonical circuit-free metamaterial runtime.
//!
//! The validated one-body [`World`] remains the product compatibility surface.
//! [`SharedWorld`] is the new composition architecture: several persistent
//! [`Body`] instances, one shared spatial field, sparse reciprocal couplings,
//! non-loading observers, and one global work ledger.

#![forbid(unsafe_code)]

pub mod body;
pub mod coupling;
pub mod field;
pub mod ir;
pub mod law;
pub mod performance;
pub mod plan;
pub mod shared_space;
pub mod shared_world;
pub mod space;
pub mod trajectory;
pub mod world;

#[cfg(feature = "compat-terminal")]
pub mod compat;

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
    ApertureSpec, ConfigurationPath, MetricSpec, RadiationSpec, ReservoirSpec,
    SpaceSpec, TransportSpec, WorldConfiguration, WorldSpec, ACTIVE_RANKS,
    MODE_COUNT, SPACE_COURSES, TRANSPORT_PAIRS,
};
pub use performance::{CostEstimate, RuntimeLayout};
pub use plan::{
    BodyInstanceSpec, BodyPose, BodySpec, DirectCouplingSpec,
    ObserverBodyTap, ObserverSpec, PlanError, SharedSpaceSpec, SpacePortSpec,
    WorldPlan, WORLD_PLAN_SCHEMA_VERSION,
};
pub use shared_space::{
    SharedSpaceControls, SharedSpaceError, SharedSpaceExchange,
    SharedSpaceFrame,
};
pub use shared_world::{
    ObserverOutput, SharedFrameOutput, SharedWorld, SharedWorldControls,
    SharedWorldError, SharedWorldReceipt,
};
pub use space::{ReciprocalSpace, SpaceControls, SpaceFrame};
pub use trajectory::{TrajectoryBindError, TrajectoryBinding, WorldCoordinates};
pub use world::{
    FrameOutput, ReconfigurationFrame, World, WorldControls, WorldError,
    WorldReceipt,
};
