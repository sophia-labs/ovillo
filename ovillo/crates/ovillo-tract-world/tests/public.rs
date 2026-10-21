//! The public engine (`TractNdAnatomy::public_v1`): every gesture of the
//! tube tract's ABI, touches, vibrato; the World's own ledger (no mocks).

use mus_ariadne::{ConnectionPolicy, DisposalPolicy};
use ovillo_tract_world::gestures::Gesture;
use ovillo_tract_world::world::{self as tw, Constriction, TractControls, TractWorldNd, Vibrato};
use ovillo_tract_world::TractNdAnatomy;

const SR: f64 = 48_000.0;

fn public() -> TractWorldNd {
    let mut w = TractWorldNd::with(SR, TractNdAnatomy::public_v1(), ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate }, tw::ART_REST).unwrap();
    w.set_controls(TractControls { articulation: tw::ART_REST, ..TractControls::default() });
    w
}

#[test]
fn every_gesture_closes_its_books() {
    for (id, name) in Gesture::NAMES.iter().enumerate() {
        let g = Gesture::from_index(id as u32).unwrap();
        let seconds = g.loop_s.map_or(g.keys.last().unwrap().at_s + 0.25, |l| 1.5 * l);
        let mut w = public();
        w.set_gesture(Some(g));
        let mut out = vec![0.0f32; (seconds * SR) as usize];
        w.process(&mut out).unwrap();
        let r = w.world().receipt();
        let t = w.totals();
        let peak = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        println!(
            "{name:<7} {seconds:.2} s: peak {peak:.3}, jet loss {:.2e} J, turbulence net {:.2e} J injected {:.2e} J, closed {} samples, moves {}, disposal {:.1e} J, max frame {:.1e} J, cumulative {:.1e} J",
            t.aero.jet_loss, t.aero.turbulence_work, t.aero.turbulence_injected, t.aero.closed_samples, w.stats().moves, w.world().disposal_total(), r.maximum_frame_residual, r.cumulative_closure_residual
        );
        assert!(out.iter().all(|v| v.is_finite()), "{name}");
        assert!(peak > 1.0e-3, "{name} must be audible");
        assert!(r.maximum_frame_residual < 4.0e-15, "{name}: {}", r.maximum_frame_residual);
        assert!(r.cumulative_closure_residual.abs() < 1.0e-12, "{name}: {}", r.cumulative_closure_residual);
        if (2..=6).contains(&id) {
            assert!(t.aero.turbulence_injected > 0.0 && t.aero.turbulence_work > 0.0, "{name} must hiss");
        }
        if [7, 8, 9, 10].contains(&id) {
            assert!(t.aero.closed_samples > 0, "{name} must close exactly");
        }
    }
}

#[test]
fn touches_anywhere_stay_bounded_and_release() {
    let mut w = public();
    w.set_controls(TractControls { articulation: tw::PUBLIC_ART_A, pitch_hz: 120.0, tenseness: 0.6, breath: 0.6 });
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut block = vec![0.0f32; 4_800];
    for _ in 0..20 {
        for slot in 1..5 {
            let degree = if next() < 0.3 { 1.0 } else { next() };
            w.set_constriction(slot, Some(Constriction { position: 0.42 + 0.58 * next(), degree }));
        }
        w.process(&mut block).unwrap();
        assert!(block.iter().all(|v| v.is_finite()));
    }
    for slot in 1..5 {
        w.set_constriction(slot, None);
    }
    for _ in 0..5 {
        w.process(&mut block).unwrap();
    }
    let r = w.world().receipt();
    println!("touches: closed {} samples, jet loss {:.2e} J, max frame {:.1e} J, cumulative {:.1e} J", w.totals().aero.closed_samples, w.totals().aero.jet_loss, r.maximum_frame_residual, r.cumulative_closure_residual);
    assert!((1..5).all(|slot| w.constriction(slot).is_none()), "every touch releases");
    assert!(r.maximum_frame_residual < 4.0e-15);
    assert!(w.totals().aero.closed_samples > 0);
}

#[test]
fn vibrato_is_booked_fold_retuning() {
    let mut w = public();
    w.set_controls(TractControls { articulation: tw::PUBLIC_ART_A, pitch_hz: 120.0, tenseness: 0.6, breath: 0.6 });
    let mut block = vec![0.0f32; 24_000];
    w.process(&mut block).unwrap();
    let before = w.stats();
    w.set_vibrato(Vibrato { rate_hz: 5.5, depth_semitones: 1.0, wobble: 0.3 });
    let mut f0 = Vec::new();
    for _ in 0..40 {
        let mut small = vec![0.0f32; 1_200];
        w.process(&mut small).unwrap();
        f0.push(w.observers().measured_f0_hz);
    }
    let after = w.stats();
    let (lo, hi) = f0.iter().fold((f64::MAX, f64::MIN), |(l, h), v| (l.min(*v), h.max(*v)));
    println!("vibrato: F0 {lo:.1}..{hi:.1} Hz over 1 s, retunes {} -> {}, valve work {:.2e} J", before.valve_retunes, after.valve_retunes, after.valve_work);
    assert!(after.valve_retunes > before.valve_retunes + 100);
    assert!(hi / lo > 1.03, "a 1-semitone vibrato must move F0");
    assert!(w.world().receipt().maximum_frame_residual < 4.0e-15);
}
