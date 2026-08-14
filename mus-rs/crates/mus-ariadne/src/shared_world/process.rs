use mus_watsn::WorkFrame;

use crate::body::BodyControls;

use super::{
    SharedFrameOutput, SharedWorld, SharedWorldControls, SharedWorldError,
    SharedWorldReceipt,
};

impl SharedWorld {
    /// Advance one shared material-clock sample.
    ///
    /// All vectors and graph bindings are allocated at construction. The hot
    /// path performs no string lookup, graph search, locking, or allocation.
    pub fn process_sample(
        &mut self,
        body_controls: &[BodyControls],
        controls: SharedWorldControls,
    ) -> Result<SharedFrameOutput, SharedWorldError> {
        if body_controls.len() != self.bodies.len() {
            return Err(SharedWorldError::BodyControlCount {
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
                .process_local(body_control, returning_space_activity)
                .map_err(|error| SharedWorldError::Body {
                    id: self.plan.bodies[index].id.clone(),
                    message: error.to_string(),
                })?;
            source_work += frame.source_work;
            internal_loss += frame.internal_loss;
            local_residual = local_residual
                .max(frame.local_residual)
                .max(frame.closure_residual.abs());
        }

        let coupling = self
            .couplings
            .process(
                self.plan.internal_sample_rate_hz,
                &mut self.bodies,
                controls.direct_coupling,
            )
            .map_err(|error| {
                SharedWorldError::Coupling(error.to_string())
            })?;
        local_residual = local_residual.max(coupling.metric_residual.abs());

        self.space
            .begin_frame()
            .map_err(|error| SharedWorldError::Space(error.to_string()))?;
        let mut space_exchange_into_bodies = 0.0;
        let mut space_metric_residual = 0.0;
        for index in 0..self.bodies.len() {
            let exchange = self
                .space
                .exchange_port(
                    self.bodies[index].momentum_mut(),
                    &self.plan.bodies[index].space_port,
                    controls.space,
                )
                .map_err(|error| {
                    SharedWorldError::Space(error.to_string())
                })?;
            space_exchange_into_bodies += exchange.signed_into_body;
            space_metric_residual += exchange.metric_residual;
            local_residual =
                local_residual.max(exchange.metric_residual.abs());
        }
        let space_frame = self
            .space
            .finish_frame(
                controls.space,
                space_exchange_into_bodies,
                space_metric_residual,
            )
            .map_err(|error| SharedWorldError::Space(error.to_string()))?;
        internal_loss += space_frame.internal_loss;
        local_residual = local_residual
            .max(space_frame.coupling_metric_residual.abs())
            .max(space_frame.closure_residual.abs());

        let mut radiation = space_frame.radiation;
        for index in 0..self.bodies.len() {
            let emitted = self.bodies[index]
                .radiate(
                    body_controls[index]
                        .sanitized()
                        .radiation_fraction,
                )
                .map_err(|error| SharedWorldError::Body {
                    id: self.plan.bodies[index].id.clone(),
                    message: error.to_string(),
                })?;
            self.body_emissions[index] = emitted.emitted_coordinate;
            radiation += emitted.radiation;
            local_residual = local_residual.max(emitted.residual.abs());
        }

        let energy_end = self.total_stored_energy();
        if !energy_end.is_finite() {
            return Err(SharedWorldError::NonFiniteState);
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
        local_residual =
            local_residual.max(ledger_frame.residual().abs());
        self.maximum_local_residual =
            self.maximum_local_residual.max(local_residual);

        for index in 0..self.observers.len() {
            let observer = &self.observers[index];
            let mut left =
                observer.space_left * space_frame.emitted_left;
            let mut right =
                observer.space_right * space_frame.emitted_right;
            for tap in &observer.body_taps {
                left += tap.left * self.body_emissions[tap.body];
                right += tap.right * self.body_emissions[tap.body];
            }
            let scale = observer.scale * controls.observation_scale;
            self.observer_outputs[index].left = scale * left;
            self.observer_outputs[index].right = scale * right;
            if !self.observer_outputs[index].left.is_finite()
                || !self.observer_outputs[index].right.is_finite()
            {
                return Err(SharedWorldError::NonFiniteState);
            }
        }
        let primary = &self.observer_outputs[self.primary_observer];
        Ok(SharedFrameOutput {
            left: primary.left,
            right: primary.right,
            source_work,
            control_work,
            internal_loss,
            radiation,
            direct_exchange_into_left: coupling.exchange_into_left,
            space_exchange_into_bodies,
            local_residual,
        })
    }

    #[must_use]
    pub fn receipt(&self) -> SharedWorldReceipt {
        SharedWorldReceipt {
            plan_digest: self.plan_digest.clone(),
            sample_frames: self.sample_frames,
            body_count: self.bodies.len(),
            direct_coupling_count: self.couplings.edge_count(),
            observer_count: self.observers.len(),
            source_work: self.ledger.source_work,
            control_work: self.ledger.control_work
                + self.pending_control_work,
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
