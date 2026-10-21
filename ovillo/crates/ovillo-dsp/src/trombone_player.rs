//! A player for [`crate::trombone_world::TromboneWorld`]: turns a note list
//! into the three things a trombonist controls, sample by sample: mouth
//! pressure, lip tension and slide.
//!
//! Nothing here makes sound. The player's knowledge of the instrument (which
//! lip tension speaks which partial, how much pressure it wants, and how far the
//! sounding pitch sits from the bore resonance) was learned by playing the
//! World (`examples/trombone_world_calibrate.rs` ->
//! `mus_analysis/trombone_player_fit.py` -> `profiles/ariadne-trombone-v0.player.json`).
//!
//! - Tonguing is a source-pressure gesture: the tongue stops the air between
//!   notes and releases it at the onset.
//! - A slur moves the lips to the next partial's tension under a legato
//!   tongue: the air closes smoothly over 10 ms and is released as in a
//!   tongued attack. A pure lip slur (air kept going) leaves the new partial
//!   about 28 dB weaker than a tongued one in this World (TROMBONE-WORLD.md).
//! - A smear (glissando) and every pitch inflection (scoop, fall, vibrato) are
//!   slide transport.
//! - A rip is a fast lip-tension ramp through the partials below the target.

use crate::trombone_world::{Breath, TromboneWorld};
use serde::Deserialize;

pub const SKILL_JSON: &str = include_str!("../../../../profiles/ariadne-trombone-v0.player.json");

/// What the player knows about one partial, at each calibrated slide point.
#[derive(Debug, Clone, Deserialize)]
pub struct PartialSkill {
    pub n: usize,
    pub slides: Vec<f64>,
    pub speaks: Vec<bool>,
    /// Lip natural frequency as a ratio of bore resonance `n`.
    pub ratio: Vec<Option<f64>>,
    /// Mouth pressure for the quietest and loudest note that still speaks.
    pub pressure_soft: Vec<Option<f64>>,
    pub pressure_loud: Vec<Option<f64>>,
    /// Sounding pitch minus bore resonance `n`, cents.
    pub offset_cents: Vec<Option<f64>>,
    /// The run of lip ratios that speak it (its width is how forgiving the cell is).
    pub window: Vec<Option<[f64; 2]>>,
}

/// The embouchure for one partial at one slide amount.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Embouchure {
    pub ratio: f64,
    pub pressure_soft: f64,
    pub pressure_loud: f64,
    pub offset_cents: f64,
    pub window: f64,
}

impl PartialSkill {
    fn at(&self, i: usize) -> Option<Embouchure> {
        if !self.speaks.get(i).copied().unwrap_or(false) {
            return None;
        }
        let w = self.window[i]?;
        Some(Embouchure {
            ratio: self.ratio[i]?,
            pressure_soft: self.pressure_soft[i]?,
            pressure_loud: self.pressure_loud[i]?,
            offset_cents: self.offset_cents[i]?,
            window: w[1] - w[0],
        })
    }

    /// Linear between the two calibrated slide points around `slide` when both
    /// speak; the nearer one alone when it is within a quarter of the spacing.
    #[must_use]
    pub fn embouchure(&self, slide: f64) -> Option<Embouchure> {
        let s = slide.clamp(0.0, 1.0);
        let k = self.slides.len();
        if k == 0 {
            return None;
        }
        let mut i = 0;
        while i + 2 < k && self.slides[i + 1] <= s {
            i += 1;
        }
        let (s0, s1) = (self.slides[i], self.slides[(i + 1).min(k - 1)]);
        let t = if s1 > s0 { ((s - s0) / (s1 - s0)).clamp(0.0, 1.0) } else { 0.0 };
        match (self.at(i), self.at((i + 1).min(k - 1))) {
            (Some(a), Some(b)) => {
                let l = |x: f64, y: f64| x + (y - x) * t;
                let g = |x: f64, y: f64| (x.ln() + (y.ln() - x.ln()) * t).exp();
                Some(Embouchure {
                    ratio: l(a.ratio, b.ratio),
                    pressure_soft: g(a.pressure_soft, b.pressure_soft),
                    pressure_loud: g(a.pressure_loud, b.pressure_loud),
                    offset_cents: l(a.offset_cents, b.offset_cents),
                    window: a.window.min(b.window),
                })
            }
            (Some(a), None) if t <= 0.25 => Some(a),
            (None, Some(b)) if t >= 0.75 => Some(b),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Skill {
    pub partials: Vec<PartialSkill>,
}

impl Skill {
    pub fn embedded() -> Result<Self, String> {
        serde_json::from_str(SKILL_JSON).map_err(|e| e.to_string())
    }

    #[must_use]
    pub fn partial(&self, n: usize) -> Option<&PartialSkill> {
        self.partials.iter().find(|p| p.n == n)
    }

    #[must_use]
    pub fn embouchure(&self, n: usize, slide: f64) -> Option<Embouchure> {
        self.partial(n)?.embouchure(slide)
    }

    /// As [`Skill::embouchure`], or failing that the calibrated slide point
    /// nearest `slide` where partial `n` speaks (for the lips while the slide
    /// is in transit through a cell that does not speak on its own).
    #[must_use]
    pub fn embouchure_near(&self, n: usize, slide: f64) -> Option<Embouchure> {
        if let Some(e) = self.embouchure(n, slide) {
            return Some(e);
        }
        let p = self.partial(n)?;
        let mut best: Option<(f64, Embouchure)> = None;
        for (i, s) in p.slides.iter().enumerate() {
            if let Some(e) = p.at(i) {
                let d = (s - slide).abs();
                if best.is_none_or(|b| d < b.0) {
                    best = Some((d, e));
                }
            }
        }
        best.map(|b| b.1)
    }

    #[must_use]
    pub fn offset(&self, n: usize, slide: f64) -> f64 {
        self.embouchure(n, slide).map_or(0.0, |e| e.offset_cents)
    }

    /// The slide amount at which partial `n` sounds `hz`.
    #[must_use]
    pub fn slide_for(&self, tw: &TromboneWorld, n: usize, hz: f64) -> f64 {
        let mut s = tw.slide_for(n, hz);
        for _ in 0..8 {
            s = tw.slide_for(n, hz / 2f64.powf(self.offset(n, s) / 1200.0));
        }
        s
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Art {
    /// Tongued ("doo").
    Tongue,
    /// The air keeps going; the lips move to the next partial.
    Slur,
    /// The slide carries the pitch from the previous note.
    Smear,
}

pub const SCOOP: u8 = 1;
pub const FALL: u8 = 2;
pub const VIB: u8 = 4;
pub const RIP: u8 = 16;
pub const SWELL: u8 = 32;
pub const DIM: u8 = 64;

#[derive(Debug, Clone, Copy)]
pub struct Note {
    pub midi: f64,
    pub start_s: f64,
    pub end_s: f64,
    /// 0..1.
    pub vel: f64,
    pub art: Art,
    pub fx: u8,
}

/// Where a note is played.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub partial: usize,
    pub slide: f64,
}

impl Placement {
    /// Nearest slide position, 1..7.
    #[must_use]
    pub fn position(&self) -> u8 {
        (1.0 + 6.0 * self.slide).round().clamp(1.0, 7.0) as u8
    }
}

#[must_use]
pub fn hz_of(midi: f64) -> f64 {
    440.0 * 2f64.powf((midi - 69.0) / 12.0)
}

/// Choose partial and slide for each note: short positions first, little slide
/// travel, smears stay on the previous partial.
#[must_use]
pub fn place(tw: &TromboneWorld, skill: &Skill, notes: &[Note]) -> Vec<Placement> {
    let mut out: Vec<Placement> = Vec::with_capacity(notes.len());
    for note in notes {
        let f = hz_of(note.midi);
        let prev = out.last().copied();
        let mut best: Option<(f64, Placement)> = None;
        for p in &skill.partials {
            if p.n < 2 {
                continue;
            }
            let s = skill.slide_for(tw, p.n, f);
            if !(-0.01..=1.01).contains(&s) {
                continue;
            }
            // a cell the calibration never heard speak is a last resort (+5)
            let mut cost = s + skill.embouchure(p.n, s).map_or(5.0, |e| 0.08 / e.window.max(0.02));
            if skill.embouchure_near(p.n, s).is_none() {
                continue;
            }
            if let Some(q) = prev {
                cost += 0.7 * (s - q.slide).abs();
                if note.art == Art::Smear && q.partial != p.n {
                    cost += 10.0;
                }
            }
            if p.n > 8 {
                cost += 0.5;
            }
            if best.is_none_or(|b| cost < b.0) {
                best = Some((cost, Placement { partial: p.n, slide: s.clamp(0.0, 1.0) }));
            }
        }
        out.push(best.map_or(Placement { partial: 2, slide: 0.0 }, |b| b.1));
    }
    out
}

fn smoothstep(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The performance, one [`Breath`] per sample, `tail_s` of silence after the last note.
#[must_use]
pub fn perform(tw: &TromboneWorld, skill: &Skill, notes: &[Note], placed: &[Placement], sr: f64, tail_s: f64) -> Vec<Breath> {
    let total = notes.last().map_or(0.0, |n| n.end_s) + tail_s;
    let len = (total * sr) as usize;
    let mut out = vec![Breath::default(); len];
    let emb = |n: usize, slide: f64| skill.embouchure_near(n, slide).expect("a partial the player has played");
    let lip_of = |n: usize, slide: f64| emb(n, slide).ratio * tw.bore_resonance_hz(n, slide);
    let semis = 1.0 / 6.0; // slide amount per semitone
    // Before the first note: lips set for it, slide in place, no air.
    let first = (notes[0].start_s * sr) as usize;
    for b in out.iter_mut().take(first) {
        *b = Breath { pressure: 0.0, lip_hz: lip_of(placed[0].partial, placed[0].slide), slide: placed[0].slide };
    }
    let mut last_pressure = 0.0;
    for (i, note) in notes.iter().enumerate() {
        let pl = placed[i];
        let skill_n = emb(pl.partial, pl.slide);
        let prev = if i > 0 { Some((notes[i - 1], placed[i - 1])) } else { None };
        let next_legato = i + 1 < notes.len() && notes[i + 1].art != Art::Tongue && (notes[i + 1].start_s - note.end_s).abs() < 0.03;
        let s0 = (note.start_s * sr) as usize;
        let owned_until = if i + 1 < notes.len() { (notes[i + 1].start_s * sr) as usize } else { len };
        let sounding = if next_legato { owned_until as f64 / sr - note.start_s } else { (note.end_s - note.start_s) * 0.90 };
        let release_s = if note.fx & FALL != 0 { 0.12 } else { 0.008 };
        let fall_s = if note.fx & FALL != 0 { (0.45 * sounding).min(0.6) } else { 0.0 };
        // pressure for this note's dynamic: log between soft and loud
        let vel = note.vel.clamp(0.0, 1.0);
        // only pressures the calibration heard speak this partial here
        let (soft, loud) = (skill_n.pressure_soft, skill_n.pressure_loud);
        let p_note = soft * (loud / soft).powf(vel);
        let p_from = if note.art == Art::Tongue { 0.0 } else { last_pressure };
        let (prev_slide, prev_lip) = prev.map_or((pl.slide, lip_of(pl.partial, pl.slide)), |(_, q)| (q.slide, lip_of(q.partial, q.slide)));
        let vib_rate = 5.0 + 0.4 * ((i * 37 % 11) as f64 / 11.0);
        for t in s0..owned_until.min(len) {
            let tau = t as f64 / sr - note.start_s;
            // the slide
            let mut slide = pl.slide;
            match note.art {
                Art::Smear => slide = prev_slide + (pl.slide - prev_slide) * smoothstep(tau / 0.17),
                Art::Slur => slide = prev_slide + (pl.slide - prev_slide) * smoothstep(tau / 0.035),
                Art::Tongue => {}
            }
            if note.fx & SCOOP != 0 {
                slide += 1.0 * semis * (1.0 - smoothstep(tau / 0.12));
            }
            if note.fx & FALL != 0 {
                let into = (tau - (sounding - fall_s)) / (fall_s + release_s);
                if into > 0.0 {
                    slide += 3.5 * semis * into.clamp(0.0, 1.0).powi(2);
                }
            }
            let base_slide = slide.clamp(0.0, 1.0);
            if note.fx & VIB != 0 {
                let onset = 0.3 * sounding.min(1.2);
                let depth = 0.20 * semis * smoothstep((tau - onset) / 0.5);
                slide += depth * (std::f64::consts::TAU * vib_rate * tau).sin();
            }
            // the lips follow the partial along the slide (not the vibrato)
            let mut lip_hz = lip_of(pl.partial, base_slide);
            if note.art == Art::Slur {
                let k = smoothstep(tau / 0.025);
                lip_hz = (prev_lip.ln() + (lip_hz.ln() - prev_lip.ln()) * k).exp();
            }
            if note.fx & RIP != 0 {
                let rip_s = 0.15;
                let low = lip_of(pl.partial.saturating_sub(4).max(2), base_slide);
                let k = (tau / rip_s).clamp(0.0, 1.0);
                lip_hz = (low.ln() + (lip_hz.ln() - low.ln()) * k).exp();
            }
            // the air
            let attack = match note.art {
                Art::Tongue => 1.0 - (-tau / 0.006).exp(),
                Art::Slur | Art::Smear => 1.0,
            };
            let mut shape = 1.0;
            if note.fx & SWELL != 0 {
                shape *= 0.75 + 0.4 * smoothstep(tau / sounding);
            }
            if note.fx & DIM != 0 {
                shape *= 1.0 - 0.45 * smoothstep(tau / sounding);
            }
            let target = p_note * shape;
            let carried = p_from + (target - p_from) * smoothstep(tau / 0.03);
            let level = match note.art {
                Art::Tongue => target * attack,
                // legato tongue: the air closes smoothly over 10 ms, then the tongue
                // releases it as in a tongued attack
                Art::Slur => {
                    if tau < 0.010 {
                        p_from * (0.5 + 0.5 * (std::f64::consts::PI * tau / 0.010).cos())
                    } else {
                        target * (1.0 - (-(tau - 0.010) / 0.006).exp())
                    }
                }
                Art::Smear => carried,
            };
            let release = if tau > sounding { (-(tau - sounding) / release_s).exp() } else { 1.0 };
            let pressure = level * release;
            last_pressure = pressure;
            out[t] = Breath { pressure, lip_hz, slide: slide.clamp(0.0, 1.0) };
        }
    }
    out
}
