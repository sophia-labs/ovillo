use crate::ir::WorldSpec;

use super::{
    BodyInstanceSpec, BodyPose, BodySpec, SharedSpaceSpec, SpacePortSpec,
    WorldPlan,
};

impl WorldPlan {
    /// First canonical physical world: one persistent string/brass-capable body
    /// coupled to one shared delayed field. Observation is intentionally not
    /// part of the plan.
    #[must_use]
    pub fn string_brass_v1(sample_rate_hz: f64) -> Self {
        let mut template = WorldSpec::string_brass_v1();
        template.internal_sample_rate_hz = sample_rate_hz;
        let body = BodySpec::from_template(&template);
        let shared_space = SharedSpaceSpec::from_space_spec(template.space);
        let space_port = SpacePortSpec::from_space_spec(template.space);
        Self::skeleton(
            template.anatomy_name.clone(),
            sample_rate_hz,
            vec![BodyInstanceSpec {
                id: "body/0".to_owned(),
                body,
                space_port,
                pose: BodyPose::default(),
            }],
            shared_space,
        )
    }
}
