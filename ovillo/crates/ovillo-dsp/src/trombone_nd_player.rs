//! A player for [`crate::trombone_nd::TromboneNd`]: a note list becomes, per
//! sample, mouth pressure, lip tension and slide extension. Nothing here makes
//! sound.
//!
//! What the player knows was learned by playing the World
//! (`examples/trombone_nd_calibrate.rs` -> `mus_analysis/trombone_nd_player_fit.py`
//! -> `profiles/ariadne-trombone-v1.player.json`): per partial and slide
//! position, the lip tension that speaks it, the pitch offset from the bore
//! mode, and the level each mouth pressure gives. A written dynamic is a
//! target level; the player blows whatever pressure gives that level on that
//! cell, as players balance their notes. Articulation is v0's
//! ([`crate::trombone_player`]): tonguing is a mouth-pressure gesture, a slur
//! moves the lips under a legato tongue, smears, scoops, falls and vibrato are
//! slide transport, a rip is a lip-tension ramp.

use crate::trombone_nd::Breath;
use crate::trombone_player::{Art, Note, DIM, FALL, RIP, SCOOP, SWELL, VIB};
use serde::Deserialize;

pub const RECEIPT_JSON: &str = include_str!("../../../../profiles/ariadne-trombone-v1.receipt.json");
pub const SKILL_JSON: &str = include_str!("../../../../profiles/ariadne-trombone-v1.player.json");
/// Embouchure firmness (the valve's adduction) throughout: open, as the
/// lip-tension sweeps found every partial locking there.
pub const EMBOUCHURE: f64 = 0.1;

#[derive(Debug, Clone, Deserialize)]
struct SlideReceipt {
    positions_m: Vec<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct TableReceipt {
    extension_m: Vec<f64>,
    frequency_hz: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, Deserialize)]
struct Receipt {
    slide: SlideReceipt,
    mode_table: TableReceipt,
}

/// The bore's computed modes against slide extension (the anatomy receipt's
/// table of the runtime's mode solve).
#[derive(Debug, Clone)]
pub struct ModeTable {
    pub positions_m: Vec<f64>,
    x: Vec<f64>,
    f: Vec<Vec<f64>>,
}

impl ModeTable {
    pub fn embedded() -> Result<Self, String> {
        let r: Receipt = serde_json::from_str(RECEIPT_JSON).map_err(|e| e.to_string())?;
        Ok(Self { positions_m: r.slide.positions_m, x: r.mode_table.extension_m, f: r.mode_table.frequency_hz })
    }

    #[must_use]
    pub fn max_extension(&self) -> f64 {
        *self.x.last().unwrap_or(&0.0)
    }

    /// Bore mode `n` (1-based) at extension `x` (log-linear between table rows).
    #[must_use]
    pub fn mode_hz(&self, n: usize, x: f64) -> f64 {
        let x = x.clamp(0.0, self.max_extension());
        let i = self.x.partition_point(|v| *v <= x).saturating_sub(1).min(self.x.len() - 2);
        let t = (x - self.x[i]) / (self.x[i + 1] - self.x[i]);
        let (a, b) = (self.f[i][n - 1], self.f[i + 1][n - 1]);
        (a.ln() + t * (b.ln() - a.ln())).exp()
    }

    /// Extension at which mode `n` is `hz` (may be outside the slide).
    #[must_use]
    pub fn extension_for(&self, n: usize, hz: f64) -> f64 {
        let (mut lo, mut hi) = (0.0, self.max_extension());
        if hz >= self.mode_hz(n, lo) {
            return -0.001 * 1200.0 * (hz / self.mode_hz(n, lo)).log2();
        }
        if hz <= self.mode_hz(n, hi) {
            return hi + 0.001 * 1200.0 * (self.mode_hz(n, hi) / hz).log2();
        }
        for _ in 0..60 {
            let mid = 0.5 * (lo + hi);
            if self.mode_hz(n, mid) > hz {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    }

    /// Extension change that lowers mode `n` by `semis` semitones from `x`.
    #[must_use]
    pub fn shift(&self, n: usize, x: f64, semis: f64) -> f64 {
        self.extension_for(n, self.mode_hz(n, x) * 2f64.powf(-semis / 12.0)) - x
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Cell {
    pub position: usize,
    pub slide_m: f64,
    pub ratio: f64,
    pub offset_cents: f64,
    pub window: [f64; 2],
    pub mouths: Vec<f64>,
    pub levels_db: Vec<f64>,
}

impl Cell {
    /// The mouth pressure whose calibrated level is `level_db` (log-mouth
    /// linear between calibrated points, clamped to the speaking range).
    #[must_use]
    pub fn mouth_for(&self, level_db: f64) -> f64 {
        let n = self.mouths.len();
        if n == 1 {
            return self.mouths[0];
        }
        let mut pts: Vec<(f64, f64)> = self.levels_db.iter().copied().zip(self.mouths.iter().map(|m| m.ln())).collect();
        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
        if level_db <= pts[0].0 {
            return pts[0].1.exp();
        }
        if level_db >= pts[n - 1].0 {
            return pts[n - 1].1.exp();
        }
        let i = pts.partition_point(|p| p.0 <= level_db) - 1;
        let t = (level_db - pts[i].0) / (pts[i + 1].0 - pts[i].0).max(1e-9);
        (pts[i].1 + t * (pts[i + 1].1 - pts[i].1)).exp()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PartialSkill {
    pub n: usize,
    pub cells: Vec<Option<Cell>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Skill {
    pub partials: Vec<PartialSkill>,
    /// The written-dynamic scale: velocity 0 aims at `level_soft_db`, 1 at `level_loud_db`.
    pub level_soft_db: f64,
    pub level_loud_db: f64,
}

/// What the player does for one partial at one slide extension.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Embouchure {
    pub ratio: f64,
    pub offset_cents: f64,
    pub mouth: f64,
    pub window: f64,
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
    pub fn target_level(&self, vel: f64) -> f64 {
        self.level_soft_db + vel.clamp(0.0, 1.0) * (self.level_loud_db - self.level_soft_db)
    }

    /// Between the two calibrated positions around `x` when both speak; the
    /// nearer alone within a quarter of the spacing.
    #[must_use]
    pub fn embouchure(&self, table: &ModeTable, n: usize, x: f64, vel: f64) -> Option<Embouchure> {
        let p = self.partial(n)?;
        let pos = &table.positions_m;
        let i = pos.partition_point(|v| *v <= x).saturating_sub(1).min(pos.len() - 2);
        let t = ((x - pos[i]) / (pos[i + 1] - pos[i])).clamp(0.0, 1.0);
        let level = self.target_level(vel);
        let one = |c: &Cell| Embouchure { ratio: c.ratio, offset_cents: c.offset_cents, mouth: c.mouth_for(level), window: c.window[1] - c.window[0] };
        match (p.cells.get(i).and_then(Option::as_ref), p.cells.get(i + 1).and_then(Option::as_ref)) {
            (Some(a), Some(b)) => {
                let (ea, eb) = (one(a), one(b));
                Some(Embouchure {
                    ratio: ea.ratio + t * (eb.ratio - ea.ratio),
                    offset_cents: ea.offset_cents + t * (eb.offset_cents - ea.offset_cents),
                    mouth: (ea.mouth.ln() + t * (eb.mouth.ln() - ea.mouth.ln())).exp(),
                    window: ea.window.min(eb.window),
                })
            }
            (Some(a), None) if t <= 0.25 => Some(one(a)),
            (None, Some(b)) if t >= 0.75 => Some(one(b)),
            _ => None,
        }
    }

    /// The nearest calibrated position where `n` speaks (lips in transit).
    #[must_use]
    pub fn embouchure_near(&self, table: &ModeTable, n: usize, x: f64, vel: f64) -> Option<Embouchure> {
        if let Some(e) = self.embouchure(table, n, x, vel) {
            return Some(e);
        }
        let p = self.partial(n)?;
        let level = self.target_level(vel);
        p.cells
            .iter()
            .flatten()
            .min_by(|a, b| (a.slide_m - x).abs().total_cmp(&(b.slide_m - x).abs()))
            .map(|c| Embouchure { ratio: c.ratio, offset_cents: c.offset_cents, mouth: c.mouth_for(level), window: c.window[1] - c.window[0] })
    }

    /// The extension at which partial `n` sounds `hz`.
    #[must_use]
    pub fn slide_for(&self, table: &ModeTable, n: usize, hz: f64, vel: f64) -> f64 {
        let mut x = table.extension_for(n, hz);
        for _ in 0..8 {
            let off = self.embouchure_near(table, n, x.clamp(0.0, table.max_extension()), vel).map_or(0.0, |e| e.offset_cents);
            x = table.extension_for(n, hz / 2f64.powf(off / 1200.0));
        }
        x
    }
}

/// Where a note is played.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub partial: usize,
    pub slide_m: f64,
}

impl Placement {
    /// Nearest of the seven positions, 1..7.
    #[must_use]
    pub fn position(&self, table: &ModeTable) -> usize {
        table
            .positions_m
            .iter()
            .enumerate()
            .min_by(|a, b| (a.1 - self.slide_m).abs().total_cmp(&(b.1 - self.slide_m).abs()))
            .map_or(1, |(i, _)| i + 1)
    }
}

/// Short positions first, little travel, smears stay on their partial; a
/// cell the calibration never heard speak is a last resort.
#[must_use]
pub fn place(table: &ModeTable, skill: &Skill, notes: &[Note]) -> Vec<Placement> {
    let mut out: Vec<Placement> = Vec::with_capacity(notes.len());
    for note in notes {
        let f = crate::trombone_player::hz_of(note.midi);
        let prev = out.last().copied();
        let mut best: Option<(f64, Placement)> = None;
        for p in &skill.partials {
            let x = skill.slide_for(table, p.n, f, note.vel);
            if !(-0.005..=table.max_extension() + 0.005).contains(&x) {
                continue;
            }
            let x = x.clamp(0.0, table.max_extension());
            let mut cost = x / table.max_extension() + skill.embouchure(table, p.n, x, note.vel).map_or(5.0, |e| 0.02 / e.window.max(0.01));
            if let Some(q) = prev {
                cost += 0.7 * (x - q.slide_m).abs() / table.max_extension();
                if note.art == Art::Smear && q.partial != p.n {
                    cost += 10.0;
                }
            }
            if best.is_none_or(|b| cost < b.0) {
                best = Some((cost, Placement { partial: p.n, slide_m: x }));
            }
        }
        out.push(best.map_or(Placement { partial: 4, slide_m: 0.0 }, |b| b.1));
    }
    out
}

fn smoothstep(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// The performance, one [`Breath`] per sample, `tail_s` of silence after.
#[must_use]
pub fn perform(table: &ModeTable, skill: &Skill, notes: &[Note], placed: &[Placement], sr: f64, tail_s: f64) -> Vec<Breath> {
    let total = notes.last().map_or(0.0, |n| n.end_s) + tail_s;
    let len = (total * sr) as usize;
    let emb = |n: usize, x: f64, vel: f64| skill.embouchure_near(table, n, x, vel).expect("a partial the player has played");
    let lip_of = |n: usize, x: f64, vel: f64| emb(n, x, vel).ratio * table.mode_hz(n, x);
    let first = (notes[0].start_s * sr) as usize;
    let mut out = vec![Breath { mouth: 0.0, lip_hz: lip_of(placed[0].partial, placed[0].slide_m, notes[0].vel), embouchure: EMBOUCHURE, slide_m: placed[0].slide_m }; len];
    let _ = first;
    let mut last_mouth = 0.0;
    for (i, note) in notes.iter().enumerate() {
        let pl = placed[i];
        let e = emb(pl.partial, pl.slide_m, note.vel);
        let prev = if i > 0 { Some((notes[i - 1], placed[i - 1])) } else { None };
        let next_legato = i + 1 < notes.len() && notes[i + 1].art != Art::Tongue && (notes[i + 1].start_s - note.end_s).abs() < 0.03;
        let s0 = (note.start_s * sr) as usize;
        let owned_until = if i + 1 < notes.len() { (notes[i + 1].start_s * sr) as usize } else { len };
        let sounding = if next_legato { owned_until as f64 / sr - note.start_s } else { (note.end_s - note.start_s) * 0.90 };
        let release_s = if note.fx & FALL != 0 { 0.12 } else { 0.008 };
        let fall_s = if note.fx & FALL != 0 { (0.45 * sounding).min(0.6) } else { 0.0 };
        let p_from = if note.art == Art::Tongue { 0.0 } else { last_mouth };
        let (prev_slide, prev_lip) =
            prev.map_or((pl.slide_m, lip_of(pl.partial, pl.slide_m, note.vel)), |(pn, q)| (q.slide_m, lip_of(q.partial, q.slide_m, pn.vel)));
        let vib_rate = 5.0 + 0.4 * ((i * 37 % 11) as f64 / 11.0);
        let semi = table.shift(pl.partial, pl.slide_m, 1.0);
        for t in s0..owned_until.min(len) {
            let tau = t as f64 / sr - note.start_s;
            let mut x = pl.slide_m;
            match note.art {
                Art::Smear => x = prev_slide + (pl.slide_m - prev_slide) * smoothstep(tau / 0.17),
                Art::Slur => x = prev_slide + (pl.slide_m - prev_slide) * smoothstep(tau / 0.035),
                Art::Tongue => {}
            }
            if note.fx & SCOOP != 0 {
                x += semi * (1.0 - smoothstep(tau / 0.12));
            }
            if note.fx & FALL != 0 {
                let into = (tau - (sounding - fall_s)) / (fall_s + release_s);
                if into > 0.0 {
                    x += 3.5 * semi * into.clamp(0.0, 1.0).powi(2);
                }
            }
            let base_x = x.clamp(0.0, table.max_extension());
            if note.fx & VIB != 0 {
                let onset = 0.3 * sounding.min(1.2);
                let depth = 0.20 * semi * smoothstep((tau - onset) / 0.5);
                x += depth * (std::f64::consts::TAU * vib_rate * tau).sin();
            }
            let mut lip_hz = lip_of(pl.partial, base_x, note.vel);
            if note.art == Art::Slur {
                let k = smoothstep(tau / 0.025);
                lip_hz = (prev_lip.ln() + (lip_hz.ln() - prev_lip.ln()) * k).exp();
            }
            if note.fx & RIP != 0 {
                let low = lip_of(pl.partial.saturating_sub(4).max(2), base_x, note.vel);
                let k = (tau / 0.15).clamp(0.0, 1.0);
                lip_hz = (low.ln() + (lip_hz.ln() - low.ln()) * k).exp();
            }
            let mut shape = 1.0;
            if note.fx & SWELL != 0 {
                shape *= 0.75 + 0.4 * smoothstep(tau / sounding);
            }
            if note.fx & DIM != 0 {
                shape *= 1.0 - 0.45 * smoothstep(tau / sounding);
            }
            let target = e.mouth * shape;
            let level = match note.art {
                Art::Tongue => target * (1.0 - (-tau / 0.006).exp()),
                Art::Slur => {
                    if tau < 0.010 {
                        p_from * (0.5 + 0.5 * (std::f64::consts::PI * tau / 0.010).cos())
                    } else {
                        target * (1.0 - (-(tau - 0.010) / 0.006).exp())
                    }
                }
                Art::Smear => p_from + (target - p_from) * smoothstep(tau / 0.03),
            };
            let release = if tau > sounding { (-(tau - sounding) / release_s).exp() } else { 1.0 };
            let mouth = (level * release).clamp(0.0, 1.0);
            last_mouth = mouth;
            out[t] = Breath { mouth, lip_hz, embouchure: EMBOUCHURE, slide_m: x.clamp(0.0, table.max_extension()) };
        }
    }
    out
}
