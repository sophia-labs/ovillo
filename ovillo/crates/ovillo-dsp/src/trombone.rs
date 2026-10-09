//! A trombone made of an Ariadne World.
//!
//! The canonical World's configuration path runs from six guitar courses to
//! one brass bore. Here both ends are the *same* bore at two lengths, so every
//! mode keeps its harmonic place as the path moves (frequencies interpolate in
//! log), and Material Organization becomes a slide: amount 0 is a bore on
//! C2 (65.4 Hz, a semitone past first position), amount 1 is eleven
//! semitones longer (C♯1). A lip buzz is the source work: additive, phase
//! continuous, brighter as it gets louder, with breath noise and an optional
//! growl. The player picks the partial; the slide puts a bore resonance under
//! the buzz, and Ariadne's body, active material and shared space do the rest.
//!
//! Above the bore's 24th mode the World has nothing to ring, so the bell's
//! direct sound (the buzz through a highpass) comes back separately as
//! [`TromboneFrame::bell`] for the caller to mix in.

use crate::ariadne::{AriadneControls, AriadneEngine};
use mus_ariadne::{ApertureSpec, WorldPlan, MODE_COUNT};

/// The bore fundamental at slide amount 0 (C2).
pub const SLIDE_TOP_HZ: f64 = 65.406_391_325;
/// How far the slide reaches, in semitones below [`SLIDE_TOP_HZ`].
pub const SLIDE_SEMITONES: f64 = 11.0;
/// Ring time of every bore mode.
const BORE_T60_S: f64 = 0.4;
const MAX_PARTIALS: usize = 64;
const SLIDE_SMOOTHING_S: f64 = 0.006;

/// The bore fundamental at the far end of the slide.
#[must_use]
pub fn slide_bottom_hz() -> f64 {
    SLIDE_TOP_HZ * 2f64.powf(-SLIDE_SEMITONES / 12.0)
}

/// What the player does this sample.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Blow {
    /// The lips' buzz frequency.
    pub hz: f64,
    /// The bore fundamental the slide should sit on (normally `hz / partial`).
    pub slide_hz: f64,
    /// Breath, 0..1.
    pub level: f64,
    /// 0 dark .. 1 brassy.
    pub brightness: f64,
    /// Breath noise, 0..1 (tongue bursts, airy tone).
    pub noise: f64,
    /// Growl depth, 0..1.
    pub growl: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TromboneFrame {
    pub left: f32,
    pub right: f32,
    /// The bell's direct high-frequency sound (mono).
    pub bell: f32,
    /// The lip source as fed to the World.
    pub source: f32,
    pub slide_amount: f32,
}

/// The partial a player would use for `hz`, keeping the slide inside
/// `[bottom·2^(low_margin/12), top·2^(-high_margin/12)]`, preferring middle
/// positions and, if given, staying on `prefer` (for smears).
#[must_use]
pub fn choose_partial(hz: f64, low_margin_semis: f64, high_margin_semis: f64, prefer: Option<u8>) -> u8 {
    let lo = slide_bottom_hz() * 2f64.powf(low_margin_semis / 12.0);
    let hi = SLIDE_TOP_HZ * 2f64.powf(-high_margin_semis / 12.0);
    let mut best = (f64::MAX, 1u8);
    for k in 1..=MODE_COUNT as u8 {
        let f = hz / k as f64;
        let cost = if (lo..=hi).contains(&f) {
            // Distance from third position (A♭1) in semitones.
            let semis_below_top = 12.0 * (SLIDE_TOP_HZ / f).log2();
            let mut c = (semis_below_top - 4.0).abs();
            if prefer.is_some_and(|p| p != k) {
                c += 8.0;
            }
            c
        } else {
            // Out of reach: only as a last resort, by how far out.
            1000.0 + 12.0 * (f / hi).log2().abs().min((lo / f).log2().abs())
        };
        if cost < best.0 {
            best = (cost, k);
        }
    }
    best.1
}

pub struct Trombone {
    sr: f64,
    engine: AriadneEngine,
    /// Ariadne's controls; `material_organization` is overwritten by the slide.
    pub controls: AriadneControls,
    phase: f64,
    slide: f64,
    slide_coef: f64,
    growl_phase: f64,
    noise_state: u64,
    breath_bp: Biquad,
    bell_hp: Biquad,
    bell_lp: Biquad,
    growl_lp: f64,
}

impl Trombone {
    pub fn new(sr: f64) -> Result<Self, String> {
        Self::with_aperture(sr, |_| {})
    }

    /// As [`Trombone::new`], with the mouthpiece aperture edited after
    /// [`mouthpiece`] has set it.
    pub fn with_aperture(sr: f64, edit: impl FnOnce(&mut ApertureSpec)) -> Result<Self, String> {
        let mut plan = WorldPlan::string_brass_v1(sr);
        {
            let path = &mut plan.bodies[0].body.configuration_path;
            let mut brass = path.right;
            mouthpiece(&mut brass.aperture);
            edit(&mut brass.aperture);
            let bore = |fundamental: f64| {
                let mut c = brass;
                let mid = SLIDE_TOP_HZ * 2f64.powf(-SLIDE_SEMITONES / 24.0);
                c.metric.frequency_hz = std::array::from_fn(|i| fundamental * (i + 1) as f64);
                // Equal ring time = equal bandwidth: every resonance peak about
                // as tall, like a real bore's impedance peaks below the bell's
                // cutoff (a constant Q would favour the low partials ~1/n).
                c.metric.t60_seconds = std::array::from_fn(|i| {
                    let _ = (mid, i);
                    BORE_T60_S
                });
                c
            };
            path.left = bore(SLIDE_TOP_HZ);
            path.right = bore(slide_bottom_hz());
            // Notes sit on partials 2–12 of the bore, so their harmonics land
            // on modes up to 24: excite and radiate those nearly evenly (the
            // canonical brass end, played on its low modes, rolls them off).
            let radiation = unit(std::array::from_fn(|i| {
                (1.0 + 0.97 * i as f64 / (MODE_COUNT - 1) as f64) / (1.0 + 0.002 * (i as f64).powi(2))
            }));
            path.left.radiation_direction = radiation;
            path.right.radiation_direction = radiation;
            path.left_name = "slide_in".to_owned();
            path.right_name = "slide_out".to_owned();
            path.initial_amount = 0.36;
            plan.bodies[0].body.source_direction = unit(std::array::from_fn(|i| {
                let parity = if i % 2 == 0 { 1.0 } else { 0.85 };
                parity / (1.0 + 0.004 * (i as f64).powi(2))
            }));
        }
        let engine = AriadneEngine::from_plan(plan)?;
        Ok(Self {
            sr,
            engine,
            controls: Self::default_controls(),
            phase: 0.0,
            slide: 0.36,
            slide_coef: 1.0 - (-1.0 / (SLIDE_SMOOTHING_S * sr)).exp(),
            growl_phase: 0.0,
            noise_state: 0x9E37_79B9_7F4A_7C15,
            breath_bp: Biquad::bandpass(sr, 1_700.0, 0.8),
            bell_hp: Biquad::highpass(sr, 1_100.0, 0.707),
            bell_lp: Biquad::lowpass(sr, 4_200.0, 0.707),
            growl_lp: 0.0,
        })
    }

    /// A dark, close, slightly smoky trombone.
    #[must_use]
    pub fn default_controls() -> AriadneControls {
        AriadneControls {
            material_organization: 0.36,
            source_coupling: 1.0,
            work_input: 0.004,
            // Gentle: the active material swells notes but must not hold
            // the bore by itself (Sustain > 0 lets it drone through rests).
            active_coupling: 0.25,
            active_drive: 0.25,
            headroom: 0.72,
            sustain: 0.0,
            sag: 0.30,
            bloom: 0.40,
            feedback: 0.04,
            active_space: 0.16,
            interlace: 0.42,
            memory: 0.28,
            space_coupling: 0.30,
            space_decay: 0.55,
            space_radiation: 0.30,
            body_radiation: 0.008,
            listener_scale: 40.0,
            mix: 1.0,
            // The World is loud at this listener scale; keep clear of the
            // output safety limit and let the caller set the level.
            output_gain: 0.05,
        }
    }

    pub fn reset(&mut self) {
        self.engine.reset();
        self.phase = 0.0;
    }

    #[must_use]
    pub fn world_errors(&self) -> u64 {
        self.engine.world_errors
    }

    #[must_use]
    pub fn safety_limited_frames(&self) -> u64 {
        self.engine.safety_limited_frames
    }

    fn noise(&mut self) -> f64 {
        // xorshift64*, uniform in [-1, 1).
        let mut x = self.noise_state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.noise_state = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 52) as f64 - 1.0
    }

    pub fn tick(&mut self, blow: Blow) -> TromboneFrame {
        let hz = blow.hz.clamp(20.0, 2_000.0);
        let level = blow.level.clamp(0.0, 1.0);

        // The slide.
        let slide_hz = if blow.slide_hz > 0.0 { blow.slide_hz } else { hz };
        let target = (12.0 * (SLIDE_TOP_HZ / slide_hz).log2() / SLIDE_SEMITONES).clamp(0.0, 1.0);
        self.slide += (target - self.slide) * self.slide_coef;

        // The lips: harmonics n^-tilt up to ~9 kHz, normalised to unit RMS.
        self.phase = (self.phase + hz / self.sr).fract();
        let tilt = 1.0 - 0.9 * blow.brightness.clamp(0.0, 1.0);
        let n_max = ((9_000.0 / hz) as usize).clamp(1, MAX_PARTIALS);
        let mut buzz = 0.0;
        let mut norm = 0.0;
        let w = std::f64::consts::TAU * self.phase;
        for n in 1..=n_max {
            let a = (n as f64).powf(-tilt);
            buzz += a * (w * n as f64).sin();
            norm += a * a;
        }
        buzz /= (0.5 * norm).sqrt();

        // Growl: a rough ~27 Hz flutter of the lips.
        let raw = self.noise();
        self.growl_lp += (raw - self.growl_lp) * 0.02;
        self.growl_phase = (self.growl_phase + (27.0 + 6.0 * self.growl_lp) / self.sr).fract();
        let flutter = 1.0 + blow.growl * (0.55 * (std::f64::consts::TAU * self.growl_phase).sin() + 0.8 * self.growl_lp);
        let hiss = self.noise();
        let breath = self.breath_bp.tick(hiss);

        let lips = level * (buzz * flutter + blow.noise * 2.5 * breath);
        let source = 0.16 * lips;

        let mut controls = self.controls;
        controls.material_organization = self.slide;
        let out = self.engine.process_sample(source as f32, source as f32, controls);
        let bell = self.bell_lp.tick(self.bell_hp.tick(lips));
        TromboneFrame {
            left: out.left,
            right: out.right,
            bell: bell as f32,
            source: source as f32,
            slide_amount: self.slide as f32,
        }
    }
}

fn unit(v: [f64; MODE_COUNT]) -> [f64; MODE_COUNT] {
    let n = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    v.map(|x| x / n)
}

/// The mouthpiece. The canonical brass aperture is a lip reed tuned to
/// 110 Hz that rings at its own pitch whatever note is asked of it (a player
/// retunes their lips per note; this one cannot). Here the lips are the buzz
/// itself, so the aperture is made a stiff, damped, passive coupler.
pub fn mouthpiece(a: &mut ApertureSpec) {
    a.natural_frequency_hz = 1_500.0;
    a.damping_per_second = 200.0;
    a.active_rate_per_second = 0.0;
    a.asymmetry = 0.0;
    a.source_to_field_rate_hz = 6_000.0;
}

/// RBJ biquad, direct form I.
#[derive(Debug, Clone, Copy)]
struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    x: [f64; 2],
    y: [f64; 2],
}

impl Biquad {
    fn new(b: [f64; 3], a0: f64, a: [f64; 2]) -> Self {
        Self { b: b.map(|v| v / a0), a: a.map(|v| v / a0), x: [0.0; 2], y: [0.0; 2] }
    }

    fn bandpass(sr: f64, f: f64, q: f64) -> Self {
        let w = std::f64::consts::TAU * f / sr;
        let alpha = w.sin() / (2.0 * q);
        Self::new([alpha, 0.0, -alpha], 1.0 + alpha, [-2.0 * w.cos(), 1.0 - alpha])
    }

    fn highpass(sr: f64, f: f64, q: f64) -> Self {
        let w = std::f64::consts::TAU * f / sr;
        let (c, alpha) = (w.cos(), w.sin() / (2.0 * q));
        Self::new([(1.0 + c) / 2.0, -(1.0 + c), (1.0 + c) / 2.0], 1.0 + alpha, [-2.0 * c, 1.0 - alpha])
    }

    fn lowpass(sr: f64, f: f64, q: f64) -> Self {
        let w = std::f64::consts::TAU * f / sr;
        let (c, alpha) = (w.cos(), w.sin() / (2.0 * q));
        Self::new([(1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0], 1.0 + alpha, [-2.0 * c, 1.0 - alpha])
    }

    fn tick(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.b[1] * self.x[0] + self.b[2] * self.x[1] - self.a[0] * self.y[0] - self.a[1] * self.y[1];
        self.x = [x, self.x[0]];
        self.y = [y, self.y[0]];
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partials_keep_the_slide_in_reach() {
        for midi in 40..=74 {
            let hz = 440.0 * 2f64.powf((midi as f64 - 69.0) / 12.0);
            let k = choose_partial(hz, 1.0, 0.3, None);
            let f = hz / k as f64;
            assert!(f >= slide_bottom_hz() && f <= SLIDE_TOP_HZ, "midi {midi}: k {k} slide {f:.2} Hz");
        }
    }
}
