//! The trombone v1 (`TROMBONE-WORLD.md`): one N-D body (a 124-volume bore,
//! the slide as its coordinate, the lips as the runtime's flow valve, the
//! lungs as its reservoir), `ovillo_dsp::trombone_nd::TromboneNd`.
//!
//! Three ways to touch it, all the World's own controls in the end (mouth
//! pressure, lip tension, embouchure, slide):
//!
//! - **direct**: set `mouth`, `lip`, `embouchure`, `slide` yourself (a
//!   creature's breath, a horn in a game);
//! - **the player**: `note` events, placed on a partial and slide position
//!   and blown by the calibrated player (`trombone_nd_player`: the skill
//!   learned by playing this World), as the solo is, one note at a time;
//! - **a score**: the film driver's note list (`trombone_nd_score.rs` TSV),
//!   streamed through the same placement and breath as `perform` computes it,
//!   hairpins included (`tests/portable.rs` checks it sample for sample).

use ovillo_dsp::trombone_nd::{Breath, TromboneNd, OUTPUT_SCALE};
use ovillo_dsp::trombone_nd_player::{place, Embouchure, ModeTable, Placement, Skill, EMBOUCHURE};
use ovillo_dsp::trombone_player::{hz_of, Art, Note, DIM, FALL, RIP, SCOOP, SWELL, VIB};
use serde_json::{json, Value};

use crate::catalog::*;
use crate::{CreateCtx, Source, MAX_ARGS};

const SR: f64 = 48_000.0;
const ARTS: [&str; 3] = ["tongue", "slur", "smear"];

pub fn kind() -> KindDef {
    let table = ModeTable::embedded().ok();
    let positions = table.as_ref().map(|t| t.positions_m.clone()).unwrap_or_default();
    let max_ext = table.as_ref().map_or(0.72, ModeTable::max_extension);
    let flag = |name: &str, about: &str| arg(name, "0/1", 0.0, 1.0, 0.0, about);
    KindDef {
        id: "trombone".into(),
        title: "Trombone v1".into(),
        family: "trombone".into(),
        summary: "One N-dimensional body: a 124-volume tube-network bore with the slide as its coordinate, the lips as the runtime's flow valve, the lungs as its reservoir. Blown by mouth pressure; the lips' tension chooses the partial.".into(),
        spec: "TROMBONE-WORLD.md".into(),
        sample_rate: Some(SR),
        channels: 1,
        anatomies: vec![anatomy("trombone-v1", "tenor trombone bore, 124 volumes", "profiles/ariadne-trombone-v1.plan.json", true)],
        open_anatomies: false,
        options: vec![
            option("score", "string", Value::Null, "a note list, tab separated: start_s end_s midi vel vel_end art fx (the film driver's, trombone_nd_score.rs); placed and blown by the player"),
            option("tailSeconds", "number", json!(2.5), "offline: silence rendered after a score's last note (the driver's TAIL_S)"),
        ],
        events: vec![
            event("note", "The player plays a note: places it on a partial and slide position the calibration heard speak, moves the slide and lips, and blows the pressure that gives the dynamic on that cell. Articulation and effects as the solo's (tonguing is pressure, slurs move the lips under a legato tongue, smears/scoops/falls/vibrato are slide transport).", vec![
                arg("pitch", "MIDI", 34.0, 77.0, 58.0, "the note (the calibration covers partials 2-10 in seven positions)"),
                arg("velocity", "0..1", 0.0, 1.0, 0.5, "the written dynamic: a target level the player blows for on that cell"),
                arg_enum("articulation", &ARTS, 0, "tongue, slur (lips move under a legato tongue), smear (the slide carries the pitch)"),
                arg("duration", "s", 0.0, 60.0, 0.0, "0: held until release; otherwise released at 90 % of it, as written notes are"),
                flag("vibrato", "slide vibrato after 30 % of the note"),
                flag("scoop", "slide from a semitone below"),
                flag("fall", "slide down 3.5 semitones at the end (needs a duration)"),
                flag("swell", "pressure rises over the note"),
                flag("dim", "pressure falls over the note"),
                flag("rip", "lip-tension ramp up through the partials below"),
            ]),
            event("release", "The tongue stops the air (8 ms; 120 ms after a fall).", vec![]),
        ],
        controls: vec![
            control("mouth", "0..1", 0.0, 1.0, 0.0, 120.0, "the World's valve supply, set every sample; ramped by the scheduler", "direct: mouth pressure as a fraction of the lips' declared maximum. Setting a direct control takes the instrument from the player"),
            control("lip", "Hz", 20.0, 1200.0, 240.0, 120.0, "a retune is booked as control work when it changes", "direct: lip tension, the valve's tuning; it chooses which bore resonance locks"),
            control("embouchure", "0..1", 0.0, 1.0, EMBOUCHURE, 30.0, "as lip", "direct: embouchure firmness (the valve's adduction); the player uses 0.1"),
            control("slide", "m", 0.0, max_ext, 0.0, 120.0, "the World moves the slide every 16 samples (mass-momentum transport, billed)", &format!("direct: each slide leg's extension; positions 1-7 at {}", positions.iter().map(|p| format!("{:.3}", p)).collect::<Vec<_>>().join(", "))),
            control("expression", "x", 0.0, 1.5, 1.0, 60.0, "scales the player's target pressure", "the player: blow harder or softer than the written dynamic (1 is as written)"),
        ],
        telemetry: vec![
            slot("mouth", "0..1", "the mouth pressure this sample"),
            slot("lipHz", "Hz", "the lips' tuning"),
            slot("slideM", "m", "the slide's extension"),
            slot("partial", "index", "the partial the player chose (0: direct)"),
            slot("position", "1..7", "the nearest slide position"),
            slot("airJ", "J", "the bore's air: the body's field energy"),
            slot("lipsJ", "J", "the lips: material energy less the field"),
            slot("lungsJ", "J", "the reservoir"),
            slot("noteMidi", "MIDI", "the note the player is on (0: none)"),
        ],
        score: Some(ScoreDef { format: "trombone-notes-tsv".into(), about: "start_s end_s midi vel vel_end art fx, tab separated, # comments (trombone_nd_score.rs). vel_end != vel is a hairpin: the player's mouth pressure moved between the two calibrated pressures of the cell.".into() }),
        output: OutputDef {
            observes: "the bell volume's pressure (Pa): the World's own emission".into(),
            gain: OUTPUT_SCALE,
            channels: "one point (left = right)".into(),
            about: "OUTPUT_SCALE (1/300) is set once, never per render; the raw emission can exceed 1 on an attack (the film's audition peaked at +2.6 dBFS)".into(),
        },
        lookahead_seconds: 0.0,
        cost: "one body of 24 modes recomputed every 16 samples while the slide moves: 1.9x real time playing the film's audition, measured in node (wasm, 128-frame quanta, tools/rtf.mjs) on a 2019 8-core i9 laptop (natively 2.44x for the solo, TROMBONE-WORLD T5)".into(),
        does_not_model: lines(&[
            "No brightening with loudness: the bore is linear acoustics (real brass brightens at ff through wave steepening in the bore).",
            "The grid (124 volumes, dx 7-43 mm) flattens the upper modes; series junctions join through the downstream half-column only.",
            "A rubber slide (all slide sections stretch), not a telescope.",
            "The lips are copied glottis mechanics with fitted mass, thickness and delay; the aerodynamic work on the lips is booked to the lungs.",
            "Partial 3 has the narrowest windows; a cell's dynamic range is a few calibration grid points (a swell spans about 6 dB).",
            "No room: the World is heard dry.",
        ]),
        does_not_model_ref: "TROMBONE-WORLD.md#limits".into(),
        build,
    }
}

fn smoothstep(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// `trombone_nd_player::place`'s choice for one note after `prev`.
pub fn place_one(table: &ModeTable, skill: &Skill, note: &Note, prev: Option<Placement>) -> Placement {
    let f = hz_of(note.midi);
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
    best.map_or(Placement { partial: 4, slide_m: 0.0 }, |b| b.1)
}

/// One note as `perform` blows it, with what it needs precomputed.
#[derive(Debug, Clone, Copy)]
struct Blown {
    note: Note,
    pl: Placement,
    e: Embouchure,
    s0: u64,
    owned_until: u64,
    sounding: f64,
    release_s: f64,
    fall_s: f64,
    prev_slide: f64,
    prev_lip: f64,
    vib_rate: f64,
    semi: f64,
    /// (m0, m1, sounding) of a hairpin (`trombone_nd_score.rs`).
    hairpin: Option<(f64, f64, f64)>,
}

/// The player: notes (from a score or live), the breath they make.
struct Player {
    table: ModeTable,
    skill: Skill,
    notes: Vec<Blown>,
    /// The note sounding now (index into `notes`).
    current: Option<usize>,
    last_mouth: f64,
    p_from: f64,
    count: usize,
    /// The breath before the first note.
    rest: Breath,
}

impl Player {
    fn emb(&self, n: usize, x: f64, vel: f64) -> Embouchure {
        self.skill
            .embouchure_near(&self.table, n, x, vel)
            .unwrap_or(Embouchure { ratio: 0.5, offset_cents: 0.0, mouth: 0.0, window: 0.0 })
    }

    fn lip_of(&self, n: usize, x: f64, vel: f64) -> f64 {
        self.emb(n, x, vel).ratio * self.table.mode_hz(n, x)
    }

    /// `perform`'s precomputation for note `i` of a known list.
    fn blow_list(&self, notes: &[Note], placed: &[Placement], vel_end: &[f64], offset: u64, len: u64) -> Vec<Blown> {
        let mut out = Vec::with_capacity(notes.len());
        for (i, note) in notes.iter().enumerate() {
            let pl = placed[i];
            let e = self.emb(pl.partial, pl.slide_m, note.vel);
            let prev = if i > 0 { Some((notes[i - 1], placed[i - 1])) } else { None };
            let next_legato = i + 1 < notes.len() && notes[i + 1].art != Art::Tongue && (notes[i + 1].start_s - note.end_s).abs() < 0.03;
            let s0 = (note.start_s * SR) as u64;
            let owned_until = if i + 1 < notes.len() { (notes[i + 1].start_s * SR) as u64 } else { len };
            let sounding = if next_legato { owned_until as f64 / SR - note.start_s } else { (note.end_s - note.start_s) * 0.90 };
            let release_s = if note.fx & FALL != 0 { 0.12 } else { 0.008 };
            let fall_s = if note.fx & FALL != 0 { (0.45 * sounding).min(0.6) } else { 0.0 };
            let (prev_slide, prev_lip) = prev.map_or((pl.slide_m, self.lip_of(pl.partial, pl.slide_m, note.vel)), |(pn, q)| (q.slide_m, self.lip_of(q.partial, q.slide_m, pn.vel)));
            let to = vel_end.get(i).copied().unwrap_or(note.vel);
            let hairpin = ((to - note.vel).abs() >= 1e-9).then(|| (e.mouth, self.emb(pl.partial, pl.slide_m, to).mouth, (note.end_s - note.start_s) * 0.90));
            out.push(Blown {
                note: *note,
                pl,
                e,
                s0: offset + s0,
                owned_until: if owned_until == len { u64::MAX } else { offset + owned_until },
                sounding,
                release_s,
                fall_s,
                prev_slide,
                prev_lip,
                vib_rate: 5.0 + 0.4 * ((i * 37 % 11) as f64 / 11.0),
                semi: self.table.shift(pl.partial, pl.slide_m, 1.0),
                hairpin,
            });
        }
        out
    }

    /// The breath at World sample `t` (`perform`'s inner loop; hairpins after,
    /// as the film driver applies them). `t` must not go backwards.
    fn breath(&mut self, t: u64, offset: u64, expression: f64) -> Breath {
        // Advance to the note that owns `t`.
        loop {
            let next = self.current.map_or(0, |c| c + 1);
            match self.notes.get(next) {
                Some(n) if n.s0 <= t && n.s0 < n.owned_until.max(n.s0 + 1) => {
                    if n.s0 == n.owned_until {
                        // A note with no samples: perform's loop never runs for it.
                        self.current = Some(next);
                        continue;
                    }
                    self.current = Some(next);
                    self.p_from = if n.note.art == Art::Tongue { 0.0 } else { self.last_mouth };
                }
                _ => break,
            }
        }
        let Some(c) = self.current else { return self.rest };
        let b = self.notes[c];
        if t >= b.owned_until {
            return self.rest_after(&b);
        }
        let note = b.note;
        let pl = b.pl;
        let tau = (t - offset) as f64 / SR - (note.start_s);
        let sounding = b.sounding;
        let mut x = pl.slide_m;
        match note.art {
            Art::Smear => x = b.prev_slide + (pl.slide_m - b.prev_slide) * smoothstep(tau / 0.17),
            Art::Slur => x = b.prev_slide + (pl.slide_m - b.prev_slide) * smoothstep(tau / 0.035),
            Art::Tongue => {}
        }
        if note.fx & SCOOP != 0 {
            x += b.semi * (1.0 - smoothstep(tau / 0.12));
        }
        if note.fx & FALL != 0 {
            let into = (tau - (sounding - b.fall_s)) / (b.fall_s + b.release_s);
            if into > 0.0 {
                x += 3.5 * b.semi * into.clamp(0.0, 1.0).powi(2);
            }
        }
        let base_x = x.clamp(0.0, self.table.max_extension());
        if note.fx & VIB != 0 {
            let onset = 0.3 * sounding.min(1.2);
            let depth = 0.20 * b.semi * smoothstep((tau - onset) / 0.5);
            x += depth * (std::f64::consts::TAU * b.vib_rate * tau).sin();
        }
        let mut lip_hz = self.lip_of(pl.partial, base_x, note.vel);
        if note.art == Art::Slur {
            let k = smoothstep(tau / 0.025);
            lip_hz = (b.prev_lip.ln() + (lip_hz.ln() - b.prev_lip.ln()) * k).exp();
        }
        if note.fx & RIP != 0 {
            let low = self.lip_of(pl.partial.saturating_sub(4).max(2), base_x, note.vel);
            let k = (tau / 0.15).clamp(0.0, 1.0);
            lip_hz = (low.ln() + (lip_hz.ln() - low.ln()) * k).exp();
        }
        let span = if sounding.is_finite() { sounding } else { 1.0 };
        let mut shape = 1.0;
        if note.fx & SWELL != 0 {
            shape *= 0.75 + 0.4 * smoothstep(tau / span);
        }
        if note.fx & DIM != 0 {
            shape *= 1.0 - 0.45 * smoothstep(tau / span);
        }
        let target = if expression == 1.0 { b.e.mouth * shape } else { b.e.mouth * shape * expression };
        let p_from = self.p_from;
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
        let release = if tau > sounding { (-(tau - sounding) / b.release_s).exp() } else { 1.0 };
        let mouth = (level * release).clamp(0.0, 1.0);
        self.last_mouth = mouth;
        let mut out = mouth;
        if let Some((m0, m1, s)) = b.hairpin {
            let k = smoothstep(tau / s);
            let factor = (m0.ln() + k * (m1.ln() - m0.ln())).exp() / m0;
            out = (mouth * factor).clamp(0.0, 1.0);
        }
        Breath { mouth: out, lip_hz, embouchure: EMBOUCHURE, slide_m: x.clamp(0.0, self.table.max_extension()) }
    }

    /// After a note's owned span with nothing next (cannot happen in a list; a live tail).
    fn rest_after(&self, b: &Blown) -> Breath {
        Breath { mouth: 0.0, lip_hz: self.lip_of(b.pl.partial, b.pl.slide_m, b.note.vel), embouchure: EMBOUCHURE, slide_m: b.pl.slide_m }
    }
}

struct TromboneSource {
    tw: TromboneNd,
    player: Player,
    direct: bool,
    breath: Breath,
    expression: f64,
    /// The instance frame of the World's sample 0 (the World's clock is `k`).
    k: u64,
    /// A score's frame offset (its 0 s).
    offset: u64,
    pending_release: Option<u64>,
}

fn build(_def: &KindDef, _ctx: &CreateCtx) -> Result<Box<dyn Source>, String> {
    let table = ModeTable::embedded()?;
    let skill = Skill::embedded()?;
    let tw = TromboneNd::new()?;
    let slide = tw.slide();
    let rest = Breath { mouth: 0.0, lip_hz: 240.0, embouchure: EMBOUCHURE, slide_m: slide };
    Ok(Box::new(TromboneSource {
        tw,
        player: Player { table, skill, notes: Vec::new(), current: None, last_mouth: 0.0, p_from: 0.0, count: 0, rest },
        direct: false,
        breath: rest,
        expression: 1.0,
        k: 0,
        offset: 0,
        pending_release: None,
    }))
}

/// The film driver's note list.
pub fn parse_notes(text: &str) -> Result<(Vec<Note>, Vec<f64>), String> {
    let mut notes = Vec::new();
    let mut ends = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("start_s") {
            continue;
        }
        let f: Vec<&str> = line.split('\t').map(str::trim).collect();
        if f.len() < 7 {
            return Err(format!("line {}: expected 7 tab-separated fields, found {}", i + 1, f.len()));
        }
        let num = |k: usize| f[k].parse::<f64>().map_err(|_| format!("line {}: field {} is not a number: {:?}", i + 1, k + 1, f[k]));
        let art = match f[5] {
            "tongue" => Art::Tongue,
            "slur" => Art::Slur,
            "smear" => Art::Smear,
            other => return Err(format!("line {}: art {other:?} is not tongue|slur|smear", i + 1)),
        };
        let mut fx = 0u8;
        if f[6] != "-" {
            for name in f[6].split(',') {
                fx |= match name.trim() {
                    "scoop" => SCOOP,
                    "fall" => FALL,
                    "vib" => VIB,
                    "rip" => RIP,
                    "swell" => SWELL,
                    "dim" => DIM,
                    other => return Err(format!("line {}: fx {other:?} is not scoop|fall|vib|rip|swell|dim", i + 1)),
                };
            }
        }
        let (start_s, end_s, midi, vel, vel_end) = (num(0)?, num(1)?, num(2)?, num(3)?, num(4)?);
        if !(end_s > start_s) || !(0.0..=1.0).contains(&vel) || !(0.0..=1.0).contains(&vel_end) {
            return Err(format!("line {}: need end_s > start_s and velocities in 0..1", i + 1));
        }
        if notes.last().is_some_and(|p: &Note| start_s < p.start_s) {
            return Err(format!("line {}: notes must be in time order (one player, one bore)", i + 1));
        }
        notes.push(Note { midi, start_s, end_s, vel, art, fx });
        ends.push(vel_end);
    }
    if notes.is_empty() {
        return Err("no notes".into());
    }
    Ok((notes, ends))
}

impl Source for TromboneSource {
    fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> Result<(), String> {
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            if self.pending_release.is_some_and(|f| f <= self.k) {
                self.pending_release = None;
                self.release_now();
            }
            let b = if self.direct { self.breath } else { self.player.breath(self.k, self.offset, self.expression) };
            self.breath = b;
            let v = self.tw.tick(b).map_err(|e| e.to_string())?;
            *l = v as f32;
            *r = v as f32;
            self.k += 1;
        }
        Ok(())
    }

    fn event(&mut self, index: usize, a: &[f64; MAX_ARGS], at: u64, _now: u64) -> Result<(), String> {
        match index {
            0 => {
                let art = match a[2].round() as usize {
                    1 => Art::Slur,
                    2 => Art::Smear,
                    _ => Art::Tongue,
                };
                let mut fx = 0u8;
                for (k, bit) in [VIB, SCOOP, FALL, SWELL, DIM, RIP].iter().enumerate() {
                    if a[4 + k] >= 0.5 {
                        fx |= bit;
                    }
                }
                let start_s = (at - self.offset) as f64 / SR;
                let duration = a[3].max(0.0);
                let end_s = if duration > 0.0 { start_s + duration } else { f64::INFINITY };
                let note = Note { midi: a[0], start_s, end_s, vel: a[1].clamp(0.0, 1.0), art, fx };
                self.play_live(note, at);
                if duration > 0.0 {
                    self.pending_release = None;
                }
                Ok(())
            }
            1 => {
                if at <= self.k {
                    self.release_now();
                } else {
                    self.pending_release = Some(at);
                }
                Ok(())
            }
            _ => Err(format!("trombone: no event {index}")),
        }
    }

    fn set(&mut self, index: usize, value: f64) -> Result<(), String> {
        if index == 4 {
            self.expression = value;
            return Ok(());
        }
        if !self.direct {
            self.direct = true;
            self.player.notes.clear();
            self.player.current = None;
        }
        match index {
            0 => self.breath.mouth = value,
            1 => self.breath.lip_hz = value,
            2 => self.breath.embouchure = value,
            3 => self.breath.slide_m = value,
            _ => return Err(format!("trombone: no control {index}")),
        }
        Ok(())
    }

    fn ledger(&self) -> Ledger {
        Ledger::of(self.tw.world())
    }

    fn receipt(&self) -> Value {
        let m = self.tw.moves();
        json!({
            "anatomy": "trombone-v1",
            "planDigest": self.tw.world().plan_digest(),
            "notes": self.player.count,
            "moves": m,
            "outputScale": OUTPUT_SCALE,
            "simulatedSeconds": self.k as f64 / SR,
            "storedEnergy": {"air": self.tw.stored_energy().air, "lips": self.tw.stored_energy().lips, "lungs": self.tw.stored_energy().lungs},
        })
    }

    fn telemetry(&self, out: &mut [f64]) {
        let b = self.breath;
        out[0] = b.mouth;
        out[1] = b.lip_hz;
        out[2] = self.tw.slide();
        let note = self.player.current.and_then(|c| self.player.notes.get(c));
        out[3] = if self.direct { 0.0 } else { note.map_or(0.0, |n| n.pl.partial as f64) };
        let p = &self.player.table.positions_m;
        out[4] = p.iter().enumerate().min_by(|x, y| (x.1 - self.tw.slide()).abs().total_cmp(&(y.1 - self.tw.slide()).abs())).map_or(1.0, |(i, _)| (i + 1) as f64);
        let e = self.tw.stored_energy();
        out[5] = e.air;
        out[6] = e.lips;
        out[7] = e.lungs;
        out[8] = if self.direct { 0.0 } else { note.map_or(0.0, |n| n.note.midi) };
    }

    fn reset(&mut self) -> Result<(), String> {
        self.tw = TromboneNd::new()?;
        self.player.notes.clear();
        self.player.current = None;
        self.player.last_mouth = 0.0;
        self.player.count = 0;
        self.direct = false;
        self.breath = self.player.rest;
        self.expression = 1.0;
        self.k = 0;
        self.offset = 0;
        self.pending_release = None;
        Ok(())
    }

    fn play_score(&mut self, text: &str, at: u64) -> Result<Value, String> {
        let (notes, vel_end) = parse_notes(text)?;
        let placed = place(&self.player.table, &self.player.skill, &notes);
        self.offset = at;
        self.direct = false;
        let blown = self.player.blow_list(&notes, &placed, &vel_end, at, u64::MAX);
        // The film driver sets the slide to the first note's place before its first sample.
        if self.k == 0 && at == 0 {
            self.tw.set_slide_now(placed[0].slide_m).map_err(|e| e.to_string())?;
        }
        let first_lip = self.player.lip_of(placed[0].partial, placed[0].slide_m, notes[0].vel);
        self.player.rest = Breath { mouth: 0.0, lip_hz: first_lip, embouchure: EMBOUCHURE, slide_m: placed[0].slide_m };
        self.player.notes = blown;
        self.player.current = None;
        self.player.count += notes.len();
        let end = notes.last().map_or(0.0, |n| n.end_s);
        Ok(json!({
            "format": "trombone-notes-tsv",
            "notes": notes.len(),
            "hairpins": vel_end.iter().zip(&notes).filter(|(e, n)| (**e - n.vel).abs() >= 1e-9).count(),
            "samples": (end * SR) as u64,
            "placed": placed.iter().map(|p| json!({"partial": p.partial, "slideM": p.slide_m, "position": p.position(&self.player.table)})).collect::<Vec<_>>(),
            "startFrame": at,
        }))
    }

    fn plan_digest(&self) -> String {
        self.tw.world().plan_digest().to_owned()
    }
}

impl TromboneSource {
    fn play_live(&mut self, note: Note, at: u64) {
        let prev = self.player.current.and_then(|c| self.player.notes.get(c)).copied();
        if self.direct {
            self.direct = false;
            self.player.notes.clear();
            self.player.current = None;
        }
        // A live note list keeps only the note sounding and the new one.
        if let Some(c) = self.player.current {
            if c > 0 {
                self.player.notes.drain(..c);
                self.player.current = Some(0);
            }
        }
        let pl = place_one(&self.player.table, &self.player.skill, &note, prev.map(|p| p.pl));
        let e = self.player.emb(pl.partial, pl.slide_m, note.vel);
        let (prev_slide, prev_lip) = match prev {
            Some(p) => (self.tw.slide(), self.player.lip_of(p.pl.partial, p.pl.slide_m, p.note.vel)),
            None => (pl.slide_m, self.player.lip_of(pl.partial, pl.slide_m, note.vel)),
        };
        let release_s = if note.fx & FALL != 0 { 0.12 } else { 0.008 };
        let sounding = if note.end_s.is_finite() { (note.end_s - note.start_s) * 0.90 } else { f64::INFINITY };
        let fall_s = if note.fx & FALL != 0 && sounding.is_finite() { (0.45 * sounding).min(0.6) } else { 0.0 };
        let i = self.player.count;
        // The note before is owned until this one starts.
        if let Some(c) = self.player.current {
            if let Some(p) = self.player.notes.get_mut(c) {
                p.owned_until = at.max(p.s0);
            }
        }
        let s0 = at.max(self.k);
        let b = Blown {
            note,
            pl,
            e,
            s0,
            owned_until: u64::MAX,
            sounding,
            release_s,
            fall_s,
            prev_slide,
            prev_lip,
            vib_rate: 5.0 + 0.4 * ((i * 37 % 11) as f64 / 11.0),
            semi: self.player.table.shift(pl.partial, pl.slide_m, 1.0),
            hairpin: None,
        };
        self.player.notes.push(b);
        self.player.count += 1;
    }

    /// The tongue stops the air now (a fall, if asked, is taken from here).
    fn release_now(&mut self) {
        let k = self.k;
        let offset = self.offset;
        if let Some(c) = self.player.current {
            if let Some(n) = self.player.notes.get_mut(c) {
                let tau = (k - offset) as f64 / SR - n.note.start_s;
                if n.note.fx & FALL != 0 {
                    n.fall_s = (0.45 * tau.max(0.0)).min(0.6);
                    n.sounding = tau + n.fall_s;
                } else {
                    n.sounding = n.sounding.min(tau.max(0.0));
                }
            }
        }
    }
}
