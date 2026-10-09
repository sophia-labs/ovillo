//! The simulated guitarist. It plays one of a repertoire of compositions on a
//! 16th-note grid, humanized in timing and touch, or a [`Score`] someone sent
//! it.
//!
//! * The four original loops: `Strum` (Em C G D), `Arpeggio`, `Riff`, `Notes`.
//! * Five pieces that do not repeat ([`crate::compose`]): a chord `Wander`, a
//!   `Travis` fingerpicked tune, a 12-bar `Blues` shuffle, a sparse `Ambient`
//!   piece, and a slow `Melody`. Each draws on a seeded generator inside fixed
//!   tables, so nothing allocates on the audio thread.
//! * `Score`: a score compiled on the UI thread ([`crate::score`]) and handed
//!   over boxed. A one-shot score ends by returning to the composition that
//!   was playing before it (or silence, if the score says `"then": "stop"`).
//!
//! Strums roll across the strings (down from the low E, up from the high e);
//! events are scheduled into a fixed queue, so nothing allocates per sample.

use crate::compose::{self, Ambient, Blues, Chord, Melody, Travis, Wander, X};
use crate::guitar::{Guitar, Rng, STRINGS};
use crate::score::{Score, Then};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Pattern {
    /// A generative chord wander: a Markov progression over real voicings,
    /// strummed, picked, palm-muted or let ring by section, with swells and
    /// the odd key change.
    Wander,
    /// Travis picking: an alternating thumb under syncopated fingers, through
    /// folk progressions in C and G.
    Travis,
    /// A 12-bar shuffle in E or A: boogie comping, stop-time choruses, licks
    /// and turnarounds.
    Blues,
    /// Open drones, high-fret plucks, slow color chords and long silences:
    /// room for Ariadne to ring.
    Ambient,
    /// A slow single-line melody in phrases, over a soft bass.
    Melody,
    /// Open chords, Em–C–G–D, a down–down–up–up–down–up strum.
    Strum,
    /// Fingerpicked eighths over Am–F–C–G, let ring.
    Arpeggio,
    /// Palm-muted E riff; every fourth bar a bent lick.
    Riff,
    /// One long note a bar with vibrato: the open strings, then a few fretted.
    Notes,
    /// The score most recently sent (`play_score`).
    Score,
    /// The guitarist stops; whatever rings, rings out.
    Off,
}

impl Pattern {
    /// Everything the guitarist can play, in display order.
    pub const ALL: [Pattern; 11] = [
        Pattern::Wander,
        Pattern::Travis,
        Pattern::Blues,
        Pattern::Ambient,
        Pattern::Melody,
        Pattern::Strum,
        Pattern::Arpeggio,
        Pattern::Riff,
        Pattern::Notes,
        Pattern::Score,
        Pattern::Off,
    ];

    /// What the PATTERN softkey cycles through. A score is started by sending
    /// one, not by the cycle.
    pub const CYCLE: [Pattern; 10] = [
        Pattern::Wander,
        Pattern::Travis,
        Pattern::Blues,
        Pattern::Ambient,
        Pattern::Melody,
        Pattern::Strum,
        Pattern::Arpeggio,
        Pattern::Riff,
        Pattern::Notes,
        Pattern::Off,
    ];

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Pattern::Wander => "WANDER",
            Pattern::Travis => "TRAVIS",
            Pattern::Blues => "BLUES",
            Pattern::Ambient => "AMBIENT",
            Pattern::Melody => "MELODY",
            Pattern::Strum => "STRUM",
            Pattern::Arpeggio => "ARPEGGIO",
            Pattern::Riff => "RIFF",
            Pattern::Notes => "NOTES",
            Pattern::Score => "SCORE",
            Pattern::Off => "OFF",
        }
    }

    /// The name used in JSON (`"wander"`, `"travis"`, …).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Pattern::Wander => "wander",
            Pattern::Travis => "travis",
            Pattern::Blues => "blues",
            Pattern::Ambient => "ambient",
            Pattern::Melody => "melody",
            Pattern::Strum => "strum",
            Pattern::Arpeggio => "arpeggio",
            Pattern::Riff => "riff",
            Pattern::Notes => "notes",
            Pattern::Score => "score",
            Pattern::Off => "off",
        }
    }

    #[must_use]
    pub fn from_id(s: &str) -> Option<Pattern> {
        Pattern::ALL.into_iter().find(|p| p.id().eq_ignore_ascii_case(s) || p.label().eq_ignore_ascii_case(s))
    }

    /// One line about it.
    #[must_use]
    pub fn about(self) -> &'static str {
        match self {
            Pattern::Wander => "generative chord wander in G/D/A/E/C: Markov progression, varied strums, palm-muted and broken sections, swells, bends, key changes; never repeats",
            Pattern::Travis => "Travis fingerpicking in C and G: alternating (often palm-muted) thumb, syncopated fingers, folk progressions, melodic variations; never repeats",
            Pattern::Blues => "12-bar shuffle in E or A: boogie comping, stop-time and solo choruses, licks with bends, three turnarounds, quick-change; never repeats",
            Pattern::Ambient => "sparse ambient in E/A dorian or aeolian: open drones, high-fret plucks, slow color chords, long silences; best for hearing Ariadne ring",
            Pattern::Melody => "slow single-line melody in A or D minor, phrased antecedent/consequent over a soft bass, with vibrato and the odd bend; never repeats",
            Pattern::Strum => "the original loop: Em C G D, down-down-up-up-down-up",
            Pattern::Arpeggio => "the original loop: fingerpicked eighths over Am F C G",
            Pattern::Riff => "the original loop: palm-muted E riff, a bent lick every fourth bar",
            Pattern::Notes => "the original loop: one long note a bar with vibrato",
            Pattern::Score => "the score most recently sent with play_score",
            Pattern::Off => "the guitarist stops; whatever rings, rings out",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Pluck { string: u8, fret: u8, velocity: f32, palm: bool },
    Mute { string: u8 },
    Bend { string: u8, semitones: f32 },
    Vibrato { string: u8, depth: f32 },
}

impl Event {
    pub fn apply(self, g: &mut Guitar) {
        match self {
            Event::Pluck { string, fret, velocity, palm } => {
                g.pluck(string as usize, fret as f64, velocity as f64, palm)
            }
            Event::Mute { string } => g.mute(string as usize),
            Event::Bend { string, semitones } => g.bend(string as usize, semitones as f64),
            Event::Vibrato { string, depth } => g.vibrato(string as usize, depth as f64),
        }
    }

    #[must_use]
    pub fn string(self) -> u8 {
        match self {
            Event::Pluck { string, .. }
            | Event::Mute { string }
            | Event::Bend { string, .. }
            | Event::Vibrato { string, .. } => string,
        }
    }
}

const QUEUE: usize = 256;

/// Events waiting for their sample, in fixed slots.
pub struct Queue {
    slots: [Option<(u64, Event)>; QUEUE],
    pending: usize,
    next_due: u64,
    /// Events dropped because every slot was full.
    pub overflow: u64,
}

impl Queue {
    fn new() -> Self {
        Self { slots: [None; QUEUE], pending: 0, next_due: u64::MAX, overflow: 0 }
    }

    fn push(&mut self, t: u64, e: Event) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.is_none()) {
            *slot = Some((t, e));
            self.pending += 1;
            self.next_due = self.next_due.min(t);
        } else {
            self.overflow += 1;
        }
    }

    /// Apply, in slot order, everything due at or before `now`.
    #[inline]
    fn due(&mut self, now: u64, mut f: impl FnMut(Event)) {
        if self.pending == 0 || now < self.next_due {
            return;
        }
        let mut next = u64::MAX;
        for slot in &mut self.slots {
            if let Some((t, e)) = *slot {
                if t <= now {
                    f(e);
                    *slot = None;
                    self.pending -= 1;
                } else {
                    next = next.min(t);
                }
            }
        }
        self.next_due = next;
    }
}

/// How a strum crosses the strings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    /// Down from the low E, or up from the high e.
    pub down: bool,
    pub v: f64,
    /// Seconds between successive strings.
    pub per_string: f64,
    /// The strings the pick crosses, inclusive.
    pub lo: usize,
    pub hi: usize,
    pub palm: bool,
    /// Damp the shape's unplayed (×) strings in range, as a fretting hand does.
    pub damp: bool,
}

impl Stroke {
    #[must_use]
    pub const fn down(v: f64, per_string: f64) -> Self {
        Self { down: true, v, per_string, lo: 0, hi: STRINGS - 1, palm: false, damp: true }
    }
    #[must_use]
    pub const fn up(v: f64, per_string: f64, lo: usize) -> Self {
        Self { down: false, v, per_string, lo, hi: STRINGS - 1, palm: false, damp: true }
    }
}

/// The guitarist's hands for one step: schedules events relative to now.
pub struct Hand<'a> {
    q: &'a mut Queue,
    pub rng: &'a mut Rng,
    now: u64,
    sr: f64,
    /// Seconds per sixteenth at the current tempo.
    pub sixteenth: f64,
    humanize: bool,
}

impl Hand<'_> {
    /// Schedule `e` about `delay_s` from now (humanized by a few ms).
    pub fn at(&mut self, delay_s: f64, e: Event) {
        let jitter = if self.humanize { (self.rng.bipolar() + self.rng.bipolar()) * 0.004 } else { 0.0 };
        self.exact(delay_s + jitter, e);
    }

    /// Schedule `e` exactly `delay_s` from now.
    pub fn exact(&mut self, delay_s: f64, e: Event) {
        let t = self.now + (delay_s.max(0.0) * self.sr) as u64;
        self.q.push(t, e);
    }

    pub fn unit(&mut self) -> f64 {
        self.rng.unit()
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.rng.unit() < p
    }

    /// 0..n.
    pub fn pick(&mut self, n: usize) -> usize {
        ((self.rng.unit() * n as f64) as usize).min(n.max(1) - 1)
    }

    pub fn between(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.rng.unit()
    }

    /// An index drawn with the given weights.
    pub fn weighted(&mut self, w: &[f64]) -> usize {
        let total: f64 = w.iter().sum();
        let mut x = self.rng.unit() * total;
        for (i, wi) in w.iter().enumerate() {
            if x < *wi {
                return i;
            }
            x -= wi;
        }
        w.len() - 1
    }

    /// A velocity near `v`.
    pub fn touch(&mut self, v: f64) -> f32 {
        let j = if self.humanize { 0.07 * self.rng.bipolar() } else { 0.0 };
        (v + j).clamp(0.05, 1.0) as f32
    }

    pub fn note(&mut self, delay_s: f64, string: usize, fret: u8, v: f64, palm: bool) {
        let velocity = self.touch(v);
        self.at(delay_s, Event::Pluck { string: string as u8, fret: fret.min(compose::MAX_FRET), velocity, palm });
    }

    pub fn mute(&mut self, delay_s: f64, string: usize) {
        self.exact(delay_s, Event::Mute { string: string as u8 });
    }

    pub fn bend(&mut self, delay_s: f64, string: usize, semitones: f64) {
        self.exact(delay_s, Event::Bend { string: string as u8, semitones: semitones as f32 });
    }

    pub fn vibrato(&mut self, delay_s: f64, string: usize, depth: f64) {
        self.exact(delay_s, Event::Vibrato { string: string as u8, depth: depth as f32 });
    }

    /// Strum `chord` starting `delay_s` from now.
    pub fn strum(&mut self, chord: &Chord, delay_s: f64, k: Stroke) {
        let (lo, hi) = (k.lo.min(STRINGS - 1), k.hi.min(STRINGS - 1));
        let mut n = 0.0;
        for i in 0..=(hi.saturating_sub(lo)) {
            let s = if k.down { lo + i } else { hi - i };
            let fret = chord[s];
            if fret < 0 {
                if k.damp {
                    self.mute(delay_s, s);
                }
                continue;
            }
            let v = self.touch(k.v);
            self.at(
                delay_s + n * k.per_string,
                Event::Pluck { string: s as u8, fret: (fret as u8).min(compose::MAX_FRET), velocity: v, palm: k.palm },
            );
            n += 1.0;
        }
    }
}

pub struct Player {
    sr: f64,
    bpm: f64,
    pattern: Pattern,
    step: u64,
    to_next: f64,
    now: u64,
    queue: Queue,
    rng: Rng,
    wander: Wander,
    travis: Travis,
    blues: Blues,
    ambient: Ambient,
    melody: Melody,
    score: Option<Box<Score>>,
    score_cursor: usize,
    score_beat: f64,
    score_loops: u32,
    score_ends: u32,
    before_score: Pattern,
}

impl Player {
    #[must_use]
    pub fn new(sr: f64) -> Self {
        Self {
            sr,
            bpm: 92.0,
            pattern: Pattern::Wander,
            step: 0,
            to_next: 0.0,
            now: 0,
            queue: Queue::new(),
            rng: Rng::new(0x51ed_2701),
            wander: Wander::new(),
            travis: Travis::new(),
            blues: Blues::new(),
            ambient: Ambient::new(),
            melody: Melody::new(),
            score: None,
            score_cursor: 0,
            score_beat: 0.0,
            score_loops: 0,
            score_ends: 0,
            before_score: Pattern::Off,
        }
    }

    /// Reseed the guitarist (the app seeds from the clock, so each session
    /// opens differently; renders keep the fixed seed).
    pub fn seed(&mut self, seed: u32) {
        self.rng = Rng::new(seed);
    }

    #[must_use]
    pub fn pattern(&self) -> Pattern {
        self.pattern
    }

    #[must_use]
    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    /// Where the score is (beats), how long it is, how many times it has
    /// looped, and how many one-shot scores have ended.
    #[must_use]
    pub fn score_position(&self) -> (f64, f64, u32, u32) {
        let len = self.score.as_ref().map_or(0.0, |s| s.length_beats);
        (self.score_beat, len, self.score_loops, self.score_ends)
    }

    /// Events dropped because the queue was full (should stay 0).
    #[must_use]
    pub fn overflow(&self) -> u64 {
        self.queue.overflow
    }

    /// The key the current piece is in (empty for the fixed loops).
    #[must_use]
    pub fn key_name(&self) -> &'static str {
        compose::key_label(&self.wander, &self.travis, &self.blues, &self.ambient, &self.melody, self.pattern)
    }

    /// Start a composition from its first bar. Selecting `Score` with no score
    /// loaded does nothing.
    pub fn set_pattern(&mut self, p: Pattern) {
        if p == self.pattern || (p == Pattern::Score && self.score.is_none()) {
            return;
        }
        if p == Pattern::Score {
            self.before_score = self.pattern;
        }
        self.begin(p);
    }

    fn begin(&mut self, p: Pattern) {
        self.pattern = p;
        self.step = 0;
        self.to_next = 0.0;
        match p {
            Pattern::Wander => self.wander.start(),
            Pattern::Travis => self.travis.start(),
            Pattern::Blues => self.blues.start(),
            Pattern::Ambient => self.ambient.start(),
            Pattern::Melody => self.melody.start(),
            Pattern::Score => {
                self.score_cursor = 0;
                self.score_beat = 0.0;
                self.score_loops = 0;
            }
            _ => {}
        }
    }

    /// Play `score` now. Returns the score it replaces, which the caller must
    /// send back to be freed off the audio thread.
    #[must_use]
    pub fn load_score(&mut self, score: Box<Score>) -> Option<Box<Score>> {
        if self.pattern != Pattern::Score {
            self.before_score = self.pattern;
        }
        self.set_bpm(score.tempo);
        let old = self.score.replace(score);
        self.begin(Pattern::Score);
        old
    }

    pub fn set_bpm(&mut self, bpm: f64) {
        self.bpm = bpm.clamp(40.0, 220.0);
    }

    fn sixteenth(&self) -> f64 {
        self.sr * 60.0 / self.bpm / 4.0
    }

    fn hand(&mut self, humanize: bool) -> Hand<'_> {
        let sixteenth = 60.0 / self.bpm / 4.0;
        Hand { q: &mut self.queue, rng: &mut self.rng, now: self.now, sr: self.sr, sixteenth, humanize }
    }

    /// Schedule what happens on 16th `step` of the current pattern.
    fn on_step(&mut self, step: u64) {
        let sixteenth = 60.0 / self.bpm / 4.0;
        let mut h = Hand { q: &mut self.queue, rng: &mut self.rng, now: self.now, sr: self.sr, sixteenth, humanize: true };
        match self.pattern {
            Pattern::Wander => self.wander.step(step, &mut h),
            Pattern::Travis => self.travis.step(step, &mut h),
            Pattern::Blues => self.blues.step(step, &mut h),
            Pattern::Ambient => self.ambient.step(step, &mut h),
            Pattern::Melody => self.melody.step(step, &mut h),
            Pattern::Strum => strum_loop(step, &mut h),
            Pattern::Arpeggio => arpeggio_loop(step, &mut h),
            Pattern::Riff => riff_loop(step, &mut h),
            Pattern::Notes => notes_loop(step, &mut h),
            Pattern::Score | Pattern::Off => {}
        }
    }

    /// Queue an event now, from outside the pattern (a key press).
    pub fn push(&mut self, e: Event) {
        self.queue.push(self.now, e);
    }

    /// Queue an event `delay_s` from now, exactly.
    pub fn push_after(&mut self, e: Event, delay_s: f64) {
        self.hand(false).exact(delay_s, e);
    }

    /// Strum a shape now, from outside the pattern.
    pub fn strum_now(&mut self, chord: &Chord, down: bool, velocity: f64, spread_s: f64) {
        let n = chord.iter().filter(|f| **f > X).count().max(2) as f64;
        let per = spread_s.max(0.0) / (n - 1.0);
        let k = if down { Stroke::down(velocity, per) } else { Stroke { lo: 0, ..Stroke::up(velocity, per, 0) } };
        self.hand(false).strum(chord, 0.0, k);
    }

    /// Advance one sample, applying whatever is due to `g`.
    #[inline]
    pub fn tick(&mut self, g: &mut Guitar) {
        self.tick_with(g, |_, _| {});
    }

    /// As [`Self::tick`], and report each event as it is applied, with its
    /// sample index: the ground truth of what the guitarist did.
    #[inline]
    pub fn tick_with(&mut self, g: &mut Guitar, mut applied: impl FnMut(u64, Event)) {
        if self.pattern == Pattern::Score {
            self.tick_score(g, &mut applied);
        } else {
            if self.to_next <= 0.0 {
                let step = self.step;
                self.on_step(step);
                self.step += 1;
                self.to_next += self.sixteenth();
            }
            self.to_next -= 1.0;
        }
        let now = self.now;
        self.queue.due(now, |e| {
            e.apply(g);
            applied(now, e);
        });
        self.now += 1;
    }

    fn tick_score(&mut self, g: &mut Guitar, applied: &mut impl FnMut(u64, Event)) {
        let Some(sc) = self.score.as_deref() else {
            self.begin(Pattern::Off);
            return;
        };
        let beats_per_s = self.bpm / 60.0;
        while let Some(t) = sc.events.get(self.score_cursor) {
            let i = self.score_cursor as u32;
            let mut jitter = 0.0;
            let mut e = t.event;
            if sc.humanize > 0.0 {
                let h = sc.humanize as f64;
                jitter = h * 0.012 * hash01(self.score_loops, i) * beats_per_s;
                if let Event::Pluck { velocity, .. } = &mut e {
                    let dv = h * 0.08 * (2.0 * hash01(self.score_loops ^ 0x5bd1_e995, i) - 1.0);
                    *velocity = (*velocity as f64 + dv).clamp(0.02, 1.0) as f32;
                }
            }
            if t.beat + jitter > self.score_beat {
                break;
            }
            e.apply(g);
            applied(self.now, e);
            self.score_cursor += 1;
        }
        self.score_beat += beats_per_s / self.sr;
        let (len, looped, then, done) =
            (sc.length_beats, sc.looped, sc.then, self.score_cursor >= sc.events.len());
        if done && self.score_beat >= len {
            if looped {
                self.score_beat -= len;
                self.score_cursor = 0;
                self.score_loops += 1;
            } else {
                self.score_ends += 1;
                let next = match then {
                    Then::Resume => self.before_score,
                    Then::Stop => Pattern::Off,
                };
                self.begin(if next == Pattern::Score { Pattern::Off } else { next });
            }
        }
    }
}

/// A well-mixed hash of two integers into [0, 1).
fn hash01(a: u32, b: u32) -> f64 {
    let mut x = (u64::from(a) << 32 | u64::from(b)).wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^= x >> 31;
    (x >> 11) as f64 / (1u64 << 53) as f64
}

// ---------------------------------------------------------------------------
// The original four loops
// ---------------------------------------------------------------------------

const EM: Chord = [0, 2, 2, 0, 0, 0];
const C: Chord = [X, 3, 2, 0, 1, 0];
const G: Chord = [3, 2, 0, 0, 0, 3];
const D: Chord = [X, X, 0, 2, 3, 2];
const AM: Chord = [X, 0, 2, 2, 1, 0];
const F: Chord = [1, 3, 3, 2, 1, 1];

fn strum_loop(step: u64, h: &mut Hand) {
    let bar = step / 16;
    let s = step % 16;
    let chord = [EM, C, G, D][(bar % 4) as usize];
    let hit = match s {
        0 => Some((true, 0.95)),
        4 => Some((true, 0.8)),
        6 => Some((false, 0.7)),
        10 => Some((false, 0.7)),
        12 => Some((true, 0.85)),
        14 => Some((false, 0.65)),
        _ => None,
    };
    if let Some((down, v)) = hit {
        let per = 0.006 + 0.004 * h.unit();
        // Upstrokes catch the top four strings.
        let k = if down { Stroke::down(v, per) } else { Stroke::up(v * 0.75, per, 2) };
        h.strum(&chord, 0.0, k);
    }
}

fn arpeggio_loop(step: u64, h: &mut Hand) {
    let bar = step / 16;
    let s = (step % 16) as u8;
    let chord = [AM, F, C, G][(bar % 4) as usize];
    if s % 2 == 0 {
        let root = (0..STRINGS).find(|&i| chord[i] >= 0).unwrap_or(0);
        let walk = [root, 2, 3, 4, 5, 4, 3, 2];
        let string = walk[(s / 2) as usize].max(root);
        if s == 0 {
            for (i, f) in chord.iter().enumerate() {
                if *f < 0 {
                    h.mute(0.0, i);
                }
            }
        }
        let v = if s == 0 { 0.75 } else { 0.5 };
        h.note(0.0, string, chord[string].max(0) as u8, v, false);
    }
}

fn riff_loop(step: u64, h: &mut Hand) {
    let bar = step / 16;
    let s = (step % 16) as u8;
    let sx = h.sixteenth;
    if bar % 4 == 3 {
        // The lick: G string bent up a whole step, held with vibrato, then down E minor pentatonic.
        match s {
            0 => {
                h.note(0.0, 3, 7, 0.85, false);
                h.at(sx * 0.5, Event::Bend { string: 3, semitones: 2.0 });
                h.at(sx * 2.0, Event::Vibrato { string: 3, depth: 0.22 });
            }
            6 => h.note(0.0, 4, 8, 0.8, false),
            8 => h.note(0.0, 4, 5, 0.75, false),
            10 => h.note(0.0, 3, 7, 0.75, false),
            12 => {
                h.note(0.0, 2, 7, 0.8, false);
                h.at(sx, Event::Vibrato { string: 2, depth: 0.18 });
            }
            15 => {
                for i in 1..STRINGS {
                    h.mute(0.0, i);
                }
            }
            _ => {}
        }
    } else {
        match s {
            0 | 2 | 3 | 6 | 7 => h.note(0.0, 0, 0, if s == 0 { 0.95 } else { 0.72 }, true),
            4 => h.note(0.0, 0, 3, 0.85, false),
            8 => h.note(0.0, 0, 5, 0.85, false),
            10 => h.note(0.0, 0, 6, 0.8, false),
            11 => {
                h.note(0.0, 0, 7, 0.88, false);
                h.at(sx * 1.5, Event::Vibrato { string: 0, depth: 0.15 });
            }
            14 => h.note(0.0, 1, 5, 0.8, false),
            15 => h.note(0.0, 1, 7, 0.8, false),
            _ => {}
        }
    }
}

fn notes_loop(step: u64, h: &mut Hand) {
    let bar = step / 16;
    if step % 16 == 0 {
        const SEQ: [(usize, u8); 8] = [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0), (2, 2), (0, 7)];
        let (string, fret) = SEQ[(bar % 8) as usize];
        for i in 0..STRINGS {
            if i != string {
                h.mute(0.0, i);
            }
        }
        h.note(0.0, string, fret, 0.8, false);
        h.at(0.45, Event::Vibrato { string: string as u8, depth: 0.2 });
    }
}
