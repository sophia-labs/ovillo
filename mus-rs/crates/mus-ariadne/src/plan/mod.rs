//! Content-addressed construction plan for one multi-body Ariadne world.

mod compat;
mod helpers;
mod types;
mod validate;
#[cfg(test)]
mod tests;

pub use types::{
    BodyInstanceSpec, BodyPose, BodySpec, DirectCouplingSpec, ObserverBodyTap,
    ObserverSpec, SharedSpaceSpec, SpacePortSpec, WorldPlan,
};

pub const WORLD_PLAN_SCHEMA_VERSION: &str = "ariadne-world-plan-v1";
pub const MAX_BODY_INSTANCES: usize = 64;
pub const MAX_DIRECT_COUPLINGS: usize = 4_096;
pub const MAX_OBSERVERS: usize = 64;
pub const MAX_OBSERVER_TAPS: usize = 4_096;

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
