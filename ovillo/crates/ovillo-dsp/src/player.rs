//! The simulated guitarist: a few patterns on a 16th-note grid, humanized in
//! timing and touch. Strums roll across the strings (down from the low E, up
//! from the high e); held notes grow vibrato; the riff palm-mutes its chugs
//! and every fourth bar bends a lick.
//!
//! Events are scheduled into a fixed queue, so nothing allocates per sample.

use crate::guitar::{Guitar, Rng, STRINGS};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Pattern {
    /// Open chords, Em–C–G–D, a down–down–up–up–down–up strum.
    Strum,
    /// Fingerpicked eighths over Am–F–C–G, let ring.
    Arpeggio,
    /// Palm-muted E riff; every fourth bar a bent lick.
    Riff,
    /// One long note a bar with vibrato: the open strings, then a few fretted.
    Notes,
    /// The guitarist stops; whatever rings, rings out.
    Off,
}

impl Pattern {
    pub const ALL: [Pattern; 5] = [Pattern::Strum, Pattern::Arpeggio, Pattern::Riff, Pattern::Notes, Pattern::Off];

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Pattern::Strum => "STRUM",
            Pattern::Arpeggio => "ARPEGGIO",
            Pattern::Riff => "RIFF",
            Pattern::Notes => "NOTES",
            Pattern::Off => "OFF",
        }
    }
}

/// Frets low E to high e; `X` is a string not played (and damped).
const X: i8 = -1;
type Chord = [i8; STRINGS];
const EM: Chord = [0, 2, 2, 0, 0, 0];
const C: Chord = [X, 3, 2, 0, 1, 0];
const G: Chord = [3, 2, 0, 0, 0, 3];
const D: Chord = [X, X, 0, 2, 3, 2];
const AM: Chord = [X, 0, 2, 2, 1, 0];
const F: Chord = [1, 3, 3, 2, 1, 1];

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
}

const QUEUE: usize = 128;

pub struct Player {
    sr: f64,
    bpm: f64,
    pattern: Pattern,
    step: u64,
    to_next: f64,
    now: u64,
    queue: [Option<(u64, Event)>; QUEUE],
    pending: usize,
    rng: Rng,
}

impl Player {
    #[must_use]
    pub fn new(sr: f64) -> Self {
        Self {
            sr,
            bpm: 92.0,
            pattern: Pattern::Strum,
            step: 0,
            to_next: 0.0,
            now: 0,
            queue: [None; QUEUE],
            pending: 0,
            rng: Rng::new(0x51ed_2701),
        }
    }

    #[must_use]
    pub fn pattern(&self) -> Pattern {
        self.pattern
    }

    #[must_use]
    pub fn bpm(&self) -> f64 {
        self.bpm
    }

    /// Change pattern at the next bar line.
    pub fn set_pattern(&mut self, p: Pattern) {
        if p != self.pattern {
            self.pattern = p;
            // Start the new pattern on its first bar.
            self.step = 0;
            self.to_next = 0.0;
        }
    }

    pub fn set_bpm(&mut self, bpm: f64) {
        self.bpm = bpm.clamp(40.0, 220.0);
    }

    fn sixteenth(&self) -> f64 {
        self.sr * 60.0 / self.bpm / 4.0
    }

    fn at(&mut self, delay_s: f64, e: Event) {
        let jitter = (self.rng.bipolar() + self.rng.bipolar()) * 0.004;
        let t = self.now + ((delay_s + jitter).max(0.0) * self.sr) as u64;
        if let Some(slot) = self.queue.iter_mut().find(|s| s.is_none()) {
            *slot = Some((t, e));
            self.pending += 1;
        }
    }

    fn touch(&mut self, v: f64) -> f32 {
        (v + 0.07 * self.rng.bipolar()).clamp(0.05, 1.0) as f32
    }

    fn strum(&mut self, chord: Chord, down: bool, v: f64) {
        let spread = 0.006 + 0.004 * self.rng.unit();
        let order: [usize; STRINGS] = if down { [0, 1, 2, 3, 4, 5] } else { [5, 4, 3, 2, 1, 0] };
        let mut k = 0.0;
        for &s in &order {
            // Upstrokes catch the top four strings.
            if !down && s < 2 {
                continue;
            }
            let fret = chord[s];
            if fret < 0 {
                self.at(0.0, Event::Mute { string: s as u8 });
                continue;
            }
            let vel = self.touch(v * if down { 1.0 } else { 0.75 });
            self.at(k * spread, Event::Pluck { string: s as u8, fret: fret as u8, velocity: vel, palm: false });
            k += 1.0;
        }
    }

    fn note(&mut self, string: usize, fret: u8, v: f64, palm: bool) {
        let velocity = self.touch(v);
        self.at(0.0, Event::Pluck { string: string as u8, fret, velocity, palm });
    }

    /// Schedule what happens on 16th `step` of the current pattern.
    fn on_step(&mut self, step: u64) {
        let bar = step / 16;
        let s = (step % 16) as u8;
        let sixteenth_s = self.sixteenth() / self.sr;
        match self.pattern {
            Pattern::Strum => {
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
                    self.strum(chord, down, v);
                }
            }
            Pattern::Arpeggio => {
                let chord = [AM, F, C, G][(bar % 4) as usize];
                if s % 2 == 0 {
                    let root = (0..STRINGS).find(|&i| chord[i] >= 0).unwrap_or(0);
                    let walk = [root, 2, 3, 4, 5, 4, 3, 2];
                    let string = walk[(s / 2) as usize].max(root);
                    if s == 0 {
                        for (i, f) in chord.iter().enumerate() {
                            if *f < 0 {
                                self.at(0.0, Event::Mute { string: i as u8 });
                            }
                        }
                    }
                    let v = if s == 0 { 0.75 } else { 0.5 };
                    self.note(string, chord[string].max(0) as u8, v, false);
                }
            }
            Pattern::Riff => {
                if bar % 4 == 3 {
                    // The lick: G string bent up a whole step, held with vibrato, then down E minor pentatonic.
                    match s {
                        0 => {
                            self.note(3, 7, 0.85, false);
                            self.at(sixteenth_s * 0.5, Event::Bend { string: 3, semitones: 2.0 });
                            self.at(sixteenth_s * 2.0, Event::Vibrato { string: 3, depth: 0.22 });
                        }
                        6 => self.note(4, 8, 0.8, false),
                        8 => self.note(4, 5, 0.75, false),
                        10 => self.note(3, 7, 0.75, false),
                        12 => {
                            self.note(2, 7, 0.8, false);
                            self.at(sixteenth_s, Event::Vibrato { string: 2, depth: 0.18 });
                        }
                        15 => {
                            for i in 1..STRINGS {
                                self.at(0.0, Event::Mute { string: i as u8 });
                            }
                        }
                        _ => {}
                    }
                } else {
                    match s {
                        0 | 2 | 3 | 6 | 7 => self.note(0, 0, if s == 0 { 0.95 } else { 0.72 }, true),
                        4 => self.note(0, 3, 0.85, false),
                        8 => self.note(0, 5, 0.85, false),
                        10 => self.note(0, 6, 0.8, false),
                        11 => {
                            self.note(0, 7, 0.88, false);
                            self.at(sixteenth_s * 1.5, Event::Vibrato { string: 0, depth: 0.15 });
                        }
                        14 => self.note(1, 5, 0.8, false),
                        15 => self.note(1, 7, 0.8, false),
                        _ => {}
                    }
                }
            }
            Pattern::Notes => {
                if s == 0 {
                    const SEQ: [(usize, u8); 8] = [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0), (2, 2), (0, 7)];
                    let (string, fret) = SEQ[(bar % 8) as usize];
                    for i in 0..STRINGS {
                        if i != string {
                            self.at(0.0, Event::Mute { string: i as u8 });
                        }
                    }
                    self.note(string, fret, 0.8, false);
                    self.at(0.45, Event::Vibrato { string: string as u8, depth: 0.2 });
                }
            }
            Pattern::Off => {}
        }
    }

    /// Queue an event now, from outside the pattern (a key press).
    pub fn push(&mut self, e: Event) {
        if let Some(slot) = self.queue.iter_mut().find(|s| s.is_none()) {
            *slot = Some((self.now, e));
            self.pending += 1;
        }
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
        if self.to_next <= 0.0 {
            let step = self.step;
            self.on_step(step);
            self.step += 1;
            self.to_next += self.sixteenth();
        }
        self.to_next -= 1.0;
        if self.pending > 0 {
            for slot in &mut self.queue {
                if let Some((t, e)) = *slot {
                    if t <= self.now {
                        e.apply(g);
                        applied(self.now, e);
                        *slot = None;
                        self.pending -= 1;
                    }
                }
            }
        }
        self.now += 1;
    }
}
