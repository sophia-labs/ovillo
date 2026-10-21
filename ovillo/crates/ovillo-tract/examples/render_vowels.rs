//! Render /a/ /i/ /u/, a sliding /a i u a/ and one "mama" from the tract World
//! to WAV, and write a JSON receipt: anatomy provenance, the ledger, and the
//! formants measured from the render against GARGANTA's rung-1 targets.
//!
//! ```text
//! cargo run --release -p ovillo-tract --example render_vowels -- [out-dir]
//! ```
//! WAVs go to `out-dir` (default `ovillo/renders/tract`, not committed); the
//! receipt goes to `out-dir/tract-receipt.json` and is copied to
//! `crates/ovillo-tract/receipts/render-receipt.json`.

use ovillo_tract::{measure, presets, Articulation, Gesture, TractControls, TractWorld};
use serde_json::{json, Value};
use std::path::PathBuf;

const SR: f64 = 48_000.0;

fn write_wav(path: &PathBuf, samples: &[f32]) {
    let spec = hound::WavSpec { channels: 1, sample_rate: SR as u32, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let mut w = hound::WavWriter::create(path, spec).expect("wav");
    for &s in samples {
        w.write_sample(s).unwrap();
    }
    w.finalize().unwrap();
}

fn ledger_json(w: &mut TractWorld) -> Value {
    let r = w.readings();
    let l = w.ledger();
    let f = r.flows;
    json!({
        "frames": l.frames,
        "sourceWork": l.source_work,
        "controlWork": l.control_work,
        "internalLoss": l.internal_loss,
        "radiation": l.radiation_loss,
        "initialEnergy": l.initial_energy,
        "finalEnergy": l.final_energy,
        "cumulativeClosureResidual": l.cumulative_residual,
        "maximumFrameResidual": l.max_abs_residual,
        "globalClosureResidual": l.global_residual(),
        "relativeGlobalResidual": l.global_residual().abs() / (l.source_work.abs() + l.control_work.abs() + 1e-30),
        "breakdown": {
            "muscleWork": f.muscle_work, "impactWork": f.impact_work, "lungSupply": f.lung_supply, "tractControlWork": f.tract_control_work,
            "foldControlWork": f.fold_control_work, "wallLoss": f.wall_loss, "jetLoss": f.jet_loss, "foldLoss": f.fold_loss,
            "radiationLips": f.radiation_lips, "radiationNose": f.radiation_nose, "foldAeroWork": f.fold_aero_work,
            "constrictionLoss": f.constriction_loss, "constrictionDissipation": f.constriction_dissipation, "turbulenceWork": f.turbulence_work,
        },
        "nonFiniteEvents": r.non_finite_events,
    })
}

fn render(w: &mut TractWorld, seconds: f64) -> Vec<f32> {
    let mut out = Vec::new();
    let mut block = [0.0f32; 128];
    for _ in 0..(seconds * SR / 128.0).ceil() as usize {
        w.process(&mut block);
        out.extend_from_slice(&block);
    }
    out
}

fn vowel(name: &str, art: Articulation, target: [f64; 3], dir: &PathBuf) -> Value {
    let mut w = TractWorld::new(SR);
    let tube = measure::spectrum_peaks(&w.impulse_response(art, 48_000, 1e-6), 2.0 * SR, 90.0, 4_500.0);
    let mut w = TractWorld::new(SR);
    let mut c = TractControls::default();
    c.articulation = presets::ART_REST;
    w.set_controls(c);
    w.snap_configuration();
    // Breathe in to the vowel: the articulators move there, then the breath comes.
    c.articulation = art;
    c.breath = 0.6;
    c.pitch_hz = 120.0;
    c.aspiration = 0.0;
    w.set_controls(c);
    let audio = render(&mut w, 1.4);
    write_wav(&dir.join(format!("tract-{name}.wav")), &audio);
    let x: Vec<f64> = audio.iter().map(|&v| f64::from(v)).collect();
    let tail = &x[x.len() - (0.4 * SR) as usize..];
    // Three 50 ms frames across the settled tail; report each and their median.
    let frames: Vec<Vec<measure::Formant>> = (0..3).map(|k| measure::lpc_formants(&tail[k * 6_000..k * 6_000 + 2_400], SR)).collect();
    let median = |i: usize| -> Option<f64> {
        let mut v: Vec<f64> = frames.iter().filter_map(|f| f.get(i).map(|q| q.hz)).collect();
        v.sort_by(f64::total_cmp);
        v.get(v.len() / 2).copied()
    };
    let lpc: Vec<Option<f64>> = (0..3).map(median).collect();
    let f0 = measure::f0_autocorrelation(&tail[..9_600], SR);
    let rms = (tail.iter().map(|v| v * v).sum::<f64>() / tail.len() as f64).sqrt();
    let err: Vec<Option<f64>> = lpc.iter().zip(target).map(|(m, t)| m.map(|m| 100.0 * (m - t) / t)).collect();
    println!("/{name}/  render LPC F1-F3 {:?}  tube {:?}  target {:?}  err% {:?}  F0 {f0:.1} Hz", lpc.iter().map(|v| v.map(f64::round)).collect::<Vec<_>>(), tube.iter().take(3).map(|v| v.round()).collect::<Vec<_>>(), target, err.iter().map(|e| e.map(|e| (e * 10.0).round() / 10.0)).collect::<Vec<_>>());
    json!({
        "vowel": name,
        "articulation": {"tongue_index": art.tongue_index, "tongue_diameter": art.tongue_diameter, "lips": art.lips, "velum": art.velum},
        "target_hz": target,
        "target_source": "mus-rs/crates/mus-dsp/src/body_profiles.rs VOWEL_*_MODES (GARGANTA rung-1 design profiles, not measurements)",
        "render_lpc_formants_hz": lpc,
        "render_lpc_frames": frames.iter().map(|f| f.iter().map(|q| json!({"hz": q.hz, "bandwidth_hz": q.bandwidth_hz})).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "render_vs_target_percent": err,
        "tube_resonances_hz": tube.iter().take(4).collect::<Vec<_>>(),
        "measured_f0_hz": f0,
        "fold_tuning_hz": 120.0,
        "breath": 0.6,
        "tail_rms": rms,
        "ledger": ledger_json(&mut w),
        "wav": format!("tract-{name}.wav"),
        "method": "LPC as GARGANTA _lpc_envelope_and_formants (16 kHz, pre-emphasis 0.97, Hamming, order 18, roots bw<800 Hz), median of three 50 ms frames from the last 0.4 s of a 1.4 s render; the tube column is the rigid-glottis impulse response of the same World",
    })
}

fn gesture(name: &str, g: Gesture, seconds: f64, dir: &PathBuf) -> Value {
    let mut w = TractWorld::new(SR);
    let mut c = TractControls::default();
    c.articulation = presets::ART_A;
    w.set_controls(c);
    w.snap_configuration();
    w.set_gesture(Some(g));
    let audio = render(&mut w, seconds);
    write_wav(&dir.join(format!("tract-{name}.wav")), &audio);
    let peak = audio.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    println!("{name}: {seconds:.1} s, peak {peak:.3}");
    json!({ "gesture": name, "seconds": seconds, "peak": peak, "ledger": ledger_json(&mut w), "wav": format!("tract-{name}.wav") })
}

/// A consonant gesture: WAV, ledger, turbulence vs the jets' dissipation,
/// spectral centroid of its noisiest 100 ms, and the peak intraoral pressure.
fn consonant(id: u32, dir: &PathBuf) -> Value {
    let name = Gesture::NAMES[id as usize];
    let mut w = TractWorld::new(SR);
    let mut c = TractControls::default();
    c.articulation = presets::ART_A;
    w.set_controls(c);
    w.snap_configuration();
    w.set_gesture(Gesture::from_index(id));
    let seconds = if id == 10 { 3.2 } else { 1.6 };
    let mut audio = Vec::new();
    let mut block = [0.0f32; 128];
    let mut peak_mouth = 0.0f64;
    let mut max_re = 0.0f64;
    let mut voiced_blocks = 0usize;
    let blocks = (seconds * SR / 128.0) as usize;
    for _ in 0..blocks {
        w.process(&mut block);
        audio.extend_from_slice(&block);
        let n = w.oral_sections();
        peak_mouth = peak_mouth.max(w.oral_pressure(n / 2));
        let r = w.readings();
        max_re = max_re.max(r.max_reynolds);
        if r.measured_f0_hz > 0.0 {
            voiced_blocks += 1;
        }
    }
    write_wav(&dir.join(format!("tract-{name}.wav")), &audio);
    let x: Vec<f64> = audio.iter().map(|&v| f64::from(v)).collect();
    // The 100 ms with the most energy.
    let win = 4_800;
    let (mut best, mut at) = (0.0, 0);
    let mut k = 0;
    while k + win <= x.len() {
        let e: f64 = x[k..k + win].iter().map(|v| v * v).sum();
        if e > best { best = e; at = k; }
        k += win / 2;
    }
    let n = 1 << 12;
    let db = measure::magnitude_db(&x[at..at + n.min(win)], n);
    let (mut num, mut den) = (0.0, 0.0);
    for (b, d) in db.iter().enumerate().skip(1) {
        let p = 10f64.powf(d / 10.0);
        num += p * b as f64 * SR / n as f64;
        den += p;
    }
    let f = w.flows();
    let peak = audio.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    println!("{name}: centroid {:.0} Hz, turbulence {:.2e} J of jet dissipation {:.2e} J, max Re {:.0}, peak mid-tract pressure {:.0} Pa, voiced {:.0}% of blocks, peak {:.3}", num / den, f.turbulence_work, f.constriction_dissipation, max_re, peak_mouth, 100.0 * voiced_blocks as f64 / blocks as f64, peak);
    json!({
        "gesture": name, "seconds": seconds, "peak": peak, "wav": format!("tract-{name}.wav"),
        "loudest_100ms_centroid_hz": num / den,
        "turbulence_work_j": f.turbulence_work, "turbulence_positive_j": f.turbulence_positive,
        "constriction_dissipation_j": f.constriction_dissipation, "constriction_net_loss_j": f.constriction_loss,
        "max_reynolds": max_re, "peak_mid_tract_pressure_pa": peak_mouth,
        "voiced_block_fraction": voiced_blocks as f64 / blocks as f64,
        "ledger": ledger_json(&mut w),
    })
}

/// Orbs: knock a silent /a/ tube, then a voiced /i/, at several places.
fn orbs(dir: &PathBuf) -> Value {
    let mut w = TractWorld::new(SR);
    let mut c = TractControls::default();
    c.articulation = presets::ART_A;
    w.set_controls(c);
    w.snap_configuration();
    let mut audio = Vec::new();
    let mut hits = Vec::new();
    for (k, pos) in [0.9, 0.6, 0.3, 0.02, 0.4].iter().enumerate() {
        audio.extend(render(&mut w, 0.35));
        w.impact(*pos, 2.0e-4);
        hits.push(json!({"t_s": audio.len() as f64 / SR, "position": pos, "energy_j": 2.0e-4, "voiced": false, "k": k}));
    }
    audio.extend(render(&mut w, 0.5));
    c.articulation = presets::ART_I;
    c.breath = 0.55;
    w.set_controls(c);
    for pos in [0.85, 0.5, 0.05, 0.4] {
        audio.extend(render(&mut w, 0.4));
        w.impact(pos, 2.0e-4);
        hits.push(json!({"t_s": audio.len() as f64 / SR, "position": pos, "energy_j": 2.0e-4, "voiced": true}));
    }
    audio.extend(render(&mut w, 0.6));
    write_wav(&dir.join("tract-orbs.wav"), &audio);
    let peak = audio.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    println!("orbs: {:.1} s, peak {peak:.3}, {} hits", audio.len() as f64 / SR, hits.len());
    json!({ "gesture": "orbs", "seconds": audio.len() as f64 / SR, "peak": peak, "hits": hits, "ledger": ledger_json(&mut w), "wav": "tract-orbs.wav" })
}

fn main() {
    let dir = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../renders/tract")));
    std::fs::create_dir_all(&dir).unwrap();
    let w = TractWorld::new(SR);
    let anatomy = w.anatomy().provenance();
    let vowels = vec![
        vowel("a", presets::ART_A, presets::TARGET_A_HZ, &dir),
        vowel("i", presets::ART_I, presets::TARGET_I_HZ, &dir),
        vowel("u", presets::ART_U, presets::TARGET_U_HZ, &dir),
    ];
    let mut gestures = vec![gesture("aiua", Gesture::aiua(), 3.2, &dir), gesture("mama", Gesture::mama(), 4.8, &dir)];
    gestures.push(orbs(&dir));
    let consonants: Vec<Value> = (2..=10u32).map(|id| consonant(id, &dir)).collect();
    let doc = json!({
        "schema": "ovillo.tract.render-receipt.v0",
        "credit": "after Neil Thapen's Pink Trombone (2017); implemented independently",
        "sampleRateHz": SR,
        "tractRateHz": 2.0 * SR,
        "oralSections": w.oral_sections(),
        "nasalSections": w.nasal_sections(),
        "velumSection": w.velum_section(),
        "ledgerSchema": "mus_watsn::WorkFrame per 128-sample block: dE = sourceWork (breathing muscles) + controlWork (area transport + fold retuning) - internalLoss (walls, jet, folds) - radiation (lips, nostrils)",
        "anatomy": anatomy.iter().map(|p| json!({"name": p.name, "value": p.value, "unit": p.unit, "label": p.label, "source": p.source})).collect::<Vec<_>>(),
        "vowels": vowels,
        "gestures": gestures,
        "consonants": consonants,
        "approximations": [
            "not a mus_ariadne::World instance: that runtime's bodies are 24-mode modal fields on a one-dimensional configuration path; the tract is a waveguide body built to the same constitution (state, lawful scattering, transport with receipts, one WorkLedger) in this crate",
            "plane-wave loss is frequency independent (GARGANTA's oracle has sqrt(f) wall loss)",
            "radiation is a parallel air-plug mass and radiation port matched to the oracle's low-ka limits; it departs above ka ~ 1",
            "the fold is one mass with a lumped mucosal delay (Titze-form intraglottal pressure), not a two-mass or continuum fold",
            "aerodynamic work on the folds is booked to the lung supply; its path through the glottis is not resolved",
            "closures stop at GARGANTA's 0.025 cm2 floor; exact closure (G2 rank change) is not implemented",
            "aspiration is a stochastic supply paid from the jet's dissipation, not a resolved turbulent flow",
            "the nasal tract's geometry is a design hypothesis; no nasal anatomy exists in this repository",
        ],
    });
    let text = serde_json::to_string_pretty(&doc).unwrap() + "\n";
    std::fs::write(dir.join("tract-receipt.json"), &text).unwrap();
    std::fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/receipts/render-receipt.json"), &text).unwrap();
    println!("wrote {}", dir.display());
}
