//! Measure the simulated strings' inharmonicity B from their partials:
//! f_k = k f0 sqrt(1 + B k^2), fitted to the first partials of an isolated pluck.
use ovillo_dsp::guitar::{Guitar, GuitarSettings, Pickup, OPEN_HZ, STRING_NAMES};

fn partials(x: &[f64], sr: f64, f_nom: f64, kmax: usize) -> Vec<(usize, f64)> {
    let n = x.len();
    // Plain DFT around each expected partial (n is modest; fine for a calibration tool).
    let mut out = Vec::new();
    for k in 1..=kmax {
        let centre = k as f64 * f_nom;
        if centre > sr * 0.4 {
            break;
        }
        let res = sr / n as f64;
        let (lo, hi) = (((centre * 0.97) / res) as usize, ((centre * 1.05) / res) as usize);
        let mag = |b: usize| {
            let w = std::f64::consts::TAU * b as f64 / n as f64;
            let (mut re, mut im) = (0.0, 0.0);
            for (i, v) in x.iter().enumerate() {
                let h = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos();
                re += v * h * (w * i as f64).cos();
                im -= v * h * (w * i as f64).sin();
            }
            (re * re + im * im).sqrt()
        };
        let mags: Vec<f64> = (lo..=hi).map(mag).collect();
        let (bi, _) = mags.iter().enumerate().fold((0, 0.0), |(bi, bm), (i, m)| if *m > bm { (i, *m) } else { (bi, bm) });
        if bi == 0 || bi + 1 >= mags.len() {
            continue;
        }
        let (a, b, c) = (mags[bi - 1].ln(), mags[bi].ln(), mags[bi + 1].ln());
        let d = 0.5 * (a - c) / (a - 2.0 * b + c);
        out.push((k, (lo + bi) as f64 * res + d * res));
    }
    out
}

fn main() {
    let sr = 48_000.0;
    for fret in [0.0, 12.0] {
        for s in 0..6 {
            let mut g = Guitar::new(sr);
            g.set(GuitarSettings { pickup: Pickup::Bridge, tone: 1.0, volume: 1.0, hum: false });
            g.pluck(s, fret, 0.9, false);
            let x: Vec<f64> = (0..(sr as usize)).map(|_| g.tick() as f64).collect();
            let seg = &x[2_400..2_400 + 16_384];
            let f_nom = OPEN_HZ[s] * 2f64.powf(fret / 12.0);
            let p = partials(seg, sr, f_nom, 14);
            // Grid fit of f0 and B.
            let mut best = (f64::MAX, 0.0, 0.0);
            for bi in 0..400 {
                let b = bi as f64 * 2.5e-6;
                // Least-squares f0 for this B: f_k = f0 * k sqrt(1+Bk^2).
                let (num, den) = p.iter().fold((0.0, 0.0), |(n, d), (k, f)| {
                    let g = *k as f64 * (1.0 + b * (*k as f64).powi(2)).sqrt();
                    (n + f * g, d + g * g)
                });
                let f0 = num / den;
                let err: f64 = p.iter().map(|(k, f)| (f - f0 * *k as f64 * (1.0 + b * (*k as f64).powi(2)).sqrt()).powi(2)).sum();
                if err < best.0 {
                    best = (err, f0, b);
                }
            }
            let cents = 1200.0 * (best.1 / f_nom).log2();
            println!(
                "{} fret {:2}: partials {:2}  f0 {:7.2} Hz ({:+5.1} c)  B {:.2e}  k=10 sharp by {:4.1} c",
                STRING_NAMES[s], fret as u8, p.len(), best.1, cents, best.2,
                1200.0 * (1.0 + best.2 * 100.0).sqrt().log2()
            );
        }
    }
}
