//! Reading resonances off the World's own computed modes (non-loading).
//!
//! With the valve held still, a volume velocity `U` into the glottal volume
//! drives each mode as `a_k'' + 2 sigma_k a_k' + omega_k^2 a_k = phi_k(g) U'`,
//! so the pressure summed over the radiating volumes is
//! `H(w) = sum_k (sum_e phi_k(e)) phi_k(g) j w / (omega_k^2 - w^2 + 2 j sigma_k w)`,
//! `sigma_k` = wall + radiation (the modal approximation the World uses).
//! This is the modal tract's "rigid-glottis" response: the counterpart of the
//! impulse-response column `ovillo-tract` reports for its tube.

use mus_ariadne::{NetworkModes, MODE_COUNT};

/// `|H(f)|` in dB at `f` (Hz).
#[must_use]
pub fn response_db(modes: &NetworkModes, f: f64) -> f64 {
    let w = std::f64::consts::TAU * f;
    let (mut re, mut im) = (0.0, 0.0);
    for k in 0..MODE_COUNT {
        let wk = std::f64::consts::TAU * modes.frequency_hz[k];
        let sigma = modes.wall_sigma[k] + (0..modes.ends).map(|e| modes.radiation_sigma[e][k]).sum::<f64>();
        let gain = (0..modes.ends).map(|e| modes.end_shapes[e][k]).sum::<f64>() * modes.valve_shape[k];
        // gain * j w / (a + j b), a = wk^2 - w^2, b = 2 sigma w
        let a = wk * wk - w * w;
        let b = 2.0 * sigma * w;
        let den = a * a + b * b;
        re += gain * w * b / den;
        im += gain * w * a / den;
    }
    10.0 * (re * re + im * im).max(1e-300).log10()
}

/// Peaks of `|H|` between `lo` and `hi` with `ovillo-tract`'s
/// `spectrum_peaks` rules (local maxima more than 3 dB above the higher of
/// the minima within 150 Hz on either side, at least 180 Hz apart), on a 1 Hz grid.
#[must_use]
pub fn response_peaks(modes: &NetworkModes, lo: f64, hi: f64) -> Vec<f64> {
    let df = 1.0;
    let start = (lo - 200.0).max(1.0);
    let count = ((hi + 200.0 - start) / df) as usize;
    let db: Vec<f64> = (0..count).map(|i| response_db(modes, start + i as f64 * df)).collect();
    let w = (150.0 / df) as usize;
    let mut peaks: Vec<f64> = Vec::new();
    for k in 1..count - 1 {
        let f = start + k as f64 * df;
        if f < lo || f > hi {
            continue;
        }
        if db[k] > db[k - 1] && db[k] >= db[k + 1] {
            let left = db[k.saturating_sub(w)..k].iter().cloned().fold(f64::MAX, f64::min);
            let right = db[k + 1..(k + w).min(count)].iter().cloned().fold(f64::MAX, f64::min);
            if db[k] - left.max(right) > 3.0 && peaks.last().is_none_or(|&p| f - p >= 180.0) {
                peaks.push(f);
            }
        }
    }
    peaks
}
