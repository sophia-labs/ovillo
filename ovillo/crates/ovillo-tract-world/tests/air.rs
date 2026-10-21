//! PoC (3) laws: the flow terms on the real `mus_ariadne::World` (no mocks).

use mus_ariadne::{ConnectionPolicy, DisposalPolicy};
use ovillo_tract_world::world::{self as tw, Articulation, TractControls, TractWorldNd};
use ovillo_tract_world::TractNdAnatomy;

const SR: f64 = 48_000.0;
/// The plan digest CHECKPOINT-2A recorded for this anatomy (44 + 31 volumes,
/// mass-momentum, 48 kHz, starting at rest).
const CHECKPOINT_DIGEST: &str = "sha256:5617b467134488c2747538b8899abffb7f659356a40eacaeb44f21b62921419c";

fn world(anatomy: TractNdAnatomy, initial: Articulation) -> TractWorldNd {
    TractWorldNd::with(SR, anatomy, ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate }, initial).unwrap()
}

/// Every flow term at zero: no contact, the aero law present with all its
/// coefficients off.
fn off(anatomy: TractNdAnatomy) -> TractNdAnatomy {
    let mut a = anatomy;
    a.contact = None;
    a.aero = a.aero.map(|mut s| {
        s.loss_scale = 0.0;
        s.dipole_efficiency = 0.0;
        s.dc_wall_loss = false;
        s
    });
    a
}

fn glide(mut w: TractWorldNd, keys: &[(Articulation, f64, f64, f64)]) -> (TractWorldNd, Vec<f32>) {
    let mut out = Vec::new();
    for &(art, tense, breath, seconds) in keys {
        w.set_controls(TractControls { articulation: art, pitch_hz: 120.0, tenseness: tense, breath });
        let mut block = vec![0.0f32; (seconds * SR) as usize];
        w.process(&mut block).unwrap();
        out.extend(block);
    }
    (w, out)
}

#[test]
fn without_flow_terms_the_world_is_the_checkpoints() {
    let linear = TractNdAnatomy::default();
    let plan = linear.plan(SR, ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate }, tw::ART_REST.point());
    assert_eq!(plan.digest().unwrap(), CHECKPOINT_DIGEST, "absent flow terms must leave the plan byte-identical");
    let keys = [(tw::ART_I, 0.6, 0.6, 0.4), (tw::ART_M, 0.6, 0.6, 0.3), (tw::ART_A, 0.6, 0.6, 0.3)];
    let (_, a) = glide(world(linear, tw::ART_A), &keys);
    let (_, b) = glide(world(off(linear.with_air()), tw::ART_A), &keys);
    assert!(a.iter().zip(&b).all(|(x, y)| x.to_bits() == y.to_bits()), "flow terms at zero must render the linear World bit for bit");
}

#[test]
fn books_close_and_the_jet_pays_for_its_noise() {
    let s = Articulation { tongue_index: 0.95, tongue_diameter: 1.0, lips: 0.6, velum: 0.0 };
    let p = Articulation { lips: 0.0, ..tw::ART_A };
    let keys = [(tw::ART_A, 0.6, 0.6, 0.3), (s, 0.0, 0.8, 0.4), (tw::ART_A, 0.6, 0.6, 0.2), (p, 0.6, 0.6, 0.15), (tw::ART_A, 0.6, 0.6, 0.25)];
    let (w, out) = glide(world(TractNdAnatomy::default().with_air(), tw::ART_A), &keys);
    let r = w.world().receipt();
    let t = w.totals();
    println!("air on: jet loss {:.3e} J, turbulence {:.3e} J, closed samples {}, disposal {:.3e} J, max frame residual {:.3e} J, cumulative {:.3e} J", t.aero.jet_loss, t.aero.turbulence_work, t.aero.closed_samples, w.world().disposal_total(), r.maximum_frame_residual, r.cumulative_closure_residual);
    assert!(out.iter().all(|v| v.is_finite()));
    assert!(r.maximum_frame_residual < 4.0e-15, "{}", r.maximum_frame_residual);
    assert!(r.cumulative_closure_residual.abs() < 1.0e-12);
    assert!(t.aero.jet_loss > 0.0 && t.aero.turbulence_work > 0.0);
    assert!(t.aero.turbulence_work < t.aero.jet_loss, "noise is a share of the jet's loss");
    assert!(t.aero.closed_samples > 0, "the lips must close exactly");
}

#[test]
fn pressure_builds_behind_a_closure() {
    let p = Articulation { lips: 0.0, ..tw::ART_A };
    let mut w = world(TractNdAnatomy::default().with_air(), tw::ART_A);
    w.set_controls(TractControls { articulation: tw::ART_A, pitch_hz: 120.0, tenseness: 0.6, breath: 0.6 });
    let mut block = vec![0.0f32; (0.2 * SR) as usize];
    w.process(&mut block).unwrap();
    w.set_controls(TractControls { articulation: p, pitch_hz: 120.0, tenseness: 0.6, breath: 0.6 });
    let behind = w.anatomy().oral_sections / 2;
    let mut peak = 0.0_f64;
    let mut closed = 0;
    for _ in 0..(0.2 * SR) as usize {
        w.tick().unwrap();
        let o = w.world().body_space_observation(0).unwrap();
        if o.aero.jet_closed {
            closed += 1;
            peak = peak.max(w.world().space_body(0).unwrap().node_pressure(behind).unwrap());
        }
    }
    println!("closed {closed} samples, peak oral pressure {peak:.1} Pa (supply {} Pa)", w.world().body_space_observation(0).unwrap().valve.supply_pressure_pa);
    assert!(closed > 2_400, "the lips reach contact about 0.1 s after the command");
    assert!(peak > 300.0, "pressure must build behind the lips: {peak} Pa");
}
