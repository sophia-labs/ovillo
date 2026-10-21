//! Typed body constitutions for the canonical shared Ariadne world.
//!
//! This plan is additive to the immutable modal `WorldPlan` v2 schema. Both
//! plan families compile into the same [`crate::World`] executor, delayed field,
//! reciprocal coupling graph, material clock, and global ledger. Keeping the
//! schema additive avoids silently changing the meaning or digest of existing
//! `ariadne-world-plan-v2` artifacts.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::configuration::ConfigurationMapSpec;
use crate::ir::MODE_COUNT;
use crate::law::hammer::HammerSpec;
use crate::law::medium::LocalMediumPlan;

use super::helpers::{validate_direction, validate_id};
use super::{
    BodyPose, BodySpec, DirectCouplingSpec, PlanError, SharedSpaceSpec, SpacePortSpec,
    WorldPlan, MAX_BODY_INSTANCES, MAX_DIRECT_COUPLINGS,
};

pub const WORLD_PLAN_V3_SCHEMA_VERSION: &str = "ariadne-world-plan-v3";

/// Active input transducer from one storage-bearing world-boundary direction
/// into a declared local-medium source. The boundary reservoir pays (or receives)
/// the exact source work caused by this transduction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMediumPortInputBinding {
    pub source: usize,
    pub boundary_direction: [f64; MODE_COUNT],
    pub wave_gain: f64,
    pub evidence_gain: f64,
}

/// Active output transducer from declared local-medium state coordinates into
/// one storage-bearing world-boundary direction. This is a physical constitutive
/// aperture, not a non-loading observation program; its work is paid from the
/// finite boundary reservoir.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMediumPortOutputBinding {
    pub wave_weights: Vec<f64>,
    pub evidence_weights: Vec<f64>,
    pub reservoir_weights: Vec<f64>,
    pub boundary_direction: [f64; MODE_COUNT],
    pub gain_per_second: f64,
}

/// Storage-bearing fixed-rank boundary between an arbitrary local medium and
/// the canonical Ariadne shared world.
///
/// Existing shared-space and direct-coupling laws act only on the boundary
/// momentum. The arbitrary local medium remains behind this stable port ABI.
/// Input/output transduction is finite-supply and work-accounted; vocabulary,
/// tasks, optimizers, and learned detectors remain outside Ariadne core.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMediumBoundarySpec {
    pub medium: LocalMediumPlan,
    #[serde(default)]
    pub inputs: Vec<LocalMediumPortInputBinding>,
    #[serde(default)]
    pub outputs: Vec<LocalMediumPortOutputBinding>,
    pub boundary_reservoir_capacity: f64,
    #[serde(default)]
    pub initial_boundary_reservoir: f64,
    #[serde(default)]
    pub boundary_decay_per_second: f64,
    pub radiation_direction: [f64; MODE_COUNT],
}

/// One body constitution accepted by the canonical `World` executor.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BodyConstitutionSpec {
    Modal { body: BodySpec },
    LocalMedium { boundary: LocalMediumBoundarySpec },
    /// A body with an N-dimensional configuration space
    /// (`SPECS/ariadne/ND-CONFIGURATION.md`). Additive: the two variants above
    /// serialize exactly as before.
    ModalSpace { body: super::SpaceBodySpec },
    /// A felt hammer: a mass with one coordinate whose felt contact with the
    /// wires it names is a stateful coupling law (`law::hammer`). It has no
    /// modal field: it takes no couplings and no shared-space port, and is
    /// thrown by `World::strike_hammer`. Additive: the variants above
    /// serialize exactly as before.
    Hammer { hammer: HammerSpec },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstitutiveBodyInstanceSpec {
    pub id: String,
    pub constitution: BodyConstitutionSpec,
    pub space_port: SpacePortSpec,
    #[serde(default)]
    pub pose: BodyPose,
}

/// Versioned physical construction plan for mixed authored modal bodies and
/// learned/authored local media.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConstitutiveWorldPlan {
    pub schema_version: String,
    pub world_name: String,
    pub internal_sample_rate_hz: f64,
    pub bodies: Vec<ConstitutiveBodyInstanceSpec>,
    pub shared_space: SharedSpaceSpec,
    #[serde(default)]
    pub direct_couplings: Vec<DirectCouplingSpec>,
}

impl LocalMediumBoundarySpec {
    pub(crate) fn validate(&self, world_hz: f64) -> Result<(), PlanError> {
        self.medium
            .validate()
            .map_err(|error| PlanError::new("invalid-local-medium", error.to_string()))?;
        if (self.medium.sample_rate_hz - world_hz).abs() > 1.0e-9 {
            return Err(PlanError::new(
                "local-medium-clock-mismatch",
                "local-medium sample_rate_hz must equal the world material clock",
            ));
        }
        if !self.medium.observers.is_empty() {
            return Err(PlanError::new(
                "embedded-local-medium-observer",
                "World bodies may not embed LocalMediumObserverSpec; bind non-loading observations to the physical world digest instead",
            ));
        }
        if !self.boundary_reservoir_capacity.is_finite()
            || self.boundary_reservoir_capacity <= 0.0
            || !self.initial_boundary_reservoir.is_finite()
            || self.initial_boundary_reservoir < 0.0
            || self.initial_boundary_reservoir > self.boundary_reservoir_capacity
            || !self.boundary_decay_per_second.is_finite()
            || self.boundary_decay_per_second < 0.0
        {
            return Err(PlanError::new(
                "invalid-local-medium-boundary-reservoir",
                "boundary reservoir values must be finite, non-negative, and within capacity",
            ));
        }
        validate_direction("local-medium radiation direction", &self.radiation_direction)?;
        for (index, binding) in self.inputs.iter().enumerate() {
            if binding.source >= self.medium.sources.len() {
                return Err(PlanError::new(
                    "invalid-local-medium-port-source",
                    format!("input binding {index} references an unknown medium source"),
                ));
            }
            validate_direction(
                "local-medium input boundary direction",
                &binding.boundary_direction,
            )?;
            if !binding.wave_gain.is_finite() || !binding.evidence_gain.is_finite() {
                return Err(PlanError::new(
                    "invalid-local-medium-port-input-gain",
                    format!("input binding {index} has a non-finite gain"),
                ));
            }
        }
        let sites = self.medium.sites.len();
        for (index, binding) in self.outputs.iter().enumerate() {
            for (name, values) in [
                ("wave", &binding.wave_weights),
                ("evidence", &binding.evidence_weights),
                ("reservoir", &binding.reservoir_weights),
            ] {
                if values.len() != sites || values.iter().any(|value| !value.is_finite()) {
                    return Err(PlanError::new(
                        "invalid-local-medium-port-output-weights",
                        format!("output binding {index} has invalid {name} weights"),
                    ));
                }
            }
            let weight_norm = binding
                .wave_weights
                .iter()
                .chain(&binding.evidence_weights)
                .chain(&binding.reservoir_weights)
                .map(|value| value * value)
                .sum::<f64>()
                .sqrt();
            if weight_norm <= 1.0e-12 {
                return Err(PlanError::new(
                    "degenerate-local-medium-port-output",
                    format!("output binding {index} has no state sensitivity"),
                ));
            }
            validate_direction(
                "local-medium output boundary direction",
                &binding.boundary_direction,
            )?;
            if !binding.gain_per_second.is_finite() {
                return Err(PlanError::new(
                    "invalid-local-medium-port-output-gain",
                    format!("output binding {index} has a non-finite gain"),
                ));
            }
        }
        Ok(())
    }
}

impl ConstitutiveWorldPlan {
    /// Upgrade an immutable modal `WorldPlan` v2 into the typed v3 schema.
    /// The original v2 plan and digest remain unchanged; the returned v3 plan
    /// receives its own content identity.
    pub fn from_modal_v2(plan: WorldPlan) -> Result<Self, PlanError> {
        plan.validate()?;
        Ok(Self {
            schema_version: WORLD_PLAN_V3_SCHEMA_VERSION.to_owned(),
            world_name: plan.world_name,
            internal_sample_rate_hz: plan.internal_sample_rate_hz,
            bodies: plan
                .bodies
                .into_iter()
                .map(|instance| ConstitutiveBodyInstanceSpec {
                    id: instance.id,
                    constitution: BodyConstitutionSpec::Modal {
                        body: instance.body,
                    },
                    space_port: instance.space_port,
                    pose: instance.pose,
                })
                .collect(),
            shared_space: plan.shared_space,
            direct_couplings: plan.direct_couplings,
        })
    }

    pub fn validate(&self) -> Result<(), PlanError> {
        if self.schema_version != WORLD_PLAN_V3_SCHEMA_VERSION {
            return Err(PlanError::new(
                "unsupported-world-plan-v3-schema",
                format!(
                    "expected {WORLD_PLAN_V3_SCHEMA_VERSION}, found {}",
                    self.schema_version
                ),
            ));
        }
        validate_id("world", &self.world_name)?;
        if !self.internal_sample_rate_hz.is_finite()
            || !(8_000.0..=384_000.0).contains(&self.internal_sample_rate_hz)
        {
            return Err(PlanError::new(
                "invalid-world-plan-v3-clock",
                "internal_sample_rate_hz must be finite in [8 kHz,384 kHz]",
            ));
        }
        if self.bodies.is_empty() || self.bodies.len() > MAX_BODY_INSTANCES {
            return Err(PlanError::new(
                "invalid-world-plan-v3-body-count",
                format!("world must contain 1..={MAX_BODY_INSTANCES} bodies"),
            ));
        }
        if self.direct_couplings.len() > MAX_DIRECT_COUPLINGS {
            return Err(PlanError::new(
                "too-many-world-plan-v3-couplings",
                "world exceeds coupling capacity",
            ));
        }
        self.shared_space.validate()?;
        let mut body_ids = BTreeSet::new();
        for body in &self.bodies {
            validate_id("body", &body.id)?;
            if !body_ids.insert(body.id.as_str()) {
                return Err(PlanError::new(
                    "duplicate-body-id",
                    format!("duplicate body id {:?}", body.id),
                ));
            }
            match &body.constitution {
                BodyConstitutionSpec::Modal { body } => {
                    body.validate(self.internal_sample_rate_hz)?;
                }
                BodyConstitutionSpec::LocalMedium { boundary } => {
                    boundary.validate(self.internal_sample_rate_hz)?;
                }
                BodyConstitutionSpec::ModalSpace { body } => {
                    body.validate(self.internal_sample_rate_hz)?;
                }
                BodyConstitutionSpec::Hammer { hammer } => {
                    hammer
                        .validate()
                        .map_err(|message| PlanError::new("invalid-hammer", message))?;
                    if body.space_port.coupling_weights.iter().any(|w| *w != 0.0) {
                        return Err(PlanError::new(
                            "hammer-space-port",
                            format!("hammer {:?} has no field to exchange with the shared space", body.id),
                        ));
                    }
                }
            }
            body.space_port.validate()?;
            body.pose.validate()?;
        }
        self.validate_hammers()?;
        let mut edge_ids = BTreeSet::new();
        for edge in &self.direct_couplings {
            validate_id("coupling", &edge.id)?;
            if !edge_ids.insert(edge.id.as_str()) {
                return Err(PlanError::new(
                    "duplicate-coupling-id",
                    format!("duplicate coupling id {:?}", edge.id),
                ));
            }
            if edge.left_body == edge.right_body
                || !body_ids.contains(edge.left_body.as_str())
                || !body_ids.contains(edge.right_body.as_str())
                || self.is_hammer(&edge.left_body)
                || self.is_hammer(&edge.right_body)
            {
                return Err(PlanError::new(
                    "invalid-coupling-endpoint",
                    format!("coupling {:?} must join two declared bodies", edge.id),
                ));
            }
            validate_direction("coupling left direction", &edge.left_direction)?;
            validate_direction("coupling right direction", &edge.right_direction)?;
            if !edge.rate_hz.is_finite()
                || edge.rate_hz < 0.0
                || edge.rate_hz > 0.25 * self.internal_sample_rate_hz
                || !edge.weight.is_finite()
                || edge.weight.abs() > 16.0
            {
                return Err(PlanError::new(
                    "invalid-coupling-law",
                    format!("coupling {:?} is outside the supported domain", edge.id),
                ));
            }
        }
        Ok(())
    }

    fn is_hammer(&self, id: &str) -> bool {
        self.bodies
            .iter()
            .any(|body| body.id == id && matches!(body.constitution, BodyConstitutionSpec::Hammer { .. }))
    }

    /// Every hammer strikes distinct stiff-wire bodies of this plan, inside
    /// their speaking length, and no wire is struck by two hammers.
    fn validate_hammers(&self) -> Result<(), PlanError> {
        let mut struck = BTreeSet::new();
        for body in &self.bodies {
            let BodyConstitutionSpec::Hammer { hammer } = &body.constitution else { continue };
            for contact in &hammer.wires {
                let wire = self.bodies.iter().find(|b| b.id == contact.body).and_then(|b| match &b.constitution {
                    BodyConstitutionSpec::ModalSpace { body } => match &body.configuration_space.map {
                        ConfigurationMapSpec::StiffWire { wire } => Some(wire),
                        _ => None,
                    },
                    _ => None,
                });
                let Some(wire) = wire else {
                    return Err(PlanError::new(
                        "invalid-hammer-wire",
                        format!("hammer {:?} strikes {:?}, which is not a stiff-wire body", body.id, contact.body),
                    ));
                };
                if !struck.insert(contact.body.as_str()) {
                    return Err(PlanError::new(
                        "invalid-hammer-wire",
                        format!("wire {:?} is struck twice", contact.body),
                    ));
                }
                if hammer.contact_width_m >= contact.strike_fraction * wire.speaking_length_m * 2.0 {
                    return Err(PlanError::new(
                        "invalid-hammer-contact",
                        format!("hammer {:?}'s felt is wider than twice its strike distance on {:?}", body.id, contact.body),
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String, PlanError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|error| {
            PlanError::new(
                "world-plan-v3-serialization",
                format!("failed to serialize constitutive world plan: {error}"),
            )
        })?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }
}
