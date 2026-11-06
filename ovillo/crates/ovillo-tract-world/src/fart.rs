//! The other end: named control paths for `TractNdAnatomy::fart_v1`, a food
//! table, and a [`Gut`] that keeps one World between calls (eat, digest,
//! release). `examples/fart.rs` and `ovillo-fart-mcp` play these;
//! `site/fart/fart.js` mirrors them.
//!
//! Digesting is the World's reservoir refill (`work_input`, booked as source
//! work); releasing spends the reservoir through the valve. The foods'
//! numbers and every path here are invented (no measurement, no source).
//! What the listener hears is the World's radiated pressure times
//! [`LISTENER_GAIN`] through a 20 Hz one-pole DC blocker (steady flow is
//! steady pressure at the mouth); the ledger is the World's, untouched.

use mus_ariadne::{ConnectionPolicy, DisposalPolicy, WorldError};

use crate::world::{Articulation, Constriction, TractControls, TractWorldNd, Vibrato};
use crate::TractNdAnatomy;

/// Listener gain: 2.5 times the tract's (1/2500), as the supply ceiling is.
pub const LISTENER_GAIN: f64 = 1.0 / 1_000.0;
/// Where the cheeks clench (constriction slot 1): inside the cleft, so the
/// radiating volume (the listener) is outside the seal, not reading the
/// pressure trapped behind it.
pub const SEAL_AT: f64 = 0.85;
const SEAL_SLOT: usize = 1;
/// Controls are set once per hop.
pub const HOP_S: f64 = 0.01;
const DC_BLOCK_HZ: f64 = 20.0;

/// Cheeks at `lips`, the tongue inert, the stub sealed.
#[must_use]
pub fn cheeks(lips: f64) -> Articulation {
    Articulation { tongue_index: 0.5, tongue_diameter: 0.5, lips, velum: 0.0 }
}

/// A named control path, set every [`HOP_S`].
#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub name: &'static str,
    pub seconds: f64,
    pub vibrato: Vibrato,
    /// Fast random tension (semitones, a new target every hop), booked as
    /// valve retunes like the vibrato.
    pub jitter_semitones: f64,
    /// Cheeks clenched at `t` (slot 1 at [`SEAL_AT`], closing in 8 ms).
    pub sealed: fn(f64) -> bool,
    pub at: fn(f64) -> TractControls,
    pub about: &'static str,
}

const STILL: Vibrato = Vibrato { rate_hz: 5.5, depth_semitones: 0.0, wobble: 0.0 };

fn never(_: f64) -> bool {
    false
}

pub const STYLES: [Style; 6] = [
    Style {
        name: "brap",
        seconds: 2.5,
        vibrato: Vibrato { wobble: 0.6, ..STILL },
        jitter_semitones: 0.0,
        sealed: never,
        at: |_| TractControls { articulation: cheeks(0.45), pitch_hz: 65.0, tenseness: 0.75, breath: 1.0 },
        about: "full push at 65 Hz, cheeks half open, until the reservoir sags",
    },
    Style {
        name: "flutter",
        seconds: 2.5,
        vibrato: Vibrato { wobble: 0.6, ..STILL },
        jitter_semitones: 4.0,
        sealed: never,
        at: |_| TractControls { articulation: cheeks(0.3), pitch_hz: 60.0, tenseness: 0.7, breath: 1.0 },
        about: "brap with the tension jittered +-4 semitones every 10 ms",
    },
    Style {
        name: "squeaker",
        seconds: 1.6,
        vibrato: Vibrato { wobble: 0.3, ..STILL },
        jitter_semitones: 0.5,
        sealed: never,
        at: |t| TractControls { articulation: cheeks(0.1), pitch_hz: 210.0 + 30.0 * t, tenseness: 0.85, breath: if t < 1.2 { 1.0 } else { 0.0 } },
        about: "tight cheeks, high tension, rising from 210 Hz",
    },
    Style {
        name: "trombone",
        seconds: 2.6,
        vibrato: Vibrato { rate_hz: 4.0, depth_semitones: 0.4, wobble: 0.4 },
        jitter_semitones: 0.0,
        sealed: never,
        at: |t| TractControls {
            articulation: cheeks(0.35),
            pitch_hz: 150.0 * (35.0f64 / 150.0).powf((t / 2.2).min(1.0)),
            tenseness: 0.7,
            breath: 0.85,
        },
        about: "a glide from 150 Hz down to 35 Hz with vibrato",
    },
    Style {
        name: "pbbt",
        seconds: 1.2,
        vibrato: STILL,
        jitter_semitones: 1.0,
        sealed: |t| t >= 0.9 || (t * 7.0).fract() >= 0.5,
        at: |t| TractControls { articulation: cheeks(0.4), pitch_hz: 90.0, tenseness: 0.8, breath: if t < 0.9 { 0.7 } else { 0.0 } },
        about: "the cheeks clench and release 7 times a second: exact seals and release bursts",
    },
    Style {
        name: "sbd",
        seconds: 2.0,
        vibrato: STILL,
        jitter_semitones: 0.0,
        sealed: never,
        at: |t| TractControls { articulation: cheeks(0.04), pitch_hz: 60.0, tenseness: 0.0, breath: if t < 1.6 { 0.8 } else { 0.0 } },
        about: "sphincter relaxed open, cheeks nearly shut: flow without a swing, turbulent hiss only",
    },
];

#[must_use]
pub fn style(name: &str) -> Option<Style> {
    STYLES.iter().find(|s| s.name.eq_ignore_ascii_case(name)).copied()
}

/// Gas a food pays into the reservoir (J; it holds `fart_v1`'s 1.5 J) over
/// its digestion time. Invented: a game's table, not nutrition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Food {
    pub name: &'static str,
    pub gas_j: f64,
    pub seconds: f64,
}

pub const FOODS: [Food; 8] = [
    Food { name: "beans", gas_j: 0.60, seconds: 8.0 },
    Food { name: "broccoli", gas_j: 0.35, seconds: 5.0 },
    Food { name: "cabbage", gas_j: 0.40, seconds: 6.0 },
    Food { name: "onion", gas_j: 0.30, seconds: 4.0 },
    Food { name: "egg", gas_j: 0.20, seconds: 3.0 },
    Food { name: "soda", gas_j: 0.25, seconds: 1.0 },
    Food { name: "cheese", gas_j: 0.15, seconds: 5.0 },
    Food { name: "salad", gas_j: 0.03, seconds: 2.0 },
];

#[must_use]
pub fn food(name: &str) -> Option<Food> {
    let name = name.trim().to_ascii_lowercase();
    let stem = name.trim_end_matches('s');
    FOODS.iter().find(|f| f.name.trim_end_matches('s') == stem).copied()
}

/// What a digest did.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Digestion {
    pub seconds: f64,
    /// The World's source work over the digest: what actually entered.
    pub paid_j: f64,
    pub stopped_full: bool,
}

/// One World kept between calls, with a stomach.
pub struct Gut {
    world: TractWorldNd,
    stomach: Vec<(Food, f64)>,
    rng: u64,
    /// The house feed (W): what digestion pays while the stomach is empty
    /// (0 by default). An ensemble's long parts are fed this way; it is the
    /// World's `work_input` like any digestion, booked as source work.
    pub house_w: f64,
}

impl Gut {
    pub fn new(sample_rate_hz: f64) -> Result<Self, WorldError> {
        Self::with_anatomy(sample_rate_hz, TractNdAnatomy::fart_v1())
    }

    /// A gut with another anatomy (a variant to compare; `new` is `fart_v1`).
    pub fn with_anatomy(sample_rate_hz: f64, anatomy: TractNdAnatomy) -> Result<Self, WorldError> {
        let mut world = TractWorldNd::with(
            sample_rate_hz,
            anatomy,
            ConnectionPolicy::MassMomentum { disposal: DisposalPolicy::Dissipate },
            cheeks(0.45),
        )?;
        world.output_gain = LISTENER_GAIN;
        let a = *world.anatomy();
        world.set_controls(TractControls { articulation: cheeks(0.45), pitch_hz: a.valve.initial_tuning_hz, tenseness: a.valve.initial_adduction, breath: 0.0 });
        Ok(Self { world, stomach: Vec::new(), rng: 0x2545_F491_4F6C_DD1D, house_w: 0.0 })
    }

    #[must_use]
    pub fn world(&self) -> &TractWorldNd {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut TractWorldNd {
        &mut self.world
    }

    #[must_use]
    pub fn stomach(&self) -> &[(Food, f64)] {
        &self.stomach
    }

    pub fn eat(&mut self, food: Food) {
        self.stomach.push((food, food.gas_j));
    }

    /// The reservoir's fill, 0..1.
    #[must_use]
    pub fn gas_fraction(&self) -> f64 {
        let held = self.world.world().space_body(0).map_or(0.0, |b| b.reservoir_energy());
        held / self.world.anatomy().lung_capacity_j
    }

    /// Digest, silently, until the stomach is empty, the reservoir is full,
    /// or `max_seconds` pass. Each food pays `gas_j / seconds` watts, the sum
    /// capped at the World's 1 W; the World books what actually enters.
    pub fn digest(&mut self, max_seconds: f64) -> Result<Digestion, WorldError> {
        let rate = self.world.sample_rate_hz();
        let hop = (HOP_S * rate) as usize;
        let mut scratch = vec![0.0f32; hop];
        let before = self.world.world().receipt().source_work;
        let mut out = Digestion::default();
        let mut c = self.world.controls();
        c.breath = 0.0;
        self.world.set_controls(c);
        while out.seconds < max_seconds && !self.stomach.is_empty() {
            if self.gas_fraction() >= 0.995 {
                out.stopped_full = true;
                break;
            }
            self.pay_hop();
            self.world.process(&mut scratch)?;
            out.seconds += HOP_S;
        }
        self.world.set_refill(0.0);
        out.paid_j = self.world.world().receipt().source_work - before;
        Ok(out)
    }

    /// Set the refill for the next hop from the stomach (0 when it is empty
    /// or the reservoir is full) and take that hop's share out of the food.
    pub fn pay_hop(&mut self) {
        let want: f64 = self.stomach.iter().map(|(f, _)| f.gas_j / f.seconds).sum();
        if self.gas_fraction() >= 0.995 {
            self.world.set_refill(0.0);
            return;
        }
        if want <= 0.0 {
            self.world.set_refill(self.house_w);
            return;
        }
        let watts = want.min(1.0);
        let share = watts / want;
        self.world.set_refill(watts);
        for (f, left) in &mut self.stomach {
            *left -= f.gas_j / f.seconds * share * HOP_S;
        }
        self.stomach.retain(|(_, left)| *left > 0.0);
    }

    /// Play `notes` in the tune voice ([`TUNE_PUSH`], [`TUNE_CHEEKS`],
    /// [`TUNE_TONE`]): each note starts at the tension `curve` says lands on
    /// it and is corrected by ear (the World's measured F0, a non-loading
    /// observer) while it sounds; every retune is booked as valve work.
    /// Tongued notes end with the cheeks clenched for [`GAP_S`] while the push
    /// holds, so the next one starts on a release. With `digest`, the stomach
    /// keeps paying the reservoir while the tune plays. Returns the
    /// listener's copy and, per note, what the World measured.
    pub fn play_tune(&mut self, notes: &[Note], curve: &TuningCurve, legato: bool, digest: bool) -> Result<(Vec<f32>, Vec<NoteReport>), WorldError> {
        let rate = self.world.sample_rate_hz();
        let hop = (HOP_S * rate) as usize;
        let mut audio = Vec::new();
        let mut block = vec![0.0f32; hop];
        let mut reports = Vec::with_capacity(notes.len());
        self.world.set_vibrato(Vibrato { rate_hz: 5.5, depth_semitones: 0.0, wobble: 0.0 });
        let (lo, hi) = {
            let v = self.world.anatomy().valve;
            (v.min_tuning_hz, v.max_tuning_hz)
        };
        let mut command = curve.command_for(notes.iter().find(|n| n.hz > 0.0).map_or(100.0, |n| n.hz));
        // Notes end on the rounded cumulative time, so parts stay in step.
        let (mut t_end, mut hops_done) = (0.0, 0usize);
        for note in notes {
            t_end += note.seconds;
            let hops = (((t_end / HOP_S).round() as usize).saturating_sub(hops_done)).max(1);
            hops_done += hops;
            let rest = note.hz <= 0.0;
            let gap = if legato || rest { 0 } else { ((GAP_S / HOP_S).round() as usize).min(hops / 3) };
            let start = if rest { command } else { curve.command_for(note.hz) };
            command = start;
            let mut heard = Vec::new();
            let mut eligible = 0usize;
            for h in 0..hops {
                if digest {
                    self.pay_hop();
                } else {
                    self.world.set_refill(0.0);
                }
                let sounding = !rest && h < hops - gap;
                let mut c = self.world.controls();
                c.articulation = cheeks(TUNE_CHEEKS);
                c.tenseness = TUNE_TONE;
                c.breath = if rest { 0.0 } else { TUNE_PUSH };
                c.pitch_hz = command;
                self.world.set_controls(c);
                self.world.set_constriction(SEAL_SLOT, (!rest && !sounding).then_some(Constriction { position: SEAL_AT, degree: 1.0 }));
                self.world.process(&mut block)?;
                audio.extend_from_slice(&block);
                if !sounding {
                    continue;
                }
                let f0 = self.world.observers().measured_f0_hz;
                let t = (h + 1) as f64 * HOP_S;
                // The ear: after the attack, nudge the tension toward the
                // target, within a third of an octave of where it started.
                if f0 > 0.0 && t >= 0.04 {
                    command = (command * (note.hz / f0).powf(0.35)).clamp(start / 1.26, start * 1.26).clamp(lo, hi);
                }
                if t >= 0.06 {
                    eligible += 1;
                    if f0 > 0.0 {
                        heard.push(f0);
                    }
                }
            }
            heard.sort_by(f64::total_cmp);
            let measured = heard.get(heard.len() / 2).copied();
            reports.push(NoteReport {
                target_hz: note.hz,
                measured_hz: measured,
                cents: measured.filter(|_| !rest).map(|m| 1200.0 * (m / note.hz).log2()),
                voiced_fraction: if eligible > 0 { heard.len() as f64 / eligible as f64 } else { 0.0 },
            });
        }
        // Let it ring out.
        let mut c = self.world.controls();
        c.breath = 0.0;
        self.world.set_controls(c);
        self.world.set_constriction(SEAL_SLOT, None);
        self.world.set_refill(0.0);
        for _ in 0..((0.25 / HOP_S) as usize) {
            self.world.process(&mut block)?;
            audio.extend_from_slice(&block);
        }
        dc_block(&mut audio, rate);
        Ok((audio, reports))
    }

    /// Play `style` from whatever the reservoir holds; returns the listener's
    /// copy (DC-blocked). `observe` sees the World after every hop.
    pub fn release_with(&mut self, style: &Style, mut observe: impl FnMut(f64, &TractWorldNd)) -> Result<Vec<f32>, WorldError> {
        let rate = self.world.sample_rate_hz();
        let hop = (HOP_S * rate) as usize;
        let mut audio = vec![0.0f32; (style.seconds * rate) as usize];
        self.world.set_vibrato(style.vibrato);
        for (k, block) in audio.chunks_mut(hop).enumerate() {
            let t = k as f64 * HOP_S;
            let mut c = (style.at)(t);
            if style.jitter_semitones > 0.0 {
                self.rng ^= self.rng << 13;
                self.rng ^= self.rng >> 7;
                self.rng ^= self.rng << 17;
                let u = (self.rng >> 11) as f64 / (1u64 << 53) as f64;
                c.pitch_hz *= ((2.0 * u - 1.0) * style.jitter_semitones / 12.0).exp2();
            }
            self.world.set_controls(c);
            self.world.set_constriction(SEAL_SLOT, (style.sealed)(t).then_some(Constriction { position: SEAL_AT, degree: 1.0 }));
            self.world.process(block)?;
            observe(t, &self.world);
        }
        let mut c = self.world.controls();
        c.breath = 0.0;
        self.world.set_controls(c);
        self.world.set_constriction(SEAL_SLOT, None);
        dc_block(&mut audio, rate);
        Ok(audio)
    }

    pub fn release(&mut self, style: &Style) -> Result<Vec<f32>, WorldError> {
        self.release_with(style, |_, _| {})
    }
}

/// The tune voice: push, cheeks and tone held while a tune plays (invented;
/// the push is low enough that tongued releases do not clip). Its range is
/// whatever [`TuningCurve::measure`] finds (`examples/fart_tuning.rs` prints
/// it).
pub const TUNE_PUSH: f64 = 0.7;
pub const TUNE_CHEEKS: f64 = 0.4;
pub const TUNE_TONE: f64 = 0.75;
/// A tongued note's cheek clench before the next note.
pub const GAP_S: f64 = 0.05;

/// One note: `hz` 0 is a rest.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Note {
    pub hz: f64,
    pub seconds: f64,
}

/// What the World measured over one note (from 60 ms in).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NoteReport {
    pub target_hz: f64,
    /// Median measured F0 over the note's voiced hops.
    pub measured_hz: Option<f64>,
    pub cents: Option<f64>,
    /// Fraction of the note's hops in which the valve swung.
    pub voiced_fraction: f64,
}

/// Equal-tempered frequency of a note name (`G2`, `Eb2`, `F#3`, `Bb1`;
/// A4 = 440 Hz), or a bare number of hertz.
#[must_use]
pub fn note_hz(name: &str) -> Option<f64> {
    let name = name.trim();
    if let Ok(hz) = name.parse::<f64>() {
        return (hz.is_finite() && hz > 0.0).then_some(hz);
    }
    let mut chars = name.chars();
    let pitch_class = match chars.next()?.to_ascii_uppercase() {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let rest: String = chars.collect();
    let digits = rest.trim_start_matches(['#', 'b', '\u{266F}', '\u{266D}']);
    let accidental: i32 = rest[..rest.len() - digits.len()].chars().map(|c| if c == '#' || c == '\u{266F}' { 1 } else { -1 }).sum();
    let octave: i32 = digits.parse().ok()?;
    let midi = 12 * (octave + 1) + pitch_class + accidental;
    Some(440.0 * ((f64::from(midi) - 69.0) / 12.0).exp2())
}

/// A tune as text: tokens `NOTE:beats` (`G2:1`, `Eb2:0.75`, `98:0.5`, `G2:1/3`
/// for a triplet), `r:beats` for a rest; a bare note is one beat. Separated by
/// spaces or commas.
pub fn parse_tune(text: &str, bpm: f64) -> Result<Vec<Note>, String> {
    if !(bpm.is_finite() && bpm > 0.0) {
        return Err(format!("bpm must be positive, not {bpm}"));
    }
    let beat = 60.0 / bpm;
    let mut notes = Vec::new();
    for token in text.split(|c: char| c.is_whitespace() || c == ',').filter(|t| !t.is_empty()) {
        let (name, beats) = token.split_once(':').unwrap_or((token, "1"));
        let beats = parse_beats(beats).ok_or_else(|| format!("\"{token}\": beats must be a number or a fraction"))?;
        if !(beats.is_finite() && beats > 0.0) {
            return Err(format!("\"{token}\": beats must be positive"));
        }
        let hz = if name.eq_ignore_ascii_case("r") || name.eq_ignore_ascii_case("rest") {
            0.0
        } else {
            note_hz(name).ok_or_else(|| format!("\"{token}\": not a note (G2, Eb2, F#3, a number of Hz, or r)"))?
        };
        notes.push(Note { hz, seconds: beats * beat });
    }
    if notes.is_empty() {
        return Err("no notes".into());
    }
    Ok(notes)
}

/// `1.5`, `3/2`, `1+1/3` (a tied sum).
fn parse_beats(text: &str) -> Option<f64> {
    text.split('+')
        .map(|term| match term.split_once('/') {
            Some((n, d)) => {
                let (n, d) = (n.parse::<f64>().ok()?, d.parse::<f64>().ok()?);
                (d != 0.0).then_some(n / d)
            }
            None => term.parse().ok(),
        })
        .sum()
}

/// The tune voice's tuning curve: measured F0 against commanded tension,
/// measured on a practice World (its own gas, its own books), so a player
/// can command what lands on a pitch, as a brass player learns the horn.
#[derive(Debug, Clone, PartialEq)]
pub struct TuningCurve {
    /// (commanded Hz, measured Hz), both increasing.
    pub points: Vec<(f64, f64)>,
}

impl TuningCurve {
    /// Sweep the commanded tension upward from 40 to 300 Hz on a practice
    /// World refilled at 1 W; keep the points where the valve swung and the
    /// measured pitch rose.
    pub fn measure(sample_rate_hz: f64) -> Result<Self, WorldError> {
        let mut practice = Gut::new(sample_rate_hz)?;
        practice.world.set_refill(1.0);
        let hop = (HOP_S * sample_rate_hz) as usize;
        let mut block = vec![0.0f32; hop];
        let mut points: Vec<(f64, f64)> = Vec::new();
        for k in 0..20 {
            let commanded = 40.0 * (300.0f64 / 40.0).powf(f64::from(k) / 19.0);
            let mut c = practice.world.controls();
            c.articulation = cheeks(TUNE_CHEEKS);
            c.tenseness = TUNE_TONE;
            c.breath = TUNE_PUSH;
            c.pitch_hz = commanded;
            practice.world.set_controls(c);
            let mut heard = Vec::new();
            for h in 0..45 {
                practice.world.process(&mut block)?;
                let f0 = practice.world.observers().measured_f0_hz;
                if h >= 20 && f0 > 0.0 {
                    heard.push(f0);
                }
            }
            if heard.len() < 15 {
                continue;
            }
            heard.sort_by(f64::total_cmp);
            let measured = heard[heard.len() / 2];
            if points.last().map_or(true, |&(_, m)| measured > m) {
                points.push((commanded, measured));
            }
        }
        Ok(Self { points })
    }

    /// Measured range (Hz).
    #[must_use]
    pub fn range(&self) -> (f64, f64) {
        (self.points.first().map_or(0.0, |p| p.1), self.points.last().map_or(0.0, |p| p.1))
    }

    /// The tension to command for `hz`: log-log interpolation between the
    /// measured points, the nearest point's ratio beyond them.
    #[must_use]
    pub fn command_for(&self, hz: f64) -> f64 {
        let p = &self.points;
        if p.is_empty() {
            return hz;
        }
        if hz <= p[0].1 {
            return p[0].0 * hz / p[0].1;
        }
        for w in p.windows(2) {
            let ((c0, m0), (c1, m1)) = (w[0], w[1]);
            if hz <= m1 {
                let u = (hz / m0).ln() / (m1 / m0).ln();
                return c0 * (c1 / c0).powf(u);
            }
        }
        let (c, m) = p[p.len() - 1];
        c * hz / m
    }
}

/// One step of a performance: the player's hands on the World, nothing
/// between. Settings hold until changed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// Sound for `seconds` with the valve's tension commanded at
    /// `tension_hz`; `target_hz` is only what the note is meant to be (for
    /// the report). With `shot`, a cannon shot instead (see [`SHOT_LOAD_S`]).
    Note { target_hz: f64, tension_hz: f64, seconds: f64, shot: bool },
    Rest { seconds: f64 },
    Push(f64),
    Cheeks(f64),
    Tone(f64),
    Wobble(f64),
    Vibrato { rate_hz: f64, depth_semitones: f64 },
    /// Clench the cheeks for [`GAP_S`] at the end of a note followed by
    /// another (on), or slur (off).
    Tongue(bool),
    Eat(Food),
    /// The stomach pays the reservoir (on) or holds its food (off).
    Digest(bool),
    /// Silent time before the first beat (trimmed from the output): the
    /// player can eat and digest.
    Pre { seconds: f64 },
    /// Random tension jitter, +- this many semitones, redrawn every hop
    /// (the `flutter` style's; 0 is steady).
    Jitter(f64),
    /// A partial clench near the exit (constriction slot [`RASP_SLOT`] at
    /// [`RASP_AT`], this degree, 0..1) held while sounding: the flow there
    /// becomes a jet and the World's turbulence law sputters. 0 is none.
    Rasp(f64),
}

/// The ensemble's anatomies by name (`TractNdAnatomy::fart_v1` and its
/// family): "alto" is `fart_v1` itself.
pub const ANATOMIES: [&str; 4] = ["soprano", "alto", "bass", "cannon"];

#[must_use]
pub fn anatomy(name: &str) -> Option<TractNdAnatomy> {
    match name {
        "soprano" => Some(TractNdAnatomy::fart_v1_soprano()),
        "alto" | "v1" => Some(TractNdAnatomy::fart_v1()),
        "bass" => Some(TractNdAnatomy::fart_v1_bass()),
        "cannon" => Some(TractNdAnatomy::fart_v1_cannon()),
        "v0" => Some(TractNdAnatomy::fart_v0()),
        // an experiment: "x:base=v0,mass=1.3e-3,cs=2.5,cd=1.5,damp=0.15,len=0.2,canal=1,cleft=5,walls=1.5,vlen=0.012,supply=2500,lung=1.5"
        // (every key optional; the base is fart_v0 unless named)
        n if n.starts_with("x:") => {
            let mut a = TractNdAnatomy::fart_v0();
            let mut canal = 1.0;
            let mut cleft = 5.0;
            for kv in n[2..].split(',') {
                let (k, v) = kv.split_once('=')?;
                if k == "base" {
                    a = anatomy(v)?;
                    if let crate::OralProfile::Cleft { canal_cm2, cleft_cm2, .. } = a.oral_profile { canal = canal_cm2; cleft = cleft_cm2; }
                    continue;
                }
                let x: f64 = v.parse().ok()?;
                match k {
                    "mass" => a.valve.mass_kg = x,
                    "cs" => a.valve.contact_stiffness_ratio = x,
                    "cd" => a.valve.contact_damping_ratio = x,
                    "damp" => a.valve.damping_ratio = x,
                    "len" => a.oral_length_m = x,
                    "canal" => canal = x,
                    "cleft" => cleft = x,
                    "walls" => a.oral_attenuation_np_per_m = x,
                    "vlen" => a.valve.length_m = x,
                    "thick" => a.valve.thickness_m = x,
                    "muc" => a.valve.mucosal_delay_s = x,
                    "supply" => a.valve.max_supply_pressure_pa = x,
                    "lung" => a.lung_capacity_j = x,
                    _ => return None,
                }
            }
            a.oral_profile = crate::OralProfile::Cleft { canal_cm2: canal, cleft_cm2: cleft, flare_start: 0.15, flare_end: 0.6 };
            a.name = "cleft-nd-x";
            Some(a)
        }
        _ => None,
    }
}

/// Where the rasp clench sits (constriction slot 2): in the flare, past the
/// cheeks' seal.
pub const RASP_SLOT: usize = 2;
pub const RASP_AT: f64 = 0.93;

/// What the World did at one hop of a performance (after the hop): the
/// observers and totals the ABI's `tract_snapshot` reads, plus what the
/// player had commanded for the hop. Reading them changes no sample.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HopTrace {
    /// The valve's measured F0 (0: not swinging).
    pub f0_hz: f64,
    pub gas_fraction: f64,
    pub supply_pa: f64,
    /// Samples the listener clamped so far.
    pub clipped: u64,
    /// The reservoir's energy (J) and what the stomach still holds (J).
    pub reservoir_j: f64,
    pub stomach_j: f64,
    /// The valve's half-gap over the hop (m), smallest and largest.
    pub gap_min_m: f64,
    pub gap_max_m: f64,
    /// Flow through the valve at the end of the hop (m³/s).
    pub flow_m3_s: f64,
    /// Energy in the canal's air (J).
    pub field_j: f64,
    /// Cumulative: radiated at the exit and at the stub (J), the
    /// turbulence the jet injected (J).
    pub radiated_j: f64,
    pub turbulence_j: f64,
    /// The jet's aperture (m²) and where it sits (0 valve .. 1 exit).
    pub jet_aperture_m2: f64,
    pub jet_position: f64,
    /// Oral section areas (m²) and RMS pressure over the hop (Pa), valve to exit.
    pub areas_m2: Vec<f32>,
    pub pressure_rms_pa: Vec<f32>,
    /// What was commanded for this hop.
    pub push: f64,
    pub cheeks: f64,
    pub tone: f64,
    pub tension_hz: f64,
    pub clench: bool,
    pub rasp: f64,
    pub digest: bool,
    pub sound: bool,
    pub shot: bool,
    /// Gas eaten just before this hop (J).
    pub eaten_j: f64,
}

/// Where a note sits in a performance's output, in hops (of [`HOP_S`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoteSpan {
    pub event: usize,
    pub start_hop: usize,
    pub end_hop: usize,
    pub target_hz: f64,
    pub tension_hz: f64,
    pub shot: bool,
}

/// A performance: the listener's copy (DC-blocked, pre-roll trimmed), one
/// trace entry per output hop, and where each note sits.
#[derive(Debug, Clone, Default)]
pub struct Performance {
    pub audio: Vec<f32>,
    pub trace: Vec<HopTrace>,
    pub notes: Vec<NoteSpan>,
}

impl Gut {
    /// Play `events` as written. Starts with breath 0, push 0.7, cheeks 0.4,
    /// tone 0.75, no wobble, tongued, not digesting (all the player's to
    /// change); the reservoir as it is. Ends with a 0.25 s ring-out.
    pub fn perform(&mut self, events: &[Event]) -> Result<Performance, WorldError> {
        let rate = self.world.sample_rate_hz();
        let hop = (HOP_S * rate) as usize;
        let mut block = vec![0.0f32; hop];
        let mut out = Performance::default();
        let (mut push, mut lips, mut tone, mut tongue, mut digest) = (0.7, 0.4, 0.75, true, false);
        let mut vib = Vibrato { rate_hz: 5.5, depth_semitones: 0.0, wobble: 0.0 };
        self.world.set_vibrato(vib);
        self.world.set_refill(0.0);
        let mut tension = self.world.controls().pitch_hz;
        let (mut jitter, mut rasp, mut eaten) = (0.0f64, 0.0f64, 0.0f64);
        let (mut t_end, mut hops_done) = (0.0f64, 0usize);
        let mut emitted = 0usize;
        let mut prms = vec![0.0f64; self.world.anatomy().oral_sections];
        self.world.take_pressure_ms(&mut prms);
        self.world.reset_windows();
        for (i, ev) in events.iter().enumerate() {
            let (sound, seconds, shot, target) = match *ev {
                Event::Push(v) => { push = v.clamp(0.0, 1.0); continue; }
                Event::Cheeks(v) => { lips = v.clamp(0.0, 1.0); continue; }
                Event::Tone(v) => { tone = v.clamp(0.0, 1.0); continue; }
                Event::Jitter(v) => { jitter = v.clamp(0.0, 12.0); continue; }
                Event::Rasp(v) => { rasp = v.clamp(0.0, 1.0); continue; }
                Event::Wobble(v) => { vib.wobble = v; self.world.set_vibrato(vib); vib = self.world.vibrato(); continue; }
                Event::Vibrato { rate_hz, depth_semitones } => {
                    vib.rate_hz = rate_hz;
                    vib.depth_semitones = depth_semitones;
                    self.world.set_vibrato(vib);
                    vib = self.world.vibrato();
                    continue;
                }
                Event::Tongue(on) => { tongue = on; continue; }
                Event::Eat(food) => { self.eat(food); eaten += food.gas_j; continue; }
                Event::Digest(on) => { digest = on; continue; }
                Event::Pre { seconds } => {
                    let n = (seconds.max(0.0) / HOP_S).round() as usize;
                    for _ in 0..n {
                        if digest { self.pay_hop(); } else { self.world.set_refill(0.0); }
                        let mut c = self.world.controls();
                        c.breath = 0.0;
                        self.world.set_controls(c);
                        self.world.process(&mut block)?;
                    }
                    // The pre-roll is trimmed: its observers are not reported.
                    self.world.take_pressure_ms(&mut prms);
                    self.world.reset_windows();
                    continue;
                }
                Event::Rest { seconds } => (false, seconds, false, 0.0),
                Event::Note { target_hz, tension_hz, seconds, shot } => {
                    if !shot {
                        tension = tension_hz;
                    }
                    (true, seconds, shot, target_hz)
                }
            };
            t_end += seconds;
            let hops = (((t_end / HOP_S).round() as usize).saturating_sub(hops_done)).max(1);
            hops_done += hops;
            // A tongued note followed by another note ends clenched.
            let next_is_note = events[i + 1..].iter().find_map(|e| match e {
                Event::Note { .. } => Some(true),
                Event::Rest { .. } => Some(false),
                _ => None,
            }) == Some(true);
            let gap = if sound && !shot && tongue && next_is_note { ((GAP_S / HOP_S).round() as usize).min(hops / 3) } else { 0 };
            if sound {
                out.notes.push(NoteSpan { event: i, start_hop: emitted, end_hop: emitted + hops, target_hz: target, tension_hz: tension, shot });
            }
            for h in 0..hops {
                if digest { self.pay_hop(); } else { self.world.set_refill(0.0); }
                let t = h as f64 * HOP_S;
                let mut c = self.world.controls();
                let clench;
                if shot {
                    let loading = t < SHOT_LOAD_S;
                    clench = loading;
                    c.articulation = cheeks(if loading { lips } else { lips.max(0.8) });
                    c.breath = if t < SHOT_LOAD_S + SHOT_OPEN_S { push } else { 0.0 };
                    c.pitch_hz = 40.0;
                } else {
                    clench = sound && h >= hops - gap;
                    c.articulation = cheeks(lips);
                    c.breath = if sound { push } else { 0.0 };
                    c.pitch_hz = tension;
                    if sound && jitter > 0.0 {
                        c.pitch_hz *= ((2.0 * self.uniform() - 1.0) * jitter / 12.0).exp2();
                    }
                }
                c.tenseness = tone;
                self.world.set_controls(c);
                self.world.set_constriction(SEAL_SLOT, clench.then_some(Constriction { position: SEAL_AT, degree: 1.0 }));
                self.world.set_constriction(RASP_SLOT, (sound && !shot && rasp > 0.0).then_some(Constriction { position: RASP_AT, degree: rasp }));
                self.world.process(&mut block)?;
                out.audio.extend_from_slice(&block);
                let mut tr = self.hop_trace(&mut prms);
                tr.push = c.breath;
                tr.cheeks = if shot { c.articulation.lips } else { lips };
                tr.tone = tone;
                tr.tension_hz = c.pitch_hz;
                tr.clench = clench;
                tr.rasp = if sound && !shot { rasp } else { 0.0 };
                tr.digest = digest;
                tr.sound = sound;
                tr.shot = shot;
                tr.eaten_j = eaten;
                eaten = 0.0;
                out.trace.push(tr);
                emitted += 1;
            }
        }
        let mut c = self.world.controls();
        c.breath = 0.0;
        self.world.set_controls(c);
        self.world.set_constriction(SEAL_SLOT, None);
        self.world.set_constriction(RASP_SLOT, None);
        self.world.set_refill(0.0);
        for _ in 0..((0.25 / HOP_S) as usize) {
            self.world.process(&mut block)?;
            out.audio.extend_from_slice(&block);
            let mut tr = self.hop_trace(&mut prms);
            tr.cheeks = lips;
            tr.tone = tone;
            tr.tension_hz = tension;
            tr.digest = digest;
            out.trace.push(tr);
        }
        dc_block(&mut out.audio, rate);
        Ok(out)
    }

    /// A uniform draw in 0..1 (xorshift, the `flutter` style's).
    fn uniform(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 11) as f64 / (1u64 << 53) as f64
    }

    /// The observers after a hop (`prms` receives the mean-square pressures
    /// and is reset, as the ABI's snapshot does). Reads only.
    fn hop_trace(&mut self, prms: &mut [f64]) -> HopTrace {
        self.world.take_pressure_ms(prms);
        let obs = self.world.observers();
        self.world.reset_windows();
        let a = self.world.anatomy();
        let n = a.oral_sections;
        let w = self.world.world();
        let body = w.space_body(0);
        let o = body.map(|b| b.observation());
        let t = body.and_then(|b| b.totals()).unwrap_or_default();
        let gap = o.as_ref().map_or(0.0, |o| o.valve_half_gap_m);
        let (gap_min_m, gap_max_m) = if obs.gap_min_m <= obs.gap_max_m { (obs.gap_min_m, obs.gap_max_m) } else { (gap, gap) };
        let areas = body.and_then(|b| b.network_areas_m2()).unwrap_or(&[]);
        HopTrace {
            f0_hz: obs.measured_f0_hz,
            gas_fraction: self.gas_fraction(),
            supply_pa: o.as_ref().map_or(0.0, |o| o.valve.supply_pressure_pa),
            clipped: obs.safety_limited,
            reservoir_j: body.map_or(0.0, |b| b.reservoir_energy()),
            stomach_j: self.stomach.iter().map(|(_, left)| left.max(0.0)).sum(),
            gap_min_m,
            gap_max_m,
            flow_m3_s: o.as_ref().map_or(0.0, |o| o.valve.flow_m3_s),
            field_j: body.map_or(0.0, |b| b.field_energy()),
            radiated_j: t.radiation_by_end.iter().sum(),
            turbulence_j: t.aero.turbulence_injected,
            jet_aperture_m2: o.as_ref().map_or(0.0, |o| o.aero.jet_aperture_m2),
            jet_position: o.as_ref().map_or(0.0, |o| (o.aero.jet_junction as f64 + 1.0) / n as f64),
            areas_m2: areas.iter().take(n).map(|a| *a as f32).collect(),
            pressure_rms_pa: prms.iter().take(n).map(|p| p.sqrt() as f32).collect(),
            ..HopTrace::default()
        }
    }
}

/// A cannon shot: the cheeks clench at 0.85 push for [`SHOT_LOAD_S`] (the
/// pressure builds behind the seal), then open wide: the release burst.
pub const SHOT_LOAD_S: f64 = 0.12;
const SHOT_OPEN_S: f64 = 0.1;

impl Gut {
    /// Fire at every note of `shots` whose `hz` is not 0 (its pitch is not
    /// used), at the note's start; rest otherwise. With `digest`, the stomach
    /// (or the house feed) pays the reservoir throughout. Returns the
    /// listener's copy and the number of shots fired.
    pub fn play_cannon(&mut self, shots: &[Note], digest: bool) -> Result<(Vec<f32>, usize), WorldError> {
        let rate = self.world.sample_rate_hz();
        let hop = (HOP_S * rate) as usize;
        let mut audio = Vec::new();
        let mut block = vec![0.0f32; hop];
        let mut fired = 0;
        self.world.set_vibrato(Vibrato { rate_hz: 5.5, depth_semitones: 0.0, wobble: 0.0 });
        let (mut t_end, mut hops_done) = (0.0, 0usize);
        for shot in shots {
            t_end += shot.seconds;
            let hops = (((t_end / HOP_S).round() as usize).saturating_sub(hops_done)).max(1);
            hops_done += hops;
            if shot.hz > 0.0 {
                fired += 1;
            }
            for h in 0..hops {
                if digest {
                    self.pay_hop();
                } else {
                    self.world.set_refill(0.0);
                }
                let t = h as f64 * HOP_S;
                let loading = shot.hz > 0.0 && t < SHOT_LOAD_S;
                let open = shot.hz > 0.0 && t < SHOT_LOAD_S + SHOT_OPEN_S;
                let mut c = self.world.controls();
                c.articulation = cheeks(if loading { 0.45 } else { 0.8 });
                c.pitch_hz = 40.0;
                c.tenseness = 0.6;
                c.breath = if open { 0.85 } else { 0.0 };
                self.world.set_controls(c);
                self.world.set_constriction(SEAL_SLOT, loading.then_some(Constriction { position: SEAL_AT, degree: 1.0 }));
                self.world.process(&mut block)?;
                audio.extend_from_slice(&block);
            }
        }
        let mut c = self.world.controls();
        c.breath = 0.0;
        self.world.set_controls(c);
        self.world.set_constriction(SEAL_SLOT, None);
        self.world.set_refill(0.0);
        for _ in 0..((0.25 / HOP_S) as usize) {
            self.world.process(&mut block)?;
            audio.extend_from_slice(&block);
        }
        dc_block(&mut audio, rate);
        Ok((audio, fired))
    }
}

/// One-pole DC blocker at [`DC_BLOCK_HZ`], in place.
pub fn dc_block(audio: &mut [f32], sample_rate_hz: f64) {
    let r = (-std::f64::consts::TAU * DC_BLOCK_HZ / sample_rate_hz).exp();
    let (mut x1, mut y1) = (0.0f64, 0.0f64);
    for v in audio {
        let x = f64::from(*v);
        y1 = x - x1 + r * y1;
        x1 = x;
        *v = y1 as f32;
    }
}
