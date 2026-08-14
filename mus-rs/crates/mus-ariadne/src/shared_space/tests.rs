use super::*;
use crate::ir::WorldSpec;
use crate::plan::{SharedSpaceSpec, SpacePortSpec};

#[test]
fn several_ports_exchange_with_one_persistent_field() {
    let spec = WorldSpec::string_brass_v1().space;
    let mut space = SharedSpace::try_new(
        96_000.0,
        SharedSpaceSpec::from_space_spec(spec),
    )
    .unwrap();
    let port = SpacePortSpec::from_space_spec(spec);
    let mut left = [0.0; crate::ir::MODE_COUNT];
    let mut right = [0.0; crate::ir::MODE_COUNT];
    left[0] = 0.8;
    right[1] = -0.4;
    let before = 0.5 / 96_000.0
        * left
            .iter()
            .chain(right.iter())
            .map(|value| value * value)
            .sum::<f64>();
    space.begin_frame().unwrap();
    let a = space
        .exchange_port(&mut left, &port, SharedSpaceControls::default())
        .unwrap();
    let b = space
        .exchange_port(&mut right, &port, SharedSpaceControls::default())
        .unwrap();
    let frame = space
        .finish_frame(
            SharedSpaceControls {
                radiation: 0.0,
                decay: 1.0,
                ..Default::default()
            },
            a.signed_into_body + b.signed_into_body,
            a.metric_residual + b.metric_residual,
        )
        .unwrap();
    let after = 0.5 / 96_000.0
        * left
            .iter()
            .chain(right.iter())
            .map(|value| value * value)
            .sum::<f64>()
        + space.stored_energy();
    assert!(frame.coupling_metric_residual.abs() < 1.0e-14);
    assert!(after <= before + 1.0e-12);
}
