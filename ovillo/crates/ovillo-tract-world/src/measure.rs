//! COPIED, not edited, from `ovillo/crates/ovillo-tract/src/measure.rs` at
//! `f43f372` (task rule: reuse the tube tract's measurement by copying), so
//! both worlds' renders are measured by byte-identical code. Its own header
//! follows.
//!
//! Measuring what the World did, from its output alone.
//!
//! `lpc_formants` mirrors GARGANTA's reference analysis
//! (`mus_analysis/garganta_anatomy.py` `_lpc_envelope_and_formants`): 16 kHz
//! analysis rate (`DEFAULT_ANALYSIS_SR`), pre-emphasis 0.97, Hamming window,
//! order `clip(2 + sr/1000, 12, 28)`, roots with bandwidth under 800 Hz between
//! 90 Hz and 5.2 kHz. `spectrum_peaks` reads resonances off an impulse
//! response's magnitude spectrum.

use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Formant {
    pub hz: f64,
    pub bandwidth_hz: f64,
}

/// Decimate by an integer factor with a windowed-sinc lowpass (cutoff 0.45 of
/// the new Nyquist band edge).
#[must_use]
pub fn decimate(x: &[f64], factor: usize) -> Vec<f64> {
    if factor <= 1 {
        return x.to_vec();
    }
    let taps = 32 * factor + 1;
    let half = (taps / 2) as isize;
    let cutoff = 0.45 / factor as f64;
    let h: Vec<f64> = (0..taps)
        .map(|i| {
            let k = i as isize - half;
            let sinc = if k == 0 { 2.0 * cutoff } else { (2.0 * PI * cutoff * k as f64).sin() / (PI * k as f64) };
            let w = 0.54 - 0.46 * (2.0 * PI * i as f64 / (taps - 1) as f64).cos();
            sinc * w
        })
        .collect();
    let mut out = Vec::with_capacity(x.len() / factor + 1);
    let mut i = 0usize;
    while i < x.len() {
        let mut acc = 0.0;
        for (t, hv) in h.iter().enumerate() {
            let idx = i as isize + t as isize - half;
            if idx >= 0 && (idx as usize) < x.len() {
                acc += hv * x[idx as usize];
            }
        }
        out.push(acc);
        i += factor;
    }
    out
}

fn levinson(r: &[f64], order: usize) -> Vec<f64> {
    let mut a = vec![0.0; order + 1];
    a[0] = 1.0;
    if r[0] <= 1e-30 {
        return a;
    }
    let mut err = r[0];
    for i in 1..=order {
        let mut acc = r[i];
        for j in 1..i {
            acc += a[j] * r[i - j];
        }
        let k = (-acc / err.max(1e-30)).clamp(-0.999, 0.999);
        let prev = a.clone();
        a[i] = k;
        for j in 1..i {
            a[j] = prev[j] + k * prev[i - j];
        }
        err *= (1.0 - k * k).max(1e-6);
    }
    a
}

/// Roots of `sum_k c[k] z^(n-k)` (c[0] = 1) by Aberth-Ehrlich iteration.
fn poly_roots(c: &[f64]) -> Vec<(f64, f64)> {
    let n = c.len() - 1;
    let eval = |zr: f64, zi: f64| -> ((f64, f64), (f64, f64)) {
        // Horner for p and p'.
        let (mut pr, mut pi) = (c[0], 0.0);
        let (mut dr, mut di) = (0.0, 0.0);
        for &ck in &c[1..] {
            let ndr = dr * zr - di * zi + pr;
            let ndi = dr * zi + di * zr + pi;
            dr = ndr;
            di = ndi;
            let npr = pr * zr - pi * zi + ck;
            let npi = pr * zi + pi * zr;
            pr = npr;
            pi = npi;
        }
        ((pr, pi), (dr, di))
    };
    let radius = 1.0 + c[1..].iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let mut z: Vec<(f64, f64)> = (0..n)
        .map(|k| {
            let th = 2.0 * PI * k as f64 / n as f64 + 0.4;
            (0.9 * radius.min(1.2) * th.cos(), 0.9 * radius.min(1.2) * th.sin())
        })
        .collect();
    for _ in 0..500 {
        let mut moved = 0.0f64;
        for i in 0..n {
            let (zr, zi) = z[i];
            let ((pr, pi), (dr, di)) = eval(zr, zi);
            // ratio = p / p'
            let den = dr * dr + di * di;
            if den < 1e-300 {
                continue;
            }
            let (rr, ri) = ((pr * dr + pi * di) / den, (pi * dr - pr * di) / den);
            let (mut sr, mut si) = (0.0, 0.0);
            for (j, &(wr, wi)) in z.iter().enumerate() {
                if j == i {
                    continue;
                }
                let (er, ei) = (zr - wr, zi - wi);
                let d = er * er + ei * ei;
                if d > 1e-300 {
                    sr += er / d;
                    si += -ei / d;
                }
            }
            // w = ratio / (1 - ratio * s)
            let (tr, ti) = (1.0 - (rr * sr - ri * si), -(rr * si + ri * sr));
            let td = tr * tr + ti * ti;
            if td < 1e-300 {
                continue;
            }
            let (wr, wi) = ((rr * tr + ri * ti) / td, (ri * tr - rr * ti) / td);
            z[i] = (zr - wr, zi - wi);
            moved = moved.max((wr * wr + wi * wi).sqrt());
        }
        if moved < 1e-13 {
            break;
        }
    }
    z
}

/// LPC formants of a voiced frame sampled at `sample_rate_hz` (resampled to
/// 16 kHz first when the rate is a multiple of it).
#[must_use]
pub fn lpc_formants(frame: &[f64], sample_rate_hz: f64) -> Vec<Formant> {
    let factor = (sample_rate_hz / 16_000.0).round().max(1.0) as usize;
    let x = decimate(frame, factor);
    let sr = sample_rate_hz / factor as f64;
    let mut e: Vec<f64> = Vec::with_capacity(x.len());
    let mut prev = 0.0;
    for &v in &x {
        e.push(v - 0.97 * prev);
        prev = v;
    }
    let len = e.len();
    for (i, v) in e.iter_mut().enumerate() {
        *v *= 0.54 - 0.46 * (2.0 * PI * i as f64 / (len - 1) as f64).cos();
    }
    let order = ((2.0 + sr / 1000.0).round() as usize).clamp(12, 28);
    let r: Vec<f64> = (0..=order).map(|lag| (lag..len).map(|i| e[i] * e[i - lag]).sum()).collect();
    let a = levinson(&r, order);
    let mut out: Vec<Formant> = poly_roots(&a)
        .into_iter()
        .filter(|&(_, im)| im > 0.0)
        .map(|(re, im)| {
            let hz = im.atan2(re) * sr / (2.0 * PI);
            let mag = (re * re + im * im).sqrt().max(1e-12);
            Formant { hz, bandwidth_hz: -0.5 * sr / PI * mag.ln() }
        })
        .filter(|f| f.hz >= 90.0 && f.hz <= 5_200.0f64.min(sr / 2.0 - 100.0) && f.bandwidth_hz > 0.0 && f.bandwidth_hz < 800.0)
        .collect();
    out.sort_by(|p, q| p.hz.total_cmp(&q.hz));
    out.truncate(4);
    out
}

/// In-place radix-2 complex FFT (len must be a power of two).
pub fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * PI / len as f64;
        let (wr, wi) = (ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for k in 0..len / 2 {
                let (ar, ai) = (re[start + k], im[start + k]);
                let (br, bi) = (re[start + k + len / 2], im[start + k + len / 2]);
                let (tr, ti) = (br * cr - bi * ci, br * ci + bi * cr);
                re[start + k] = ar + tr;
                im[start + k] = ai + ti;
                re[start + k + len / 2] = ar - tr;
                im[start + k + len / 2] = ai - ti;
                let ncr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = ncr;
            }
        }
        len <<= 1;
    }
}

/// Magnitude spectrum (dB) of `x` zero-padded to `n` (power of two), with bin
/// spacing `rate / n`.
#[must_use]
pub fn magnitude_db(x: &[f64], n: usize) -> Vec<f64> {
    let mut re = vec![0.0; n];
    let mut im = vec![0.0; n];
    for (i, v) in x.iter().take(n).enumerate() {
        re[i] = *v;
    }
    fft(&mut re, &mut im);
    (0..n / 2).map(|k| 10.0 * (re[k] * re[k] + im[k] * im[k]).max(1e-300).log10()).collect()
}

/// Resonance peaks of an impulse response between `lo` and `hi` Hz: local
/// maxima at least 6 dB above the lower of their flanking minima, at least
/// 180 Hz apart (GARGANTA `modeled_formants` spacing).
#[must_use]
pub fn spectrum_peaks(ir: &[f64], rate_hz: f64, lo: f64, hi: f64) -> Vec<f64> {
    spectrum_peaks_n(ir, rate_hz, lo, hi, 1 << 17)
}

/// As [`spectrum_peaks`] with an explicit FFT size (power of two).
#[must_use]
pub fn spectrum_peaks_n(ir: &[f64], rate_hz: f64, lo: f64, hi: f64, n: usize) -> Vec<f64> {
    let db = magnitude_db(ir, n);
    let df = rate_hz / n as f64;
    let k_lo = (lo / df) as usize;
    let k_hi = ((hi / df) as usize).min(db.len() - 2);
    let mut peaks = Vec::new();
    for k in k_lo.max(1)..k_hi {
        if db[k] > db[k - 1] && db[k] >= db[k + 1] {
            let w = (150.0 / df) as usize;
            let left = db[k.saturating_sub(w)..k].iter().cloned().fold(f64::MAX, f64::min);
            let right = db[k + 1..(k + w).min(db.len())].iter().cloned().fold(f64::MAX, f64::min);
            if db[k] - left.max(right) > 3.0 {
                let hz = k as f64 * df;
                if peaks.last().is_none_or(|&p: &f64| hz - p >= 180.0) {
                    peaks.push(hz);
                }
            }
        }
    }
    peaks
}

/// Fundamental from autocorrelation (60..500 Hz).
#[must_use]
pub fn f0_autocorrelation(x: &[f64], rate_hz: f64) -> f64 {
    let lo = (rate_hz / 500.0) as usize;
    let hi = (rate_hz / 60.0) as usize;
    let mean = x.iter().sum::<f64>() / x.len() as f64;
    let y: Vec<f64> = x.iter().map(|v| v - mean).collect();
    let r0: f64 = y.iter().map(|v| v * v).sum();
    let mut best = (0.0, 0usize);
    for lag in lo..hi.min(y.len() / 2) {
        let r: f64 = (lag..y.len()).map(|i| y[i] * y[i - lag]).sum::<f64>() / r0.max(1e-30);
        if r > best.0 {
            best = (r, lag);
        }
    }
    if best.1 == 0 {
        0.0
    } else {
        rate_hz / best.1 as f64
    }
}
