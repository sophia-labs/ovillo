//! /a/ /i/ /u/ from the N-D tract World and from the tube tract, measured by
//! the same code; plus an /a i u a/ glide and "mama" for listening, with the
//! World's ledger. Writes WAVs to `ovillo/renders/tract-world/` and a receipt.
//!
//! ```text
//! cargo run --release -p ovillo-tract-world --example render_vowels -- [oral] [nasal] [interval] [connection]
//! ```

use mus_ariadne::{ConnectionPolicy, DisposalPolicy};
use ovillo_tract_world::world::{self as tw, Articulation, TractControls, TractWorldNd};
use ovillo_tract_world::{measure, modal, TractNdAnatomy};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const SR: f64 = 48_000.0;

fn write_wav(path: &Path, samples: &[f32]) {
    let spec = hound::WavSpec { channels: 1, sample_rate: SR as u32, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let mut w = hound::WavWriter::create(path, spec).expect("wav");
    for &s in samples {
        w.write_sample(s).unwrap();
    }
    w.finalize().unwrap();
}

fn lpc_median(audio: &[f32]) -> (Vec<Option<f64>>, f64, Vec<Vec<(f64, f64)>>) {
    // Exactly ovillo-tract's render_vowels.rs procedure.
    let x: Vec<f64> = audio.iter().map(|&v| f64::from(v)).collect();
    let tail = &x[x.len() - (0.4 * SR) as usize..];
    let frames: Vec<Vec<measure::Formant>> = (0..3).map(|k| measure::lpc_formants(&tail[k * 6_000..k * 6_000 + 2_400], SR)).collect();
    let median = |i: usize| -> Option<f64> {
        let mut v: Vec<f64> = frames.iter().filter_map(|f| f.get(i).map(|q| q.hz)).collect();
        v.sort_by(f64::total_cmp);
        v.get(v.len() / 2).copied()
    };
    let f0 = measure::f0_autocorrelation(&tail[..9_600], SR);
    (
        (0..3).map(median).collect(),
        f0,
        frames.iter().map(|f| f.iter().map(|q| (q.hz, q.bandwidth_hz)).collect()).collect(),
    )
}

fn ledger(world: &TractWorldNd) -> Value {
    let r = world.world().receipt();
    let s = world.stats();
    json!({
        "frames": r.sample_frames,
        "sourceWork": r.source_work, "controlWork": r.control_work, "internalLoss": r.internal_loss,
        "radiation": r.radiation, "initialEnergy": r.initial_energy, "finalEnergy": r.current_energy,
        "cumulativeClosureResidual": r.cumulative_closure_residual, "maximumFrameResidual": r.maximum_frame_residual,
        "globalClosureResidual": r.global_closure_residual, "maximumLocalResidual": r.maximum_local_residual,
        "planDigest": r.plan_digest,
        "moves": s.moves, "moveControlWork": s.control_work, "disposal": s.disposal, "valveRetunes": s.valve_retunes, "valveWork": s.valve_work,
    })
}

fn nd_vowel(anatomy: TractNdAnatomy, connection: ConnectionPolicy, interval: usize, art: Articulation, name: &str, dir: &Path) -> Value {
    let mut w = TractWorldNd::with(SR, anatomy, connection, tw::ART_REST).unwrap();
    w.update_interval = interval;
    w.set_controls(TractControls { articulation: art, pitch_hz: 120.0, tenseness: 0.6, breath: 0.6 });
    let mut audio = vec![0.0f32; (1.4 * SR) as usize];
    w.process(&mut audio).unwrap();
    write_wav(&dir.join(format!("nd-{name}.wav")), &audio);
    let (lpc, f0, frames) = lpc_median(&audio);
    let modes = w.world().space_body(0).unwrap().network_modes().unwrap();
    let peaks = modal::response_peaks(&modes, 90.0, 4_500.0);
    let tail: Vec<f64> = audio[audio.len() - 19_200..].iter().map(|&v| f64::from(v)).collect();
    let rms = (tail.iter().map(|v| v * v).sum::<f64>() / tail.len() as f64).sqrt();
    json!({
        "lpc_hz": lpc, "lpc_frames": frames, "f0_hz": f0, "tail_rms": rms,
        "modal_response_peaks_hz": peaks.iter().take(4).collect::<Vec<_>>(),
        "lowest_modes_hz": modes.frequency_hz[..8].to_vec(),
        "oral_fraction": modes.first_tube_fraction[..8].to_vec(),
        "ledger": ledger(&w),
        "wav": format!("nd-{name}.wav"),
    })
}

fn tube_vowel(art: Articulation, name: &str, dir: &Path) -> Value {
    use ovillo_tract as ot;
    let a = ot::Articulation { tongue_index: art.tongue_index, tongue_diameter: art.tongue_diameter, lips: art.lips, velum: art.velum };
    let mut w = ot::TractWorld::new(SR);
    let tube = measure::spectrum_peaks(&w.impulse_response(a, 48_000, 1e-6), 2.0 * SR, 90.0, 4_500.0);
    let mut w = ot::TractWorld::new(SR);
    let mut c = ot::TractControls::default();
    c.articulation = ot::presets::ART_REST;
    w.set_controls(c);
    w.snap_configuration();
    c.articulation = a;
    c.breath = 0.6;
    c.pitch_hz = 120.0;
    c.aspiration = 0.0;
    w.set_controls(c);
    let mut audio = Vec::new();
    let mut block = [0.0f32; 128];
    for _ in 0..(1.4 * SR / 128.0).ceil() as usize {
        w.process(&mut block);
        audio.extend_from_slice(&block);
    }
    write_wav(&dir.join(format!("tube-{name}.wav")), &audio);
    let (lpc, f0, _) = lpc_median(&audio);
    json!({ "lpc_hz": lpc, "f0_hz": f0, "tube_peaks_hz": tube.iter().take(4).collect::<Vec<_>>() })
}

fn gesture(anatomy: TractNdAnatomy, connection: ConnectionPolicy, interval: usize, name: &str, keys: &[(f64, Articulation, f64, f64)], seconds: f64, dir: &Path) -> Value {
    let mut w = TractWorldNd::with(SR, anatomy, connection, keys[0].1).unwrap();
    w.update_interval = interval;
    let mut audio = Vec::with_capacity((seconds * SR) as usize);
    for i in 0..(seconds * SR) as usize {
        let t = i as f64 / SR;
        let key = keys.iter().rev().find(|k| k.0 <= t).unwrap_or(&keys[0]);
        w.set_controls(TractControls { articulation: key.1, pitch_hz: key.2, tenseness: 0.6, breath: key.3 });
        audio.push(w.tick().unwrap().clamp(-1.0, 1.0) as f32);
    }
    write_wav(&dir.join(format!("nd-{name}.wav")), &audio);
    let peak = audio.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    json!({ "gesture": name, "seconds": seconds, "peak": peak, "ledger": ledger(&w), "wav": format!("nd-{name}.wav") })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let oral: usize = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(44);
    let nasal: usize = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(31);
    let interval: usize = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(32);
    let connection = match args.get(4).map(String::as_str) {
        Some("modal") => ConnectionPolicy::ModalCoordinates,
        _ => ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate },
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir: PathBuf = root.join(format!("renders/tract-world/o{oral}-n{nasal}-k{interval}-{}", if matches!(connection, ConnectionPolicy::ModalCoordinates) { "modal" } else { "massmomentum" }));
    std::fs::create_dir_all(&dir).unwrap();
    let anatomy = TractNdAnatomy::adult_v0(oral, nasal);
    let mut vowels = Vec::new();
    for (name, art, target) in [("a", tw::ART_A, tw::TARGET_A_HZ), ("i", tw::ART_I, tw::TARGET_I_HZ), ("u", tw::ART_U, tw::TARGET_U_HZ)] {
        let nd = nd_vowel(anatomy, connection, interval, art, name, &dir);
        let tube = tube_vowel(art, name, &dir);
        let err = |v: &Value| -> Vec<Option<f64>> {
            (0..3).map(|i| v["lpc_hz"][i].as_f64().map(|m| (1000.0 * (m - target[i]) / target[i]).round() / 10.0)).collect()
        };
        println!(
            "/{name}/ target {target:?} | N-D World LPC {:?} err% {:?} modal peaks {:?} F0 {:.1} | tube LPC {:?} err% {:?} tube peaks {:?} F0 {:.1}",
            nd["lpc_hz"].as_array().unwrap().iter().map(|v| v.as_f64().map(f64::round)).collect::<Vec<_>>(),
            err(&nd),
            nd["modal_response_peaks_hz"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap().round()).collect::<Vec<_>>(),
            nd["f0_hz"].as_f64().unwrap(),
            tube["lpc_hz"].as_array().unwrap().iter().map(|v| v.as_f64().map(f64::round)).collect::<Vec<_>>(),
            err(&tube),
            tube["tube_peaks_hz"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap().round()).collect::<Vec<_>>(),
            tube["f0_hz"].as_f64().unwrap(),
        );
        println!("     N-D ledger: cumulative {:.3e} J, max frame {:.3e} J, moves {}, disposal {:.3e} J", nd["ledger"]["cumulativeClosureResidual"].as_f64().unwrap(), nd["ledger"]["maximumFrameResidual"].as_f64().unwrap(), nd["ledger"]["moves"], nd["ledger"]["disposal"].as_f64().unwrap());
        vowels.push(json!({ "vowel": name, "articulation": art.point(), "target_hz": target, "nd_world": nd, "tube_tract": tube, "nd_err_percent": err(&nd), "tube_err_percent": err(&tube) }));
    }
    let aiua = [(0.0, tw::ART_A, 120.0, 0.6), (0.7, tw::ART_I, 128.0, 0.6), (1.4, tw::ART_U, 116.0, 0.6), (2.1, tw::ART_A, 110.0, 0.6), (2.8, tw::ART_A, 110.0, 0.0)];
    let mama = [(0.0, tw::ART_M, 118.0, 0.55), (0.2, tw::ART_A, 122.0, 0.62), (0.62, tw::ART_M, 112.0, 0.55), (0.8, tw::ART_A, 104.0, 0.6), (1.4, tw::ART_A, 98.0, 0.0)];
    let g1 = gesture(anatomy, connection, interval, "aiua", &aiua, 3.2, &dir);
    let g2 = gesture(anatomy, connection, interval, "mama", &mama, 2.0, &dir);
    for g in [&g1, &g2] {
        println!("{}: peak {:.3}, cumulative {:.3e} J, max frame {:.3e} J, moves {}, move work {:.3e} J, disposal {:.3e} J", g["gesture"], g["peak"].as_f64().unwrap(), g["ledger"]["cumulativeClosureResidual"].as_f64().unwrap(), g["ledger"]["maximumFrameResidual"].as_f64().unwrap(), g["ledger"]["moves"], g["ledger"]["moveControlWork"].as_f64().unwrap(), g["ledger"]["disposal"].as_f64().unwrap());
    }
    let receipt = json!({
        "schema": "ovillo.tract-world.vowels.v0",
        "sampleRateHz": SR,
        "oralSections": oral, "nasalSections": nasal, "updateInterval": interval, "minMove": TractWorldNd::new(SR).unwrap().min_move,
        "connection": format!("{connection:?}"),
        "method": "both worlds: ART_REST start, target vowel at articulator speed, breath 0.6, fold tuning 120 Hz, tenseness 0.6, 1.4 s; LPC = ovillo-tract measure::lpc_formants (copied byte-identical), median of three 50 ms frames in the last 0.4 s; N-D modal peaks = peaks of the World's own modal response H(w) (glottal volume velocity to radiating-volume pressure); tube peaks = ovillo-tract rigid-glottis impulse response",
        "targetSource": "mus-rs/crates/mus-dsp/src/body_profiles.rs VOWEL_*_MODES (GARGANTA rung-1 design profiles, not measurements)",
        "articulationSource": "ovillo-tract presets.rs ART_A/I/U (fitted there against the tube tract; not refitted)",
        "vowels": vowels,
        "gestures": [g1, g2],
        "provenance": anatomy.provenance().iter().map(|p| json!({"name": p.name, "value": p.value, "unit": p.unit, "label": p.label, "source": p.source})).collect::<Vec<_>>(),
    });
    let text = serde_json::to_string_pretty(&receipt).unwrap();
    std::fs::write(dir.join("receipt.json"), &text).unwrap();
    println!("wrote {}", dir.join("receipt.json").display());
}
