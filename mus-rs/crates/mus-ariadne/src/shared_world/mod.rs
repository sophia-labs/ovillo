//! Canonical multi-body Ariadne composition runtime.

mod build;
mod process;
mod types;

pub use types::{
    ObserverOutput, SharedFrameOutput, SharedWorldControls, SharedWorldError,
    SharedWorldReceipt,
};

use std::collections::BTreeMap;

use mus_watsn::WorkLedger;

use crate::body::Body;
use crate::coupling::CouplingNetwork;
use crate::plan::WorldPlan;
use crate::shared_space::SharedSpace;

#[derive(Debug, Clone)]
pub(super) struct BoundObserverTap {
    pub(super) body: usize,
    pub(super) left: f64,
    pub(super) right: f64,
}

#[derive(Debug, Clone)]
pub(super) struct BoundObserver {
    pub(super) id: String,
    pub(super) body_taps: Vec<BoundObserverTap>,
    pub(super) space_left: f64,
    pub(super) space_right: f64,
    pub(super) scale: f64,
}

/// Several persistent bodies, one shared field, one coupling graph, and one ledger.
pub struct SharedWorld {
    pub(super) plan: WorldPlan,
    pub(super) plan_digest: String,
    pub(super) body_indices: BTreeMap<String, usize>,
    pub(super) bodies: Vec<Body>,
    pub(super) space: SharedSpace,
    pub(super) couplings: CouplingNetwork,
    pub(super) observers: Vec<BoundObserver>,
    pub(super) primary_observer: usize,
    pub(super) body_emissions: Vec<f64>,
    pub(super) observer_outputs: Vec<ObserverOutput>,
    pub(super) ledger: WorkLedger,
    pub(super) sample_frames: u64,
    pub(super) pending_control_work: f64,
    pub(super) pending_energy_start: Option<f64>,
    pub(super) maximum_local_residual: f64,
}

#[cfg(test)]
mod tests;
