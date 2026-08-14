//! Canonical multi-body Ariadne physical runtime.

mod build;
mod observe;
mod process;
mod types;

pub use types::{WorldControls, WorldError, WorldFrame, WorldReceipt};

use std::collections::BTreeMap;

use mus_watsn::WorkLedger;

use crate::body::Body;
use crate::coupling::{CouplingFrame, CouplingNetwork};
use crate::plan::WorldPlan;
use crate::shared_space::{SharedSpace, SharedSpaceFrame};

/// Several persistent bodies, one shared field, one coupling graph, and one
/// global physical ledger. Observation and presentation are separate programs.
pub struct World {
    pub(super) plan: WorldPlan,
    pub(super) plan_digest: String,
    pub(super) body_indices: BTreeMap<String, usize>,
    pub(super) bodies: Vec<Body>,
    pub(super) space: SharedSpace,
    pub(super) couplings: CouplingNetwork,
    pub(super) body_emissions: Vec<f64>,
    pub(super) body_source_work: Vec<f64>,
    pub(super) body_internal_loss: Vec<f64>,
    pub(super) last_space_frame: SharedSpaceFrame,
    pub(super) last_coupling_frame: CouplingFrame,
    pub(super) ledger: WorkLedger,
    pub(super) sample_frames: u64,
    pub(super) pending_control_work: f64,
    pub(super) pending_energy_start: Option<f64>,
    pub(super) maximum_local_residual: f64,
}

#[cfg(test)]
mod tests;
