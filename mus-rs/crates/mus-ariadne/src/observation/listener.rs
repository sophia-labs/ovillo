use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ir::MODE_COUNT;
use crate::plan::{ConstitutiveWorldPlan, PlanError, WorldPlan};
use crate::shared_world::{World, WorldFrame};

pub const STEREO_LISTENER_VERSION: &str = "ariadne-stereo-listener-v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StereoBodyTap {
    pub body_id: String,
    pub left_projection: f64,
    pub right_projection: f64,
}

/// A linear readout of one body's modal state, `left = sum_k (ld_k q_k + lm_k
/// m_k)` and `right` likewise: for instance a microphone's far-field pressure
/// in one direction, from each mode's radiation pattern. It reads the state
/// after the World's step and never acts on the body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StereoStateTap {
    pub body_id: String,
    pub left_displacement: [f64; MODE_COUNT],
    pub left_momentum: [f64; MODE_COUNT],
    pub right_displacement: [f64; MODE_COUNT],
    pub right_momentum: [f64; MODE_COUNT],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StereoListenerSpec {
    pub schema_version: String,
    pub physical_world_digest: String,
    pub body_taps: Vec<StereoBodyTap>,
    pub space_left_projection: f64,
    pub space_right_projection: f64,
    pub output_scale: f64,
    /// Additive: a listener without state taps serializes (and digests) as before.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state_taps: Vec<StereoStateTap>,
}

impl StereoListenerSpec {
    pub fn all_bodies(
        plan: &WorldPlan,
        output_scale: f64,
    ) -> Result<Self, ListenerError> {
        let body_taps = plan
            .bodies
            .iter()
            .map(|body| {
                let pan = (0.5 + 0.35 * body.pose.position[0]).clamp(0.0, 1.0);
                StereoBodyTap {
                    body_id: body.id.clone(),
                    left_projection: (1.0 - pan).sqrt(),
                    right_projection: pan.sqrt(),
                }
            })
            .collect();
        let result = Self {
            schema_version: STEREO_LISTENER_VERSION.to_owned(),
            physical_world_digest: plan.digest()?,
            body_taps,
            space_left_projection: 1.0,
            space_right_projection: 1.0,
            output_scale,
            state_taps: Vec::new(),
        };
        result.validate(plan)?;
        Ok(result)
    }

    pub fn validate(&self, plan: &WorldPlan) -> Result<(), ListenerError> {
        plan.validate()?;
        let ids: Vec<&str> = plan.bodies.iter().map(|body| body.id.as_str()).collect();
        self.validate_parts(plan.digest()?, &ids)
    }

    /// A listener over a typed (plan-v3) world: the given emission and state
    /// taps, the shared space heard at unit projection on both sides.
    pub fn for_constitutive(
        plan: &ConstitutiveWorldPlan,
        body_taps: Vec<StereoBodyTap>,
        state_taps: Vec<StereoStateTap>,
        output_scale: f64,
    ) -> Result<Self, ListenerError> {
        let result = Self {
            schema_version: STEREO_LISTENER_VERSION.to_owned(),
            physical_world_digest: plan.digest()?,
            body_taps,
            space_left_projection: 1.0,
            space_right_projection: 1.0,
            output_scale,
            state_taps,
        };
        result.validate_constitutive(plan)?;
        Ok(result)
    }

    pub fn validate_constitutive(&self, plan: &ConstitutiveWorldPlan) -> Result<(), ListenerError> {
        plan.validate()?;
        let ids: Vec<&str> = plan.bodies.iter().map(|body| body.id.as_str()).collect();
        self.validate_parts(plan.digest()?, &ids)
    }

    fn validate_parts(&self, expected: String, body_ids: &[&str]) -> Result<(), ListenerError> {
        if self.schema_version != STEREO_LISTENER_VERSION {
            return Err(ListenerError::UnsupportedVersion(
                self.schema_version.clone(),
            ));
        }
        if self.physical_world_digest != expected {
            return Err(ListenerError::WorldDigestMismatch {
                expected,
                actual: self.physical_world_digest.clone(),
            });
        }
        if !self.output_scale.is_finite() || !(0.0..=64.0).contains(&self.output_scale) {
            return Err(ListenerError::InvalidScale);
        }
        if !self.space_left_projection.is_finite()
            || !self.space_right_projection.is_finite()
        {
            return Err(ListenerError::NonFiniteProjection);
        }
        let mut nonzero = self.space_left_projection != 0.0
            || self.space_right_projection != 0.0;
        for tap in &self.body_taps {
            if !body_ids.contains(&tap.body_id.as_str()) {
                return Err(ListenerError::UnknownBody(tap.body_id.clone()));
            }
            if !tap.left_projection.is_finite() || !tap.right_projection.is_finite() {
                return Err(ListenerError::NonFiniteProjection);
            }
            nonzero |= tap.left_projection != 0.0 || tap.right_projection != 0.0;
        }
        for tap in &self.state_taps {
            if !body_ids.contains(&tap.body_id.as_str()) {
                return Err(ListenerError::UnknownBody(tap.body_id.clone()));
            }
            let all = tap.left_displacement.iter().chain(&tap.left_momentum).chain(&tap.right_displacement).chain(&tap.right_momentum);
            if all.clone().any(|v| !v.is_finite()) {
                return Err(ListenerError::NonFiniteProjection);
            }
            nonzero |= all.into_iter().any(|v| *v != 0.0);
        }
        if !nonzero {
            return Err(ListenerError::Silent);
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String, ListenerError> {
        let canonical = serde_json::to_vec(self)
            .map_err(|error| ListenerError::Serialization(error.to_string()))?;
        Ok(format!("sha256:{:x}", Sha256::digest(canonical)))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StereoFrame {
    pub left: f64,
    pub right: f64,
}

#[derive(Debug, Clone)]
struct BoundStereoTap {
    body: usize,
    left: f64,
    right: f64,
}

#[derive(Debug, Clone)]
struct BoundStateTap {
    body: usize,
    tap: StereoStateTap,
}

#[derive(Debug, Clone)]
pub struct StereoListener {
    physical_world_digest: String,
    listener_digest: String,
    taps: Vec<BoundStereoTap>,
    state_taps: Vec<BoundStateTap>,
    space_left: f64,
    space_right: f64,
    scale: f64,
}

impl StereoListener {
    pub fn bind(spec: &StereoListenerSpec, plan: &WorldPlan) -> Result<Self, ListenerError> {
        spec.validate(plan)?;
        let ids: Vec<&str> = plan.bodies.iter().map(|body| body.id.as_str()).collect();
        Self::bind_parts(spec, &ids)
    }

    /// Bind to a typed (plan-v3) world.
    pub fn bind_constitutive(spec: &StereoListenerSpec, plan: &ConstitutiveWorldPlan) -> Result<Self, ListenerError> {
        spec.validate_constitutive(plan)?;
        let ids: Vec<&str> = plan.bodies.iter().map(|body| body.id.as_str()).collect();
        Self::bind_parts(spec, &ids)
    }

    fn bind_parts(spec: &StereoListenerSpec, body_ids: &[&str]) -> Result<Self, ListenerError> {
        let taps = spec
            .body_taps
            .iter()
            .map(|tap| {
                let body = body_ids
                    .iter()
                    .position(|id| *id == tap.body_id)
                    .ok_or_else(|| ListenerError::UnknownBody(tap.body_id.clone()))?;
                Ok(BoundStereoTap {
                    body,
                    left: tap.left_projection,
                    right: tap.right_projection,
                })
            })
            .collect::<Result<Vec<_>, ListenerError>>()?;
        let state_taps = spec
            .state_taps
            .iter()
            .map(|tap| {
                let body = body_ids
                    .iter()
                    .position(|id| *id == tap.body_id)
                    .ok_or_else(|| ListenerError::UnknownBody(tap.body_id.clone()))?;
                Ok(BoundStateTap { body, tap: tap.clone() })
            })
            .collect::<Result<Vec<_>, ListenerError>>()?;
        Ok(Self {
            physical_world_digest: spec.physical_world_digest.clone(),
            listener_digest: spec.digest()?,
            taps,
            state_taps,
            space_left: spec.space_left_projection,
            space_right: spec.space_right_projection,
            scale: spec.output_scale,
        })
    }

    #[must_use]
    pub fn physical_world_digest(&self) -> &str {
        &self.physical_world_digest
    }

    #[must_use]
    pub fn listener_digest(&self) -> &str {
        &self.listener_digest
    }

    pub fn observe(&self, world: &World, frame: &WorldFrame) -> Result<StereoFrame, ListenerError> {
        if world.plan_digest() != self.physical_world_digest {
            return Err(ListenerError::BoundToDifferentWorld);
        }
        let mut left = self.space_left * frame.space_emitted_left;
        let mut right = self.space_right * frame.space_emitted_right;
        for tap in &self.taps {
            let emission = world
                .body_emission(tap.body)
                .ok_or(ListenerError::BoundIndex)?;
            left += tap.left * emission;
            right += tap.right * emission;
        }
        for bound in &self.state_taps {
            let state = world.body_state(bound.body).ok_or(ListenerError::BoundIndex)?;
            let tap = &bound.tap;
            for k in 0..MODE_COUNT {
                let (q, m) = (state.displacement[k], state.momentum[k]);
                left += tap.left_displacement[k] * q + tap.left_momentum[k] * m;
                right += tap.right_displacement[k] * q + tap.right_momentum[k] * m;
            }
        }
        let output = StereoFrame {
            left: self.scale * left,
            right: self.scale * right,
        };
        if !output.left.is_finite() || !output.right.is_finite() {
            return Err(ListenerError::NonFiniteOutput);
        }
        Ok(output)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ListenerError {
    #[error(transparent)]
    Plan(#[from] PlanError),
    #[error("unsupported stereo-listener version {0:?}")]
    UnsupportedVersion(String),
    #[error("listener world digest mismatch: expected {expected}, found {actual}")]
    WorldDigestMismatch { expected: String, actual: String },
    #[error("listener references unknown body {0:?}")]
    UnknownBody(String),
    #[error("listener projections must be finite")]
    NonFiniteProjection,
    #[error("listener output scale must be finite in [0,64]")]
    InvalidScale,
    #[error("listener has no nonzero aperture")]
    Silent,
    #[error("listener is bound to a different physical world")]
    BoundToDifferentWorld,
    #[error("listener bound body index is invalid")]
    BoundIndex,
    #[error("listener output became non-finite")]
    NonFiniteOutput,
    #[error("listener serialization failed: {0}")]
    Serialization(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BodyControls;
    use crate::plan::WorldPlan;
    use crate::shared_world::WorldControls;

    #[test]
    fn listening_does_not_load_the_world() {
        let plan = WorldPlan::string_brass_v1(48_000.0);
        let listener = StereoListener::bind(
            &StereoListenerSpec::all_bodies(&plan, 6.0).unwrap(),
            &plan,
        )
        .unwrap();
        let mut listened = World::from_plan(plan.clone()).unwrap();
        let mut silent_observer = World::from_plan(plan).unwrap();
        let mut bodies = vec![BodyControls::default()];
        for tick in 0..2_048 {
            bodies[0].source_wave = if tick == 0 { 0.7 } else { 0.0 };
            let frame = listened
                .process_sample(&bodies, WorldControls::default())
                .unwrap();
            let _ = listener.observe(&listened, &frame).unwrap();
            silent_observer
                .process_sample(&bodies, WorldControls::default())
                .unwrap();
        }
        assert_eq!(listened.body_state(0), silent_observer.body_state(0));
        assert_eq!(listened.receipt(), silent_observer.receipt());
    }
}
