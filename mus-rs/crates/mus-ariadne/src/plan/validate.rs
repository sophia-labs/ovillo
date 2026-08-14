use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use crate::ir::{MODE_COUNT, SPACE_COURSES, TRANSPORT_PAIRS};

use super::helpers::*;
use super::{
    BodyPose, BodySpec, PlanError, SharedSpaceSpec, SpacePortSpec, WorldPlan,
    MAX_BODY_INSTANCES, MAX_DIRECT_COUPLINGS, WORLD_PLAN_SCHEMA_VERSION,
};

const INDEPENDENCE_EPS: f64 = 1.0e-8;
const MAX_WEIGHT: f64 = 16.0;

impl BodyPose {
    pub(crate) fn validate(self) -> Result<(), PlanError> {
        if self
            .position
            .iter()
            .chain(self.orientation.iter())
            .any(|value| !value.is_finite())
        {
            return Err(PlanError::new(
                "invalid-body-pose",
                "body pose values must be finite",
            ));
        }
        let norm = self
            .orientation
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        if norm <= 1.0e-12 {
            return Err(PlanError::new(
                "invalid-body-orientation",
                "body orientation quaternion must be nondegenerate",
            ));
        }
        Ok(())
    }
}

impl BodySpec {
    pub(crate) fn validate(&self, sample_rate_hz: f64) -> Result<(), PlanError> {
        if self.anatomy_name.trim().is_empty() {
            return Err(PlanError::new(
                "empty-body-anatomy",
                "body anatomy_name must be nonempty",
            ));
        }
        let path = &self.configuration_path;
        if path.coordinate.trim().is_empty()
            || path.left_name.trim().is_empty()
            || path.right_name.trim().is_empty()
            || !path.initial_amount.is_finite()
            || !(0.0..=1.0).contains(&path.initial_amount)
        {
            return Err(PlanError::new(
                "invalid-body-configuration-path",
                "configuration path names must be nonempty and initial_amount must be in [0,1]",
            ));
        }
        // Validate the complete authored path rather than only its endpoints.
        for step in 0..=32 {
            path.configuration_at(step as f64 / 32.0)
                .validate(sample_rate_hz)
                .map_err(|error| {
                    PlanError::new("invalid-body-configuration", error.to_string())
                })?;
        }
        validate_direction("body source direction", &self.source_direction)?;
        validate_independent("body active directions", &self.active_directions)?;
        for pair in 0..TRANSPORT_PAIRS {
            let (left, right) = self.transport.pairs[pair];
            if left == right || left >= MODE_COUNT || right >= MODE_COUNT {
                return Err(PlanError::new(
                    "invalid-body-transport-pair",
                    format!("transport pair {pair} is outside the {MODE_COUNT}-mode body"),
                ));
            }
            let weight = self.transport.weights[pair];
            if !weight.is_finite() || weight.abs() > MAX_WEIGHT {
                return Err(PlanError::new(
                    "invalid-body-transport-weight",
                    format!(
                        "transport weight {pair} must be finite with abs <= {MAX_WEIGHT}"
                    ),
                ));
            }
        }
        if !self.reservoir.capacity.is_finite() || self.reservoir.capacity <= 0.0 {
            return Err(PlanError::new(
                "invalid-body-reservoir",
                "body reservoir capacity must be finite and positive",
            ));
        }
        Ok(())
    }
}

impl SharedSpaceSpec {
    pub(crate) fn validate(self) -> Result<(), PlanError> {
        for (course, delay) in self.delays_seconds.into_iter().enumerate() {
            if !delay.is_finite() || !(0.000_25..=2.0).contains(&delay) {
                return Err(PlanError::new(
                    "invalid-shared-space-delay",
                    format!(
                        "shared-space course {course} delay {delay} is outside [0.25 ms,2 s]"
                    ),
                ));
            }
        }
        validate_direction(
            "shared-space scattering direction",
            &self.scattering_direction,
        )?;
        validate_direction("shared-space left radiation", &self.radiation_left)?;
        validate_direction("shared-space right radiation", &self.radiation_right)?;
        let correlation = normalized_dot(&self.radiation_left, &self.radiation_right)?;
        if correlation.abs() >= 1.0 - INDEPENDENCE_EPS {
            return Err(PlanError::new(
                "dependent-shared-space-radiation",
                "shared-space left/right radiation apertures must be linearly independent",
            ));
        }
        Ok(())
    }
}

impl SpacePortSpec {
    pub(crate) fn validate(self) -> Result<(), PlanError> {
        for course in 0..SPACE_COURSES {
            validate_direction(
                "body shared-space port",
                &self.body_directions[course],
            )?;
            let weight = self.coupling_weights[course];
            if !weight.is_finite() || weight.abs() > MAX_WEIGHT {
                return Err(PlanError::new(
                    "invalid-space-port-weight",
                    format!(
                        "space-port weight {course} must be finite with abs <= {MAX_WEIGHT}"
                    ),
                ));
            }
        }
        Ok(())
    }
}

impl WorldPlan {
    pub fn validate(&self) -> Result<(), PlanError> {
        if self.schema_version != WORLD_PLAN_SCHEMA_VERSION {
            return Err(PlanError::new(
                "unsupported-world-plan-schema",
                format!(
                    "expected {WORLD_PLAN_SCHEMA_VERSION}, found {}",
                    self.schema_version
                ),
            ));
        }
        validate_id("world", &self.world_name)?;
        if !self.internal_sample_rate_hz.is_finite()
            || !(8_000.0..=384_000.0).contains(&self.internal_sample_rate_hz)
        {
            return Err(PlanError::new(
                "invalid-world-plan-clock",
                "internal_sample_rate_hz must be finite in [8 kHz,384 kHz]",
            ));
        }
        if self.bodies.is_empty() || self.bodies.len() > MAX_BODY_INSTANCES {
            return Err(PlanError::new(
                "invalid-body-count",
                format!("world must contain 1..={MAX_BODY_INSTANCES} bodies"),
            ));
        }
        if self.direct_couplings.len() > MAX_DIRECT_COUPLINGS {
            return Err(PlanError::new(
                "too-many-couplings",
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
            body.body.validate(self.internal_sample_rate_hz)?;
            body.space_port.validate()?;
            body.pose.validate()?;
        }

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
            {
                return Err(PlanError::new(
                    "invalid-coupling-endpoint",
                    format!(
                        "coupling {:?} must join two distinct declared bodies",
                        edge.id
                    ),
                ));
            }
            validate_direction("coupling left direction", &edge.left_direction)?;
            validate_direction("coupling right direction", &edge.right_direction)?;
            if !edge.rate_hz.is_finite()
                || edge.rate_hz < 0.0
                || edge.rate_hz > 0.25 * self.internal_sample_rate_hz
                || !edge.weight.is_finite()
                || edge.weight.abs() > MAX_WEIGHT
            {
                return Err(PlanError::new(
                    "invalid-coupling-law",
                    format!(
                        "coupling {:?} rate/weight is outside the supported domain",
                        edge.id
                    ),
                ));
            }
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String, PlanError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|error| {
            PlanError::new(
                "world-plan-serialization",
                format!("failed to serialize world plan: {error}"),
            )
        })?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }
}
