//! Fit the public engine's vowel presets (a fit, with its receipt): the
//! World's own computed resonances (`modal::response_peaks`, the counterpart
//! of the tube tract's rigid-glottis response) against GARGANTA's rung-1
//! targets, objective sum over F1-F3 of ln(measured/target)^2 (the tube
//! tract's `fit_presets.rs` objective), on grids around ovillo-tract fe4fb03's
//! sealed-velum re-fit. Writes `receipts/public-preset-fit.json`.
use mus_ariadne::{ConnectionPolicy, DisposalPolicy};
use ovillo_tract_world::world::{self as tw, Articulation, TractWorldNd};
use ovillo_tract_world::{modal, TractNdAnatomy};
use serde_json::json;

fn main() {
    let sr = 48_000.0;
    let mut w = TractWorldNd::with(sr, TractNdAnatomy::public_v1(), ConnectionPolicy::ModalCoordinates, tw::ART_REST).unwrap();
    let dims = w.anatomy().dimensions();
    let mut eval = |a: Articulation| -> (f64, Vec<f64>) {
        let mut p = vec![0.0; dims];
        p[..4].copy_from_slice(&a.point());
        for k in 0..(dims - 4) / 2 {
            p[4 + 2 * k] = 0.9;
        }
        w.world_mut().set_body_configuration_point(0, &p).unwrap();
        let modes = w.world().space_body(0).unwrap().network_modes().unwrap();
        let peaks = modal::response_peaks(&modes, 90.0, 4_500.0);
        (0.0, peaks)
    };
    let mut vowels = Vec::new();
    let starts = [
        ("a", Articulation { tongue_index: 0.065, tongue_diameter: 0.725, lips: 0.610, velum: 0.0 }, tw::TARGET_A_HZ, [0.0, 0.0]),
        ("i", Articulation { tongue_index: 0.885, tongue_diameter: 0.690, lips: 0.760, velum: 0.0 }, tw::TARGET_I_HZ, [0.0, 0.0]),
        ("u", Articulation { tongue_index: 0.395, tongue_diameter: 0.885, lips: 0.050, velum: 0.015 }, tw::TARGET_U_HZ, [0.0, 0.04]),
    ];
    for (name, start, target, velum_range) in starts {
        let objective = |peaks: &[f64]| -> f64 {
            if peaks.len() < 3 { return f64::INFINITY; }
            (0..3).map(|k| (peaks[k] / target[k]).ln().powi(2)).sum()
        };
        let mut best = (objective(&eval(start).1), start);
        let mut evaluations = 1;
        for step in [0.04, 0.015, 0.005] {
            let centre = best.1;
            for di in -3..=3 {
                for dd in -3..=3 {
                    for dl in -3..=3 {
                        for dv in 0..=(if velum_range[1] > 0.0 { 4 } else { 0 }) {
                            let a = Articulation {
                                tongue_index: (centre.tongue_index + di as f64 * step).clamp(0.0, 1.0),
                                tongue_diameter: (centre.tongue_diameter + dd as f64 * step).clamp(0.0, 1.0),
                                lips: (centre.lips + dl as f64 * step).clamp(0.021, 1.0),
                                velum: if velum_range[1] > 0.0 { (velum_range[1] * dv as f64 / 4.0).clamp(0.0, 1.0) } else { 0.0 },
                            };
                            let (_, peaks) = eval(a);
                            evaluations += 1;
                            let j = objective(&peaks);
                            if j < best.0 {
                                best = (j, a);
                            }
                        }
                    }
                }
            }
        }
        let (_, peaks) = eval(best.1);
        println!("/{name}/ fitted {:?} peaks {:?} target {:?} rms log error {:.4} ({evaluations} evaluations)", best.1, peaks.iter().take(3).map(|v| v.round()).collect::<Vec<_>>(), target, (best.0 / 3.0).sqrt());
        vowels.push(json!({"vowel": name, "fitted": best.1.point(), "peaks_hz": peaks.iter().take(4).collect::<Vec<_>>(), "target_hz": target, "rms_log_error": (best.0 / 3.0).sqrt(), "evaluations": evaluations, "start": start.point()}));
    }
    let receipt = json!({
        "schema": "ovillo.tract-world.public-preset-fit.v0",
        "label": "fit",
        "anatomy": "TractNdAnatomy::public_v1 (44+31, contact follow-below-floor, lip seal 0.02, 5 slots)",
        "method": "coordinate grids (7^3, x5 velum values for /u/) at steps 0.04, 0.015, 0.005 around ovillo-tract fe4fb03's sealed-velum presets; objective sum over F1-F3 of ln(peak/target)^2; peaks = modal::response_peaks of the World's computed modes",
        "target_source": "mus-rs/crates/mus-dsp/src/body_profiles.rs VOWEL_*_MODES (GARGANTA rung-1 design profiles, not measurements)",
        "vowels": vowels,
    });
    std::fs::write(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("receipts/public-preset-fit.json"), serde_json::to_string_pretty(&receipt).unwrap()).unwrap();
}
