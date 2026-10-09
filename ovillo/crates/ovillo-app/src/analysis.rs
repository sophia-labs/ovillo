//! What the screen measures, on the UI thread: a history of the last second
//! or so of samples (dry, wet L, wet R), a spectrum analyzer with video
//! averaging and max hold, a triggered scope window, and a history of the
//! World's own readings for the strip chart.

use ovillo_dsp::rig::{ScopeFrame, Telemetry};
use realfft::num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};
use std::collections::VecDeque;
use std::sync::Arc;

pub const HISTORY: usize = 1 << 16;
pub const FFT_N: usize = 8192;
/// Seconds of World readings kept for the strip chart.
pub const WORLD_SPAN_S: f64 = 10.0;

pub struct History {
    buf: Vec<ScopeFrame>,
    w: usize,
    filled: usize,
}

impl History {
    pub fn new() -> Self {
        Self { buf: vec![ScopeFrame::default(); HISTORY], w: 0, filled: 0 }
    }

    pub fn push(&mut self, f: ScopeFrame) {
        self.buf[self.w] = f;
        self.w = (self.w + 1) % HISTORY;
        self.filled = (self.filled + 1).min(HISTORY);
    }

    /// The newest `n` frames, oldest first.
    pub fn latest(&self, n: usize, out: &mut Vec<ScopeFrame>) {
        let n = n.min(self.filled);
        out.clear();
        let start = (self.w + HISTORY - n) % HISTORY;
        for i in 0..n {
            out.push(self.buf[(start + i) % HISTORY]);
        }
    }
}

pub struct Spectrum {
    fft: Arc<dyn RealToComplex<f32>>,
    window: Vec<f32>,
    norm: f32,
    input: Vec<f32>,
    out: Vec<Complex<f32>>,
    scratch: Vec<Complex<f32>>,
    dry_pow: Vec<f32>,
    wet_pow: Vec<f32>,
    pub dry_db: Vec<f32>,
    pub wet_db: Vec<f32>,
    pub hold_db: Vec<f32>,
}

impl Spectrum {
    pub fn new() -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_N);
        let window: Vec<f32> = (0..FFT_N)
            .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / FFT_N as f32).cos())
            .collect();
        // A full-scale sine at a bin centre reads 0 dBFS.
        let norm = 2.0 / window.iter().sum::<f32>();
        let bins = FFT_N / 2 + 1;
        Self {
            input: vec![0.0; FFT_N],
            out: fft.make_output_vec(),
            scratch: fft.make_scratch_vec(),
            fft,
            window,
            norm,
            dry_pow: vec![0.0; bins],
            wet_pow: vec![0.0; bins],
            dry_db: vec![-160.0; bins],
            wet_db: vec![-160.0; bins],
            hold_db: vec![-160.0; bins],
        }
    }

    fn power(&mut self, frames: &[ScopeFrame], wet: bool, avg: &mut [f32]) {
        for (i, f) in frames.iter().enumerate().take(FFT_N) {
            let x = if wet { 0.5 * (f.left + f.right) } else { f.dry };
            self.input[i] = x * self.window[i];
        }
        if self.fft.process_with_scratch(&mut self.input, &mut self.out, &mut self.scratch).is_err() {
            return;
        }
        for (a, c) in avg.iter_mut().zip(&self.out) {
            let m = c.norm() * self.norm;
            // Video averaging in the power domain.
            *a = 0.65 * *a + 0.35 * m * m;
        }
    }

    /// Analyze the newest `FFT_N` frames (`frames.len()` must be `FFT_N`).
    pub fn update(&mut self, frames: &[ScopeFrame]) {
        if frames.len() < FFT_N {
            return;
        }
        let mut dry = std::mem::take(&mut self.dry_pow);
        let mut wet = std::mem::take(&mut self.wet_pow);
        self.power(frames, false, &mut dry);
        self.power(frames, true, &mut wet);
        for k in 0..dry.len() {
            self.dry_db[k] = 10.0 * dry[k].max(1e-16).log10();
            self.wet_db[k] = 10.0 * wet[k].max(1e-16).log10();
            self.hold_db[k] = self.hold_db[k].max(self.wet_db[k]);
        }
        self.dry_pow = dry;
        self.wet_pow = wet;
    }

    pub fn clear_hold(&mut self) {
        self.hold_db.fill(-160.0);
    }
}

/// Start index of a triggered scope window of `len` frames in `hist` (oldest
/// first): the latest rising crossing of the lowpassed dry signal through
/// zero, after it has been at least `arm` below zero. Falls back to the
/// newest window when nothing triggers.
pub fn trigger(hist: &[ScopeFrame], len: usize, sr: f64) -> (usize, bool) {
    if hist.len() <= len + 2 {
        return (0, false);
    }
    let last_start = hist.len() - len;
    // One-pole lowpass at ~250 Hz so harmonics do not double-trigger.
    let a = (-std::f64::consts::TAU * 250.0 / sr).exp() as f32;
    let mut lp = vec![0.0f32; last_start + 1];
    let mut y = 0.0f32;
    for i in 0..=last_start {
        y = (1.0 - a) * hist[i].dry + a * y;
        lp[i] = y;
    }
    let peak = lp.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    if peak < 1e-4 {
        return (last_start, false);
    }
    let arm = -0.1 * peak;
    let mut armed_at = None;
    // Walk backward: find a rising crossing whose recent past dipped below `arm`.
    for i in (1..=last_start).rev() {
        if lp[i - 1] < 0.0 && lp[i] >= 0.0 {
            let back = i.saturating_sub((sr / 60.0) as usize);
            if lp[back..i].iter().any(|v| *v < arm) {
                armed_at = Some(i);
                break;
            }
        }
    }
    match armed_at {
        Some(i) => (i, true),
        None => (last_start, false),
    }
}

/// The World's readings, compacted for the strip chart.
#[derive(Debug, Clone, Copy)]
pub struct WorldPoint {
    pub t: f64,
    pub material: f32,
    pub reservoir: f32,
    pub space: f32,
}

pub struct WorldHistory {
    pub points: VecDeque<WorldPoint>,
}

impl WorldHistory {
    pub fn new() -> Self {
        Self { points: VecDeque::with_capacity(4096) }
    }

    pub fn push(&mut self, t: &Telemetry) {
        let db = |e: f64| (10.0 * e.max(1e-30).log10()) as f32;
        let now = t.sample as f64 / t.sample_rate;
        self.points.push_back(WorldPoint {
            t: now,
            material: db(t.world.material_energy),
            reservoir: db(t.world.reservoir_energy),
            space: db(t.world.space_energy),
        });
        while self.points.front().is_some_and(|p| p.t < now - WORLD_SPAN_S - 0.5) {
            self.points.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(f: f32, n: usize, sr: f32, amp: f32) -> Vec<ScopeFrame> {
        (0..n)
            .map(|i| {
                let x = amp * (std::f32::consts::TAU * f * i as f32 / sr).sin();
                ScopeFrame { dry: x, left: x, right: x }
            })
            .collect()
    }

    #[test]
    fn a_full_scale_sine_reads_near_0_dbfs_at_its_bin() {
        let sr = 48_000.0;
        let k = 200;
        let f = k as f32 * sr / FFT_N as f32;
        let frames = sine(f, FFT_N, sr, 1.0);
        let mut s = Spectrum::new();
        for _ in 0..40 {
            s.update(&frames);
        }
        assert!((s.dry_db[k]).abs() < 0.2, "{}", s.dry_db[k]);
        assert!(s.dry_db[k + 20] < -60.0);
    }

    #[test]
    fn the_scope_triggers_on_a_rising_zero_crossing() {
        let sr = 48_000.0;
        let frames = sine(110.0, 20_000, sr as f32, 0.3);
        let (start, ok) = trigger(&frames, 2_400, sr);
        assert!(ok);
        // The lowpass delays the crossing a little; the raw signal is near zero and rising.
        let lag = (sr / (std::f64::consts::TAU * 250.0)) as usize;
        let raw = start.saturating_sub(lag);
        assert!(frames[raw].dry.abs() < 0.1 && frames[raw + 5].dry > frames[raw].dry);
    }
}
