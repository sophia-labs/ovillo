//! Learn the trombone World the way a player does: for each partial, slide
//! position, lip tension (as a ratio of the bore resonance) and mouth
//! pressure, play a tongued note on the World and listen to what it sounds.
//!
//! ```text
//! cargo run --release -p ovillo-dsp --example trombone_world_calibrate -- <out.tsv> [threads]
//! ```
//! Every note is a fresh World, tongued from silence, 0.5 s long; the pitch
//! is the autocorrelation peak of the last 0.15 s; harmonic_centroid is
//! sum k a_k^2 / sum a_k^2 over the first 8 harmonics there (1 = a sine), and
//! h2_db, h3_db the 2nd and 3rd harmonics against the 1st.

use ovillo_dsp::trombone_world::{Breath, TromboneWorld};

pub fn pitch(x: &[f64], sr: f64, fmin: f64, fmax: f64) -> (f64, f64) {
    let n = x.len();
    let mean = x.iter().sum::<f64>() / n as f64;
    let y: Vec<f64> = x.iter().map(|v| v - mean).collect();
    let e0: f64 = y.iter().map(|v| v * v).sum();
    if e0 <= 0.0 {
        return (0.0, 0.0);
    }
    let lo = (sr / fmax) as usize;
    let hi = ((sr / fmin) as usize).min(n / 2);
    let ac: Vec<f64> = (0..=hi + 1)
        .map(|lag| {
            let s: f64 = (0..n - lag).map(|i| y[i] * y[i + lag]).sum();
            let ea: f64 = (0..n - lag).map(|i| y[i] * y[i]).sum::<f64>();
            let eb: f64 = (lag..n).map(|i| y[i] * y[i]).sum::<f64>();
            s / (ea * eb).sqrt().max(1e-300)
        })
        .collect();
    let m = (lo..=hi).map(|l| ac[l]).fold(f64::MIN, f64::max);
    let mut best = lo;
    for l in lo.max(1)..=hi {
        if ac[l] >= ac[l - 1] && ac[l] >= ac[l + 1] && ac[l] > 0.9 * m {
            best = l;
            break;
        }
    }
    let (a, b, c) = (ac[best - 1], ac[best], ac[best + 1]);
    let d = 0.5 * (a - c) / (a - 2.0 * b + c);
    let d = if d.is_finite() { d.clamp(-0.5, 0.5) } else { 0.0 };
    (sr / (best as f64 + d), b)
}

struct Job {
    plan: std::sync::Arc<(String, String)>,
    secs: f64,
    n: usize,
    slide: f64,
    ratio: f64,
    pressure: f64,
}

fn run(job: &Job) -> String {
    let sr = 48_000.0;
    let mut tw = TromboneWorld::from_json(&job.plan.0, &job.plan.1).expect("trombone world");
    let f_n = tw.bore_resonance_hz(job.n, job.slide);
    let lip_hz = job.ratio * f_n;
    let len = (job.secs * sr) as usize;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f64 / sr;
        let pressure = job.pressure * (1.0 - (-t / 0.008).exp());
        let o = tw.tick(Breath { pressure, lip_hz, slide: job.slide }).expect("tick");
        out.push(o.left);
    }
    let tail = &out[len - (0.15 * sr) as usize..];
    let rms = (tail.iter().map(|v| v * v).sum::<f64>() / tail.len() as f64).sqrt();
    let (f, clarity) = pitch(tail, sr, 25.0, 1500.0);
    // harmonic amplitudes at k f (Goertzel, Hann window) and their centroid
    let mut amp = [0.0f64; 8];
    for (k, a) in amp.iter_mut().enumerate() {
        let w = std::f64::consts::TAU * f * (k + 1) as f64 / sr;
        if f <= 0.0 || w >= std::f64::consts::PI {
            continue;
        }
        let (mut re, mut im) = (0.0, 0.0);
        let m = tail.len() as f64;
        for (i, x) in tail.iter().enumerate() {
            let h = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / m).cos();
            re += h * x * (w * i as f64).cos();
            im -= h * x * (w * i as f64).sin();
        }
        *a = (re * re + im * im).sqrt();
    }
    let p: f64 = amp.iter().map(|a| a * a).sum();
    let centroid = if p > 0.0 { amp.iter().enumerate().map(|(k, a)| (k + 1) as f64 * a * a).sum::<f64>() / p } else { 0.0 };
    let db = |a: f64| 20.0 * (a / amp[0].max(1e-300)).max(1e-15).log10();
    format!(
        "{}\t{:.4}\t{:.4}\t{:.1}\t{:.3}\t{:.3}\t{:.4}\t{:.4e}\t{:.3}\t{:.1}\t{:.1}\n",
        job.n, job.slide, job.ratio, job.pressure, f_n, f, clarity, rms, centroid, db(amp[1]), db(amp[2])
    )
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).cloned().unwrap_or_else(|| "calibration.tsv".into());
    let threads: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(8);
    // optional: <plan.json> <receipt.json> quick
    let plan = std::sync::Arc::new(match (args.get(3), args.get(4)) {
        (Some(p), Some(r)) => (std::fs::read_to_string(p).unwrap(), std::fs::read_to_string(r).unwrap()),
        _ => (ovillo_dsp::trombone_world::PLAN_JSON.to_owned(), ovillo_dsp::trombone_world::RECEIPT_JSON.to_owned()),
    });
    let quick = args.get(5).is_some_and(|s| s == "quick");
    let mut jobs = Vec::new();
    let (partials, slides, ratios, pressures, secs): (Vec<usize>, Vec<f64>, Vec<f64>, Vec<f64>, f64) = if quick {
        ((2..=7).collect(), vec![0.0], (0..10).map(|k| 0.62 + 0.06 * k as f64).collect(), vec![800.0, 1500.0, 3000.0, 6000.0], 0.6)
    } else {
        ((2..=10).collect(), (0..7).map(|k| k as f64 / 6.0).collect(), (0..20).map(|k| 0.28 + 0.04 * k as f64).collect(), vec![1000.0, 1600.0, 2400.0, 3500.0, 5000.0, 7000.0], 0.5)
    };
    for &n in &partials {
        for &slide in &slides {
            for &ratio in &ratios {
                for &pressure in &pressures {
                    jobs.push(Job { plan: plan.clone(), secs, n, slide, ratio, pressure });
                }
            }
        }
    }
    let jobs = std::sync::Arc::new(jobs);
    let next = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let t0 = std::time::Instant::now();
    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let jobs = jobs.clone();
            let next = next.clone();
            std::thread::spawn(move || {
                let mut rows = Vec::new();
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if i >= jobs.len() {
                        break;
                    }
                    rows.push((i, run(&jobs[i])));
                }
                rows
            })
        })
        .collect();
    let mut rows: Vec<(usize, String)> = handles.into_iter().flat_map(|h| h.join().unwrap()).collect();
    rows.sort_by_key(|r| r.0);
    let mut text = String::from("partial\tslide\tratio\tpressure\tbore_hz\tf_hz\tclarity\trms\tharmonic_centroid\th2_db\th3_db\n");
    for (_, r) in rows {
        text += &r;
    }
    std::fs::write(&out, text).unwrap();
    eprintln!("{} notes in {:.1} s -> {out}", jobs.len(), t0.elapsed().as_secs_f64());
}
