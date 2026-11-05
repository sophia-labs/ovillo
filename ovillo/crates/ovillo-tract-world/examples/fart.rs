//! `TractNdAnatomy::fart_v1`: the tract World with a sphincter for a glottis
//! and cheeks for lips. Renders `fart::STYLES` (and "beans": brap while the
//! reservoir is refilled at 1 W) to WAVs with the World's own ledger and a
//! 10 ms track of the reservoir, the supply pressure and the valve's
//! measured F0 (the World's non-loading observer). The WAV is the
//! listener's copy (`fart::Gut`: radiated pressure x 1/1000, 20 Hz DC
//! blocker).
//!
//! ```text
//! cargo run --release -p ovillo-tract-world --example fart -- [out_dir]
//! ```

use ovillo_tract_world::fart::{self, Gut, Style};
use ovillo_tract_world::world::{TractControls, Vibrato};
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

const BEANS: Style = Style {
    name: "beans",
    seconds: 4.0,
    vibrato: Vibrato { rate_hz: 5.5, depth_semitones: 0.0, wobble: 0.8 },
    jitter_semitones: 1.5,
    sealed: |_| false,
    at: |t| TractControls { articulation: fart::cheeks(0.3 + 0.15 * (t * 1.7).sin()), pitch_hz: 55.0 + 25.0 * (t * 0.9).sin(), tenseness: 0.75, breath: 1.0 },
    about: "brap while the reservoir is refilled at 1 W",
};

fn render(style: &Style, refill_w: f64, dir: &Path) -> Value {
    let mut gut = Gut::new(SR).unwrap();
    gut.world_mut().set_refill(refill_w);
    let capacity = gut.world().anatomy().lung_capacity_j;
    let mut track = Vec::new();
    let mut f0s = Vec::new();
    let audio = gut
        .release_with(style, |t, w| {
            let body = w.world().space_body(0).unwrap();
            let f0 = w.observers().measured_f0_hz;
            if f0 > 0.0 {
                f0s.push(f0);
            }
            track.push(json!([(t * 1e3).round() / 1e3, body.reservoir_energy() / capacity, body.observation().valve.supply_pressure_pa, f0]));
        })
        .unwrap();
    let wav = format!("fart-{}.wav", style.name);
    write_wav(&dir.join(&wav), &audio);
    let w = gut.world();
    let rc = w.world().receipt();
    let t = w.totals();
    let peak = audio.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    f0s.sort_by(f64::total_cmp);
    let voiced = f0s.len() as f64 * fart::HOP_S;
    let median_f0 = f0s.get(f0s.len() / 2).copied().unwrap_or(0.0);
    let reservoir_end = gut.gas_fraction();
    println!(
        "{:<9} {:.1} s  peak {peak:.3}  clipped {}  voiced {voiced:.2} s  median F0 {median_f0:.0} Hz  reservoir {:.0}% left  radiated {:.1e} J  turbulence {:.1e} J  closed {}  max frame {:.1e} J  cumulative {:.1e} J",
        style.name,
        style.seconds,
        w.observers().safety_limited,
        100.0 * reservoir_end,
        rc.radiation,
        t.aero.turbulence_injected,
        t.aero.closed_samples,
        rc.maximum_frame_residual,
        rc.cumulative_closure_residual
    );
    json!({
        "name": style.name, "about": style.about, "wav": wav, "seconds": style.seconds, "refillW": refill_w, "jitterSemitones": style.jitter_semitones,
        "peak": peak, "safetyLimited": w.observers().safety_limited,
        "voicedSeconds": voiced, "medianF0Hz": median_f0, "reservoirLeft": reservoir_end,
        "turbulenceInjected": t.aero.turbulence_injected, "closedSamples": t.aero.closed_samples,
        "valveRetunes": w.stats().valve_retunes, "valveWork": w.stats().valve_work,
        "ledger": {
            "sourceWork": rc.source_work, "controlWork": rc.control_work, "internalLoss": rc.internal_loss,
            "radiation": rc.radiation, "initialEnergy": rc.initial_energy, "finalEnergy": rc.current_energy,
            "cumulativeClosureResidual": rc.cumulative_closure_residual, "maximumFrameResidual": rc.maximum_frame_residual,
            "planDigest": rc.plan_digest,
        },
        "trackColumns": ["t_s", "reservoirFraction", "supplyPa", "f0Hz"],
        "track": track,
    })
}

fn main() {
    let dir = std::env::args().nth(1).map_or_else(|| PathBuf::from("renders/fart"), PathBuf::from);
    std::fs::create_dir_all(&dir).unwrap();
    let mut receipts: Vec<Value> = fart::STYLES.iter().map(|s| render(s, 0.0, &dir)).collect();
    receipts.push(render(&BEANS, 1.0, &dir));
    let anatomy = ovillo_tract_world::TractNdAnatomy::fart_v1();
    let receipt = json!({ "anatomy": format!("{anatomy:?}"), "listener": "radiated pressure x 1/1000, 20 Hz one-pole DC blocker", "renders": receipts });
    std::fs::write(dir.join("receipt.json"), serde_json::to_string_pretty(&receipt).unwrap()).unwrap();
    println!("wrote {}", dir.display());
}
