//! PoC (3): air that is more than linear acoustics, in the N-D tract World.
//!
//! 1. Linear limit: with the flow terms off the World is the checkpoint's,
//!    bit for bit (plan digest and render fingerprints).
//! 2. A fricative: a narrow tongue-tip constriction, abducted folds; the
//!    jet's separation loss and its turbulent share; the spectrum, with and
//!    without the turbulent source.
//! 3. A plosive /apa/: the lips close exactly, pressure builds behind them,
//!    the release bursts; the energy of each stage.
//!
//! `cargo run --release -p ovillo-tract-world --example poc_air`

use mus_ariadne::{ConnectionPolicy, DisposalPolicy};
use ovillo_tract_world::world::{self as tw, Articulation, TractControls, TractWorldNd};
use ovillo_tract_world::{measure, TractNdAnatomy};
use serde_json::json;
use std::path::Path;

const SR: f64 = 48_000.0;

fn fnv(samples: &[f32]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for s in samples {
        for b in s.to_bits().to_le_bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}

fn write_wav(path: &Path, samples: &[f32]) {
    let spec = hound::WavSpec { channels: 1, sample_rate: SR as u32, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for &s in samples {
        w.write_sample(s).unwrap();
    }
    w.finalize().unwrap();
}

fn tract(anatomy: TractNdAnatomy, initial: Articulation) -> TractWorldNd {
    TractWorldNd::with(SR, anatomy, ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate }, initial).unwrap()
}

/// Long-term spectrum (8192-point Hann frames, half overlap), power per bin.
fn ltas(x: &[f64]) -> Vec<f64> {
    let n = 8192;
    let mut acc = vec![0.0; n / 2];
    let mut start = 0;
    let mut frames: f64 = 0.0;
    while start + n <= x.len() {
        let frame: Vec<f64> = (0..n).map(|i| x[start + i] * (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos())).collect();
        for (a, d) in acc.iter_mut().zip(measure::magnitude_db(&frame, n)) {
            *a += 10f64.powf(d / 10.0);
        }
        start += n / 2;
        frames += 1.0;
    }
    acc.iter().map(|v| v / frames.max(1.0)).collect()
}

fn band_db(power: &[f64], lo: f64, hi: f64) -> f64 {
    let df = SR / 8192.0;
    10.0 * power[(lo / df) as usize..(hi / df) as usize].iter().sum::<f64>().max(1e-300).log10()
}

fn spectrum_summary(x: &[f64]) -> serde_json::Value {
    let p = ltas(x);
    let df = SR / 8192.0;
    let (lo, hi) = ((500.0 / df) as usize, (12_000.0 / df) as usize);
    let total: f64 = p[lo..hi].iter().sum();
    let centroid = p[lo..hi].iter().enumerate().map(|(i, v)| (lo + i) as f64 * df * v).sum::<f64>() / total;
    let peak = (lo..hi).max_by(|&a, &b| p[a].total_cmp(&p[b])).unwrap() as f64 * df;
    // 1/3-octave levels 250 Hz .. 16 kHz relative to the loudest band.
    let mut bands = Vec::new();
    let mut f = 250.0;
    while f < 16_000.0 {
        let g = f * 2f64.powf(1.0 / 3.0);
        bands.push((f * 2f64.powf(1.0 / 6.0), band_db(&p, f, g)));
        f = g;
    }
    let top = bands.iter().map(|b| b.1).fold(f64::MIN, f64::max);
    json!({
        "centroid_500_12k_hz": centroid,
        "peak_500_12k_hz": peak,
        "band_db_0_1k_1_3k_3_6k_6_12k": [band_db(&p, 50.0, 1_000.0), band_db(&p, 1_000.0, 3_000.0), band_db(&p, 3_000.0, 6_000.0), band_db(&p, 6_000.0, 12_000.0)],
        "third_octaves_hz_db_rel": bands.iter().map(|(c, d)| json!([c.round(), ((d - top) * 10.0).round() / 10.0])).collect::<Vec<_>>(),
    })
}

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../renders/tract-world/poc3");
    std::fs::create_dir_all(&dir).unwrap();
    let linear = TractNdAnatomy::default();
    let air = linear.with_air();
    let mut report = serde_json::Map::new();

    // 1. Linear limit.
    let digest = |a: TractNdAnatomy| a.plan(SR, ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate }, tw::ART_REST.point()).digest().unwrap();
    let mut off = air;
    off.contact = None;
    off.aero = off.aero.map(|mut spec| {
        spec.loss_scale = 0.0;
        spec.dipole_efficiency = 0.0;
        spec.dc_wall_loss = false;
        spec
    });
    let fingerprint = |a: TractNdAnatomy| -> String {
        let mut w = tract(a, tw::ART_A);
        w.set_controls(TractControls { articulation: tw::ART_I, pitch_hz: 128.0, tenseness: 0.6, breath: 0.6 });
        let mut out = vec![0.0f32; 48_000];
        w.process(&mut out).unwrap();
        format!("{:016x}", fnv(&out))
    };
    let lin = json!({
        "plan_digest_flow_absent": digest(linear),
        "plan_digest_flow_present": digest(air),
        "render_fnv_flow_absent": fingerprint(linear),
        "render_fnv_flow_terms_zero": fingerprint(off),
        "render_fnv_flow_on": fingerprint(air),
    });
    println!("linear limit: {lin}");
    report.insert("linear_limit".into(), lin);

    // 2. Fricative.
    let art_s = Articulation { tongue_index: 0.95, tongue_diameter: 1.0, lips: 0.6, velum: 0.0 };
    let mut fricatives = serde_json::Map::new();
    for (label, anatomy) in [("air", air), ("air_no_turbulence", {
        let mut a = air;
        a.aero = a.aero.map(|mut s| { s.dipole_efficiency = 0.0; s });
        a
    }), ("linear", linear)] {
        let mut w = tract(anatomy, art_s);
        w.set_controls(TractControls { articulation: art_s, pitch_hz: 120.0, tenseness: 0.0, breath: 0.8 });
        let warm = (0.5 * SR) as usize;
        let window = (0.5 * SR) as usize;
        let mut audio = vec![0.0f32; warm + window];
        w.process(&mut audio[..warm]).unwrap();
        let before = w.totals();
        let receipt_before = w.world().receipt();
        w.process(&mut audio[warm..]).unwrap();
        let after = w.totals();
        let receipt_after = w.world().receipt();
        write_wav(&dir.join(format!("fricative-{label}.wav")), &audio);
        let tail: Vec<f64> = audio[warm..].iter().map(|&v| f64::from(v)).collect();
        let o = w.world().body_space_observation(0).unwrap();
        let budget = json!({
            "window_s": 0.5,
            "muscles_source_work": receipt_after.source_work - receipt_before.source_work,
            "reservoir_draw": after.reservoir_draw - before.reservoir_draw,
            "glottal_jet_loss": after.valve_jet_loss - before.valve_jet_loss,
            "glottal_field_work": after.valve_field_work - before.valve_field_work,
            "constriction_jet_loss": after.aero.jet_loss - before.aero.jet_loss,
            "turbulence_work": after.aero.turbulence_work - before.aero.turbulence_work,
            "disposal": w.world().disposal_total(),
            "radiation_booked_modal": receipt_after.radiation - receipt_before.radiation,
            "radiation_physical_g_p2": (after.physical_radiation_by_end[0] + after.physical_radiation_by_end[1]) - (before.physical_radiation_by_end[0] + before.physical_radiation_by_end[1]),
            "wall_loss": after.wall_loss - before.wall_loss,
            "fold_loss": after.fold_loss - before.fold_loss,
            "internal_loss": receipt_after.internal_loss - receipt_before.internal_loss,
            "cumulative_residual": receipt_after.cumulative_closure_residual,
            "maximum_frame_residual": receipt_after.maximum_frame_residual,
        });
        let state = json!({
            "jet_aperture_cm2": o.aero.jet_aperture_m2 * 1e4,
            "jet_flow_cm3_s": o.aero.jet_flow_m3_s * 1e6,
            "jet_speed_m_s": o.aero.jet_speed_m_s,
            "mach": o.aero.mach,
            "reynolds": o.aero.reynolds,
            "jet_junction": o.aero.jet_junction,
            "glottal_flow_cm3_s": o.valve.flow_m3_s * 1e6,
            "supply_pressure_pa": o.valve.supply_pressure_pa,
        });
        let spectrum = spectrum_summary(&tail);
        println!("fricative [{label}]: state {state}\n   spectrum {}\n   budget {budget}", spectrum);
        fricatives.insert(label.into(), json!({ "articulation": art_s.point(), "state_at_end": state, "spectrum_last_0_5_s": spectrum, "energy_budget": budget, "wav": format!("fricative-{label}.wav") }));
    }
    report.insert("fricative".into(), serde_json::Value::Object(fricatives));

    // 3. Plosive /apa/.
    let art_p = Articulation { lips: 0.0, ..tw::ART_A };
    let mut w = tract(air, tw::ART_A);
    let behind = w.anatomy().oral_sections / 2;
    let mut audio = Vec::new();
    let mut trace = Vec::new();
    let mut closed_at = None;
    let mut released_at = None;
    let mut stored_at_release = 0.0;
    let mut peak_oral_pa: f64 = 0.0;
    let mut before_release = None;
    for i in 0..(0.9 * SR) as usize {
        let t = i as f64 / SR;
        let art = if (0.30..0.48).contains(&t) { art_p } else { tw::ART_A };
        w.set_controls(TractControls { articulation: art, pitch_hz: 120.0, tenseness: 0.6, breath: 0.6 });
        audio.push(w.tick().unwrap().clamp(-1.0, 1.0) as f32);
        let o = w.world().body_space_observation(0).unwrap();
        let oral = w.world().space_body(0).unwrap().node_pressure(behind).unwrap_or(0.0);
        if o.aero.jet_closed && closed_at.is_none() {
            closed_at = Some(t);
        }
        if closed_at.is_some() && !o.aero.jet_closed && released_at.is_none() {
            released_at = Some(t);
            stored_at_release = w.world().space_body(0).unwrap().field_energy();
            before_release = Some((w.totals(), w.world().receipt()));
        }
        if o.aero.jet_closed {
            peak_oral_pa = peak_oral_pa.max(oral);
        }
        if i % 48 == 0 {
            trace.push(json!([((t * 1e4).round() / 1e4), (oral * 10.0).round() / 10.0, ((o.valve.flow_m3_s * 1e7).round() / 10.0), ((o.aero.jet_flow_m3_s * 1e7).round() / 10.0), o.aero.jet_closed]));
        }
    }
    write_wav(&dir.join("plosive-apa.wav"), &audio);
    let released = released_at.expect("the lips must release");
    let (t0, r0) = before_release.unwrap();
    // Burst window: 30 ms after release, re-rendered by reading the run's totals.
    let rel = (released * SR) as usize;
    let burst: Vec<f64> = audio[rel..rel + (0.03 * SR) as usize].iter().map(|&v| f64::from(v)).collect();
    let pre: Vec<f64> = audio[rel - (0.03 * SR) as usize..rel].iter().map(|&v| f64::from(v)).collect();
    let vowel: Vec<f64> = audio[(0.2 * SR) as usize..(0.29 * SR) as usize].iter().map(|&v| f64::from(v)).collect();
    let rms = |x: &[f64]| (x.iter().map(|v| v * v).sum::<f64>() / x.len() as f64).sqrt();
    let end = w.totals();
    let r1 = w.world().receipt();
    // Re-run to the release + 20 ms for the burst's own budget.
    let mut w2 = tract(air, tw::ART_A);
    let burst_end = rel + (0.02 * SR) as usize;
    let mut at_release = None;
    let mut glottal_flow_late_closure = 0.0;
    for i in 0..burst_end {
        let t = i as f64 / SR;
        let art = if (0.30..0.48).contains(&t) { art_p } else { tw::ART_A };
        w2.set_controls(TractControls { articulation: art, pitch_hz: 120.0, tenseness: 0.6, breath: 0.6 });
        w2.tick().unwrap();
        if i + 1 == rel {
            at_release = Some((w2.totals(), w2.world().receipt(), w2.world().space_body(0).unwrap().field_energy()));
        }
        if i + (0.01 * SR) as usize == rel {
            glottal_flow_late_closure = w2.world().body_space_observation(0).unwrap().valve.flow_m3_s;
        }
    }
    let (b0, rb0, field0) = at_release.unwrap();
    let (b1, rb1) = (w2.totals(), w2.world().receipt());
    let burst_budget = json!({
        "window_s": 0.02,
        "field_energy_at_release": field0,
        "field_energy_after_20ms": w2.world().space_body(0).unwrap().field_energy(),
        "source_work": rb1.source_work - rb0.source_work,
        "glottal_field_work": b1.valve_field_work - b0.valve_field_work,
        "constriction_jet_loss": b1.aero.jet_loss - b0.aero.jet_loss,
        "turbulence_work": b1.aero.turbulence_work - b0.aero.turbulence_work,
        "wall_loss": b1.wall_loss - b0.wall_loss,
        "radiation_booked_modal": rb1.radiation - rb0.radiation,
        "radiation_physical_g_p2": (b1.physical_radiation_by_end[0] + b1.physical_radiation_by_end[1]) - (b0.physical_radiation_by_end[0] + b0.physical_radiation_by_end[1]),
        "control_work": rb1.control_work - rb0.control_work,
        "disposal": w2.world().disposal_total(),
        "glottal_flow_10ms_before_release_cm3_s": glottal_flow_late_closure * 1e6,
        "peak_output_in_window": audio[rel..burst_end].iter().fold(0.0f32, |m, v| m.max(v.abs())),
    });
    println!("burst budget: {burst_budget}");
    let plosive = json!({
        "closure_s": closed_at,
        "release_s": released,
        "closed_duration_s": released - closed_at.unwrap(),
        "peak_oral_pressure_during_closure_pa": peak_oral_pa,
        "oral_pressure_node": behind,
        "field_energy_at_release_j": stored_at_release,
        "rms_vowel_before": rms(&vowel),
        "rms_30ms_before_release": rms(&pre),
        "rms_30ms_after_release": rms(&burst),
        "burst_20ms": burst_budget,
        "after_release_to_end": {
            "constriction_jet_loss": end.aero.jet_loss - t0.aero.jet_loss,
            "turbulence_work": end.aero.turbulence_work - t0.aero.turbulence_work,
            "radiation": r1.radiation - r0.radiation,
        },
        "whole_run": {
            "disposal": w.world().disposal_total(),
            "closed_samples": end.aero.closed_samples,
            "constriction_jet_loss": end.aero.jet_loss,
            "turbulence_work": end.aero.turbulence_work,
            "source_work": r1.source_work,
            "radiation": r1.radiation,
            "cumulative_residual": r1.cumulative_closure_residual,
            "maximum_frame_residual": r1.maximum_frame_residual,
        },
        "trace_every_1ms_t_oralpa_glottal_cm3s_jet_cm3s_closed": trace,
        "wav": "plosive-apa.wav",
    });
    println!("plosive: {}", serde_json::to_string(&json!({"closure": closed_at, "release": released, "peak_oral_pa": peak_oral_pa, "field_energy_at_release": stored_at_release, "rms_before": rms(&pre), "rms_after": rms(&burst), "vowel_rms": rms(&vowel), "after_release": plosive["after_release_to_end"], "whole": plosive["whole_run"]})).unwrap());
    report.insert("plosive".into(), plosive);
    let path = dir.join("poc3-receipt.json");
    std::fs::write(&path, serde_json::to_string_pretty(&serde_json::Value::Object(report)).unwrap()).unwrap();
    println!("wrote {}", path.display());
}
