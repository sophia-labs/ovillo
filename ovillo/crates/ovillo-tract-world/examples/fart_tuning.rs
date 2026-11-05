//! How the sphincter answers a commanded tension (`fart::TuningCurve`, on
//! its own practice World), then a tune played with it: the opening of
//! Chopin's Funeral March (Sonata No. 2, 1839), on a gut of beans that keeps
//! digesting while it plays. Per note: target, the World's measured F0,
//! cents. Writes `renders/fart/tune-funeral-march.wav`.
//!
//! ```text
//! cargo run --release -p ovillo-tract-world --example fart_tuning -- [out_dir]
//! ```

use ovillo_tract_world::fart::{self, Gut, TuningCurve};

const SR: f64 = 48_000.0;
const FUNERAL_MARCH: &str = "Bb2:1 Bb2:0.75 Bb2:0.25 Bb2:1 Db3:0.75 C3:0.25 C3:0.75 Bb2:0.25 Bb2:0.75 A2:0.25 Bb2:2";

fn main() {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "renders/fart".into()));
    std::fs::create_dir_all(&dir).unwrap();
    let t0 = std::time::Instant::now();
    let curve = TuningCurve::measure(SR).unwrap();
    println!("tuning curve ({:?}): measured {:.1}-{:.1} Hz", t0.elapsed(), curve.range().0, curve.range().1);
    for (c, m) in &curve.points {
        println!("  commanded {c:6.1} Hz -> {m:6.1} Hz ({:+5.0} cents)", 1200.0 * (m / c).log2());
    }
    let notes = fart::parse_tune(FUNERAL_MARCH, 66.0).unwrap();
    let mut gut = Gut::new(SR).unwrap();
    for _ in 0..6 {
        gut.eat(fart::food("beans").unwrap());
    }
    let (audio, reports) = gut.play_tune(&notes, &curve, false, true).unwrap();
    for (tok, r) in FUNERAL_MARCH.split_whitespace().zip(&reports) {
        println!(
            "  {tok:<10} target {:6.1} Hz  measured {:>6}  {:>6}  voiced {:3.0}%",
            r.target_hz,
            r.measured_hz.map_or("-".into(), |m| format!("{m:.1}")),
            r.cents.map_or("-".into(), |c| format!("{c:+.0}c")),
            100.0 * r.voiced_fraction
        );
    }
    let r = gut.world().world().receipt();
    let peak = audio.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    println!(
        "gas left {:.0}%, stomach {} items; peak {peak:.3}; clipped {}; source work {:.3} J, radiated {:.4} J; max frame {:.1e} J, cumulative {:.1e} J",
        100.0 * gut.gas_fraction(),
        gut.stomach().len(),
        gut.world().observers().safety_limited,
        r.source_work,
        r.radiation,
        r.maximum_frame_residual,
        r.cumulative_closure_residual
    );
    let spec = hound::WavSpec { channels: 1, sample_rate: SR as u32, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let mut w = hound::WavWriter::create(dir.join("tune-funeral-march.wav"), spec).unwrap();
    for s in audio {
        w.write_sample(s).unwrap();
    }
    w.finalize().unwrap();
}
