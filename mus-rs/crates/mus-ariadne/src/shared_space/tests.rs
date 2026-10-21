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
    let controls = SharedSpaceControls { coupling: 0.9, decay: 0.7, radiation: 0.3 };
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
            .finish_frame(controls, exchange.signed_into_body, exchange.metric_residual)
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

#[test]
fn a_compiled_port_exchanges_bit_for_bit_as_the_spec_port() {
    use super::CompiledSpacePort;
    let spec = crate::plan::WorldPlan::string_brass_v1(48_000.0);
    let port = spec.bodies[0].space_port;
    let mut plain = SharedSpace::try_new(48_000.0, spec.shared_space).unwrap();
    let mut compiled = SharedSpace::try_new(48_000.0, spec.shared_space).unwrap();
    let mut compiled_port = CompiledSpacePort::compile(&port).unwrap();
    let mut a: [f64; crate::ir::MODE_COUNT] = std::array::from_fn(|i| 0.3 * (i as f64 * 0.7).sin());
    let mut b = a;
    for step in 0..2_000 {
        let controls = SharedSpaceControls { coupling: 0.2 + 0.6 * ((step / 500) as f64 / 4.0), ..Default::default() };
        plain.begin_frame().unwrap();
        compiled.begin_frame().unwrap();
        let x = plain.exchange_port(&mut a, &port, controls).unwrap();
        let y = compiled.exchange_compiled_port(&mut b, &mut compiled_port, controls).unwrap();
        assert_eq!(x, y, "step {step}");
        assert_eq!(a, b, "step {step}");
        let fx = plain.finish_frame(controls, x.signed_into_body, x.metric_residual).unwrap();
        let fy = compiled.finish_frame(controls, y.signed_into_body, y.metric_residual).unwrap();
        assert_eq!(fx, fy);
    }
}
