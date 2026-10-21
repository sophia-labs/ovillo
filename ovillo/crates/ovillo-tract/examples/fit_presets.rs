//! Fit the vowel presets: search tongue index, tongue diameter and lips so the
//! rigid-glottis tube's resonances land near GARGANTA's rung-1 vowel bodies
//! (`mus-dsp/src/body_profiles.rs`, design profiles). Writes
//! `receipts/preset-fit.json`.
//!
//! `cargo run --release -p ovillo-tract --example fit_presets`
use ovillo_tract::{measure, presets, Articulation, TractWorld};
use serde_json::json;

const SR: f64 = 48_000.0;

fn peaks(w: &mut TractWorld, art: Articulation) -> Vec<f64> {
    let ir = w.impulse_response(art, 16_384, 1e-6);
    measure::spectrum_peaks_n(&ir, 2.0 * SR, 90.0, 4_500.0, 1 << 15)
}

fn cost(p: &[f64], target: &[f64; 3]) -> f64 {
    if p.len() < 3 {
        return 1e9;
    }
    target.iter().zip(p).map(|(t, m)| (m / t).ln().powi(2)).sum()
}

fn main() {
    let mut w = TractWorld::new(SR);
    let mut receipts = Vec::new();
    // /u/ with the velum exactly sealed cannot reach its target (F3 goes to
    // ~3.2 kHz); a small velopharyngeal opening is part of its fit.
    for (name, target, velums) in [
        ("a", presets::TARGET_A_HZ, &[0.0][..]),
        ("i", presets::TARGET_I_HZ, &[0.0][..]),
        ("u", presets::TARGET_U_HZ, &[0.0, 0.015, 0.025, 0.04][..]),
    ] {
        let mut best = (f64::MAX, Articulation::default(), Vec::new());
        let mut evals = 0usize;
        // Coarse grid, then two local refinements.
        let mut grid: Vec<Articulation> = Vec::new();
        for &v in velums { for ti in 0..=10 { for td in 0..=10 { for li in 1..=10 {
            grid.push(Articulation { tongue_index: ti as f64 / 10.0, tongue_diameter: td as f64 / 10.0, lips: li as f64 / 10.0, velum: v });
        }}}}
        for a in grid { let p = peaks(&mut w, a); evals += 1; let c = cost(&p, &target); if c < best.0 { best = (c, a, p); } }
        for step in [0.025f64, 0.01] {
            let c0 = best.1;
            for di in -4..=4 { for dd in -4..=4 { for dl in -4..=4 {
                let a = Articulation {
                    tongue_index: (c0.tongue_index + di as f64 * step).clamp(0.0, 1.0),
                    tongue_diameter: (c0.tongue_diameter + dd as f64 * step).clamp(0.0, 1.0),
                    lips: (c0.lips + dl as f64 * step).clamp(0.02, 1.0),
                    velum: c0.velum,
                };
                let p = peaks(&mut w, a); evals += 1; let c = cost(&p, &target); if c < best.0 { best = (c, a, p); }
            }}}
        }
        let (c, a, p) = best;
        println!("{name}: velum {:.3} index {:.3} diameter {:.3} lips {:.3}  peaks {:?}  target {:?}  rms log err {:.3}", a.velum, a.tongue_index, a.tongue_diameter, a.lips, p.iter().take(4).map(|v| v.round()).collect::<Vec<_>>(), target, (c / 3.0).sqrt());
        receipts.push(json!({
            "vowel": name, "target_hz": target, "target_source": "mus-rs/crates/mus-dsp/src/body_profiles.rs (GARGANTA rung-1 design profiles, not measurements)",
            "fitted": {"tongue_index": a.tongue_index, "tongue_diameter": a.tongue_diameter, "lips": a.lips, "velum": a.velum},
            "tube_resonances_hz": p.iter().take(4).collect::<Vec<_>>(), "rms_log_error": (c / 3.0).sqrt(), "evaluations": evals,
        }));
    }
    let doc = json!({
        "schema": "ovillo.tract.preset-fit.v0",
        "label": "fit",
        "method": "grid 11x11x10 (x4 velum openings for /u/: 0, 0.015, 0.025, 0.04) then 9x9x9 refinements at 0.025 and 0.01; objective sum over F1..F3 of ln(measured/target)^2; measured = peaks of the rigid-glottis impulse response of the tract World itself (spectrum_peaks_n, 2^15 FFT at 96 kHz)",
        "anatomy": "tract-v0 (TractAnatomy::adult_v0)",
        "vowels": receipts,
    });
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/receipts/preset-fit.json");
    std::fs::write(path, serde_json::to_string_pretty(&doc).unwrap() + "\n").unwrap();
    println!("wrote {path}");
}
