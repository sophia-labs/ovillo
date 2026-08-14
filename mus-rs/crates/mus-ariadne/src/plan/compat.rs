use crate::ir::WorldSpec;

use super::{
    BodyInstanceSpec, BodyPose, BodySpec, ObserverBodyTap, ObserverSpec,
    SharedSpaceSpec, SpacePortSpec, WorldPlan,
};

impl WorldPlan {
    /// Exact one-body embedding of the validated `WorldSpec` surface.
    #[must_use]
    pub fn from_single_body(spec: WorldSpec) -> Self {
        let body = BodySpec::from_world_spec(&spec);
        let shared_space = SharedSpaceSpec::from_space_spec(spec.space);
        let space_port = SpacePortSpec::from_space_spec(spec.space);
        let output_scale = spec.radiation.observation_scale;
        Self::skeleton(
            spec.anatomy_name.clone(),
            spec.internal_sample_rate_hz,
            vec![BodyInstanceSpec {
                id: "body/0".to_owned(),
                body,
                space_port,
                pose: BodyPose::default(),
            }],
            shared_space,
            vec![ObserverSpec {
                id: "observer/main".to_owned(),
                body_taps: vec![ObserverBodyTap {
                    body_id: "body/0".to_owned(),
                    left_projection: 1.0,
                    right_projection: 1.0,
                }],
                space_left_projection: 1.0,
                space_right_projection: 1.0,
                output_scale,
            }],
            "observer/main".to_owned(),
        )
    }
}
