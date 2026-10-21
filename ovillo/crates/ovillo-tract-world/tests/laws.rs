//! Laws of the N-D tract World, on the real `mus_ariadne::World` (no mocks).

use mus_ariadne::{ConnectionPolicy, DisposalPolicy};
use ovillo_tract_world::world::{self as tw, Articulation, TractControls, TractWorldNd};
use ovillo_tract_world::{modal, TractNdAnatomy};

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
    // The port never closes exactly here: it bottoms out at GARGANTA's
    // 0.025 cm² floor (this is the linear anatomy; POC-3's contact seals it).
    // The tube tract at f43f372 leaked the same way: its receipt
    // (`ovillo-tract/receipts/render-receipt.json` at f43f372, vowel /a/)
    // books radiationNose 2.908e-5 J against radiationLips 5.830e-4 J, a
    // 4.75% share, and 10.95% with the velum open (measured at the
    // checkpoint with that build). Since fe4fb03 that tube tract seals the
    // velum exactly, so the live comparison is pinned to those numbers.
    let closed = nose_share(0.0);
    let open = nose_share(1.0);
    let (tube_closed, tube_open) = (2.9078432390458533e-5 / (2.9078432390458533e-5 + 5.829516271642762e-4), 0.1095);
    println!("nasal share of radiated energy: N-D World velum 0 {closed:.4}, velum 1 {open:.4}; tube tract at f43f372 velum 0 {tube_closed:.4}, velum 1 {tube_open:.4}");
    assert!(open > 2.0 * closed, "opening the velum must open the nose: {closed} -> {open}");
    assert!((closed / tube_closed).ln().abs() < 0.7, "closed-velum leak {closed} vs the tube tract's {tube_closed}");
}

/// The World's computed resonances sit on the tube tract's (two independent
/// discretizations of the same area function): within 2.5% for F1-F3. The
/// tube's peaks are its rigid-glottis impulse response at f43f372, recorded
/// in `ovillo-tract/receipts/preset-fit.json` there (`tube_resonances_hz`);
/// since fe4fb03 that tube tract seals the velum and was re-fitted, so the
/// comparison is pinned to the numbers the checkpoint measured.
#[test]
fn computed_modes_agree_with_the_tube_tracts_resonances() {
    let tube_f43f372 = [[717.7734375, 1157.2265625, 2545.8984375], [292.96875, 2267.578125, 2950.1953125], [304.6875, 703.125, 2396.484375]];
    for ((art, name), tube) in [(tw::ART_A, "a"), (tw::ART_I, "i"), (tw::ART_U, "u")].into_iter().zip(tube_f43f372) {
        let w = world(art);
        let modes = w.world().space_body(0).unwrap().network_modes().unwrap();
        let nd = modal::response_peaks(&modes, 90.0, 4_500.0);
        println!("/{name}/ N-D {:?} tube@f43f372 {:?}", &nd[..3], tube);
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
