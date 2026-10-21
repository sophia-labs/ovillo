use std::collections::BTreeMap;

use crate::body::BodyReconfigurationFrame;
use crate::constitutive_body::{ConstitutiveBodyError, WorldBody};
use crate::coupling::CouplingNetwork;
use crate::field::MetricFieldState;
use crate::ir::{MODE_COUNT, SPACE_COURSES};
use crate::law::medium::LocalMediumState;
use crate::plan::{
    BodyConstitutionSpec, BodyPose, ConstitutiveWorldPlan, PlanError, SpacePortSpec,
    WorldPlan,
};
use crate::shared_space::SharedSpace;

use super::{hammer, World, WorldError};

impl From<PlanError> for WorldError {
    fn from(error: PlanError) -> Self {
        Self::Plan(error.to_string())
    }
}

impl From<ConstitutiveBodyError> for WorldError {
    fn from(error: ConstitutiveBodyError) -> Self {
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
        let mut bodies = Vec::with_capacity(plan.bodies.len());
        for instance in &plan.bodies {
            bodies.push(
                WorldBody::from_constitution(
                    plan.internal_sample_rate_hz,
                    BodyConstitutionSpec::Modal {
                        body: instance.body.clone(),
                    },
                )
                .map_err(|error| WorldError::Body {
                    id: instance.id.clone(),
                    message: error.to_string(),
                })?,
            );
        }
        let couplings = CouplingNetwork::compile(&plan)
            .map_err(|error| WorldError::Coupling(error.to_string()))?;
        let body_ids = plan.bodies.iter().map(|body| body.id.clone()).collect();
        let body_poses = plan.bodies.iter().map(|body| body.pose).collect();
        let space_ports = plan.bodies.iter().map(|body| body.space_port).collect();
        Self::from_parts(
            Some(plan.clone()),
            None,
            plan.world_name.clone(),
            plan.internal_sample_rate_hz,
            plan_digest,
            body_ids,
            body_poses,
            space_ports,
            bodies,
            plan.shared_space,
            couplings,
        )
    }

    pub fn from_constitutive_plan(plan: ConstitutiveWorldPlan) -> Result<Self, WorldError> {
        plan.validate()?;
        let plan_digest = plan.digest()?;
        let mut bodies = Vec::with_capacity(plan.bodies.len());
        for instance in &plan.bodies {
            bodies.push(
                WorldBody::from_constitution(
                    plan.internal_sample_rate_hz,
                    instance.constitution.clone(),
                )
                .map_err(|error| WorldError::Body {
                    id: instance.id.clone(),
                    message: error.to_string(),
                })?,
            );
        }
        let couplings = CouplingNetwork::compile_constitutive(&plan)
            .map_err(|error| WorldError::Coupling(error.to_string()))?;
        let body_ids = plan.bodies.iter().map(|body| body.id.clone()).collect();
        let body_poses = plan.bodies.iter().map(|body| body.pose).collect();
        let space_ports = plan.bodies.iter().map(|body| body.space_port).collect();
        Self::from_parts(
            None,
            Some(plan.clone()),
            plan.world_name.clone(),
            plan.internal_sample_rate_hz,
            plan_digest,
            body_ids,
            body_poses,
            space_ports,
            bodies,
            plan.shared_space,
            couplings,
        )
        .and_then(|mut world| {
            world.hammers = hammer::bind_hammers(&plan, &world)?;
            Ok(world)
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn from_parts(
        legacy_plan: Option<WorldPlan>,
        constitutive_plan: Option<ConstitutiveWorldPlan>,
        world_name: String,
        sample_rate_hz: f64,
        plan_digest: String,
        body_ids: Vec<String>,
        body_poses: Vec<BodyPose>,
        space_ports: Vec<SpacePortSpec>,
        bodies: Vec<WorldBody>,
        shared_space: crate::plan::SharedSpaceSpec,
        couplings: CouplingNetwork,
    ) -> Result<Self, WorldError> {
        let body_indices = body_ids
            .iter()
            .enumerate()
            .map(|(index, id)| (id.clone(), index))
            .collect::<BTreeMap<_, _>>();
        let space = SharedSpace::try_new(sample_rate_hz, shared_space)
            .map_err(|error| WorldError::Space(error.to_string()))?;
        let body_count = bodies.len();
        let space_port_inert = bodies
            .iter()
            .zip(&space_ports)
            .map(|(body, port)| {
                (body.space_body().is_some() || body.hammer().is_some())
                    && port.coupling_weights.iter().all(|w| *w == 0.0)
            })
            .collect();
        Ok(Self {
            space_port_inert,
            legacy_plan,
            constitutive_plan,
            world_name,
            sample_rate_hz,
            plan_digest,
            body_indices,
            body_ids,
            body_poses,
            space_ports,
            bodies,
            space,
            couplings,
            body_emissions: vec![0.0; body_count],
            body_radiation: vec![Vec::new(); body_count],
            body_source_work: vec![0.0; body_count],
            body_internal_loss: vec![0.0; body_count],
            last_space_frame: Default::default(),
            last_coupling_frame: Default::default(),
            ledger: Default::default(),
            sample_frames: 0,
            pending_control_work: 0.0,
            pending_energy_start: None,
            maximum_local_residual: 0.0,
            pending_disposal_internal: 0.0,
            pending_disposal_radiation: 0.0,
            disposal_total: 0.0,
            hammers: Vec::new(),
            pending_source_work: 0.0,
            pending_contact_loss: 0.0,
        })
    }

    /// Legacy modal plan. New mixed-constitution code should use
    /// `constitutive_plan()` or plan-independent world metadata.
    #[must_use]
    pub fn plan(&self) -> &WorldPlan {
        self.legacy_plan
            .as_ref()
            .expect("World::plan() is only defined for legacy modal plans")
    }

    #[must_use]
    pub fn legacy_plan(&self) -> Option<&WorldPlan> {
        self.legacy_plan.as_ref()
    }

    #[must_use]
    pub fn constitutive_plan(&self) -> Option<&ConstitutiveWorldPlan> {
        self.constitutive_plan.as_ref()
    }

    #[must_use]
    pub fn world_name(&self) -> &str {
        &self.world_name
    }

    #[must_use]
    pub fn plan_digest(&self) -> &str {
        &self.plan_digest
    }

    #[must_use]
    pub fn sample_rate_hz(&self) -> f64 {
        self.sample_rate_hz
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
        self.body_ids.get(index).map(String::as_str)
    }

    #[must_use]
    pub fn body_pose(&self, index: usize) -> Option<BodyPose> {
        self.body_poses.get(index).copied()
    }

    #[must_use]
    pub fn body_state(&self, index: usize) -> Option<MetricFieldState<MODE_COUNT>> {
        self.bodies.get(index).map(WorldBody::field_state)
    }

    #[must_use]
    pub fn body_local_medium_state(&self, index: usize) -> Option<&LocalMediumState> {
        self.bodies.get(index).and_then(WorldBody::local_medium_state)
    }

    #[must_use]
    pub fn body_material_energy(&self, index: usize) -> Option<f64> {
        self.bodies.get(index).map(WorldBody::material_stored_energy)
    }

    #[must_use]
    pub fn body_reservoir_energy(&self, index: usize) -> Option<f64> {
        self.bodies.get(index).map(WorldBody::reservoir_energy)
    }

    #[must_use]
    pub fn body_configuration_amount(&self, index: usize) -> Option<f64> {
        self.bodies.get(index).and_then(WorldBody::configuration_amount)
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
    pub fn body_radiation(&self, index: usize) -> Option<&[f64]> {
        self.body_radiation.get(index).map(Vec::as_slice)
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
            .map(WorldBody::total_stored_energy)
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
            .body_ids
            .get(index)
            .cloned()
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
        let replacement = if let Some(plan) = self.legacy_plan.clone() {
            Self::from_plan(plan)?
        } else if let Some(plan) = self.constitutive_plan.clone() {
            Self::from_constitutive_plan(plan)?
        } else {
            return Err(WorldError::Plan("world has no construction plan".to_owned()));
        };
        *self = replacement;
        Ok(())
    }
}
