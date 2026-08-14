use std::collections::BTreeMap;

use crate::body::{Body, BodyError, BodyReconfigurationFrame};
use crate::coupling::CouplingNetwork;
use crate::field::MetricFieldState;
use crate::ir::MODE_COUNT;
use crate::plan::{PlanError, WorldPlan};
use crate::shared_space::SharedSpace;

use super::{
    BoundObserver, BoundObserverTap, ObserverOutput, SharedWorld,
    SharedWorldError,
};

impl From<PlanError> for SharedWorldError {
    fn from(error: PlanError) -> Self {
        Self::Plan(error.to_string())
    }
}

impl From<BodyError> for SharedWorldError {
    fn from(error: BodyError) -> Self {
        Self::Body {
            id: "<unbound>".to_owned(),
            message: error.to_string(),
        }
    }
}

impl SharedWorld {
    pub fn from_plan(plan: WorldPlan) -> Result<Self, SharedWorldError> {
        plan.validate()?;
        let plan_digest = plan.digest()?;
        let body_indices = plan
            .bodies
            .iter()
            .enumerate()
            .map(|(index, instance)| (instance.id.clone(), index))
            .collect::<BTreeMap<_, _>>();
        let mut bodies = Vec::with_capacity(plan.bodies.len());
        for instance in &plan.bodies {
            bodies.push(
                Body::try_new(
                    plan.internal_sample_rate_hz,
                    instance.body.clone(),
                )
                .map_err(|error| SharedWorldError::Body {
                    id: instance.id.clone(),
                    message: error.to_string(),
                })?,
            );
        }
        let space = SharedSpace::try_new(
            plan.internal_sample_rate_hz,
            plan.shared_space,
        )
        .map_err(|error| SharedWorldError::Space(error.to_string()))?;
        let couplings = CouplingNetwork::compile(&plan)
            .map_err(|error| SharedWorldError::Coupling(error.to_string()))?;

        let mut observers = Vec::with_capacity(plan.observers.len());
        for observer in &plan.observers {
            let body_taps = observer
                .body_taps
                .iter()
                .map(|tap| {
                    let body = body_indices
                        .get(&tap.body_id)
                        .copied()
                        .ok_or_else(|| {
                            SharedWorldError::UnknownBody(tap.body_id.clone())
                        })?;
                    Ok(BoundObserverTap {
                        body,
                        left: tap.left_projection,
                        right: tap.right_projection,
                    })
                })
                .collect::<Result<Vec<_>, SharedWorldError>>()?;
            observers.push(BoundObserver {
                id: observer.id.clone(),
                body_taps,
                space_left: observer.space_left_projection,
                space_right: observer.space_right_projection,
                scale: observer.output_scale,
            });
        }
        let primary_observer = plan
            .observers
            .iter()
            .position(|observer| observer.id == plan.primary_observer)
            .ok_or_else(|| {
                SharedWorldError::Plan(
                    "primary observer is not declared".to_owned(),
                )
            })?;
        let observer_outputs = observers
            .iter()
            .map(|observer| ObserverOutput {
                id: observer.id.clone(),
                left: 0.0,
                right: 0.0,
            })
            .collect();

        Ok(Self {
            body_emissions: vec![0.0; bodies.len()],
            plan,
            plan_digest,
            body_indices,
            bodies,
            space,
            couplings,
            observers,
            primary_observer,
            observer_outputs,
            ledger: Default::default(),
            sample_frames: 0,
            pending_control_work: 0.0,
            pending_energy_start: None,
            maximum_local_residual: 0.0,
        })
    }

    #[must_use]
    pub fn plan(&self) -> &WorldPlan {
        &self.plan
    }

    #[must_use]
    pub fn plan_digest(&self) -> &str {
        &self.plan_digest
    }

    #[must_use]
    pub fn sample_rate_hz(&self) -> f64 {
        self.plan.internal_sample_rate_hz
    }

    #[must_use]
    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    #[must_use]
    pub fn observer_outputs(&self) -> &[ObserverOutput] {
        &self.observer_outputs
    }

    #[must_use]
    pub fn delay_cell_count(&self) -> usize {
        self.space.delay_cell_count()
    }

    #[must_use]
    pub fn total_stored_energy(&self) -> f64 {
        self.bodies
            .iter()
            .map(Body::total_stored_energy)
            .sum::<f64>()
            + self.space.stored_energy()
    }

    #[must_use]
    pub fn body_state(
        &self,
        index: usize,
    ) -> Option<MetricFieldState<MODE_COUNT>> {
        self.bodies.get(index).map(Body::field_state)
    }

    pub fn set_body_configuration(
        &mut self,
        body_id: &str,
        amount: f64,
    ) -> Result<BodyReconfigurationFrame, SharedWorldError> {
        let index = self
            .body_indices
            .get(body_id)
            .copied()
            .ok_or_else(|| SharedWorldError::UnknownBody(body_id.to_owned()))?;
        self.set_body_configuration_index(index, amount)
    }

    pub fn set_body_configuration_index(
        &mut self,
        index: usize,
        amount: f64,
    ) -> Result<BodyReconfigurationFrame, SharedWorldError> {
        let id = self
            .plan
            .bodies
            .get(index)
            .map(|body| body.id.clone())
            .ok_or_else(|| {
                SharedWorldError::UnknownBody(format!("index/{index}"))
            })?;
        let energy_before = self.total_stored_energy();
        if self.pending_energy_start.is_none() {
            self.pending_energy_start = Some(energy_before);
        }
        let frame = self.bodies[index]
            .set_configuration_amount(amount)
            .map_err(|error| SharedWorldError::Body {
                id,
                message: error.to_string(),
            })?;
        self.pending_control_work += frame.total_control_work;
        Ok(frame)
    }

    pub fn reset(&mut self) -> Result<(), SharedWorldError> {
        let replacement = Self::from_plan(self.plan.clone())?;
        *self = replacement;
        Ok(())
    }
}
