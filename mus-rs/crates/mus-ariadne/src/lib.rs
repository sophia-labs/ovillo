//! Ariadne's canonical circuit-free metamaterial runtime.
//!
//! ```text
//! WorldPlan / ConstitutiveWorldPlan
//!   typed bodies + shared fields + reciprocal coupling graph
//!       |
//!       v
//! World::from_plan / World::from_constitutive_plan
//!   one persistent physical state + one material clock + one global ledger
//!       |
//!       +---- body/world controls     physical interventions
//!       +---- radiation/state getters non-loading observation boundary
//!       +---- legacy v2 bindings      trajectory/listener/typed observation
//! ```
//!
//! There is no instrument-specific renderer, compatibility terminal, embedded
//! listener, or alternate one-body execution path. A one-body instrument is a
//! one-element plan; learned local media and authored modal bodies execute in
//! the same canonical `World`. Plan-v3 trajectory and typed-observation binding
//! are explicit follow-on adapters rather than alternate execution paths.

#![forbid(unsafe_code)]

pub mod body;
pub mod configuration;
mod constitutive_body;
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
pub mod space_body;

pub use body::{Body, BodyControls, BodyError, BodyLocalFrame, BodyReconfigurationFrame};
pub use constitutive_body::{
    ConstitutiveBodyControls, ConstitutiveBodyError, LocalMediumBodyControls,
};
pub use coupling::{CouplingControls, CouplingError, CouplingFrame, CouplingNetwork};
pub use ir::{
    ApertureSpec, ConfigurationPath, MetricSpec, ReservoirSpec, TransportSpec,
    WorldConfiguration, ACTIVE_RANKS, MODE_COUNT, SPACE_COURSES,
    TRANSPORT_PAIRS,
};
pub use observation::{
    BoundObservationProgram, ListenerError, ObservationProgramError,
    ObservationRuntimeError, ObservationSource, StereoBodyTap, StereoFrame,
    StereoListener, StereoListenerSpec, StereoStateTap, WorldObservationChannel,
    WorldObservationProgram,
};
pub use performance::{WorldCostEstimate, WorldLayout};
pub use plan::{
    BodyConstitutionSpec, BodyInstanceSpec, BodyPose, BodySpec,
    ConstitutiveBodyInstanceSpec, ConstitutiveWorldPlan, DirectCouplingSpec,
    LocalMediumBoundarySpec, LocalMediumPortInputBinding,
    LocalMediumPortOutputBinding, PlanError, SharedSpaceSpec, SpacePortSpec,
    WorldPlan, WORLD_PLAN_V3_SCHEMA_VERSION,
    WORLD_PLAN_SCHEMA_VERSION,
};
pub use shared_space::{
    SharedSpaceControls, SharedSpaceError, SharedSpaceExchange, SharedSpaceFrame,
};
pub use shared_trajectory::{TrajectoryBindError, TrajectoryBinding, WorldCoordinates};
pub use shared_world::{World, WorldControls, WorldError, WorldFrame, WorldReceipt};
pub use configuration::{
    AffineSpec, AreaTermSpec, ConfigurationCoordinate, ContactSpec, ConfigurationMapSpec,
    ConfigurationSpaceSpec, ConnectionPolicy, DisposalPolicy, RadiatingEndSpec, StiffStringSpec,
    StiffWireSpec, TubeBranchSpec,
    TubeNetworkSpec, TubeSpec,
};
pub use law::aero::{AeroFrame, AeroSpec, AeroTotals};
pub use law::damper::DamperSpec;
pub use law::hammer::{HammerContactSpec, HammerFrame, HammerSpec, HammerTotals};
pub use law::valve::{FlowValveControls, FlowValveFrame, FlowValveSpec};
pub use plan::{SpaceBodyLaws, SpaceBodySpec};
pub use space_body::{
    NetworkModes, SpaceBody, SpaceBodyControls, SpaceBodyError, SpaceBodyObservation, SpaceTotals,
    SpaceReconfigurationFrame,
};
