//! Laws of the tract World, checked on the World itself (no mocks).

use ovillo_tract::{measure, presets, Articulation, Gesture, TractControls, TractWorld};

const SR: f64 = 48_000.0;

fn render(w: &mut TractWorld, seconds: f64) -> Vec<f64> {
    let mut out = Vec::new();
    let mut block = [0.0f32; 128];
    for _ in 0..(seconds * SR / 128.0).ceil() as usize {
        w.process(&mut block);
        out.extend(block.iter().map(|&v| f64::from(v)));
    }
    out
}

fn voiced(art: Articulation) -> (TractWorld, Vec<f64>) {
    let mut w = TractWorld::new(SR);
    let mut c = TractControls::default();
    c.articulation = presets::ART_REST;
    w.set_controls(c);
    w.snap_configuration();
    c.articulation = art;
    c.breath = 0.6;
    c.pitch_hz = 120.0;
    c.aspiration = 0.0;
    w.set_controls(c);
    let out = render(&mut w, 1.4);
    (w, out)
}

fn assert_books_close(w: &TractWorld, what: &str) {
    let l = w.ledger();
    let scale = l.source_work.abs() + l.control_work.abs() + l.internal_loss + l.radiation_loss;
    assert!(scale > 0.0, "{what}: nothing happened");
    assert!(l.max_abs_residual < 1e-12, "{what}: frame residual {} J", l.max_abs_residual);
    assert!(l.global_residual().abs() < 1e-9 * scale.max(1e-3), "{what}: global residual {} J of {} J", l.global_residual(), scale);
}

#[test]
fn the_ledger_closes_while_speaking() {
    let mut w = TractWorld::new(SR);
    w.set_gesture(Some(Gesture::mama()));
    let out = render(&mut w, 5.0);
    assert!(out.iter().all(|v| v.is_finite()));
    assert!(out.iter().any(|v| v.abs() > 1e-3), "mama should be audible");
    assert_books_close(&w, "mama");
    let f = w.flows();
    // Internal transfers balance too: what the lungs gave went to the air and folds.
    assert!(f.lung_supply > 0.0 && f.jet_loss > 0.0 && f.radiation_lips > 0.0 && f.radiation_nose > 0.0);
    assert_eq!(w.readings().non_finite_events, 0);
}

#[test]
fn breath_zero_supplies_no_work_and_makes_no_sound() {
    let mut w = TractWorld::new(SR);
    let mut c = TractControls::default();
    c.breath = 0.0;
    w.set_controls(c);
    let out = render(&mut w, 0.5);
    assert!(out.iter().all(|&v| v == 0.0), "silence in, silence out");
    assert_eq!(w.flows().lung_supply, 0.0);
    assert_eq!(w.flows().radiation_lips + w.flows().radiation_nose, 0.0);
}

#[test]
fn block_segmentation_does_not_change_audio() {
    let run = |block: usize| {
        let mut w = TractWorld::new(SR);
        w.set_gesture(Some(Gesture::aiua()));
        let mut out = Vec::new();
        let mut buf = vec![0.0f32; block];
        for _ in 0..(48_000 / block) {
            w.process(&mut buf);
            out.extend_from_slice(&buf);
        }
        out
    };
    assert_eq!(run(128), run(32));
    assert_eq!(run(128), run(480));
}

#[test]
fn the_folds_self_oscillate_from_lung_pressure_and_f0_follows_tension() {
    let mut last = 0.0;
    for pitch in [80.0, 120.0, 200.0, 320.0] {
        let mut w = TractWorld::new(SR);
        let mut c = TractControls::default();
        c.articulation = presets::ART_A;
        c.breath = 0.6;
        c.pitch_hz = pitch;
        c.aspiration = 0.0;
        w.set_controls(c);
        w.snap_configuration();
        let out = render(&mut w, 1.0);
        let f0 = measure::f0_autocorrelation(&out[out.len() - 9_600..], SR);
        assert!(f0 > last, "F0 {f0} at tuning {pitch} should exceed {last}");
        assert!(f0 > 0.9 * pitch && f0 < 1.4 * pitch, "F0 {f0} far from tuning {pitch}");
        last = f0;
    }
}

#[test]
fn vowels_settle_near_their_targets() {
    for (name, art, target) in [
        ("a", presets::ART_A, presets::TARGET_A_HZ),
        ("i", presets::ART_I, presets::TARGET_I_HZ),
        ("u", presets::ART_U, presets::TARGET_U_HZ),
    ] {
        // The tube itself (rigid glottis): the presets were fit to this.
        let mut w = TractWorld::new(SR);
        let tube = measure::spectrum_peaks(&w.impulse_response(art, 48_000, 1e-6), 2.0 * SR, 90.0, 4_500.0);
        for k in 0..3 {
            let err = (tube[k] - target[k]).abs() / target[k];
            assert!(err < 0.03, "/{name}/ tube F{} {} vs {}", k + 1, tube[k], target[k]);
        }
        // The voiced render, measured from its audio.
        let (w, out) = voiced(art);
        assert_books_close(&w, name);
        let tail = &out[out.len() - 19_200..];
        let mut f1 = Vec::new();
        let mut f2 = Vec::new();
        let mut f3 = Vec::new();
        for k in 0..3 {
            let f = measure::lpc_formants(&tail[k * 6_000..k * 6_000 + 2_400], SR);
            assert!(f.len() >= 3, "/{name}/ frame {k}: {f:?}");
            f1.push(f[0].hz);
            f2.push(f[1].hz);
            f3.push(f[2].hz);
        }
        let med = |v: &mut Vec<f64>| {
            v.sort_by(f64::total_cmp);
            v[1]
        };
        let (m1, m2, m3) = (med(&mut f1), med(&mut f2), med(&mut f3));
        // An open glottis loads the tube and raises F1 (most for close vowels).
        assert!(m1 >= 0.97 * tube[0] && (m1 - target[0]).abs() / target[0] < 0.25, "/{name}/ F1 {m1} (tube {}, target {})", tube[0], target[0]);
        assert!((m2 - target[1]).abs() / target[1] < 0.15, "/{name}/ F2 {m2} vs {}", target[1]);
        assert!((m3 - target[2]).abs() / target[2] < 0.05, "/{name}/ F3 {m3} vs {}", target[2]);
    }
}

#[test]
fn no_blowup_across_control_corners() {
    // Every corner of the control box, entered abruptly, plus a fast random walk.
    let mut corners = Vec::new();
    for bits in 0u32..256 {
        let b = |k: u32| f64::from((bits >> k) & 1);
        corners.push(TractControls {
            articulation: Articulation { tongue_index: b(0), tongue_diameter: b(1), lips: b(2), velum: b(3) },
            pitch_hz: if b(4) > 0.0 { 420.0 } else { 60.0 },
            tenseness: b(5),
            breath: if b(6) > 0.0 { 1.0 } else { 0.15 },
            aspiration: b(7),
        });
    }
    let mut w = TractWorld::new(SR);
    let mut block = [0.0f32; 128];
    let mut peak_energy = 0.0f64;
    for c in &corners {
        w.set_controls(*c);
        for _ in 0..12 {
            w.process(&mut block);
            assert!(block.iter().all(|v| v.is_finite()), "corner {c:?}");
            peak_energy = peak_energy.max(w.tract_energy());
        }
    }
    // Bounded: the air never holds more than a few millijoules (the lungs hold 3 J).
    assert!(peak_energy < 5e-3, "peak acoustic energy {peak_energy} J");
    let mut seed = 0x1234_5678_9abc_def0u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    for _ in 0..2_000 {
        let c = TractControls {
            articulation: Articulation { tongue_index: next(), tongue_diameter: next(), lips: next(), velum: next() },
            pitch_hz: 60.0 + 360.0 * next(),
            tenseness: next(),
            breath: next(),
            aspiration: next(),
        };
        w.set_controls(c);
        w.process(&mut block);
        assert!(block.iter().all(|v| v.is_finite()));
        peak_energy = peak_energy.max(w.tract_energy());
    }
    assert!(peak_energy < 5e-3, "peak acoustic energy {peak_energy} J");
    let r = w.readings();
    assert_eq!(r.non_finite_events, 0);
    eprintln!("peak acoustic energy {peak_energy:.3e} J, safety-limited frames {} of {}", r.safety_limited_frames, (256 * 12 + 2_000) * 128);
    assert_books_close(&w, "corners and random walk");
    // Non-finite requests are sanitized, not obeyed.
    let mut bad = TractControls::default();
    bad.breath = f64::NAN;
    bad.pitch_hz = f64::INFINITY;
    bad.articulation.lips = -3.0;
    w.set_controls(bad);
    w.process(&mut block);
    assert!(block.iter().all(|v| v.is_finite()));
}

#[test]
fn an_orb_knocks_the_tube_and_the_books_still_close() {
    // Silent tract (no breath): an orb hit alone must sound, at the tube's
    // own resonances, and its work must appear on the books.
    let mut w = TractWorld::new(SR);
    let mut c = TractControls::default();
    c.articulation = presets::ART_A;
    c.breath = 0.0;
    w.set_controls(c);
    w.snap_configuration();
    render(&mut w, 0.05);
    w.impact(0.7, 2.0e-4);
    let out = render(&mut w, 0.6);
    let peak = out.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    assert!(peak > 1e-3, "the knock is audible: peak {peak}");
    let f = w.flows();
    assert!(f.impact_work > 1.0e-4 && f.impact_work < 3.0e-4, "impact work {}", f.impact_work);
    assert!(f.tract_control_work != 0.0, "the dent moved the walls");
    // No breath: the lungs supply nothing through the jet. The knock's
    // pressure still pushes the open folds, and that aerodynamic work is
    // booked to the lungs (the declared lumped approximation); bound it.
    assert_eq!(f.lung_supply, f.fold_aero_work);
    assert!(f.fold_aero_work.abs() < 0.05 * f.impact_work, "fold aero {} vs impact {}", f.fold_aero_work, f.impact_work);
    assert_books_close(&w, "orb knock");
    // A light knock (small dent) at the glottal end of a tube whose glottis
    // is pressed shut rings at the rigid-glottis tube's own F1 and F2. (With
    // the glottis open the boundary changes and so do the resonances; a knock
    // mid-tract sees the back cavity as a side branch, whose zeros move the
    // spectral maxima as a fricative source's do.)
    let mut w2 = TractWorld::new(SR);
    let tube = measure::spectrum_peaks(&w2.impulse_response(presets::ART_A, 48_000, 1e-6), 2.0 * SR, 90.0, 4_500.0);
    let mut w2 = TractWorld::new(SR);
    let mut pressed = c;
    pressed.tenseness = 1.0;
    w2.set_controls(pressed);
    w2.snap_configuration();
    render(&mut w2, 0.3);
    w2.impact(0.0, 2.0e-6);
    let knock = render(&mut w2, 0.5);
    let peaks = measure::spectrum_peaks(&knock, SR, 90.0, 4_500.0);
    for k in 0..2 {
        assert!(peaks.iter().any(|p| (p - tube[k]).abs() / tube[k] < 0.06), "knock peaks {peaks:?} vs tube {tube:?}");
    }
    // Many orbs during speech, including the larynx and velum.
    let mut w = TractWorld::new(SR);
    w.set_gesture(Some(Gesture::mama()));
    let mut block = [0.0f32; 128];
    for k in 0..400 {
        if k % 9 == 0 {
            w.impact((k as f64 * 0.137) % 1.0, 1.0e-4 + 1.0e-4 * ((k % 3) as f64));
        }
        w.process(&mut block);
        assert!(block.iter().all(|v| v.is_finite()));
    }
    assert!(w.impact_count() > 40);
    assert_books_close(&w, "orbs during mama");
}
