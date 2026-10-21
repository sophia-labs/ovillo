//! Canonical multi-body Ariadne physical runtime.

mod build;
mod hammer;
mod observe;
mod process;
mod space;
mod types;

pub use types::{WorldControls, WorldError, WorldFrame, WorldReceipt};

use std::collections::BTreeMap;

use mus_watsn::WorkLedger;

use crate::constitutive_body::WorldBody;
use crate::coupling::{CouplingFrame, CouplingNetwork};
use crate::plan::{
    BodyPose, ConstitutiveWorldPlan, SpacePortSpec, WorldPlan,
};
use crate::shared_space::{SharedSpace, SharedSpaceFrame};

/// Several persistent bodies, one shared field, one coupling graph, and one
/// global physical ledger. Modal and local-medium constitutions compile into
/// this same executor. Observation and presentation remain separate programs.
pub struct World {
    pub(super) legacy_plan: Option<WorldPlan>,
    pub(super) constitutive_plan: Option<ConstitutiveWorldPlan>,
    pub(super) world_name: String,
    pub(super) sample_rate_hz: f64,
    pub(super) plan_digest: String,
    pub(super) body_indices: BTreeMap<String, usize>,
    pub(super) body_ids: Vec<String>,
    pub(super) body_poses: Vec<BodyPose>,
    pub(super) space_ports: Vec<SpacePortSpec>,
    pub(super) bodies: Vec<WorldBody>,
    pub(super) space: SharedSpace,
    pub(super) couplings: CouplingNetwork,
    pub(super) body_emissions: Vec<f64>,
    pub(super) body_radiation: Vec<Vec<f64>>,
    pub(super) body_source_work: Vec<f64>,
    pub(super) body_internal_loss: Vec<f64>,
    pub(super) last_space_frame: SharedSpaceFrame,
    pub(super) last_coupling_frame: CouplingFrame,
    pub(super) ledger: WorkLedger,
    pub(super) sample_frames: u64,
    pub(super) pending_control_work: f64,
    pub(super) pending_energy_start: Option<f64>,
    pub(super) maximum_local_residual: f64,
    /// Disposal from N-D moves since the last ledger frame (zero for every
    /// world without an N-D tube-network body).
    pub(super) pending_disposal_internal: f64,
    pub(super) pending_disposal_radiation: f64,
    pub(super) disposal_total: f64,
    /// Per body: an N-D body (or a hammer) whose shared-space port weights are all zero.
    pub(super) space_port_inert: Vec<bool>,
    /// Per body: its port, unit directions taken once (`exchange_compiled_port`).
    pub(super) compiled_ports: Vec<crate::shared_space::CompiledSpacePort>,
    /// Hammers and the wires each strikes (empty unless the plan declares hammers).
    pub(super) hammers: Vec<hammer::HammerBinding>,
    /// Source work and contact loss booked between frames by `strike_hammer`
    /// (zero for every world without hammers).
    pub(super) pending_source_work: f64,
    pub(super) pending_contact_loss: f64,
}

#[cfg(test)]
mod tests;
