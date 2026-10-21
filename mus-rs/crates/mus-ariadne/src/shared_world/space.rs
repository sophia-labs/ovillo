//! World-level controls and observation for N-D bodies.

use crate::configuration::DisposalPolicy;
use crate::space_body::{SpaceBody, SpaceBodyObservation, SpaceReconfigurationFrame};

use super::{World, WorldError};

impl World {
    /// Move an N-D body to `point` (clamped into its box) at fixed physical
    /// state. The move's control work, and any disposal, enter the next ledger
    /// frame exactly as a legacy reconfiguration's work does.
    pub fn set_body_configuration_point(
        &mut self,
        index: usize,
        point: &[f64],
    ) -> Result<SpaceReconfigurationFrame, WorldError> {
        let id = self
            .body_ids
            .get(index)
            .cloned()
            .ok_or_else(|| WorldError::UnknownBody(format!("index/{index}")))?;
        let energy_before = self.total_stored_energy();
        let body = self.bodies[index]
            .space_body_mut()
            .ok_or_else(|| WorldError::Body {
                id: id.clone(),
                message: "body has no N-D configuration space".to_owned(),
            })?;
        let frame = body
            .set_configuration_point(point)
            .map_err(|error| WorldError::Body {
                id,
                message: error.to_string(),
            })?;
        if self.pending_energy_start.is_none() {
            self.pending_energy_start = Some(energy_before);
        }
        self.pending_control_work += frame.control_work;
        match frame.disposal_policy {
            Some(DisposalPolicy::Radiate) => self.pending_disposal_radiation += frame.disposal,
            Some(DisposalPolicy::Dissipate) | Some(DisposalPolicy::Reject) => {
                self.pending_disposal_internal += frame.disposal
            }
            None => {}
        }
        self.disposal_total += frame.disposal;
        Ok(frame)
    }

    /// Retune an N-D body's flow valve at fixed valve state; the work is
    /// booked as control work.
    pub fn set_body_valve_tuning(
        &mut self,
        index: usize,
        tuning_hz: f64,
        adduction: f64,
    ) -> Result<f64, WorldError> {
        let id = self
            .body_ids
            .get(index)
            .cloned()
            .ok_or_else(|| WorldError::UnknownBody(format!("index/{index}")))?;
        let energy_before = self.total_stored_energy();
        let body = self.bodies[index]
            .space_body_mut()
            .ok_or_else(|| WorldError::Body {
                id: id.clone(),
                message: "body has no flow valve".to_owned(),
            })?;
        let work = body
            .retune_valve(tuning_hz, adduction)
            .map_err(|error| WorldError::Body {
                id,
                message: error.to_string(),
            })?;
        if work != 0.0 {
            if self.pending_energy_start.is_none() {
                self.pending_energy_start = Some(energy_before);
            }
            self.pending_control_work += work;
        }
        Ok(work)
    }

    #[must_use]
    pub fn body_configuration_point(&self, index: usize) -> Option<&[f64]> {
        self.bodies
            .get(index)
            .and_then(|body| body.space_body())
            .map(SpaceBody::configuration_point)
    }

    #[must_use]
    pub fn body_space_observation(&self, index: usize) -> Option<SpaceBodyObservation> {
        self.bodies
            .get(index)
            .and_then(|body| body.space_body())
            .map(SpaceBody::observation)
    }

    #[must_use]
    pub fn space_body(&self, index: usize) -> Option<&SpaceBody> {
        self.bodies.get(index).and_then(|body| body.space_body())
    }

    /// Cumulative disposal booked by N-D moves (J).
    #[must_use]
    pub fn disposal_total(&self) -> f64 {
        self.disposal_total
    }
}
