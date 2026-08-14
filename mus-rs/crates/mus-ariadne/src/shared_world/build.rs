use std::collections::BTreeMap;

use crate::body::{Body, BodyError, BodyReconfigurationFrame};
use crate::coupling::CouplingNetwork;
use crate::field::MetricFieldState;
use crate::ir::{MODE_COUNT, SPACE_COURSES};
use crate::plan::{BodyPose, PlanError, WorldPlan};
use crate::shared_space::SharedSpace;

use super::{World, WorldError};

impl From<PlanError> for WorldError {
    fn from(error: PlanError) -> Self {
        Self::Plan(error.to_string())
    }
}

impl From<BodyError> for WorldError {
    fn from(error: BodyError) -> Self {
        Self::Body {
            id: "<unbound>".to_owned(),
            message: error.to_string(),
        }
    }
}

impl World {
    pub fn from_plan(plan: WorldPlan) -> Result<Self, WorldError> {
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
                Body::try_new(plan.internal_sample_rate_hz, instance.body.clone()).map_err(
                    |error| WorldError::Body {
                        id: instance.id.clone(),
                        message: error.to_string(),
                    },
                )?,
            );
        }
        let space = SharedSpace::try_new(plan.internal_sample_rate_hz, plan.shared_space)
            .map_err(|error| WorldError::Space(error.to_string()))?;
        let couplings = CouplingNetwork::compile(&plan)
            .map_err(|error| WorldError::Coupling(error.to_string()))?;
        let body_count = bodies.len();

        Ok(Self {
            plan,
            plan_digest,
            body_indices,
            bodies,
            space,
            couplings,
            body_emissions: vec![0.0; body_count],
            body_source_work: vec![0.0; body_count],
            body_internal_loss: vec![0.0; body_count],
            last_space_frame: Default::default(),
            last_coupling_frame: Default::default(),
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
    pub fn body_index(&self, id: &str) -> Option<usize> {
        self.body_indices.get(id).copied()
    }

    #[must_use]
    pub fn body_id(&self, index: usize) -> Option<&str> {
        self.plan.bodies.get(index).map(|body| body.id.as_str())
    }

    #[must_use]
    pub fn body_pose(&self, index: usize) -> Option<BodyPose> {
        self.plan.bodies.get(index).map(|body| body.pose)
    }

    #[must_use]
    pub fn body_state(&self, index: usize) -> Option<MetricFieldState<MODE_COUNT>> {
        self.bodies.get(index).map(Body::field_state)
    }

    #[must_use]
    pub fn body_material_energy(&self, index: usize) -> Option<f64> {
        self.bodies.get(index).map(Body::material_stored_energy)
    }

    #[must_use]
    pub fn body_reservoir_energy(&self, index: usize) -> Option<f64> {
        self.bodies.get(index).map(Body::reservoir_energy)
    }

    #[must_use]
    pub fn body_configuration_amount(&self, index: usize) -> Option<f64> {
        self.bodies.get(index).map(Body::configuration_amount)
    }

    #[must_use]
    pub fn body_emission(&self, index: usize) -> Option<f64> {
        self.body_emissions.get(index).copied()
    }

    #[must_use]
    pub fn body_emissions(&self) -> &[f64] {
        &self.body_emissions
    }

    #[must_use]
    pub fn body_last_source_work(&self, index: usize) -> Option<f64> {
        self.body_source_work.get(index).copied()
    }

    #[must_use]
    pub fn body_last_internal_loss(&self, index: usize) -> Option<f64> {
        self.body_internal_loss.get(index).copied()
    }

    #[must_use]
    pub fn space_stored_energy(&self) -> f64 {
        self.space.stored_energy()
    }

    #[must_use]
    pub fn space_course_head(&self, index: usize) -> Option<f64> {
        self.space.course_head(index)
    }

    #[must_use]
    pub fn space_course_count(&self) -> usize {
        SPACE_COURSES
    }

    #[must_use]
    pub fn last_space_frame(&self) -> crate::shared_space::SharedSpaceFrame {
        self.last_space_frame
    }

    #[must_use]
    pub fn coupling_count(&self) -> usize {
        self.couplings.edge_count()
    }

    #[must_use]
    pub fn coupling_id(&self, index: usize) -> Option<&str> {
        self.couplings.edge_id(index)
    }

    #[must_use]
    pub fn coupling_last_exchange_into_left(&self, index: usize) -> Option<f64> {
        self.couplings.last_exchange_into_left(index)
    }

    #[must_use]
    pub fn last_coupling_frame(&self) -> crate::coupling::CouplingFrame {
        self.last_coupling_frame
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

    pub fn set_body_configuration(
        &mut self,
        body_id: &str,
        amount: f64,
    ) -> Result<BodyReconfigurationFrame, WorldError> {
        let index = self
            .body_indices
            .get(body_id)
            .copied()
            .ok_or_else(|| WorldError::UnknownBody(body_id.to_owned()))?;
        self.set_body_configuration_index(index, amount)
    }

    pub fn set_body_configuration_index(
        &mut self,
        index: usize,
        amount: f64,
    ) -> Result<BodyReconfigurationFrame, WorldError> {
        let id = self
            .plan
            .bodies
            .get(index)
            .map(|body| body.id.clone())
            .ok_or_else(|| WorldError::UnknownBody(format!("index/{index}")))?;
        let energy_before = self.total_stored_energy();
        if self.pending_energy_start.is_none() {
            self.pending_energy_start = Some(energy_before);
        }
        let frame = self.bodies[index]
            .set_configuration_amount(amount)
            .map_err(|error| WorldError::Body {
                id,
                message: error.to_string(),
            })?;
        self.pending_control_work += frame.total_control_work;
        Ok(frame)
    }

    pub fn reset(&mut self) -> Result<(), WorldError> {
        let replacement = Self::from_plan(self.plan.clone())?;
        *self = replacement;
        Ok(())
    }
}
