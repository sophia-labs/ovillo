//! Learn the v1 trombone World the way a player does: for each partial, slide
//! position, lip tension (as a ratio of the bore mode) and mouth pressure,
//! tongue a 0.6 s note on a fresh World and listen to the last 0.2 s.
//!
//! ```text
//! cargo run --release -p ovillo-dsp --example trombone_nd_calibrate -- <out.tsv> [threads]
//! ```
//! Columns: partial, position (1..7), slide_m, ratio, mouth, bore_hz (the
//! computed bore mode n there), f_hz (autocorrelation pitch), clarity, rms,
//! harmonic_centroid (sum k a_k^2 / sum a_k^2 over 8 harmonics; 1 = a sine).

use ovillo_dsp::trombone_nd::{Breath, TromboneNd, PLAN_JSON};
use ovillo_dsp::trombone_nd_player::{ModeTable, EMBOUCHURE};

fn pitch(x: &[f64], sr: f64, fmin: f64, fmax: f64) -> (f64, f64) {
    let n = x.len();
    let mean = x.iter().sum::<f64>() / n as f64;
    let y: Vec<f64> = x.iter().map(|v| v - mean).collect();
    if y.iter().all(|v| *v == 0.0) {
        return (0.0, 0.0);
    }
    let lo = (sr / fmax) as usize;
    let hi = ((sr / fmin) as usize).min(n / 2);
    let ac: Vec<f64> = (0..=hi + 1)
        .map(|lag| {
            let s: f64 = (0..n - lag).map(|i| y[i] * y[i + lag]).sum();
            let ea: f64 = (0..n - lag).map(|i| y[i] * y[i]).sum();
            let eb: f64 = (lag..n).map(|i| y[i] * y[i]).sum();
            s / (ea * eb).sqrt().max(1e-300)
        })
        .collect();
    let peaks: Vec<usize> = (lo.max(2)..hi).filter(|&l| ac[l] >= ac[l - 1] && ac[l] >= ac[l + 1]).collect();
    let Some(best) = peaks.iter().map(|&l| ac[l]).reduce(f64::max) else { return (0.0, 0.0) };
    let l = *peaks.iter().find(|&&l| ac[l] >= 0.9 * best).unwrap();
    let (a, b, c) = (ac[l - 1], ac[l], ac[l + 1]);
    let d = 0.5 * (a - c) / (a - 2.0 * b + c);
    (sr / (l as f64 + if d.is_finite() { d.clamp(-0.5, 0.5) } else { 0.0 }), b)
}

fn harmonic_centroid(x: &[f64], f: f64, sr: f64) -> f64 {
    let mut p = [0.0f64; 8];
    for (k, a) in p.iter_mut().enumerate() {
        let w = std::f64::consts::TAU * f * (k + 1) as f64 / sr;
        if f <= 0.0 || w >= std::f64::consts::PI {
            continue;
        }
        let (mut re, mut im) = (0.0, 0.0);
        let m = x.len() as f64;
        for (i, v) in x.iter().enumerate() {
            let h = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / m).cos();
            re += h * v * (w * i as f64).cos();
            im -= h * v * (w * i as f64).sin();
        }
        *a = re * re + im * im;
    }
    let s: f64 = p.iter().sum();
    if s > 0.0 {
        p.iter().enumerate().map(|(k, a)| (k + 1) as f64 * a).sum::<f64>() / s
    } else {
        0.0
    }
}

struct Job {
    n: usize,
    position: usize,
    slide: f64,
    ratio: f64,
    mouth: f64,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| "calibration.tsv".into());
    let threads: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(8);
    let table = ModeTable::embedded().expect("mode table");
    let mut jobs = Vec::new();
    for n in 2..=10 {
        for (k, &slide) in table.positions_m.iter().enumerate() {
            for r in 0..21 {
                for &mouth in &[0.12, 0.18, 0.26, 0.37, 0.52, 0.72] {
                    jobs.push(Job { n, position: k + 1, slide, ratio: 0.80 + 0.025 * r as f64, mouth });
                }
            }
        }
    }
    let jobs = std::sync::Arc::new(jobs);
    let table = std::sync::Arc::new(table);
    let next = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let t0 = std::time::Instant::now();
    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let (jobs, next, table) = (jobs.clone(), next.clone(), table.clone());
            std::thread::spawn(move || {
                let sr = 48_000.0;
                let mut rows = Vec::new();
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if i >= jobs.len() {
                        break;
                    }
                    let j = &jobs[i];
                    let mut tw = TromboneNd::from_json(PLAN_JSON).expect("plan");
                    tw.set_slide_now(j.slide).expect("slide");
                    let bore = table.mode_hz(j.n, j.slide);
                    let lip_hz = j.ratio * bore;
                    let len = (0.6 * sr) as usize;
                    let mut y = Vec::with_capacity(len);
                    for s in 0..len {
                        let t = s as f64 / sr;
                        let mouth = j.mouth * (1.0 - (-t / 0.006).exp());
                        y.push(tw.tick(Breath { mouth, lip_hz, embouchure: EMBOUCHURE, slide_m: j.slide }).expect("tick"));
                    }
                    let tail = &y[len - (0.2 * sr) as usize..];
                    let rms = (tail.iter().map(|v| v * v).sum::<f64>() / tail.len() as f64).sqrt();
                    let (f, clarity) = pitch(tail, sr, 25.0, 1500.0);
                    let hc = harmonic_centroid(tail, f, sr);
                    rows.push((i, format!(
                        "{}\t{}\t{:.5}\t{:.4}\t{:.3}\t{:.3}\t{:.3}\t{:.4}\t{:.4e}\t{:.3}\n",
                        j.n, j.position, j.slide, j.ratio, j.mouth, bore, f, clarity, rms, hc
                    )));
                }
                rows
            })
        })
        .collect();
    let mut rows: Vec<(usize, String)> = handles.into_iter().flat_map(|h| h.join().unwrap()).collect();
    rows.sort_by_key(|r| r.0);
    let mut text = String::from("partial\tposition\tslide_m\tratio\tmouth\tbore_hz\tf_hz\tclarity\trms\tharmonic_centroid\n");
    for (_, r) in rows {
        text += &r;
    }
    std::fs::write(&out, text).unwrap();
    eprintln!("{} notes in {:.1} s -> {out}", jobs.len(), t0.elapsed().as_secs_f64());
}
