//! The click lab: why a held fart note clicks once per cycle, and which
//! anatomy change softens it. Each variant of `fart_v0` plays the same two
//! held notes (tensions 60, 90, 130 and 200 Hz, 1.2 s each, push 0.7, cheeks 0.4,
//! tone 0.75, slurred); the table reports what the World measured and the
//! listener's copy's shape. WAVs go to `<out>/<variant>.wav`.
//!
//!   cargo run --release -p ovillo-tract-world --example fart_click -- <out dir> [push cheeks tone [tension Hz...]]
//!
//! sharpness: the 99.9th percentile of |x[n]-x[n-1]| over its RMS (a sine is
//! 1.4). duty: the share of samples above 0.3 of the note's peak. hf: energy
//! above 2 kHz as a share of the note's, dB. Every anatomy number here is
//! as invented as fart_v0's.
use ovillo_tract_world::fart::{Event, Gut, HOP_S};
use ovillo_tract_world::TractNdAnatomy;
use std::io::Write;

const SR: f64 = 48_000.0;

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "renders/fart-click".into());
    let arg = |k: usize, d: f64| std::env::args().nth(k).and_then(|v| v.parse().ok()).unwrap_or(d);
    let (push, cheeks, tone) = (arg(2, 0.7), arg(3, 0.4), arg(4, 0.75));
    let tensions: Vec<f64> = { let t: Vec<f64> = std::env::args().skip(5).filter_map(|v| v.parse().ok()).collect(); if t.is_empty() { vec![60.0, 90.0, 130.0, 200.0] } else { t } };
    println!("push {push} cheeks {cheeks} tone {tone}");
    std::fs::create_dir_all(&out).unwrap();
    let base = TractNdAnatomy::fart_v1();
    let variants: Vec<(&str, Box<dyn Fn(&mut TractNdAnatomy)>)> = vec![
        ("v0", Box::new(|a| *a = TractNdAnatomy::fart_v0())),
        ("alto (v1)", Box::new(|_| {})),
        ("soprano", Box::new(|a| *a = TractNdAnatomy::fart_v1_soprano())),
        ("bass", Box::new(|a| *a = TractNdAnatomy::fart_v1_bass())),
        ("cannon", Box::new(|a| *a = TractNdAnatomy::fart_v1_cannon())),
    ];
    println!("{:<14} {}", "variant", tensions.iter().map(|t| format!(" {:>6} {:>5} {:>5} {:>5} {:>6} {:>6} |", format!("f0@{t}"), "cents", "sharp", "duty", "hf dB", "rms dB")).collect::<Vec<_>>().join(""));
    for (name, change) in &variants {
        let mut a = base;
        change(&mut a);
        let mut gut = match Gut::with_anatomy(SR, a) {
            Ok(g) => g,
            Err(e) => { println!("{name:<18} failed: {e:?}"); continue; }
        };
        let mut events = vec![Event::Push(push), Event::Cheeks(cheeks), Event::Tone(tone), Event::Tongue(false)];
        for (k, hz) in tensions.iter().enumerate() {
            if k > 0 { events.push(Event::Rest { seconds: 0.3 }); }
            events.push(Event::Note { target_hz: *hz, tension_hz: *hz, seconds: 1.2, shot: false });
        }
        let p = match gut.perform(&events) {
            Ok(p) => p,
            Err(e) => { println!("{name:<18} failed: {e:?}"); continue; }
        };
        let hop = (HOP_S * SR) as usize;
        let mut line = format!("{name:<14}");
        for span in &p.notes {
            let body = (span.start_hop + 20)..span.end_hop;
            let mut f0s: Vec<f64> = p.trace[body.clone()].iter().map(|t| t.f0_hz).filter(|f| *f > 0.0).collect();
            f0s.sort_by(f64::total_cmp);
            let f0 = f0s.get(f0s.len() / 2).copied();
            let x = &p.audio[body.start * hop..body.end * hop];
            let m = shape(x);
            line += &format!(
                " {:>6} {:>5} {:>5.1} {:>5.3} {:>6.1} {:>6.1} |",
                f0.map_or("none".into(), |f| format!("{f:.1}")),
                f0.map_or("-".into(), |f| format!("{:+.0}", 1200.0 * (f / span.tension_hz).log2())),
                m.0, m.1, m.2, m.3
            );
        }
        println!("{line}");
        write_wav(&format!("{out}/{name}.wav"), &p.audio);
    }
}

/// (sharpness, duty, hf share dB, rms dB)
fn shape(x: &[f32]) -> (f64, f64, f64, f64) {
    let rms = (x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len() as f64).sqrt();
    let peak = x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    let duty = x.iter().filter(|v| v.abs() > 0.3 * peak).count() as f64 / x.len() as f64;
    let mut d: Vec<f64> = x.windows(2).map(|w| f64::from(w[1] - w[0]).abs()).collect();
    let d_rms = (d.iter().map(|v| v * v).sum::<f64>() / d.len() as f64).sqrt().max(1e-12);
    d.sort_by(f64::total_cmp);
    let sharp = d[((d.len() as f64 * 0.999) as usize).min(d.len() - 1)] / d_rms;
    // energy above 2 kHz: a one-pole split is enough for a share (first-order high-pass at 2 kHz)
    let k = (-std::f64::consts::TAU * 2000.0 / SR).exp();
    let (mut lp, mut hi, mut tot) = (0.0f64, 0.0f64, 0.0f64);
    for v in x {
        let v = f64::from(*v);
        lp = k * lp + (1.0 - k) * v;
        hi += (v - lp) * (v - lp);
        tot += v * v;
    }
    (sharp, duty, 10.0 * (hi / tot.max(1e-18)).max(1e-12).log10(), 20.0 * rms.max(1e-9).log10())
}

fn write_wav(path: &str, x: &[f32]) {
    let mut f = std::fs::File::create(path).unwrap();
    let data = 4 * x.len() as u32;
    f.write_all(b"RIFF").unwrap();
    f.write_all(&(36 + data).to_le_bytes()).unwrap();
    f.write_all(b"WAVEfmt ").unwrap();
    f.write_all(&16u32.to_le_bytes()).unwrap();
    f.write_all(&3u16.to_le_bytes()).unwrap();
    f.write_all(&1u16.to_le_bytes()).unwrap();
    f.write_all(&(SR as u32).to_le_bytes()).unwrap();
    f.write_all(&((SR as u32) * 4).to_le_bytes()).unwrap();
    f.write_all(&4u16.to_le_bytes()).unwrap();
    f.write_all(&32u16.to_le_bytes()).unwrap();
    f.write_all(b"data").unwrap();
    f.write_all(&data.to_le_bytes()).unwrap();
    for v in x {
        f.write_all(&v.to_le_bytes()).unwrap();
    }
}
