use mus_watsn::WorkFrame;

use crate::body::BodyControls;
use crate::constitutive_body::ConstitutiveBodyControls;

use super::{World, WorldControls, WorldError, WorldFrame, WorldReceipt};

impl World {
    /// Legacy modal-body entrypoint. It remains allocation-free and executes the
    /// same coupling, shared-space, radiation, and global-ledger path as mixed
    /// constitutive worlds.
    pub fn process_sample(
        &mut self,
        body_controls: &[BodyControls],
        controls: WorldControls,
    ) -> Result<WorldFrame, WorldError> {
        if body_controls.len() != self.bodies.len() {
            return Err(WorldError::BodyControlCount {
                expected: self.bodies.len(),
                actual: body_controls.len(),
            });
        }
        let controls = controls.sanitized();
        let physical_energy_start = self.total_stored_energy();
        let energy_start = self
            .pending_energy_start
            .take()
            .unwrap_or(physical_energy_start);
        let control_work = std::mem::take(&mut self.pending_control_work);
        let returning_space_activity = self.space.activity();
        let mut source_work = 0.0;
        let mut internal_loss = 0.0;
        let mut local_residual = 0.0_f64;
        for (index, (body, body_control)) in self
            .bodies
            .iter_mut()
            .zip(body_controls.iter().copied())
            .enumerate()
        {
            let frame = body
                .process_local(
                    &ConstitutiveBodyControls::Modal(body_control),
                    returning_space_activity,
                )
                .map_err(|error| WorldError::Body {
                    id: self.body_ids[index].clone(),
                    message: error.to_string(),
                })?;
            self.body_source_work[index] = frame.source_work;
            self.body_internal_loss[index] = frame.internal_loss;
            source_work += frame.source_work;
            internal_loss += frame.internal_loss;
            local_residual = local_residual
                .max(frame.local_residual)
                .max(frame.closure_residual.abs());
        }
        self.finish_after_local(
            controls,
            energy_start,
            control_work,
            source_work,
            internal_loss,
            local_residual,
            |index| body_controls[index].sanitized().radiation_fraction,
        )
    }

    /// Mixed-constitution entrypoint for modal and local-medium bodies in the
    /// canonical shared world.
    pub fn process_constitutive_sample(
        &mut self,
        body_controls: &[ConstitutiveBodyControls],
        controls: WorldControls,
    ) -> Result<WorldFrame, WorldError> {
        if body_controls.len() != self.bodies.len() {
            return Err(WorldError::BodyControlCount {
                expected: self.bodies.len(),
                actual: body_controls.len(),
            });
        }
        let controls = controls.sanitized();
        let physical_energy_start = self.total_stored_energy();
        let energy_start = self
            .pending_energy_start
            .take()
            .unwrap_or(physical_energy_start);
        let control_work = std::mem::take(&mut self.pending_control_work);
        let returning_space_activity = self.space.activity();
        let mut source_work = 0.0;
        let mut internal_loss = 0.0;
        let mut local_residual = 0.0_f64;
        for (index, (body, body_control)) in
            self.bodies.iter_mut().zip(body_controls.iter()).enumerate()
        {
            let frame = body
                .process_local(body_control, returning_space_activity)
                .map_err(|error| WorldError::Body {
                    id: self.body_ids[index].clone(),
                    message: error.to_string(),
                })?;
            self.body_source_work[index] = frame.source_work;
            self.body_internal_loss[index] = frame.internal_loss;
            source_work += frame.source_work;
            internal_loss += frame.internal_loss;
            local_residual = local_residual
                .max(frame.local_residual)
                .max(frame.closure_residual.abs());
        }
        self.finish_after_local(
            controls,
            energy_start,
            control_work,
            source_work,
            internal_loss,
            local_residual,
            |index| match &body_controls[index] {
                ConstitutiveBodyControls::Modal(controls) => {
                    controls.sanitized().radiation_fraction
                }
                ConstitutiveBodyControls::LocalMedium(controls) => {
                    if controls.radiation_fraction.is_finite() {
                        controls.radiation_fraction.clamp(0.0, 1.0)
                    } else {
                        0.0
                    }
                }
                ConstitutiveBodyControls::ModalSpace(controls) => {
                    controls.body.sanitized().radiation_fraction
                }
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_after_local<F>(
        &mut self,
        controls: WorldControls,
        energy_start: f64,
        control_work: f64,
        mut source_work: f64,
        mut internal_loss: f64,
        mut local_residual: f64,
        mut radiation_fraction: F,
    ) -> Result<WorldFrame, WorldError>
    where
        F: FnMut(usize) -> f64,
    {
        // Stateful contact laws (hammers) open the coupling stage; a world
        // without hammers skips this exactly.
        if !self.hammers.is_empty() {
            if self.pending_source_work != 0.0 {
                source_work += std::mem::take(&mut self.pending_source_work);
            }
            if self.pending_contact_loss != 0.0 {
                internal_loss += std::mem::take(&mut self.pending_contact_loss);
            }
            internal_loss += self.process_hammers();
        }
        let coupling = self
            .couplings
            .process_constitutive(
                self.sample_rate_hz,
                &mut self.bodies,
                controls.direct_coupling,
            )
            .map_err(|error| WorldError::Coupling(error.to_string()))?;
        self.last_coupling_frame = coupling;
        local_residual = local_residual.max(coupling.metric_residual.abs());

        // A world whose every body is an N-D body with an inert port never
        // touches the shared space: it stays exactly at rest, so skip it.
        if self.space_port_inert.iter().all(|inert| *inert) {
            self.last_space_frame = Default::default();
            return self.finish_without_space(
                energy_start,
                control_work,
                source_work,
                internal_loss,
                local_residual,
                radiation_fraction,
                coupling,
            );
        }
        self.space
            .begin_frame()
            .map_err(|error| WorldError::Space(error.to_string()))?;
        let mut space_exchange_into_bodies = 0.0;
        let mut space_metric_residual = 0.0;
        for index in 0..self.bodies.len() {
            // An N-D body whose port weights are all exactly zero exchanges
            // nothing (every angle is 0): skip the no-op. Legacy bodies always run it.
            if self.space_port_inert[index] {
                continue;
            }
            let exchange = self
                .space
                .exchange_compiled_port(
                    self.bodies[index].momentum_mut(),
                    &mut self.compiled_ports[index],
                    controls.space,
                )
                .map_err(|error| WorldError::Space(error.to_string()))?;
            space_exchange_into_bodies += exchange.signed_into_body;
            space_metric_residual += exchange.metric_residual;
            local_residual = local_residual.max(exchange.metric_residual.abs());
        }
        let space_frame = self
            .space
            .finish_frame(
                controls.space,
                space_exchange_into_bodies,
                space_metric_residual,
            )
            .map_err(|error| WorldError::Space(error.to_string()))?;
        self.last_space_frame = space_frame;
        internal_loss += space_frame.internal_loss;
        if self.pending_disposal_internal != 0.0 {
            internal_loss += std::mem::take(&mut self.pending_disposal_internal);
        }
        local_residual = local_residual
            .max(space_frame.coupling_metric_residual.abs())
            .max(space_frame.closure_residual.abs());

        let mut radiation = space_frame.radiation;
        if self.pending_disposal_radiation != 0.0 {
            radiation += std::mem::take(&mut self.pending_disposal_radiation);
        }
        for index in 0..self.bodies.len() {
            if let Some(space) = self.bodies[index].space_body_mut() {
                let (radiated, residual) = space
                    .radiate(radiation_fraction(index), &mut self.body_radiation[index])
                    .map_err(|error| WorldError::Body {
                        id: self.body_ids[index].clone(),
                        message: error.to_string(),
                    })?;
                self.body_emissions[index] =
                    self.body_radiation[index].first().copied().unwrap_or(0.0);
                radiation += radiated;
                local_residual = local_residual.max(residual.abs());
                continue;
            }
            let emitted = self.bodies[index]
                .radiate(radiation_fraction(index))
                .map_err(|error| WorldError::Body {
                    id: self.body_ids[index].clone(),
                    message: error.to_string(),
                })?;
            self.body_emissions[index] = emitted.emitted.first().copied().unwrap_or(0.0);
            self.body_radiation[index].clone_from(&emitted.emitted);
            radiation += emitted.radiation;
            local_residual = local_residual.max(emitted.residual.abs());
        }

        let energy_end = self.total_stored_energy();
        if !energy_end.is_finite() {
            return Err(WorldError::NonFiniteState);
        }
        let ledger_frame = WorkFrame {
            energy_start,
            source_work,
            control_work,
            internal_loss,
            radiation_loss: radiation,
            output_work: 0.0,
            energy_end,
        };
        self.ledger.push(ledger_frame);
        self.sample_frames += 1;
        local_residual = local_residual.max(ledger_frame.residual().abs());
        self.maximum_local_residual = self.maximum_local_residual.max(local_residual);

        Ok(WorldFrame {
            source_work,
            control_work,
            internal_loss,
            radiation,
            space_emitted_left: space_frame.emitted_left,
            space_emitted_right: space_frame.emitted_right,
            direct_exchange_into_left: coupling.exchange_into_left,
            space_exchange_into_bodies,
            local_residual,
        })
    }

    /// The tail of `finish_after_local` for a world that does not touch the
    /// shared space (N-D bodies with inert ports only).
    #[allow(clippy::too_many_arguments)]
    fn finish_without_space<F>(
        &mut self,
        energy_start: f64,
        control_work: f64,
        source_work: f64,
        mut internal_loss: f64,
        mut local_residual: f64,
        mut radiation_fraction: F,
        coupling: crate::coupling::CouplingFrame,
    ) -> Result<WorldFrame, WorldError>
    where
        F: FnMut(usize) -> f64,
    {
        if self.pending_disposal_internal != 0.0 {
            internal_loss += std::mem::take(&mut self.pending_disposal_internal);
        }
        let mut radiation = std::mem::take(&mut self.pending_disposal_radiation);
        for index in 0..self.bodies.len() {
            let Some(space) = self.bodies[index].space_body_mut() else {
                // Inert ports belong to N-D bodies or to hammers, which radiate nothing.
                self.body_emissions[index] = 0.0;
                self.body_radiation[index].clear();
                self.body_radiation[index].push(0.0);
                continue;
            };
            let (radiated, residual) = space
                .radiate(radiation_fraction(index), &mut self.body_radiation[index])
                .map_err(|error| WorldError::Body {
                    id: self.body_ids[index].clone(),
                    message: error.to_string(),
                })?;
            self.body_emissions[index] = self.body_radiation[index].first().copied().unwrap_or(0.0);
            radiation += radiated;
            local_residual = local_residual.max(residual.abs());
        }
        let energy_end = self.total_stored_energy();
        if !energy_end.is_finite() {
            return Err(WorldError::NonFiniteState);
        }
        let ledger_frame = WorkFrame {
            energy_start,
            source_work,
            control_work,
            internal_loss,
            radiation_loss: radiation,
            output_work: 0.0,
            energy_end,
        };
        self.ledger.push(ledger_frame);
        self.sample_frames += 1;
        local_residual = local_residual.max(ledger_frame.residual().abs());
        self.maximum_local_residual = self.maximum_local_residual.max(local_residual);
        Ok(WorldFrame {
            source_work,
            control_work,
            internal_loss,
            radiation,
            space_emitted_left: 0.0,
            space_emitted_right: 0.0,
            direct_exchange_into_left: coupling.exchange_into_left,
            space_exchange_into_bodies: 0.0,
            local_residual,
        })
    }

    #[must_use]
    pub fn receipt(&self) -> WorldReceipt {
        WorldReceipt {
            plan_digest: self.plan_digest.clone(),
            sample_frames: self.sample_frames,
            body_count: self.bodies.len(),
            direct_coupling_count: self.couplings.edge_count(),
            source_work: self.ledger.source_work,
            control_work: self.ledger.control_work + self.pending_control_work,
            internal_loss: self.ledger.internal_loss,
            radiation: self.ledger.radiation_loss,
            initial_energy: self
                .ledger
                .initial_energy
                .unwrap_or_else(|| self.total_stored_energy()),
            current_energy: self.total_stored_energy(),
            global_closure_residual: self.ledger.global_residual(),
            cumulative_closure_residual: self.ledger.cumulative_residual,
            maximum_frame_residual: self.ledger.max_abs_residual,
            maximum_local_residual: self.maximum_local_residual,
        }
    }
}
