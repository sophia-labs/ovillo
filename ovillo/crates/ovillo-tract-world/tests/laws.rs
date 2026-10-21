//! Laws of the N-D tract World, on the real `mus_ariadne::World` (no mocks).

use mus_ariadne::{ConnectionPolicy, DisposalPolicy};
use ovillo_tract_world::world::{self as tw, Articulation, TractControls, TractWorldNd};
use ovillo_tract_world::{measure, modal, TractNdAnatomy};

const SR: f64 = 48_000.0;

fn world(initial: Articulation) -> TractWorldNd {
    TractWorldNd::with(SR, TractNdAnatomy::default(), ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate }, initial).unwrap()
}

fn render(w: &mut TractWorldNd, seconds: f64) -> Vec<f64> {
    (0..(seconds * SR) as usize).map(|_| w.tick().unwrap()).collect()
}

#[test]
fn the_ledger_closes_while_gliding_through_four_dimensions() {
    let mut w = world(tw::ART_A);
    let keys = [(tw::ART_A, 120.0), (tw::ART_I, 128.0), (tw::ART_U, 116.0), (tw::ART_M, 104.0), (tw::ART_A, 110.0)];
    let mut out = Vec::new();
    for (art, pitch) in keys {
        w.set_controls(TractControls { articulation: art, pitch_hz: pitch, tenseness: 0.6, breath: 0.6 });
        out.extend(render(&mut w, 0.5));
    }
    let r = w.world().receipt();
    let s = w.stats();
    println!("glide: moves {}, move work {:.3e} J, disposal {:.3e} J, valve retunes {}, max frame residual {:.3e} J, cumulative {:.3e} J", s.moves, s.control_work, s.disposal, s.valve_retunes, r.maximum_frame_residual, r.cumulative_closure_residual);
    assert!(out.iter().all(|v| v.is_finite()) && out.iter().any(|v| v.abs() > 1e-3));
    assert!(s.moves > 500 && s.valve_retunes > 100);
    // The lungs hold 3 J; the residuals are a few ulps of that.
    assert!(r.maximum_frame_residual < 4.0e-15, "{}", r.maximum_frame_residual);
    assert!(r.cumulative_closure_residual.abs() < 1.0e-12, "{}", r.cumulative_closure_residual);
    assert!(r.source_work > 0.0 && r.radiation > 0.0 && r.internal_loss > 0.0);
}

#[test]
fn no_breath_no_sound_no_work() {
    let mut w = world(tw::ART_A);
    w.set_controls(TractControls { articulation: tw::ART_I, pitch_hz: 120.0, tenseness: 0.6, breath: 0.0 });
    let out = render(&mut w, 0.4);
    let r = w.world().receipt();
    assert!(out.iter().all(|v| *v == 0.0), "a silent tract must stay exactly silent");
    assert_eq!(r.source_work, 0.0, "full lungs and no breath: the muscles do nothing");
    assert_eq!(r.radiation, 0.0);
}

#[test]
fn radiation_scale_zero_removes_radiation_exactly() {
    let mut w = world(tw::ART_A);
    w.set_radiation_scale(0.0);
    w.set_controls(TractControls { articulation: tw::ART_A, pitch_hz: 120.0, tenseness: 0.6, breath: 0.6 });
    render(&mut w, 0.3);
    let r = w.world().receipt();
    assert_eq!(r.radiation, 0.0);
    assert!(r.source_work > 0.0);
    assert!(r.maximum_frame_residual < 4.0e-15);
}

#[test]
fn the_velum_opens_the_nose() {
    let nose_share = |velum: f64| {
        let art = Articulation { velum, ..tw::ART_A };
        let mut w = world(art);
        w.set_controls(TractControls { articulation: art, pitch_hz: 120.0, tenseness: 0.6, breath: 0.6 });
        let mut lips = 0.0;
        let mut nose = 0.0;
        for i in 0..(0.6 * SR) as usize {
            w.tick().unwrap();
            if i > (0.3 * SR) as usize {
                let o = w.world().body_space_observation(0).unwrap();
                lips += o.end_radiation[0];
                nose += o.end_radiation[1];
            }
        }
        nose / (lips + nose)
    };
    // The port never closes exactly: it bottoms out at GARGANTA's 0.025 cm²
    // floor (exact closure is G2), so the nose leaks at velum 0. Compare with
    // the tube tract's own leak under the same procedure.
    let tube_share = |velum: f64| {
        use ovillo_tract as ot;
        let mut w = ot::TractWorld::new(SR);
        let mut c = ot::TractControls::default();
        c.articulation = ot::Articulation { velum, ..ot::presets::ART_A };
        c.breath = 0.6;
        c.aspiration = 0.0;
        w.set_controls(c);
        w.snap_configuration();
        let mut block = [0.0f32; 128];
        for _ in 0..(0.3 * SR / 128.0) as usize {
            w.process(&mut block);
        }
        let before = w.flows();
        for _ in 0..(0.3 * SR / 128.0) as usize {
            w.process(&mut block);
        }
        let after = w.flows();
        let nose = after.radiation_nose - before.radiation_nose;
        nose / (nose + after.radiation_lips - before.radiation_lips)
    };
    let closed = nose_share(0.0);
    let open = nose_share(1.0);
    let (tube_closed, tube_open) = (tube_share(0.0), tube_share(1.0));
    println!("nasal share of radiated energy: N-D World velum 0 {closed:.4}, velum 1 {open:.4}; tube tract velum 0 {tube_closed:.4}, velum 1 {tube_open:.4}");
    assert!(open > 2.0 * closed, "opening the velum must open the nose: {closed} -> {open}");
    assert!((closed / tube_closed).ln().abs() < 1.0_f64.ln() + 0.7, "closed-velum leak {closed} vs the tube tract's {tube_closed}");
}

/// The World's computed resonances sit on the tube tract's (two independent
/// discretizations of the same area function): within 2.5% for F1-F3.
#[test]
fn computed_modes_agree_with_the_tube_tracts_resonances() {
    use ovillo_tract as ot;
    for (art, name) in [(tw::ART_A, "a"), (tw::ART_I, "i"), (tw::ART_U, "u")] {
        let w = world(art);
        let modes = w.world().space_body(0).unwrap().network_modes().unwrap();
        let nd = modal::response_peaks(&modes, 90.0, 4_500.0);
        let a = ot::Articulation { tongue_index: art.tongue_index, tongue_diameter: art.tongue_diameter, lips: art.lips, velum: art.velum };
        let mut tube_world = ot::TractWorld::new(SR);
        let tube = measure::spectrum_peaks(&tube_world.impulse_response(a, 48_000, 1e-6), 2.0 * SR, 90.0, 4_500.0);
        println!("/{name}/ N-D {:?} tube {:?}", &nd[..3], &tube[..3]);
        for k in 0..3 {
            assert!((nd[k] / tube[k] - 1.0).abs() < 0.025, "/{name}/ F{}: {} vs {}", k + 1, nd[k], tube[k]);
        }
    }
}

#[test]
fn the_plan_is_content_addressed() {
    let a = TractNdAnatomy::default().plan(SR, ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate }, tw::ART_REST.point());
    let b = TractNdAnatomy::default().plan(SR, ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate }, tw::ART_REST.point());
    let c = TractNdAnatomy::default().plan(SR, ConnectionPolicy::ModalCoordinates, tw::ART_REST.point());
    assert_eq!(a.digest().unwrap(), b.digest().unwrap());
    assert_ne!(a.digest().unwrap(), c.digest().unwrap(), "the connection is part of the physical identity");
}
