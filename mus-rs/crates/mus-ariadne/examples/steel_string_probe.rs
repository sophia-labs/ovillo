//! Probe a steel-string world plan: pluck each course (open and stopped), and
//! report the sounding pitch, the decay, the loudness, and the ledger.
//!
//! cargo run --release -p mus-ariadne --example steel_string_probe -- <plan.json> [out.wav]
//!
//! Experiments override the plan or the playing through the environment:
//! STEEL_CONTACT_RATE, STEEL_APERTURE_RATE, STEEL_BRIDGE_RATE (Hz),
//! STEEL_STRING_RADIATION, STEEL_BOX_RADIATION (radiation fraction), STEEL_SOURCE_GAIN.

use std::f64::consts::PI;

use mus_ariadne::{BodyControls, StereoListener, StereoListenerSpec, World, WorldControls, WorldPlan};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut plan: WorldPlan = serde_json::from_str(&std::fs::read_to_string(&args[1])?)?;
    let env = |name: &str| std::env::var(name).ok().and_then(|v| v.parse::<f64>().ok());
    for body in plan.bodies.iter_mut().filter(|b| b.id.starts_with("string/")) {
        let path = &mut body.body.configuration_path;
        for config in [&mut path.left, &mut path.right] {
            if let Some(rate) = env("STEEL_CONTACT_RATE") { config.aperture.source_to_field_rate_hz = rate; }
            if let Some(rate) = env("STEEL_APERTURE_RATE") { config.aperture.source_to_aperture_rate_hz = rate; }
        }
    }
    if let Some(rate) = env("STEEL_BRIDGE_RATE") { plan.direct_couplings.iter_mut().for_each(|e| e.rate_hz = rate); }
    let string_radiation = env("STEEL_STRING_RADIATION").unwrap_or(2.0e-6);   // as mus-engine plays it
    let box_radiation = env("STEEL_BOX_RADIATION").unwrap_or(0.02);
    let source_gain = env("STEEL_SOURCE_GAIN").unwrap_or(25.0);
    let pulse = env("STEEL_PULSE_SECONDS").unwrap_or(0.00006 + 0.0003 * 0.55);
    let out = args.get(2).cloned();
    let rate = plan.internal_sample_rate_hz;
    let listener = StereoListener::bind(&StereoListenerSpec::all_bodies(&plan, 1.0)?, &plan)?;
    let mut world = World::from_plan(plan.clone())?;
    let strings: Vec<usize> = (0..world.body_count()).filter(|&i| world.body_id(i).unwrap().starts_with("string/")).collect();
    let mut controls = vec![BodyControls::default(); world.body_count()];
    for (i, c) in controls.iter_mut().enumerate() {
        let f = &plan.bodies[i].body.configuration_path.left.metric.frequency_hz;
        let mean = f.iter().sum::<f64>() / f.len() as f64;      // as mus-engine: faster radiation for higher bands
        c.radiation_fraction = if strings.contains(&i) { string_radiation } else { (box_radiation * (mean / 1000.0).max(1.0)).min(0.5) };
    }
    let mut recording = Vec::new();
    println!("{:<10} {:>6} {:>9} {:>9} {:>8} {:>8} {:>8}", "string", "fret", "want Hz", "got Hz", "cents", "T60 s", "peak");
    for &index in &strings {
        let id = world.body_id(index).unwrap().to_owned();
        let body = &plan.bodies[index].body.configuration_path;
        let open = body.left.metric.frequency_hz[0];
        for fret in [0.0_f64, 5.0] {
            let want = open * 2f64.powf(fret / 12.0);
            world.set_body_configuration(&id, fret / 12.0)?;           // the path is log-frequency: fret k is k/12
            let seconds = 2.5;
            let n = (seconds * rate) as usize;
            let mut take = Vec::with_capacity(n);
            for k in 0..n {
                let t = k as f64 / rate;
                let width = pulse;                          // as mus-engine plucks with bare fingers
                controls[index].source_wave = if t < width { source_gain * 0.5 * (1.0 - (2.0 * PI * t / width).cos()) } else { 0.0 };
                let frame = world.process_sample(&controls, WorldControls::default())?;
                let heard = listener.observe(&world, &frame)?;
                take.push(0.5 * (heard.left + heard.right));
            }
            controls[index].source_wave = 0.0;
            let got = peak_frequency(&take[(0.05 * rate) as usize..(1.05 * rate) as usize], rate, want);
            let peak = take.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
            println!("{:<10} {:>6} {:>9.2} {:>9.2} {:>8.1} {:>8.2} {:>8.4}", id, fret, want, got,
                     1200.0 * (got / want).log2(), t60(&take, rate), peak);
            recording.extend(take);
            for _ in 0..(0.3 * rate) as usize {             // let the world settle between plucks
                let frame = world.process_sample(&controls, WorldControls::default())?;
                recording.push(0.5 * { let h = listener.observe(&world, &frame)?; h.left + h.right });
            }
        }
        world.set_body_configuration(&id, 0.0)?;
    }
    let r = world.receipt();
    println!("ledger: source {:.4e} control {:.4e} loss {:.4e} radiation {:.4e} closure {:.2e} (max frame {:.2e})",
             r.source_work, r.control_work, r.internal_loss, r.radiation, r.cumulative_closure_residual, r.maximum_frame_residual);
    if let Some(path) = out {
        let peak = recording.iter().fold(1e-12_f64, |m, v| m.max(v.abs()));
        write_wav(&path, &recording.iter().map(|v| v / peak * 0.8).collect::<Vec<_>>(), rate as u32)?;
        println!("wrote {path} ({:.1} s, normalized from peak {peak:.4})", recording.len() as f64 / rate);
    }
    Ok(())
}

/// The strongest spectral peak within a minor third of the expected pitch (Goertzel scan, 0.1 Hz steps).
fn peak_frequency(x: &[f64], rate: f64, near: f64) -> f64 {
    let mut best = (0.0, near);
    let mut f = near * 0.84;
    while f < near * 1.19 {
        let w = 2.0 * PI * f / rate;
        let (mut s1, mut s2) = (0.0, 0.0);
        for (i, v) in x.iter().enumerate() {
            let hann = 0.5 - 0.5 * (2.0 * PI * i as f64 / x.len() as f64).cos();
            let s = v * hann + 2.0 * w.cos() * s1 - s2;
            s2 = s1;
            s1 = s;
        }
        let power = s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2;
        if power > best.0 {
            best = (power, f);
        }
        f += 0.1;
    }
    best.1
}

/// T60 from the slope of the RMS envelope between -6 and -26 dB below its peak.
fn t60(x: &[f64], rate: f64) -> f64 {
    let hop = (0.01 * rate) as usize;
    let env: Vec<f64> = x.chunks(hop).map(|c| 10.0 * (c.iter().map(|v| v * v).sum::<f64>() / c.len() as f64 + 1e-30).log10()).collect();
    let top = env.iter().cloned().fold(f64::MIN, f64::max);
    let start = env.iter().position(|&e| e < top - 6.0).unwrap_or(0);
    let end = env.iter().position(|&e| e < top - 26.0).unwrap_or(env.len() - 1);
    if end <= start { return f64::NAN; }
    60.0 * ((end - start) as f64 * 0.01) / 20.0
}

fn write_wav(path: &str, samples: &[f64], rate: u32) -> std::io::Result<()> {
    let mut bytes = Vec::with_capacity(44 + samples.len() * 2);
    let data = (samples.len() * 2) as u32;
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    for v in [16u32] { bytes.extend_from_slice(&v.to_le_bytes()); }
    for v in [1u16, 1] { bytes.extend_from_slice(&v.to_le_bytes()); }
    for v in [rate, rate * 2] { bytes.extend_from_slice(&v.to_le_bytes()); }
    for v in [2u16, 16] { bytes.extend_from_slice(&v.to_le_bytes()); }
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data.to_le_bytes());
    for s in samples { bytes.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()); }
    std::fs::write(path, bytes)
}
