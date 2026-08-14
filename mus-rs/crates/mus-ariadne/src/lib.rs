//! Ariadne's canonical circuit-free metamaterial runtime.
//!
//! ```text
//! WorldPlan
//!   bodies + shared fields + reciprocal coupling graph
//!       |
//!       v
//! World::from_plan
//!   one persistent physical state + one material clock + one global ledger
//!       |
//!       +---- TrajectoryBinding       physical interventions
//!       +---- StereoListener          non-loading sound observation
//!       +---- BoundObservationProgram non-loading typed data observation
//! ```
//!
//! There is no instrument-specific renderer, compatibility terminal, embedded
//! listener, or alternate one-body execution path. A one-body instrument is a
//! one-element `WorldPlan`; a score is a larger plan over the same executor.

#![forbid(unsafe_code)]

pub mod body;
pub mod coupling;
pub mod field;
mod ir;
pub mod law;
pub mod observation;
pub mod performance;
pub mod plan;
mod shared_space;
mod shared_trajectory;
mod shared_world;

pub use body::{Body, BodyControls, BodyError, BodyLocalFrame, BodyReconfigurationFrame};
pub use coupling::{CouplingControls, CouplingError, CouplingFrame, CouplingNetwork};
pub use ir::{
    ApertureSpec, ConfigurationPath, MetricSpec, ReservoirSpec, TransportSpec,
    WorldConfiguration, ACTIVE_RANKS, MODE_COUNT, SPACE_COURSES,
    TRANSPORT_PAIRS,
};
pub use law::medium::{
    LocalConstitutiveMedium, LocalMediumDamageReceipt, LocalMediumEdgeSpec,
    LocalMediumError, LocalMediumInputFrame, LocalMediumObserverSpec,
    LocalMediumPlan, LocalMediumReconfigurationReceipt, LocalMediumSiteSpec,
    LocalMediumSourceDrive, LocalMediumSourceSpec, LocalMediumState,
    LocalMediumStepReceipt, LOCAL_MEDIUM_SCHEMA_VERSION,
};
pub use observation::{
    BoundObservationProgram, ListenerError, ObservationProgramError,
    ObservationRuntimeError, ObservationSource, StereoBodyTap, StereoFrame,
    StereoListener, StereoListenerSpec, WorldObservationChannel,
    WorldObservationProgram,
};
pub use performance::{WorldCostEstimate, WorldLayout};
pub use plan::{
    BodyInstanceSpec, BodyPose, BodySpec, DirectCouplingSpec, PlanError,
    SharedSpaceSpec, SpacePortSpec, WorldPlan, WORLD_PLAN_SCHEMA_VERSION,
};
pub use shared_space::{
    SharedSpaceControls, SharedSpaceError, SharedSpaceExchange, SharedSpaceFrame,
};
pub use shared_trajectory::{TrajectoryBindError, TrajectoryBinding, WorldCoordinates};
pub use shared_world::{World, WorldControls, WorldError, WorldFrame, WorldReceipt};
