//! A simulated electric guitar, heard as a DI signal: what the jack would put
//! into an audio interface's instrument input.
//!
//! Each string is an extended Karplus–Strong waveguide: a fractional delay
//! line (third-order Lagrange read) closed through a one-pole loop lowpass and
//! a per-pass gain set from the string's T60. The loop lowpass's phase delay
//! at the fundamental is subtracted from the line, so strings play in tune.
//! Stiffness is a cascade of first-order allpass sections in the loop, so
//! partials run sharp (inharmonicity), more on the plain strings than the
//! wound ones; its phase delay at the fundamental is compensated too.
//!
//! A pluck injects one period of noise, darkened by a soft pick and combed at
//! the picking point. The pick and the pickups sit at fixed distances from the
//! bridge, so their combs are fixed in time per string and move, relative to
//! the harmonics, as the fret shortens the string: the neck pickup loses every
//! fourth harmonic of an open string and every second at the twelfth fret.
//! These are the cues that tell one string from another for the same pitch.
//! The coil and cable form a resonant lowpass whose peak the tone knob pulls
//! down. Single coils hum at 60 Hz; humbuckers do not.
//!
//! [`Guitar::truth`] reports what each string is doing (pitch and level), so
//! analysis of the DI signal can be scored against it.

use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

pub const STRINGS: usize = 6;
/// Standard tuning, low E to high e: the same six courses Ariadne's string
/// body is tuned to.
pub const OPEN_HZ: [f64; STRINGS] =
    [82.406_889_228, 110.0, 146.832_383_959, 195.997_717_991, 246.941_650_628, 329.627_556_913];
pub const STRING_NAMES: [&str; STRINGS] = ["E2", "A2", "D3", "G3", "B3", "E4"];

const BUF: usize = 8192;
const MASK: usize = BUF - 1;

/// Sustain of each open string (s), wound strings longest.
const T60_OPEN: [f64; STRINGS] = [8.0, 7.0, 6.2, 5.2, 4.4, 3.8];
/// Loop lowpass pole per string: wound strings lose their highs sooner.
const LOOP_POLE: [f64; STRINGS] = [0.42, 0.38, 0.33, 0.27, 0.23, 0.20];
/// Stiffness: allpass sections and coefficient per string. Plain strings
/// (G, B, e) are stiffer for their pitch than wound ones; the plain G most.
const DISPERSION_SECTIONS: usize = 4;
const DISPERSION: [f64; STRINGS] = [-0.70, -0.68, -0.64, -0.72, -0.62, -0.52];
const PALM_T60: f64 = 0.16;
const PALM_POLE: f64 = 0.78;
const MUTE_T60: f64 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Pickup {
    Neck,
    Middle,
    Bridge,
    Humbucker,
}

impl Pickup {
    pub const ALL: [Pickup; 4] = [Pickup::Neck, Pickup::Middle, Pickup::Bridge, Pickup::Humbucker];

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Pickup::Neck => "NECK SC",
            Pickup::Middle => "MID SC",
            Pickup::Bridge => "BRIDGE SC",
            Pickup::Humbucker => "HUMBUCKER",
        }
    }

    /// Coil positions as fractions of the scale from the bridge.
    fn positions(self) -> &'static [f64] {
        match self {
            Pickup::Neck => &[0.25],
            Pickup::Middle => &[0.16],
            Pickup::Bridge => &[0.085],
            Pickup::Humbucker => &[0.068, 0.098],
        }
    }

    /// Coil + cable resonance (Hz, Q) with the tone knob fully open.
    fn resonance(self) -> (f64, f64) {
        match self {
            Pickup::Humbucker => (2_900.0, 1.6),
            _ => (5_000.0, 2.2),
        }
    }

    fn hum(self) -> f64 {
        match self {
            Pickup::Humbucker => 0.000_05,
            _ => 0.000_9,
        }
    }
}

/// What one string is doing, from inside the model.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StringTruth {
    /// The fret last plucked (0 = open).
    pub fret: u8,
    /// Sounding pitch in semitones above the open string (fret + bend + vibrato).
    pub semitones: f32,
    /// Sounding fundamental (Hz).
    pub hz: f32,
    /// Envelope of this string's pickup signal (linear, same scale as the DI before the coil).
    pub level: f32,
    /// Plucked since the last mute and not yet decayed below audibility.
    pub sounding: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GuitarSettings {
    pub pickup: Pickup,
    /// Tone knob, 0 (dark) to 1 (open).
    pub tone: f64,
    /// Volume knob, 0 to 1.
    pub volume: f64,
    /// Mains hum and noise floor on or off.
    pub hum: bool,
}

impl Default for GuitarSettings {
    fn default() -> Self {
        Self { pickup: Pickup::Bridge, tone: 0.85, volume: 0.8, hum: true }
    }
}

/// Small deterministic noise source (xorshift32).
#[derive(Debug, Clone)]
pub struct Rng(u32);

impl Rng {
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self(seed.max(1))
    }
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    /// Uniform in [-1, 1).
    #[inline]
    pub fn bipolar(&mut self) -> f64 {
        (self.next_u32() as f64 / u32::MAX as f64) * 2.0 - 1.0
    }
    /// Uniform in [0, 1).
    #[inline]
    pub fn unit(&mut self) -> f64 {
        self.next_u32() as f64 / (u32::MAX as f64 + 1.0)
    }
}

struct GString {
    open_hz: f64,
    buf: Vec<f32>,
    exc: Vec<f32>,
    w: usize,
    exc_len: usize,
    exc_pos: usize,
    fret: f64,
    bend: f64,
    bend_target: f64,
    vib_depth: f64,
    vib_target: f64,
    vib_phase: f64,
    lp: f64,
    pole: f64,
    pole_target: f64,
    t60: f64,
    t60_target: f64,
    base_t60: f64,
    base_pole: f64,
    period: f64,
    open_period: f64,
    disp: f64,
    ap_x: [f64; DISPERSION_SECTIONS],
    ap_y: [f64; DISPERSION_SECTIONS],
    semis: f64,
    env: f64,
    plucked: bool,
}

impl GString {
    fn new(i: usize) -> Self {
        Self {
            open_hz: OPEN_HZ[i],
            buf: vec![0.0; BUF],
            exc: vec![0.0; BUF],
            w: 0,
            exc_len: 0,
            exc_pos: 0,
            fret: 0.0,
            bend: 0.0,
            bend_target: 0.0,
            vib_depth: 0.0,
            vib_target: 0.0,
            vib_phase: 0.0,
            lp: 0.0,
            pole: LOOP_POLE[i],
            pole_target: LOOP_POLE[i],
            t60: T60_OPEN[i],
            t60_target: T60_OPEN[i],
            base_t60: T60_OPEN[i],
            base_pole: LOOP_POLE[i],
            period: 0.0,
            open_period: 0.0,
            disp: DISPERSION[i],
            ap_x: [0.0; DISPERSION_SECTIONS],
            ap_y: [0.0; DISPERSION_SECTIONS],
            semis: 0.0,
            env: 0.0,
            plucked: false,
        }
    }

    /// Read the line `delay` samples back (delay ≥ 2), third-order Lagrange.
    #[inline]
    fn read(&self, delay: f64) -> f64 {
        let i = delay.floor();
        let t = delay - i + 1.0; // position past the first tap, in [1, 2)
        let base = self.w.wrapping_sub(i as usize - 1);
        let x0 = self.buf[base & MASK] as f64;
        let x1 = self.buf[base.wrapping_sub(1) & MASK] as f64;
        let x2 = self.buf[base.wrapping_sub(2) & MASK] as f64;
        let x3 = self.buf[base.wrapping_sub(3) & MASK] as f64;
        let h0 = -(t - 1.0) * (t - 2.0) * (t - 3.0) / 6.0;
        let h1 = t * (t - 2.0) * (t - 3.0) / 2.0;
        let h2 = -t * (t - 1.0) * (t - 3.0) / 2.0;
        let h3 = t * (t - 1.0) * (t - 2.0) / 6.0;
        h0 * x0 + h1 * x1 + h2 * x2 + h3 * x3
    }

    fn pluck(&mut self, sr: f64, fret: f64, velocity: f64, palm: bool, pick_at: f64, rng: &mut Rng) {
        self.fret = fret;
        self.bend = 0.0;
        self.bend_target = 0.0;
        self.vib_depth = 0.0;
        self.vib_target = 0.0;
        // Fretted notes ring a little shorter than open strings.
        let fretted = 1.0 - 0.012 * fret.min(20.0);
        if palm {
            self.t60_target = PALM_T60;
            self.pole_target = PALM_POLE;
        } else {
            self.t60_target = self.base_t60 * fretted;
            self.pole_target = self.base_pole;
        }
        self.t60 = self.t60_target;
        self.pole = self.pole_target;
        // Pick and fretting hand stop most of what was ringing.
        for x in &mut self.buf {
            *x *= 0.2;
        }
        let f = self.open_hz * 2f64.powf(fret / 12.0);
        let n = ((sr / f).round() as usize).clamp(8, BUF / 2);
        let v = velocity.clamp(0.0, 1.0);
        // A soft pick is dark, a hard one bright.
        let bright = 0.18 + 0.6 * v * v;
        let mut y = 0.0;
        let mut mean = 0.0;
        for k in 0..n {
            y += bright * (rng.bipolar() - y);
            self.exc[k] = y as f32;
            mean += y;
        }
        mean /= n as f64;
        // Comb at the picking point, a fixed distance from the bridge (so a
        // fixed time on this string), then level by velocity, normalized by length.
        let d = ((pick_at * sr / self.open_hz).round() as usize).clamp(1, n - 1);
        let mut rms = 0.0;
        for k in (0..n).rev() {
            let a = self.exc[k] as f64 - mean;
            let b = if k >= d { self.exc[k - d] as f64 - mean } else { 0.0 };
            let e = a - b;
            self.exc[k] = e as f32;
            rms += e * e;
        }
        rms = (rms / n as f64).sqrt().max(1e-9);
        let level = (0.12 + 0.88 * v) * 0.5 / rms * (n as f64 / 300.0).powf(-0.25);
        if palm {
            // A palm on the strings also takes the edge off the attack.
            let mut z = 0.0;
            for k in 0..n {
                z += 0.35 * (self.exc[k] as f64 - z);
                self.exc[k] = (z * level * 1.4) as f32;
            }
        } else {
            for k in 0..n {
                self.exc[k] = (self.exc[k] as f64 * level) as f32;
            }
        }
        self.exc_len = n;
        self.exc_pos = 0;
        self.plucked = true;
    }

    fn mute(&mut self) {
        self.t60_target = MUTE_T60;
        self.vib_target = 0.0;
        self.plucked = false;
    }

    /// One sample. Returns what each pickup coil position hears.
    #[inline]
    fn tick(&mut self, sr: f64, positions: &[f64], out: &mut [f64; 2]) {
        self.bend += (self.bend_target - self.bend) * 0.0012;
        self.vib_depth += (self.vib_target - self.vib_depth) * 0.0002;
        self.vib_phase = (self.vib_phase + 5.4 / sr).fract();
        self.pole += (self.pole_target - self.pole) * 0.002;
        self.t60 += (self.t60_target - self.t60) * 0.004;
        let semis = self.fret + self.bend + self.vib_depth * (TAU * self.vib_phase).sin();
        self.semis = semis;
        let f = self.open_hz * 2f64.powf(semis / 12.0);
        let period = sr / f;
        self.period = period;
        self.open_period = sr / self.open_hz;
        let a = self.pole;
        let omega = TAU * f / sr;
        let lp_delay = (a * omega.sin()).atan2(1.0 - a * omega.cos()) / omega;
        let delay = (period - lp_delay - allpass_delay(self.disp, omega) * DISPERSION_SECTIONS as f64)
            .clamp(2.0, (BUF - 8) as f64);
        let read = self.read(delay);
        self.lp = (1.0 - a) * read + a * self.lp;
        let mut x = self.lp;
        let c = self.disp;
        for k in 0..DISPERSION_SECTIONS {
            let yk = c * x + self.ap_x[k] - c * self.ap_y[k];
            self.ap_x[k] = x;
            self.ap_y[k] = yk;
            x = yk;
        }
        let g = 10f64.powf(-3.0 / (f * self.t60));
        let mut y = g * x;
        if self.exc_pos < self.exc_len {
            y += self.exc[self.exc_pos] as f64;
            self.exc_pos += 1;
        }
        self.buf[self.w] = y as f32;
        self.w = (self.w + 1) & MASK;
        // The pickup sits a fixed distance from the bridge: a fixed time on this string.
        for (o, p) in out.iter_mut().zip(positions) {
            let tap = (p * self.open_period).max(2.0);
            *o = y - self.read(tap + 1.0);
        }
        let level = out[0].abs();
        let k = if level > self.env { 0.01 } else { 0.0005 };
        self.env += k * (level - self.env);
    }
}

/// Phase delay (samples) at `omega` of the allpass `(c + z⁻¹) / (1 + c z⁻¹)`.
fn allpass_delay(c: f64, omega: f64) -> f64 {
    let num = (-omega.sin()).atan2(c + omega.cos());
    let den = (-c * omega.sin()).atan2(1.0 + c * omega.cos());
    -(num - den) / omega
}

/// RBJ biquad lowpass, transposed direct form II.
#[derive(Debug, Clone, Copy, Default)]
pub struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    z1: f64,
    z2: f64,
}

impl Biquad {
    pub fn set_lowpass(&mut self, sr: f64, f0: f64, q: f64) {
        let w0 = TAU * (f0 / sr).min(0.45);
        let alpha = w0.sin() / (2.0 * q);
        let c = w0.cos();
        let a0 = 1.0 + alpha;
        self.b0 = (1.0 - c) / 2.0 / a0;
        self.b1 = (1.0 - c) / a0;
        self.b2 = self.b0;
        self.a1 = -2.0 * c / a0;
        self.a2 = (1.0 - alpha) / a0;
    }
    #[inline]
    pub fn tick(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

pub struct Guitar {
    sr: f64,
    strings: Vec<GString>,
    settings: GuitarSettings,
    resonance: Biquad,
    dc_x: f64,
    dc_y: f64,
    hum_phase: f64,
    rng: Rng,
    volume: f64,
}

impl Guitar {
    #[must_use]
    pub fn new(sr: f64) -> Self {
        let mut g = Self {
            sr,
            strings: (0..STRINGS).map(GString::new).collect(),
            settings: GuitarSettings::default(),
            resonance: Biquad::default(),
            dc_x: 0.0,
            dc_y: 0.0,
            hum_phase: 0.0,
            rng: Rng::new(0x9e37_79b9),
            volume: 0.0,
        };
        g.set(GuitarSettings::default());
        g.volume = g.settings.volume;
        g
    }

    #[must_use]
    pub fn settings(&self) -> GuitarSettings {
        self.settings
    }

    pub fn set(&mut self, s: GuitarSettings) {
        self.settings = s;
        let (f, q) = s.pickup.resonance();
        let t = s.tone.clamp(0.0, 1.0);
        let fc = 900.0 + (f - 900.0) * t.powf(0.8);
        let qc = 0.9 + (q - 0.9) * t;
        self.resonance.set_lowpass(self.sr, fc, qc);
    }

    /// Pluck `string` (0 = low E) at `fret`, velocity 0..1, optionally palm-muted.
    pub fn pluck(&mut self, string: usize, fret: f64, velocity: f64, palm: bool) {
        let Some(s) = self.strings.get_mut(string) else { return };
        // Players pick between about a seventh and a fifth of the way up from the bridge.
        let pick_at = 0.13 + 0.06 * self.rng.unit();
        s.pluck(self.sr, fret, velocity, palm, pick_at, &mut self.rng);
    }

    pub fn mute(&mut self, string: usize) {
        if let Some(s) = self.strings.get_mut(string) {
            s.mute();
        }
    }

    pub fn mute_all(&mut self) {
        for s in &mut self.strings {
            s.mute();
        }
    }

    /// Bend `string` toward `semitones` above its fretted note.
    pub fn bend(&mut self, string: usize, semitones: f64) {
        if let Some(s) = self.strings.get_mut(string) {
            s.bend_target = semitones;
        }
    }

    /// Fade vibrato on `string` to `depth` semitones (peak).
    pub fn vibrato(&mut self, string: usize, depth: f64) {
        if let Some(s) = self.strings.get_mut(string) {
            s.vib_target = depth;
        }
    }

    /// What each string is doing: the decomposition's ground truth.
    pub fn truth(&self, out: &mut [StringTruth; STRINGS]) {
        for (o, s) in out.iter_mut().zip(&self.strings) {
            let level = s.env as f32;
            *o = StringTruth {
                fret: s.fret.round().clamp(0.0, 24.0) as u8,
                semitones: s.semis as f32,
                hz: if s.period > 0.0 { (self.sr / s.period) as f32 } else { s.open_hz as f32 },
                level,
                // About 40 dB below a firm pluck's pickup level.
                sounding: s.plucked && level > 2e-3,
            };
        }
    }

    /// The sounding frequency of each string (Hz).
    pub fn frequencies(&self, out: &mut [f64; STRINGS]) {
        for (o, s) in out.iter_mut().zip(&self.strings) {
            *o = if s.period > 0.0 { self.sr / s.period } else { s.open_hz };
        }
    }

    /// One sample of DI signal.
    #[inline]
    pub fn tick(&mut self) -> f32 {
        let positions = self.settings.pickup.positions();
        let mut sum = 0.0;
        let mut taps = [0.0; 2];
        for s in &mut self.strings {
            s.tick(self.sr, positions, &mut taps);
            sum += taps[..positions.len()].iter().sum::<f64>() / positions.len() as f64;
        }
        let mut y = self.resonance.tick(sum * 0.12);
        // Coupling cap: no DC reaches the jack.
        let dc = y - self.dc_x + 0.9995 * self.dc_y;
        self.dc_x = y;
        self.dc_y = dc;
        y = dc;
        self.volume += (self.settings.volume - self.volume) * 0.002;
        y *= self.volume * self.volume;
        if self.settings.hum {
            self.hum_phase = (self.hum_phase + 60.0 / self.sr).fract();
            let ph = TAU * self.hum_phase;
            let h = self.settings.pickup.hum();
            y += h * (ph.sin() + 0.45 * (3.0 * ph).sin() + 0.2 * (5.0 * ph).sin());
            y += 0.000_03 * self.rng.bipolar();
        }
        y as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Zero crossings per second of a long open-string note, against its pitch.
    fn measured_hz(string: usize, fret: f64) -> f64 {
        let sr = 48_000.0;
        let mut g = Guitar::new(sr);
        g.set(GuitarSettings { pickup: Pickup::Neck, tone: 0.0, volume: 1.0, hum: false });
        g.pluck(string, fret, 0.5, false);
        let x: Vec<f64> = (0..(sr as usize * 2)).map(|_| g.tick() as f64).collect();
        // Autocorrelation peak over the second half, near the expected period.
        let tail = &x[x.len() / 2..];
        let expect = sr / (OPEN_HZ[string] * 2f64.powf(fret / 12.0));
        let (lo, hi) = ((expect * 0.8) as usize, (expect * 1.25) as usize);
        let ac = |lag: usize| tail.iter().zip(&tail[lag..]).map(|(a, b)| a * b).sum::<f64>();
        let best = (lo..hi).max_by(|a, b| ac(*a).total_cmp(&ac(*b))).unwrap();
        // Parabolic refinement.
        let (y0, y1, y2) = (ac(best - 1), ac(best), ac(best + 1));
        let lag = best as f64 + 0.5 * (y0 - y2) / (y0 - 2.0 * y1 + y2);
        sr / lag
    }

    #[test]
    fn strings_play_in_tune() {
        for (s, f) in [(0, 0.0), (1, 0.0), (3, 0.0), (5, 0.0), (5, 12.0), (0, 7.0)] {
            let want = OPEN_HZ[s] * 2f64.powf(f / 12.0);
            let got = measured_hz(s, f);
            let cents = 1200.0 * (got / want).log2();
            assert!(cents.abs() < 6.0, "string {s} fret {f}: {got:.2} Hz vs {want:.2} Hz ({cents:.1} cents)");
        }
    }

    #[test]
    fn notes_decay_and_stay_finite() {
        let mut g = Guitar::new(48_000.0);
        for s in 0..STRINGS {
            g.pluck(s, 0.0, 1.0, false);
        }
        let peak = |g: &mut Guitar, n| (0..n).map(|_| g.tick().abs()).fold(0.0f32, f32::max);
        let early = peak(&mut g, 4_800);
        let _ = peak(&mut g, 48_000 * 3);
        let late = peak(&mut g, 4_800);
        assert!(early > 0.02 && early < 1.0, "strum peak {early}");
        assert!(late < early * 0.5, "decays: {early} -> {late}");
    }
}
