use serde::{Deserialize, Serialize};

use crate::ir::{
    ConfigurationPath, ReservoirSpec, SpaceSpec, TransportSpec, WorldSpec, ACTIVE_RANKS,
    MODE_COUNT, SPACE_COURSES,
};

use super::WORLD_PLAN_SCHEMA_VERSION;

/// Reusable local anatomy. It owns no shared field, observer, placement, or ledger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodySpec {
    pub anatomy_name: String,
    pub configuration_path: ConfigurationPath,
    pub source_direction: [f64; MODE_COUNT],
    pub active_directions: [[f64; MODE_COUNT]; ACTIVE_RANKS],
    pub transport: TransportSpec,
    pub reservoir: ReservoirSpec,
}

impl BodySpec {
    #[must_use]
    pub(crate) fn from_template(spec: &WorldSpec) -> Self {
        Self {
            anatomy_name: spec.anatomy_name.clone(),
            configuration_path: spec.configuration_path.clone(),
            source_direction: spec.source_direction,
            active_directions: spec.active_directions,
            transport: spec.transport,
            reservoir: spec.reservoir,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn from_world_spec(spec: &WorldSpec) -> Self {
        Self::from_template(spec)
    }
}

/// World-space placement metadata. Coupling remains explicit in ports and edges.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyPose {
    pub position: [f64; 3],
    /// Unit quaternion in `[x, y, z, w]` order.
    pub orientation: [f64; 4],
}

impl Default for BodyPose {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            orientation: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// Shared delayed-field anatomy, independent of any particular body aperture.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedSpaceSpec {
    pub delays_seconds: [f64; SPACE_COURSES],
    pub scattering_direction: [f64; SPACE_COURSES],
    pub radiation_left: [f64; SPACE_COURSES],
    pub radiation_right: [f64; SPACE_COURSES],
}

impl SharedSpaceSpec {
    #[must_use]
    pub(crate) fn from_space_spec(spec: SpaceSpec) -> Self {
        Self {
            delays_seconds: spec.delays_seconds,
            scattering_direction: spec.scattering_direction,
            radiation_left: spec.radiation_left,
            radiation_right: spec.radiation_right,
        }
    }
}

/// One body's finite-rank reciprocal aperture into the shared delayed field.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpacePortSpec {
    pub body_directions: [[f64; MODE_COUNT]; SPACE_COURSES],
    pub coupling_weights: [f64; SPACE_COURSES],
}

impl SpacePortSpec {
    #[must_use]
    pub(crate) fn from_space_spec(spec: SpaceSpec) -> Self {
        Self {
            body_directions: spec.body_directions,
            coupling_weights: spec.coupling_weights,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyInstanceSpec {
    pub id: String,
    pub body: BodySpec,
    pub space_port: SpacePortSpec,
    #[serde(default)]
    pub pose: BodyPose,
}

/// Storage-free reciprocal exchange between two body apertures.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectCouplingSpec {
    pub id: String,
    pub left_body: String,
    pub right_body: String,
    pub left_direction: [f64; MODE_COUNT],
    pub right_direction: [f64; MODE_COUNT],
    pub rate_hz: f64,
    pub weight: f64,
}

/// Content-addressed physical construction plan.
///
/// Observation, presentation, and GPU constitution are deliberately absent.
/// A visual or listening program binds to this plan by digest without changing
/// the identity of the physical world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldPlan {
    pub schema_version: String,
    pub world_name: String,
    pub internal_sample_rate_hz: f64,
    pub bodies: Vec<BodyInstanceSpec>,
    pub shared_space: SharedSpaceSpec,
    #[serde(default)]
    pub direct_couplings: Vec<DirectCouplingSpec>,
}

impl WorldPlan {
    pub(crate) fn skeleton(
        world_name: String,
        internal_sample_rate_hz: f64,
        bodies: Vec<BodyInstanceSpec>,
        shared_space: SharedSpaceSpec,
    ) -> Self {
        Self {
            schema_version: WORLD_PLAN_SCHEMA_VERSION.to_owned(),
            world_name,
            internal_sample_rate_hz,
            bodies,
            shared_space,
            direct_couplings: Vec::new(),
        }
    }
}
