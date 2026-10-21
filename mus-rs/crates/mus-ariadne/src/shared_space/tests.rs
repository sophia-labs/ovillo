use super::*;
use crate::ir::WorldSpec;
use crate::plan::{SharedSpaceSpec, SpacePortSpec};

#[test]
fn several_ports_exchange_with_one_persistent_field() {
    let spec = WorldSpec::string_brass_v1().space;
    let mut space = SharedSpace::try_new(96_000.0, SharedSpaceSpec::from_space_spec(spec)).unwrap();
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

#[test]
fn running_stored_energy_tracks_the_exact_sum_over_many_laps() {
    let spec = WorldSpec::string_brass_v1().space;
    let mut space = SharedSpace::try_new(48_000.0, SharedSpaceSpec::from_space_spec(spec)).unwrap();
    let port = SpacePortSpec::from_space_spec(spec);
    let longest = space.longest_line();
    let mut seed = 0x2545_f491_u64;
    let mut noise = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
    };
    let controls = SharedSpaceControls {
        coupling: 0.9,
        decay: 0.7,
        radiation: 0.3,
    };
    let mut worst = 0.0_f64;
    for frame in 0..(3 * longest + 17) {
        let mut momentum = [0.0; crate::ir::MODE_COUNT];
        // Loud, then silent, so the field both fills and decays toward zero.
        if frame < longest {
            for value in &mut momentum {
                *value = 40.0 * noise();
            }
        }
        space.begin_frame().unwrap();
        let exchange = space.exchange_port(&mut momentum, &port, controls).unwrap();
        space
            .finish_frame(
                controls,
                exchange.signed_into_body,
                exchange.metric_residual,
            )
            .unwrap();
        let exact = space.stored_energy_exact();
        let running = space.stored_energy();
        let scale = exact.abs().max(1.0e-300);
        worst = worst.max((running - exact).abs() / scale);
        assert!(
            (running - exact).abs() <= 1.0e-9 * scale + 1.0e-30,
            "frame {frame}: running {running:e} vs exact {exact:e}"
        );
        assert!(space.activity().is_finite());
    }
    assert!(worst < 1.0e-9, "worst relative error {worst:e}");
}
