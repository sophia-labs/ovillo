//! Content-addressed construction plans for one multi-body Ariadne world.
//!
//! Legacy modal plans and mixed constitutive plans compile into the same
//! canonical `World` executor. Observation programs, optical constitutions,
//! sensors, and presentation bind by digest without changing physical identity.

mod constitutive;
mod helpers;
mod preset;
mod space;
mod types;
mod validate;
#[cfg(test)]
mod tests;

pub use constitutive::{
    BodyConstitutionSpec, ConstitutiveBodyInstanceSpec, ConstitutiveWorldPlan,
    LocalMediumBoundarySpec, LocalMediumPortInputBinding,
    LocalMediumPortOutputBinding, WORLD_PLAN_V3_SCHEMA_VERSION,
};
pub use space::{SpaceBodyLaws, SpaceBodySpec};
pub use types::{
    BodyInstanceSpec, BodyPose, BodySpec, DirectCouplingSpec, SharedSpaceSpec,
    SpacePortSpec, WorldPlan,
};

pub const WORLD_PLAN_SCHEMA_VERSION: &str = "ariadne-world-plan-v2";
/// Bodies one World may hold. A piano's strings, hammers and soundboard
/// need more than the guitar's nine; this is a capacity, not a law, and
/// every plan valid under the earlier 64 is valid and unchanged.
pub const MAX_BODY_INSTANCES: usize = 256;
pub const MAX_DIRECT_COUPLINGS: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanError {
    pub code: &'static str,
    pub message: String,
}

impl PlanError {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for PlanError {}
