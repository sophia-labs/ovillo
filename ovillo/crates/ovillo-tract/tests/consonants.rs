//! The consonant side: constrictions, fricatives, plosives, vibrato. Checked
//! on the World itself (no mocks).

use ovillo_tract::world::{Constriction, Vibrato};
use ovillo_tract::{measure, presets, Gesture, GlottisMode, TractControls, TractWorld};

const SR: f64 = 48_000.0;

fn render(w: &mut TractWorld, seconds: f64) -> Vec<f64> {
    let mut out = Vec::new();
    let mut block = [0.0f32; 128];
    for _ in 0..(seconds * SR / 128.0).ceil() as usize {
        w.process(&mut block);
        assert!(block.iter().all(|v| v.is_finite()));
        out.extend(block.iter().map(|&v| f64::from(v)));
    }
    out
}

fn assert_books_close(w: &TractWorld, what: &str) {
    let l = w.ledger();
    let scale = l.source_work.abs() + l.control_work.abs() + l.internal_loss + l.radiation_loss;
    assert!(l.max_abs_residual < 1e-12, "{what}: frame residual {} J", l.max_abs_residual);
    assert!(l.global_residual().abs() < 1e-9 * scale.max(1e-3), "{what}: global residual {} J of {} J", l.global_residual(), scale);
}

fn rms(x: &[f64]) -> f64 {
    (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt()
}

/// Spectral centroid (Hz) of a stretch of audio.
fn centroid(x: &[f64]) -> f64 {
    let n = 1 << 14;
    let db = measure::magnitude_db(&x[..n.min(x.len())], n);
    let (mut num, mut den) = (0.0, 0.0);
    for (k, d) in db.iter().enumerate().skip(1) {
        let p = 10f64.powf(d / 10.0);
        num += p * k as f64 * SR / n as f64;
        den += p;
    }
    num / den
}

#[test]
fn consonant_gestures_close_the_books() {
    for id in 2..=10u32 {
        let name = Gesture::NAMES[id as usize];
        let mut w = TractWorld::new(SR);
        let mut c = TractControls::default();
        c.articulation = presets::ART_A;
        w.set_controls(c);
        w.snap_configuration();
        w.set_gesture(Gesture::from_index(id));
        let out = render(&mut w, if id == 10 { 3.4 } else { 1.7 });
        assert!(rms(&out) > 1e-4, "{name} is audible");
        assert_books_close(&w, name);
        assert_eq!(w.readings().non_finite_events, 0, "{name}");
    }
}

#[test]
fn touch_constrictions_anywhere_stay_bounded_and_release() {
    let mut w = TractWorld::new(SR);
    let mut c = TractControls::default();
    c.articulation = presets::ART_A;
    c.breath = 0.7;
    w.set_controls(c);
    let mut peak = 0.0f64;
    let mut block = [0.0f32; 128];
    for p in 0..=14 {
        for &d in &[0.0, 0.3, 0.7, 0.9, 0.97, 1.0] {
            w.set_constriction(1, Some(Constriction { position: 0.44 + 0.04 * p as f64, degree: d }));
            for _ in 0..6 {
                w.process(&mut block);
                assert!(block.iter().all(|v| v.is_finite()));
                peak = peak.max(w.tract_energy());
            }
        }
    }
    // Four fingers wandering at once.
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    for _ in 0..1_500 {
        let slot = 1 + (next() * 4.0) as usize % 4;
        if next() < 0.15 {
            w.set_constriction(slot, None);
        } else {
            w.set_constriction(slot, Some(Constriction { position: 0.44 + 0.56 * next(), degree: next().powf(0.3) }));
        }
        w.process(&mut block);
        assert!(block.iter().all(|v| v.is_finite()));
        peak = peak.max(w.tract_energy());
    }
    assert!(peak < 2e-2, "peak acoustic energy {peak} J");
    assert_books_close(&w, "touches");
    // Release everything: the slots empty and the wall returns.
    for slot in 1..5 {
        w.set_constriction(slot, None);
    }
    render(&mut w, 0.2);
    for slot in 1..5 {
        assert!(w.constriction(slot).is_none(), "slot {slot} still held");
    }
    let r = w.readings();
    assert!(r.min_aperture_m2 > 0.02e-4, "aperture back to the vowel's: {} m²", r.min_aperture_m2);
    assert_books_close(&w, "release");
}

#[test]
fn a_fricatives_noise_is_paid_by_its_own_jet() {
    // /s/: voiceless, noisy, high; the turbulence never exceeds the jet's loss.
    let mut w = TractWorld::new(SR);
    w.set_gesture(Gesture::from_index(2));
    let out = render(&mut w, 1.1);
    let f = w.flows();
    assert!(f.turbulence_work > 0.0, "/s/ makes turbulence");
    assert!(f.turbulence_work <= f.constriction_dissipation, "noise {} > jet dissipation {}", f.turbulence_work, f.constriction_dissipation);
    assert!(f.constriction_loss >= 0.0);
    let tail = &out[out.len() - 19_200..];
    let c_s = centroid(tail);
    let r_s = w.readings();
    let s_f0 = r_s.measured_f0_hz;
    assert!(s_f0 == 0.0, "/s/ is voiceless (abducted folds): tracker {s_f0}");
    assert!(c_s > 2_500.0, "/s/ centroid {c_s} Hz");
    assert_books_close(&w, "sss");
    eprintln!("/s/: centroid {c_s:.0} Hz, turbulence {:.3e} J of {:.3e} J jet dissipation, max Re {:.0}", f.turbulence_work, f.constriction_dissipation, r_s.max_reynolds);

    // /z/: the same place, voiced: folds oscillate and the jet still hisses.
    let mut w = TractWorld::new(SR);
    w.set_gesture(Gesture::from_index(5));
    let out = render(&mut w, 1.1);
    let _ = out;
    let f0 = w.readings().measured_f0_hz;
    assert!(f0 > 60.0, "/z/ is voiced: the folds' own cycle tracker reads {f0} Hz");
    assert!(w.flows().turbulence_work > 0.0);
    assert!(w.flows().turbulence_work <= w.flows().constriction_dissipation);

    // An open vowel's mild narrowings lose little, so their noise (a share of
    // that loss) stays far below a fricative's.
    let s_turb = f.turbulence_work;
    let mut w = TractWorld::new(SR);
    let mut c = TractControls::default();
    c.articulation = presets::ART_A;
    c.breath = 0.6;
    c.aspiration = 0.0;
    w.set_controls(c);
    w.snap_configuration();
    render(&mut w, 1.1);
    let a_turb = w.flows().turbulence_work;
    assert!(a_turb < 0.02 * s_turb, "/a/ turbulence {a_turb} J vs /s/ {s_turb} J");
    eprintln!("/a/: turbulence {a_turb:.3e} J (max Re {:.0}) vs /s/ {s_turb:.3e} J", w.readings().max_reynolds);
}

#[test]
fn an_exact_closure_seals_and_pressure_builds_behind_it() {
    // Lips sealed, velum shut, breath on: no flow leaves, pressure rises.
    let mut w = TractWorld::new(SR);
    let mut c = TractControls::default();
    c.articulation = presets::ART_P;
    c.breath = 0.8;
    c.tenseness = 0.15;
    w.set_controls(c);
    w.snap_configuration();
    render(&mut w, 0.3);
    let before = w.flows();
    render(&mut w, 0.1);
    let after = w.flows();
    let radiated = (after.radiation_lips - before.radiation_lips) + (after.radiation_nose - before.radiation_nose);
    assert!(radiated < 1e-12, "a sealed tract radiates {radiated} J in 100 ms");
    let n = w.oral_sections();
    let p_mouth = w.oral_pressure(n - 2);
    let ps = w.readings().lung_pressure_pa;
    assert!(p_mouth > 0.5 * ps, "intraoral pressure {p_mouth} Pa vs lungs {ps} Pa");
    assert_books_close(&w, "sealed");
}

#[test]
fn a_plosive_burst_is_paid_by_the_pressure_stored_behind_the_closure() {
    // /t/: alveolar closure with breath on, then the glottis is made rigid at
    // the release so the only energy that can come out is what is stored.
    let mut w = TractWorld::new(SR);
    let mut c = TractControls::default();
    c.articulation = presets::ART_S;
    c.breath = 0.8;
    c.tenseness = 0.15;
    w.set_controls(c);
    w.snap_configuration();
    w.set_constriction(1, Some(Constriction { position: 0.86, degree: 1.0 }));
    render(&mut w, 0.25);
    let n = w.oral_sections();
    let closure = (0.86 * n as f64) as usize;
    let behind: f64 = (0..closure.saturating_sub(1)).map(|i| w.oral_section_energy(i)).sum();
    let total = w.tract_energy();
    assert!(behind > 1e-6, "pressure energy stored behind the closure: {behind} J");
    w.glottis_mode = GlottisMode::Rigid;
    let before = w.flows();
    w.set_constriction(1, None);
    let out = render(&mut w, 0.06);
    let after = w.flows();
    let burst = (after.radiation_lips - before.radiation_lips) + (after.radiation_nose - before.radiation_nose);
    let spent = burst + (after.wall_loss - before.wall_loss) + (after.constriction_loss - before.constriction_loss);
    assert!(burst > 0.0 && out.iter().any(|v| v.abs() > 1e-4), "the release is heard");
    assert!(burst <= behind, "burst {burst} J > stored {behind} J");
    assert!(spent <= total + 1e-12, "spent {spent} J > all stored {total} J");
    assert!(after.turbulence_positive > before.turbulence_positive, "the release is turbulent");
    assert_books_close(&w, "burst");
    eprintln!("/t/ burst: stored behind {behind:.3e} J (all {total:.3e}), radiated {burst:.3e} J, spent {spent:.3e} J");
}

#[test]
fn vibrato_moves_f0_and_is_booked_as_tension_work() {
    let mut w = TractWorld::new(SR);
    let mut c = TractControls::default();
    c.articulation = presets::ART_A;
    c.breath = 0.6;
    c.aspiration = 0.0;
    w.set_controls(c);
    w.snap_configuration();
    render(&mut w, 0.3);
    w.set_vibrato(Vibrato { rate_hz: 5.0, depth_semitones: 1.0, wobble: 0.5 });
    let before = w.flows().fold_control_work;
    let out = render(&mut w, 1.0);
    let f0s: Vec<f64> = (0..20).map(|k| measure::f0_autocorrelation(&out[k * 2_400..k * 2_400 + 2_400], SR)).collect();
    let (lo, hi) = f0s.iter().fold((f64::MAX, 0.0f64), |(a, b), &f| (a.min(f), b.max(f)));
    let semis = 12.0 * (hi / lo).log2();
    assert!(semis > 0.8, "vibrato spans {semis:.2} semitones ({lo:.1}..{hi:.1} Hz)");
    assert!(w.flows().fold_control_work != before, "retuning is booked");
    assert_books_close(&w, "vibrato");
}
