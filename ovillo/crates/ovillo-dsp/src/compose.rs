//! The guitarist's repertoire: five pieces that do not repeat.
//!
//! Each is a small state machine stepped on the 16th-note grid. The music
//! lives in fixed tables (real voicings, bass patterns, licks, strum
//! patterns, progressions); a seeded generator walks them, so a piece never
//! comes round the same way twice and nothing allocates on the audio thread.
//!
//! * [`Wander`]: a Markov chord progression in G, D, A, E or C over open and
//!   barre voicings with colors (sus, add9, maj7, 7ths). Sections of 4–16 bars
//!   each take a feel (open strumming, half time, palm-muted drive, broken
//!   chords, sparse), an intensity and a strum pattern that mutates bar to
//!   bar (dropped hits, ghost upstrokes, chucks); phrases swell; cadences go
//!   to V; now and then a section pivots through the new key's V7 into a
//!   neighbouring key. Bends and top-string melody notes come and go.
//! * [`Travis`]: alternating-bass fingerpicking in C and G. The thumb keeps
//!   root–fifth (often palm-muted), the fingers syncopate over it from six
//!   templates, and the top string borrows scale tones for a melody; verses
//!   of four four-bar phrases end on a cadence.
//! * [`Blues`]: a 12-bar shuffle in E or A with swing that breathes. Choruses
//!   are boogie (5–6 or 5–6–♭7–6), stop-time, or solo; licks with curls and
//!   bends from two positions; quick-change and bar-10 choices; three
//!   turnarounds.
//! * [`Ambient`]: E or A, dorian or aeolian, in tides of 8–16 bars. Open
//!   drones, slowly rolled color chords, high-fret plucks with late vibrato,
//!   pairs a fourth apart, rare slow bends, and bars of silence.
//! * [`Melody`]: a single line in A or D minor, phrased as antecedent and
//!   consequent (the motif's rhythm and contour come back in the third bar),
//!   ending on the fifth and then the tonic, over a soft bass.
//!
//! Every voicing here is checked by the tests below: playable span, in key,
//! and named correctly by its pitch classes.

use crate::guitar::STRINGS;
use crate::player::{Hand, Stroke};

/// Frets low E to high e; [`X`] is a string not played.
pub type Chord = [i8; STRINGS];
pub const X: i8 = -1;
/// MIDI note of each open string, low E to high e.
pub const OPEN_MIDI: [i32; STRINGS] = [40, 45, 50, 55, 59, 64];
/// The highest fret anything may play.
pub const MAX_FRET: u8 = 22;
const MAJOR: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
const MINOR: [i32; 7] = [0, 2, 3, 5, 7, 8, 10];

/// Named shapes, for scores and the `strum` tool.
pub const SHAPES: &[(&str, Chord)] = &[
    ("C", [X, 3, 2, 0, 1, 0]),
    ("Cmaj7", [X, 3, 2, 0, 0, 0]),
    ("Cadd9", [X, 3, 2, 0, 3, 0]),
    ("C7", [X, 3, 2, 3, 1, 0]),
    ("C5", [X, 3, 5, 5, X, X]),
    ("D", [X, X, 0, 2, 3, 2]),
    ("Dm", [X, X, 0, 2, 3, 1]),
    ("D7", [X, X, 0, 2, 1, 2]),
    ("Dmaj7", [X, X, 0, 2, 2, 2]),
    ("Dm7", [X, X, 0, 2, 1, 1]),
    ("Dsus2", [X, X, 0, 2, 3, 0]),
    ("Dsus4", [X, X, 0, 2, 3, 3]),
    ("D5", [X, X, 0, 2, 3, X]),
    ("E", [0, 2, 2, 1, 0, 0]),
    ("Em", [0, 2, 2, 0, 0, 0]),
    ("E7", [0, 2, 0, 1, 0, 0]),
    ("Em7", [0, 2, 2, 0, 3, 0]),
    ("Esus4", [0, 2, 2, 2, 0, 0]),
    ("Em9", [0, 2, 0, 0, 0, 2]),
    ("Eadd9", [0, 2, 2, 1, 0, 2]),
    ("E5", [0, 2, 2, X, X, X]),
    ("F", [1, 3, 3, 2, 1, 1]),
    ("Fmaj7", [X, X, 3, 2, 1, 0]),
    ("Fmaj7#11", [X, X, 3, 2, 0, 0]),
    ("F5", [1, 3, 3, X, X, X]),
    ("F#m", [2, 4, 4, 2, 2, 2]),
    ("F#m7", [2, X, 2, 2, 2, X]),
    ("F#m11", [2, X, 2, 2, 0, 0]),
    ("G", [3, 2, 0, 0, 0, 3]),
    ("G7", [3, 2, 0, 0, 0, 1]),
    ("Gmaj7", [3, X, 0, 0, 0, 2]),
    ("G6", [3, 2, 0, 0, 0, 0]),
    ("Gsus4", [3, X, 0, 0, 1, 3]),
    ("G/B", [X, 2, 0, 0, 0, 3]),
    ("G5", [3, 5, 5, X, X, X]),
    ("G#m", [4, 6, 6, 4, 4, 4]),
    ("G#m7", [4, X, 4, 4, 4, X]),
    ("A", [X, 0, 2, 2, 2, 0]),
    ("Am", [X, 0, 2, 2, 1, 0]),
    ("A7", [X, 0, 2, 0, 2, 0]),
    ("Am7", [X, 0, 2, 0, 1, 0]),
    ("Amaj7", [X, 0, 2, 1, 2, 0]),
    ("Asus2", [X, 0, 2, 2, 0, 0]),
    ("Asus4", [X, 0, 2, 2, 3, 0]),
    ("A6/9", [X, 0, 2, 4, 2, 2]),
    ("Am(add9)", [X, 0, 2, 4, 1, 0]),
    ("A5", [X, 0, 2, 2, X, X]),
    ("Bb", [X, 1, 3, 3, 3, 1]),
    ("B", [X, 2, 4, 4, 4, 2]),
    ("Bm", [X, 2, 4, 4, 3, 2]),
    ("B7", [X, 2, 1, 2, 0, 2]),
    ("Bm7", [X, 2, 0, 2, 0, 2]),
    ("Bsus4", [X, 2, 4, 4, 0, 0]),
    ("Bm(add11)", [X, 2, 4, 4, 3, 0]),
    ("B5", [X, 2, 4, 4, X, X]),
    ("C#m", [X, 4, 6, 6, 5, 4]),
    ("C#m7", [X, 4, 6, 4, 5, 4]),
];

/// A named shape (exact name first, then ignoring case; `♯`/`♭` accepted).
#[must_use]
pub fn chord_shape(name: &str) -> Option<Chord> {
    let n = name.trim().replace('♯', "#").replace('♭', "b");
    SHAPES
        .iter()
        .find(|(k, _)| *k == n)
        .or_else(|| SHAPES.iter().find(|(k, _)| k.eq_ignore_ascii_case(&n)))
        .map(|(_, c)| *c)
}

/// Pitch class of `string` at `fret`.
#[must_use]
pub fn pc(string: usize, fret: i32) -> i32 {
    (OPEN_MIDI[string] + fret).rem_euclid(12)
}

/// Which pitch classes a shape sounds.
#[must_use]
pub fn pitch_classes(c: &Chord) -> [bool; 12] {
    let mut p = [false; 12];
    for (s, f) in c.iter().enumerate() {
        if *f >= 0 {
            p[pc(s, *f as i32) as usize] = true;
        }
    }
    p
}

fn in_scale(tonic: i32, scale: &[i32; 7], p: i32) -> bool {
    scale.iter().any(|d| (tonic + d).rem_euclid(12) == p.rem_euclid(12))
}

fn lowest(c: &Chord) -> usize {
    (0..STRINGS).find(|&s| c[s] >= 0).unwrap_or(0)
}

fn sounding_above(c: &Chord, s: usize) -> Option<usize> {
    (s + 1..STRINGS).find(|&i| c[i] >= 0)
}

/// The sounding string nearest `s` (at or above first, then below).
fn sounding_near(c: &Chord, s: usize) -> usize {
    (s..STRINGS).find(|&i| c[i] >= 0).or_else(|| (0..s).rev().find(|&i| c[i] >= 0)).unwrap_or(s)
}

fn damp_unplayed(h: &mut Hand, c: &Chord) {
    for (s, f) in c.iter().enumerate() {
        if *f < 0 {
            h.mute(0.0, s);
        }
    }
}

// ---------------------------------------------------------------------------
// Wander
// ---------------------------------------------------------------------------

/// A key: its tonic and six diatonic chords (I ii iii IV V vi), each as a
/// plain voicing and two colors. V is always [V, Vsus4, V7].
struct KeyDef {
    name: &'static str,
    tonic: i32,
    deg: [[Chord; 3]; 6],
}

const KEYS: [KeyDef; 5] = [
    KeyDef {
        name: "G",
        tonic: 7,
        deg: [
            [[3, 2, 0, 0, 0, 3], [3, X, 0, 0, 0, 2], [3, 2, 0, 0, 0, 0]], // G Gmaj7 G6
            [[X, 0, 2, 2, 1, 0], [X, 0, 2, 0, 1, 0], [X, 0, 2, 4, 1, 0]], // Am Am7 Am(add9)
            [[X, 2, 4, 4, 3, 2], [X, 2, 0, 2, 0, 2], [X, 2, 4, 4, 3, 0]], // Bm Bm7 Bm(add11)
            [[X, 3, 2, 0, 1, 0], [X, 3, 2, 0, 3, 3], [X, 3, 2, 0, 0, 0]], // C Cadd9 Cmaj7
            [[X, X, 0, 2, 3, 2], [X, X, 0, 2, 3, 3], [X, X, 0, 2, 1, 2]], // D Dsus4 D7
            [[0, 2, 2, 0, 0, 0], [0, 2, 2, 0, 3, 0], [0, 2, 0, 0, 0, 2]], // Em Em7 Em9
        ],
    },
    KeyDef {
        name: "D",
        tonic: 2,
        deg: [
            [[X, X, 0, 2, 3, 2], [X, X, 0, 2, 3, 0], [X, X, 0, 2, 2, 2]], // D Dsus2 Dmaj7
            [[0, 2, 2, 0, 0, 0], [0, 2, 2, 0, 3, 0], [0, 2, 0, 0, 0, 2]], // Em Em7 Em9
            [[2, 4, 4, 2, 2, 2], [2, X, 2, 2, 2, X], [2, X, 2, 2, 0, 0]], // F#m F#m7 F#m11
            [[3, 2, 0, 0, 0, 3], [3, X, 0, 0, 0, 2], [X, 2, 0, 0, 0, 3]], // G Gmaj7 G/B
            [[X, 0, 2, 2, 2, 0], [X, 0, 2, 2, 3, 0], [X, 0, 2, 0, 2, 0]], // A Asus4 A7
            [[X, 2, 4, 4, 3, 2], [X, 2, 0, 2, 0, 2], [X, 2, 4, 4, 3, 0]], // Bm Bm7 Bm(add11)
        ],
    },
    KeyDef {
        name: "A",
        tonic: 9,
        deg: [
            [[X, 0, 2, 2, 2, 0], [X, 0, 2, 2, 0, 0], [X, 0, 2, 1, 2, 0]], // A Asus2 Amaj7
            [[X, 2, 4, 4, 3, 2], [X, 2, 0, 2, 0, 2], [X, 2, 4, 4, 3, 0]], // Bm Bm7 Bm(add11)
            [[X, 4, 6, 6, 5, 4], [X, 4, 6, 4, 5, 4], [X, 4, 2, 4, 0, 0]], // C#m C#m7 C#m7(open)
            [[X, X, 0, 2, 3, 2], [X, X, 0, 2, 3, 0], [X, X, 0, 2, 2, 2]], // D Dsus2 Dmaj7
            [[0, 2, 2, 1, 0, 0], [0, 2, 2, 2, 0, 0], [0, 2, 0, 1, 0, 0]], // E Esus4 E7
            [[2, 4, 4, 2, 2, 2], [2, X, 2, 2, 2, X], [2, X, 2, 2, 0, 0]], // F#m F#m7 F#m11
        ],
    },
    KeyDef {
        name: "E",
        tonic: 4,
        deg: [
            [[0, 2, 2, 1, 0, 0], [0, 2, 2, 2, 0, 0], [0, 2, 2, 1, 0, 2]], // E Esus4 Eadd9
            [[2, 4, 4, 2, 2, 2], [2, X, 2, 2, 2, X], [2, X, 2, 2, 0, 0]], // F#m F#m7 F#m11
            [[4, 6, 6, 4, 4, 4], [4, X, 4, 4, 4, X], [4, 6, 6, 4, 4, 4]], // G#m G#m7 G#m
            [[X, 0, 2, 2, 2, 0], [X, 0, 2, 2, 0, 0], [X, 0, 2, 1, 2, 0]], // A Asus2 Amaj7
            [[X, 2, 4, 4, 4, 2], [X, 2, 4, 4, 0, 0], [X, 2, 1, 2, 0, 2]], // B Bsus4 B7
            [[X, 4, 6, 6, 5, 4], [X, 4, 6, 4, 5, 4], [X, 4, 2, 4, 0, 0]], // C#m C#m7 C#m7(open)
        ],
    },
    KeyDef {
        name: "C",
        tonic: 0,
        deg: [
            [[X, 3, 2, 0, 1, 0], [X, 3, 2, 0, 0, 0], [X, 3, 2, 0, 3, 0]], // C Cmaj7 Cadd9
            [[X, X, 0, 2, 3, 1], [X, X, 0, 2, 1, 1], [X, 5, 7, 7, 6, 5]], // Dm Dm7 Dm(barre)
            [[0, 2, 2, 0, 0, 0], [0, 2, 2, 0, 3, 0], [0, 2, 0, 0, 0, 0]], // Em Em7 Em7(open)
            [[1, 3, 3, 2, 1, 1], [X, X, 3, 2, 1, 0], [X, X, 3, 2, 0, 0]], // F Fmaj7 Fmaj7#11
            [[3, 2, 0, 0, 0, 3], [3, X, 0, 0, 1, 3], [3, 2, 0, 0, 0, 1]], // G Gsus4 G7
            [[X, 0, 2, 2, 1, 0], [X, 0, 2, 0, 1, 0], [X, 0, 2, 4, 1, 0]], // Am Am7 Am(add9)
        ],
    },
];

/// Neighbouring keys a section may move to (a fifth either way, or a step).
const MODULATE: [[usize; 2]; 5] = [[1, 4], [2, 0], [3, 1], [2, 0], [0, 1]];

/// Degree to degree (I ii iii IV V vi): functional harmony, loosely.
const TRANS: [[f64; 6]; 6] = [
    [0.00, 0.14, 0.08, 0.32, 0.24, 0.22],
    [0.08, 0.00, 0.06, 0.20, 0.56, 0.10],
    [0.06, 0.08, 0.00, 0.36, 0.06, 0.44],
    [0.32, 0.14, 0.04, 0.00, 0.38, 0.12],
    [0.52, 0.06, 0.06, 0.14, 0.00, 0.22],
    [0.10, 0.28, 0.08, 0.36, 0.18, 0.00],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Feel {
    Open,
    Halftime,
    Drive,
    Broken,
    Sparse,
}

const FEELS: [Feel; 5] = [Feel::Open, Feel::Halftime, Feel::Drive, Feel::Broken, Feel::Sparse];
const FEEL_WEIGHT: [f64; 5] = [0.34, 0.16, 0.16, 0.18, 0.16];

// Strum codes on the 16th grid.
const NO: u8 = 0;
const DN: u8 = 1;
const UP: u8 = 2;
const GH: u8 = 3; // ghost: a light upstroke on the top two strings
const CK: u8 = 4; // chuck: a palm-damped strum, then the hand mutes everything
const PM: u8 = 5; // palm-muted bass
const AC: u8 = 6; // accented downstroke

const OPEN_PATS: [[u8; 16]; 6] = [
    [DN, NO, NO, NO, DN, NO, UP, NO, NO, NO, UP, NO, DN, NO, UP, NO],
    [DN, NO, NO, NO, DN, NO, UP, NO, DN, NO, UP, NO, DN, NO, UP, NO],
    [DN, NO, NO, UP, NO, UP, DN, NO, NO, UP, NO, UP, DN, NO, UP, NO],
    [DN, NO, NO, NO, NO, NO, UP, NO, DN, NO, NO, NO, DN, NO, UP, NO],
    [DN, NO, CK, NO, NO, UP, CK, UP, DN, NO, CK, NO, NO, UP, CK, UP],
    [DN, NO, NO, UP, NO, NO, DN, NO, NO, UP, NO, NO, DN, NO, UP, NO],
];
const HALF_PATS: [[u8; 16]; 3] = [
    [AC, NO, NO, NO, NO, NO, NO, NO, DN, NO, NO, NO, NO, NO, NO, NO],
    [AC, NO, NO, NO, NO, NO, NO, NO, DN, NO, NO, NO, NO, NO, UP, NO],
    [AC, NO, NO, NO, NO, NO, NO, NO, NO, NO, UP, NO, DN, NO, NO, NO],
];
const DRIVE_PATS: [[u8; 16]; 4] = [
    [AC, NO, PM, NO, PM, NO, PM, NO, PM, NO, PM, NO, PM, NO, PM, NO],
    [AC, NO, PM, NO, PM, NO, AC, NO, PM, NO, PM, NO, PM, NO, AC, NO],
    [AC, NO, PM, PM, PM, NO, PM, NO, AC, NO, PM, PM, PM, NO, PM, NO],
    [AC, NO, PM, NO, PM, NO, PM, NO, AC, NO, NO, NO, DN, NO, UP, NO],
];
const SPARSE_PAT: [u8; 16] = [DN, NO, NO, NO, NO, NO, NO, NO, NO, NO, NO, NO, NO, NO, NO, NO];
/// Broken-chord orders, one string per eighth: 0 is the bass, 2–5 the
/// nearest sounding string at or above that one.
const ORDERS: [[usize; 8]; 5] = [
    [0, 2, 3, 4, 5, 4, 3, 2],
    [0, 3, 4, 5, 3, 4, 5, 4],
    [0, 4, 3, 5, 3, 4, 2, 4],
    [0, 2, 4, 3, 5, 3, 4, 2],
    [0, 3, 5, 4, 2, 4, 5, 3],
];

pub struct Wander {
    key: usize,
    bar_key: usize,
    degree: usize,
    color: usize,
    /// The chord for the second half of the bar, when it changes at beat 3.
    split: Option<(usize, usize)>,
    /// From this step the bar's chord takes another color (16: never).
    swap_at: u8,
    swap_color: usize,
    bar: u32,
    section_len: u32,
    feel: Feel,
    intensity: f64,
    next_key: Option<usize>,
    hold: u32,
    base: usize,
    order: usize,
    pat: [u8; 16],
    v: f64,
    /// A bent note on beat 4: (string, target fret).
    lick: Option<(usize, u8)>,
    /// A top-string melody note: (step, string, fret).
    tune: Option<(u8, usize, u8)>,
}

impl Default for Wander {
    fn default() -> Self {
        Self::new()
    }
}

impl Wander {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            key: 0,
            bar_key: 0,
            degree: 0,
            color: 0,
            split: None,
            swap_at: 16,
            swap_color: 0,
            bar: 0,
            section_len: 0,
            feel: Feel::Open,
            intensity: 0.7,
            next_key: None,
            hold: 0,
            base: 0,
            order: 0,
            pat: [NO; 16],
            v: 0.7,
            lick: None,
            tune: None,
        }
    }

    pub fn start(&mut self) {
        self.bar = 0;
        self.section_len = 0;
        self.next_key = None;
    }

    /// The key it is in now.
    #[must_use]
    pub fn key_name(&self) -> &'static str {
        KEYS[self.key].name
    }

    fn chord_at(&self, s: u8) -> Chord {
        let (d, c) = match self.split {
            Some(sp) if s >= 8 => sp,
            _ => (self.degree, self.color),
        };
        let c = if s >= self.swap_at { self.swap_color } else { c };
        KEYS[self.bar_key].deg[d][c]
    }

    pub fn step(&mut self, step: u64, h: &mut Hand) {
        let s = (step % 16) as u8;
        if s == 0 {
            self.new_bar(h);
        }
        let chord = self.chord_at(s);
        if s == 0 || (s == 8 && self.split.is_some()) {
            damp_unplayed(h, &chord);
        }
        if self.feel == Feel::Broken {
            self.broken(s, &chord, h);
        } else {
            self.hit(self.pat[s as usize], s, &chord, h);
        }
        if let Some((at, string, fret)) = self.tune {
            if at == s {
                h.note(0.0, string, fret, self.v * 0.85, false);
                let d = h.between(0.3, 0.55);
                h.vibrato(d, string, 0.14);
            }
        }
        if s == 12 {
            if let Some((string, t)) = self.lick {
                let sx = h.sixteenth;
                h.note(0.0, string, t - 2, (self.v * 1.05).min(1.0), false);
                h.bend(sx * 0.7, string, 2.0);
                h.vibrato(sx * 2.4, string, 0.16);
            }
        }
    }

    fn hit(&mut self, code: u8, s: u8, chord: &Chord, h: &mut Hand) {
        let v = self.v;
        let accent = match s {
            0 => 1.0,
            8 => 0.93,
            4 | 12 => 0.86,
            _ => 0.78,
        };
        match code {
            DN | AC => {
                let per = 0.005 + 0.008 * (1.0 - self.intensity) + 0.003 * h.unit();
                let per = if self.feel == Feel::Sparse || self.feel == Feel::Halftime { per * 1.8 } else { per };
                let vv = v * accent * if code == AC { 1.08 } else { 1.0 };
                h.strum(chord, 0.0, Stroke::down(vv.min(1.0), per));
            }
            UP => {
                let lo = if h.chance(0.5) { 2 } else { 3 };
                let per = 0.005 + 0.003 * h.unit();
                h.strum(chord, 0.0, Stroke::up(v * 0.68, per, lo));
            }
            GH => h.strum(chord, 0.0, Stroke::up(v * 0.32, 0.004, 4)),
            CK => {
                let lo = lowest(chord);
                h.strum(chord, 0.0, Stroke { lo, palm: true, ..Stroke::down(0.22 + 0.1 * v, 0.003) });
                for st in 0..STRINGS {
                    h.mute(0.035, st);
                }
            }
            PM => {
                let lo = lowest(chord);
                h.note(0.0, lo, chord[lo].max(0) as u8, v * 0.85 * accent, true);
                if s % 4 == 0 || h.chance(0.4) {
                    if let Some(n) = sounding_above(chord, lo) {
                        h.note(0.0, n, chord[n].max(0) as u8, v * 0.7 * accent, true);
                    }
                }
            }
            _ => {}
        }
    }

    fn broken(&mut self, s: u8, chord: &Chord, h: &mut Hand) {
        if s % 2 == 1 {
            if h.chance(0.05) {
                let top = sounding_near(chord, 5);
                h.note(0.0, top, chord[top].max(0) as u8, self.v * 0.6, false);
            }
            return;
        }
        let code = ORDERS[self.order][(s / 2) as usize];
        let string = if code == 0 { lowest(chord) } else { sounding_near(chord, code) };
        let v = self.v * if code == 0 { 1.15 } else if s % 8 == 0 { 1.0 } else { 0.85 };
        h.note(0.0, string, chord[string].max(0) as u8, v.min(1.0), false);
    }

    fn new_section(&mut self, h: &mut Hand) {
        if let Some(k) = self.next_key.take() {
            self.key = k;
        }
        self.bar = 0;
        self.section_len = [4, 8, 8, 8, 12, 16][h.pick(6)];
        let prev = self.feel;
        let mut w = FEEL_WEIGHT;
        for (i, f) in FEELS.iter().enumerate() {
            if *f == prev {
                w[i] = 0.0;
            }
        }
        self.feel = FEELS[h.weighted(&w)];
        let (a, b) = match self.feel {
            Feel::Open => (0.62, 0.85),
            Feel::Halftime => (0.55, 0.75),
            Feel::Drive => (0.70, 0.92),
            Feel::Broken => (0.42, 0.60),
            Feel::Sparse => (0.32, 0.50),
        };
        self.intensity = h.between(a, b);
        self.base = h.pick(OPEN_PATS.len());
        self.order = h.pick(ORDERS.len());
        self.next_key = if h.chance(0.24) { Some(MODULATE[self.key][h.pick(2)]) } else { None };
    }

    fn next_degree(d: usize, h: &mut Hand) -> usize {
        h.weighted(&TRANS[d])
    }

    fn new_bar(&mut self, h: &mut Hand) {
        if self.bar >= self.section_len {
            self.new_section(h);
        }
        let first = self.bar == 0;
        let last = self.bar + 1 == self.section_len;
        self.bar_key = self.key;
        self.split = None;
        self.swap_at = 16;
        self.lick = None;
        self.tune = None;
        let pivot = last && self.next_key.is_some();
        if first {
            self.degree = if h.chance(0.72) { 0 } else { 5 };
            self.hold = 0;
        } else if pivot {
            // Through the new key's V7.
            self.bar_key = self.next_key.unwrap_or(self.key);
            self.degree = 4;
        } else if last {
            if self.degree != 4 && h.chance(0.4) && matches!(self.feel, Feel::Open | Feel::Drive | Feel::Broken) {
                if self.degree == 0 {
                    self.degree = 3;
                }
                self.split = Some((4, 0));
            } else {
                self.degree = 4;
            }
        } else if self.hold > 0 {
            self.hold -= 1;
        } else {
            self.degree = Self::next_degree(self.degree, h);
            self.hold = if matches!(self.feel, Feel::Halftime | Feel::Sparse) && h.chance(0.4) { 1 } else { 0 };
            if matches!(self.feel, Feel::Open | Feel::Drive) && h.chance(0.18) {
                let d2 = Self::next_degree(self.degree, h);
                self.split = Some((d2, 0));
            }
        }
        self.color = if pivot { 2 } else { h.weighted(&[0.6, 0.25, 0.15]) };
        if !last && self.split.is_none() && h.chance(0.2) {
            self.swap_at = 12;
            self.swap_color = (self.color + 1 + h.pick(2)) % 3;
        }
        let len = self.section_len.max(1) as f64;
        let swell = (std::f64::consts::PI * (self.bar as f64 + 0.5) / len).sin();
        let wob = 0.05 * h.rng.bipolar();
        self.v = (self.intensity * (0.84 + 0.2 * swell) * (1.0 + wob)).clamp(0.15, 1.0);

        // This bar's strum pattern: the section's, or a sibling, then mutated.
        self.pat = match self.feel {
            Feel::Open => {
                let i = if h.chance(0.6) { self.base } else { h.pick(OPEN_PATS.len()) };
                OPEN_PATS[i % OPEN_PATS.len()]
            }
            Feel::Halftime => HALF_PATS[h.pick(HALF_PATS.len())],
            Feel::Drive => DRIVE_PATS[if h.chance(0.6) { self.base % DRIVE_PATS.len() } else { h.pick(DRIVE_PATS.len()) }],
            Feel::Sparse => SPARSE_PAT,
            Feel::Broken => [NO; 16],
        };
        if self.feel == Feel::Open {
            for s in 1..16 {
                if s != 8 && self.pat[s] != NO && h.chance(0.12) {
                    self.pat[s] = NO;
                }
            }
            for s in [2usize, 6, 10, 14] {
                if self.pat[s] == NO && h.chance(0.1) {
                    self.pat[s] = GH;
                } else if self.pat[s] == UP && h.chance(0.08) {
                    self.pat[s] = GH;
                }
            }
        } else if self.feel == Feel::Sparse && h.chance(0.35) {
            self.pat[8] = UP;
        }

        // Embellishments.
        if matches!(self.feel, Feel::Open | Feel::Halftime | Feel::Sparse) && !first && !last && h.chance(0.08) {
            let chord = self.chord_at(12);
            let pcs = pitch_classes(&chord);
            let string = if h.chance(0.5) { 3 } else { 4 };
            let tonic = KEYS[self.bar_key].tonic;
            let mut cands = [0u8; 9];
            let mut n = 0;
            for t in 4..=12 {
                if pcs[pc(string, t) as usize] && in_scale(tonic, &MAJOR, pc(string, t - 2)) {
                    cands[n] = t as u8;
                    n += 1;
                }
            }
            if n > 0 {
                self.lick = Some((string, cands[h.pick(n)]));
                for s in 12..16 {
                    self.pat[s] = NO;
                }
            }
        }
        if matches!(self.feel, Feel::Sparse | Feel::Halftime) && h.chance(if self.feel == Feel::Sparse { 0.55 } else { 0.3 }) {
            let at = [10u8, 12, 14][h.pick(3)];
            let chord = self.chord_at(at);
            let pcs = pitch_classes(&chord);
            let top = chord[5].max(0) as i32;
            let mut cands = [0u8; 8];
            let mut n = 0;
            for f in top..=(top + 7).min(15) {
                if pcs[pc(5, f) as usize] && n < cands.len() {
                    cands[n] = f as u8;
                    n += 1;
                }
            }
            if n > 0 {
                self.tune = Some((at, 5, cands[h.pick(n)]));
                if self.pat[at as usize] != NO {
                    self.pat[at as usize] = NO;
                }
            }
        }
        self.bar += 1;
    }
}

// ---------------------------------------------------------------------------
// Travis
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct TChord {
    frets: Chord,
    /// The thumb's four quarter notes: (string, fret).
    bass: [(u8, u8); 4],
}

const fn tc(frets: Chord, bass: [(u8, u8); 4]) -> TChord {
    TChord { frets, bass }
}

const C_BASS: [(u8, u8); 4] = [(1, 3), (2, 2), (0, 3), (2, 2)];
const AM_BASS: [(u8, u8); 4] = [(1, 0), (2, 2), (0, 0), (2, 2)];
const G_BASS: [(u8, u8); 4] = [(0, 3), (2, 0), (1, 2), (2, 0)];
const EM_BASS: [(u8, u8); 4] = [(0, 0), (2, 2), (1, 2), (2, 2)];
const E7_BASS: [(u8, u8); 4] = [(0, 0), (2, 0), (1, 2), (2, 0)];
const D_BASS: [(u8, u8); 4] = [(2, 0), (1, 0), (2, 0), (1, 0)];
const F_BASS: [(u8, u8); 4] = [(0, 1), (2, 3), (1, 3), (2, 3)];
const B7_BASS: [(u8, u8); 4] = [(1, 2), (2, 1), (0, 2), (2, 1)];

// Harmonic functions, indexes into a Travis key's table.
const T_I: usize = 0;
const T_VI: usize = 1;
const T_II: usize = 2;
const T_V7: usize = 3;
const T_V: usize = 4;
const T_III7: usize = 5;
const T_VI7: usize = 6;
const T_I7: usize = 7;
const T_IV: usize = 8;
const T_II7: usize = 9;

struct TKey {
    name: &'static str,
    chords: [TChord; 10],
    /// Scale frets in first position on the B and high e strings.
    b_frets: [u8; 3],
    e_frets: [u8; 3],
}

const TKEYS: [TKey; 2] = [
    TKey {
        name: "C",
        chords: [
            tc([X, 3, 2, 0, 1, 0], C_BASS),  // C
            tc([X, 0, 2, 2, 1, 0], AM_BASS), // Am
            tc([X, X, 0, 2, 3, 1], D_BASS),  // Dm
            tc([3, 2, 0, 0, 0, 1], G_BASS),  // G7
            tc([3, 2, 0, 0, 0, 3], G_BASS),  // G
            tc([0, 2, 0, 1, 0, 0], E7_BASS), // E7
            tc([X, 0, 2, 0, 2, 0], AM_BASS), // A7
            tc([X, 3, 2, 3, 1, 0], C_BASS),  // C7
            tc([1, 3, 3, 2, 1, 1], F_BASS),  // F
            tc([X, X, 0, 2, 1, 2], D_BASS),  // D7
        ],
        b_frets: [0, 1, 3],
        e_frets: [0, 1, 3],
    },
    TKey {
        name: "G",
        chords: [
            tc([3, 2, 0, 0, 0, 3], G_BASS),  // G
            tc([0, 2, 2, 0, 0, 0], EM_BASS), // Em
            tc([X, 0, 2, 2, 1, 0], AM_BASS), // Am
            tc([X, X, 0, 2, 1, 2], D_BASS),  // D7
            tc([X, X, 0, 2, 3, 2], D_BASS),  // D
            tc([X, 2, 1, 2, 0, 2], B7_BASS), // B7
            tc([0, 2, 0, 1, 0, 0], E7_BASS), // E7
            tc([3, 2, 0, 0, 0, 1], G_BASS),  // G7
            tc([X, 3, 2, 0, 1, 0], C_BASS),  // C
            tc([X, 0, 2, 0, 2, 0], AM_BASS), // A7
        ],
        b_frets: [0, 1, 3],
        e_frets: [0, 2, 3],
    },
];

const T_PHRASES: [[usize; 4]; 8] = [
    [T_I, T_I, T_IV, T_I],
    [T_I, T_VI, T_II, T_V7],
    [T_I, T_III7, T_VI, T_VI7],
    [T_II, T_V7, T_I, T_I],
    [T_IV, T_I, T_II7, T_V7],
    [T_I, T_I7, T_IV, T_IV],
    [T_VI, T_VI, T_II, T_V7],
    [T_IV, T_V, T_I, T_VI],
];
const T_CADENCES: [[usize; 4]; 2] = [[T_II, T_V7, T_I, T_I], [T_IV, T_V7, T_I, T_I]];

/// Finger templates over eight eighths: the string a finger plucks (3 G,
/// 4 B, 5 e) or −1. An even slot is a pinch with the thumb.
const TEMPLATES: [[i8; 8]; 6] = [
    [-1, 4, -1, 3, -1, 5, -1, 4],
    [5, -1, -1, 4, -1, 3, -1, 5],
    [-1, 5, -1, 4, -1, 5, -1, 3],
    [5, 4, -1, 3, 5, -1, -1, 4],
    [-1, 3, -1, 4, -1, 5, 4, -1],
    [5, -1, 4, 3, -1, 5, -1, 4],
];

pub struct Travis {
    key: usize,
    phrase: [usize; 4],
    last_phrase: usize,
    bar: u32,
    phrase_n: u32,
    chord: usize,
    tpl: usize,
    base_tpl: usize,
    palm_bass: bool,
    melodic: f64,
    fresh: bool,
}

impl Default for Travis {
    fn default() -> Self {
        Self::new()
    }
}

impl Travis {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            key: 0,
            phrase: T_PHRASES[0],
            last_phrase: 0,
            bar: 0,
            phrase_n: 0,
            chord: 0,
            tpl: 0,
            base_tpl: 0,
            palm_bass: true,
            melodic: 0.25,
            fresh: true,
        }
    }

    pub fn start(&mut self) {
        self.fresh = true;
    }

    #[must_use]
    pub fn key_name(&self) -> &'static str {
        TKEYS[self.key].name
    }

    fn new_bar(&mut self, h: &mut Hand) {
        if self.fresh || self.bar >= 4 {
            if self.fresh || self.phrase_n >= 4 {
                // A new verse: maybe a new key, a new touch.
                if !self.fresh && h.chance(0.3) {
                    self.key = 1 - self.key;
                }
                self.fresh = false;
                self.phrase_n = 0;
                self.palm_bass = h.chance(0.65);
                self.melodic = h.between(0.1, 0.4);
                self.base_tpl = h.pick(TEMPLATES.len());
            }
            self.phrase = if self.phrase_n == 3 {
                T_CADENCES[h.pick(T_CADENCES.len())]
            } else {
                let mut i = h.pick(T_PHRASES.len());
                if i == self.last_phrase {
                    i = (i + 1 + h.pick(T_PHRASES.len() - 1)) % T_PHRASES.len();
                }
                self.last_phrase = i;
                T_PHRASES[i]
            };
            self.phrase_n += 1;
            self.bar = 0;
        }
        self.chord = self.phrase[self.bar as usize];
        self.tpl = if h.chance(0.65) { self.base_tpl } else { h.pick(TEMPLATES.len()) };
        // Damp strings neither the shape nor the thumb plays.
        let c = TKEYS[self.key].chords[self.chord];
        for s in 0..STRINGS {
            if c.frets[s] < 0 && !c.bass.iter().any(|(bs, _)| *bs as usize == s) {
                h.mute(0.0, s);
            }
        }
        self.bar += 1;
    }

    fn finger_fret(&self, string: usize, chord_fret: u8, slot: usize, h: &mut Hand) -> u8 {
        let k = &TKEYS[self.key];
        let options = match string {
            5 => k.e_frets,
            4 => k.b_frets,
            _ => return chord_fret,
        };
        // Off-beat fingers may borrow a neighbouring scale tone: the melody.
        if slot % 2 == 1 && chord_fret <= 3 && h.chance(self.melodic) {
            let mut best = chord_fret;
            let mut dist = i32::MAX;
            let flip = h.chance(0.5);
            for &o in &options {
                let d = (o as i32 - chord_fret as i32).abs();
                if o != chord_fret && (d < dist || (d == dist && flip)) {
                    best = o;
                    dist = d;
                }
            }
            return best;
        }
        chord_fret
    }

    pub fn step(&mut self, step: u64, h: &mut Hand) {
        let s = (step % 16) as u8;
        if s == 0 {
            self.new_bar(h);
        }
        let c = TKEYS[self.key].chords[self.chord];
        if s % 2 == 1 {
            // Now and then a sixteenth roll between the eighths.
            if s % 4 == 3 && h.chance(0.07) {
                let string = if h.chance(0.5) { 3 } else { 4 };
                if c.frets[string] >= 0 {
                    h.note(0.0, string, c.frets[string] as u8, 0.42, false);
                }
            }
            return;
        }
        let slot = (s / 2) as usize;
        if slot % 2 == 0 {
            let (bs, bf) = c.bass[slot / 2];
            let v = if slot == 0 { 0.74 } else { 0.62 };
            h.note(0.0, bs as usize, bf, v, self.palm_bass);
        }
        let f = TEMPLATES[self.tpl][slot];
        if f >= 0 {
            let string = f as usize;
            if c.frets[string] >= 0 {
                let fret = self.finger_fret(string, c.frets[string] as u8, slot, h);
                let v = if slot % 2 == 0 { 0.6 } else { 0.5 };
                h.note(0.0, string, fret, v, false);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Blues
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BStyle {
    Boogie,
    Boogie7,
    StopTime,
    Solo,
}

struct BKey {
    name: &'static str,
    /// Root (string, fret) of I, IV and V: the boogie's lower string.
    root: [(u8, u8); 3],
    /// I7, IV7, V7.
    dom: [Chord; 3],
    /// Fret shift for licks written in E at the open position and at the twelfth.
    shift: [i32; 2],
}

const BKEYS: [BKey; 2] = [
    BKey {
        name: "E",
        root: [(0, 0), (1, 0), (1, 2)],
        dom: [[0, 2, 0, 1, 0, 0], [X, 0, 2, 0, 2, 0], [X, 2, 1, 2, 0, 2]],
        shift: [0, 0],
    },
    BKey {
        name: "A",
        root: [(1, 0), (2, 0), (0, 0)],
        dom: [[X, 0, 2, 0, 2, 0], [X, X, 0, 2, 1, 2], [0, 2, 0, 1, 0, 0]],
        shift: [5, -7],
    },
];

/// A lick note, in E: at `t` triplet-eighths into the bar (three per beat).
#[derive(Clone, Copy)]
struct LN {
    t: u8,
    s: u8,
    f: u8,
    bend: f32,
    vib: f32,
}

const fn ln(t: u8, s: u8, f: u8) -> LN {
    LN { t, s, f, bend: 0.0, vib: 0.0 }
}
const fn lb(t: u8, s: u8, f: u8, bend: f32, vib: f32) -> LN {
    LN { t, s, f, bend, vib }
}

struct Lick {
    twelfth: bool,
    notes: &'static [LN],
}

const LICKS: [Lick; 8] = [
    // Down the pentatonic to the root.
    Lick { twelfth: false, notes: &[ln(3, 5, 3), ln(5, 5, 0), ln(6, 4, 3), ln(8, 4, 0), ln(9, 3, 2), ln(10, 3, 0), lb(11, 2, 2, 0.0, 0.2)] },
    // A curl on the fourth, down to the fifth.
    Lick { twelfth: false, notes: &[lb(3, 3, 2, 0.5, 0.0), ln(5, 3, 0), ln(6, 2, 2), ln(8, 2, 0), lb(9, 1, 2, 0.0, 0.18)] },
    // Low: root, minor third, fourth, blue note, fifth.
    Lick { twelfth: false, notes: &[ln(3, 0, 0), ln(5, 0, 3), ln(6, 1, 0), ln(7, 1, 1), lb(8, 1, 2, 0.0, 0.18)] },
    // Double-stops a fourth apart, then the open B and e.
    Lick {
        twelfth: false,
        notes: &[ln(3, 4, 3), ln(3, 5, 3), ln(5, 4, 3), ln(5, 5, 3), ln(6, 4, 3), ln(6, 5, 3), ln(9, 4, 0), lb(9, 5, 0, 0.0, 0.15)],
    },
    // Minor third to major third.
    Lick { twelfth: false, notes: &[ln(3, 3, 0), ln(4, 3, 1), ln(5, 2, 2), ln(6, 1, 2), ln(8, 2, 0), lb(9, 2, 2, 0.0, 0.2)] },
    // Up at the twelfth: bend the seventh to the root.
    Lick { twelfth: true, notes: &[lb(3, 4, 15, 2.0, 0.0), ln(6, 5, 12), ln(8, 4, 15), ln(9, 4, 12), lb(11, 3, 14, 0.0, 0.2)] },
    // A repeated triplet, then a bend to the fifth.
    Lick { twelfth: false, notes: &[ln(3, 5, 0), ln(4, 4, 3), ln(5, 5, 0), ln(6, 4, 3), ln(7, 5, 0), ln(8, 4, 3), lb(9, 3, 2, 2.0, 0.2)] },
    // Down the box at the twelfth.
    Lick { twelfth: true, notes: &[ln(3, 5, 12), ln(5, 4, 15), ln(6, 4, 12), ln(8, 3, 14), ln(9, 3, 12), lb(11, 2, 14, 0.0, 0.2)] },
];

const BOOGIE: [u8; 8] = [5, 5, 6, 6, 5, 5, 6, 6];
const BOOGIE7: [u8; 8] = [5, 5, 6, 6, 7, 7, 6, 6];

pub struct Blues {
    key: usize,
    bar: u32,
    chorus: u32,
    style: BStyle,
    quick: bool,
    bar10_iv: bool,
    turn: u8,
    swing: f64,
    lick: Option<usize>,
    last_lick: usize,
    /// This bar's comping: interval per eighth (5, 6, ♭7; 0 rests).
    pat: [u8; 8],
    /// A fill on the last off-beat instead of the comp.
    fill: Option<u8>,
    /// Beat one unmuted and harder.
    open_accent: bool,
    fresh: bool,
}

impl Default for Blues {
    fn default() -> Self {
        Self::new()
    }
}

impl Blues {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            key: 0,
            bar: 0,
            chorus: 0,
            style: BStyle::Boogie,
            quick: false,
            bar10_iv: true,
            turn: 0,
            swing: 0.66,
            lick: None,
            last_lick: 0,
            pat: BOOGIE,
            fill: None,
            open_accent: false,
            fresh: true,
        }
    }

    pub fn start(&mut self) {
        self.fresh = true;
    }

    #[must_use]
    pub fn key_name(&self) -> &'static str {
        BKEYS[self.key].name
    }

    /// I, IV or V for the current bar.
    fn chord(&self) -> usize {
        match self.bar {
            1 => usize::from(self.quick),
            4 | 5 => 1,
            8 => 2,
            9 => {
                if self.bar10_iv {
                    1
                } else {
                    2
                }
            }
            11 => 2,
            _ => 0,
        }
    }

    fn new_chorus(&mut self, h: &mut Hand) {
        if !self.fresh && h.chance(0.25) {
            self.key = 1 - self.key;
        }
        let prev = self.style;
        self.style = match h.weighted(&[0.35, 0.3, 0.15, 0.2]) {
            0 => BStyle::Boogie,
            1 => BStyle::Boogie7,
            2 if prev != BStyle::StopTime && !self.fresh => BStyle::StopTime,
            3 if prev != BStyle::Solo && !self.fresh => BStyle::Solo,
            _ => BStyle::Boogie7,
        };
        self.quick = h.chance(0.5);
        self.bar10_iv = h.chance(0.6);
        self.turn = h.pick(3) as u8;
        self.swing = h.between(0.62, 0.69);
        self.chorus += 1;
        self.fresh = false;
    }

    fn new_bar(&mut self, h: &mut Hand) {
        if self.fresh {
            self.bar = 0;
            self.new_chorus(h);
        } else {
            self.bar += 1;
            if self.bar >= 12 {
                self.bar = 0;
                self.new_chorus(h);
            }
        }
        let p = match self.style {
            BStyle::Boogie | BStyle::Boogie7 => match self.bar {
                3 => 0.7,
                7 => 0.5,
                _ => 0.0,
            },
            BStyle::StopTime => match self.bar {
                0..=3 => 1.0,
                7 => 0.5,
                _ => 0.0,
            },
            BStyle::Solo => match self.bar {
                0..=9 if self.bar % 2 == 0 => 0.8,
                0..=9 => 0.35,
                _ => 0.0,
            },
        };
        self.lick = if h.chance(p) {
            let mut i = h.pick(LICKS.len());
            if i == self.last_lick {
                i = (i + 1 + h.pick(LICKS.len() - 1)) % LICKS.len();
            }
            self.last_lick = i;
            Some(i)
        } else {
            None
        };
        // The comping breathes: a dropped off-beat, a ♭7 walk, a pushed sixth,
        // an open accent on one, a fill to close the bar.
        let mut pat = if self.style == BStyle::Boogie7 { BOOGIE7 } else { BOOGIE };
        for e in (1..8).step_by(2) {
            if h.chance(0.08) {
                pat[e] = 0;
            }
        }
        if h.chance(0.25) {
            let b = 2 + h.pick(2);
            pat[2 * b] = 7;
            pat[2 * b + 1] = if h.chance(0.5) { 7 } else { 6 };
        }
        if h.chance(0.15) {
            pat[1] = 6;
        }
        self.pat = pat;
        self.fill = if self.lick.is_none() && self.bar < 10 && h.chance(0.28) { Some(h.pick(4) as u8) } else { None };
        self.open_accent = h.chance(0.3);
    }

    /// A short fill in the last beat's off-beat (written in E at the open position).
    fn play_fill(&self, kind: u8, at: f64, beat_s: f64, h: &mut Hand) {
        let shift = BKEYS[self.key].shift[0] as u8;
        match kind {
            0 => {
                h.note(at, 4, 3 + shift, 0.66, false);
                h.note(at, 5, 3 + shift, 0.62, false);
            }
            1 => {
                h.note(at, 3, shift, 0.64, false);
                h.note(at + beat_s / 6.0, 2, 2 + shift, 0.6, false);
            }
            2 => h.note(at, 2, 2 + shift, 0.68, false),
            _ => {
                h.note(at, 4, shift, 0.6, false);
                h.note(at, 5, shift, 0.56, false);
            }
        }
    }

    fn boogie(&self, b: u8, beat_s: f64, ch: usize, h: &mut Hand) {
        let (rs, rf) = BKEYS[self.key].root[ch];
        for half in 0..2u8 {
            let e = (2 * b + half) as usize;
            let at = if half == 0 { 0.0 } else { self.swing * beat_s };
            if half == 1 && b == 3 {
                if let Some(kind) = self.fill {
                    self.play_fill(kind, at, beat_s, h);
                    continue;
                }
            }
            let up = match self.pat[e] {
                0 => continue,
                5 => 2,
                6 => 4,
                _ => 5,
            };
            let open = e == 0 && self.open_accent;
            let v = if half == 0 { if b % 2 == 0 { 0.76 } else { 0.68 } } else { 0.52 } + if open { 0.08 } else { 0.0 };
            h.note(at, rs as usize, rf, v, !open);
            h.note(at, rs as usize + 1, rf + up, v - 0.05, !open);
        }
    }

    pub fn step(&mut self, step: u64, h: &mut Hand) {
        let s = (step % 16) as u8;
        if s == 0 {
            self.new_bar(h);
        }
        if s % 4 != 0 {
            return;
        }
        let b = s / 4;
        let beat_s = h.sixteenth * 4.0;
        let key = &BKEYS[self.key];
        let ch = self.chord();
        let pedal = key.shift[0] as u8;
        match (self.bar, self.turn) {
            (10, 0) => {
                // The turnaround: a falling inner line under the high e.
                let f = 4 - b + pedal;
                h.note(0.0, 5, pedal, 0.62, false);
                h.note(0.0, 3, f, 0.7, false);
                h.note(self.swing * beat_s, 3, f, 0.5, false);
                return;
            }
            (10, 1) => {
                // A walk up to V.
                let walk: [(u8, u8); 4] = if self.key == 0 { [(0, 0), (0, 4), (1, 0), (1, 1)] } else { [(1, 0), (1, 4), (2, 0), (2, 1)] };
                let (ws, wf) = walk[b as usize];
                h.note(0.0, ws as usize, wf, 0.74, false);
                return;
            }
            (11, 0 | 1) => {
                if b == 0 {
                    let (rs, rf) = key.root[0];
                    h.note(0.0, rs as usize, rf, 0.74, false);
                    h.note(0.0, rs as usize + 1, rf + 2, 0.66, false);
                } else if b == 2 {
                    let v7 = key.dom[2];
                    h.strum(&v7, 0.0, Stroke::down(0.78, 0.012));
                    h.vibrato(0.35, 5, 0.12);
                }
                return;
            }
            _ => {}
        }
        let lick_on = self.lick.is_some() && b >= 1;
        match self.style {
            BStyle::StopTime if self.bar < 4 => {
                if b == 0 {
                    h.strum(&key.dom[ch], 0.0, Stroke::down(0.86, 0.008));
                    for st in 0..STRINGS {
                        h.mute(0.28, st);
                    }
                }
            }
            _ if lick_on => {}
            _ if self.lick.is_some() => {
                let (rs, rf) = key.root[ch];
                h.note(0.0, rs as usize, rf, 0.74, true);
            }
            _ => self.boogie(b, beat_s, ch, h),
        }
        if let Some(li) = self.lick {
            let lick = &LICKS[li];
            let shift = key.shift[usize::from(lick.twelfth)];
            for n in lick.notes {
                if n.t / 3 == b {
                    let at = f64::from(n.t % 3) / 3.0 * beat_s;
                    let fret = (n.f as i32 + shift).clamp(0, MAX_FRET as i32) as u8;
                    h.note(at, n.s as usize, fret, 0.8, false);
                    if n.bend > 0.0 {
                        h.bend(at + 0.06, n.s as usize, n.bend as f64);
                    }
                    if n.vib > 0.0 {
                        h.vibrato(at + 0.25, n.s as usize, n.vib as f64);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Ambient
// ---------------------------------------------------------------------------

/// Color chords for the ambient piece; each is used only where its pitch
/// classes fit the current mode.
const AMB_CHORDS: [Chord; 10] = [
    [0, 2, 0, 0, 0, 2], // Em9
    [X, 0, 2, 2, 0, 0], // Asus2
    [X, 0, 2, 4, 2, 2], // A6/9
    [X, 2, 0, 2, 0, 2], // Bm7
    [X, X, 0, 2, 3, 0], // Dsus2
    [3, X, 0, 0, 0, 2], // Gmaj7
    [0, 2, 2, 0, 3, 0], // Em7
    [X, 3, 2, 0, 0, 0], // Cmaj7
    [X, 0, 2, 0, 1, 0], // Am7
    [X, 3, 2, 0, 3, 0], // Cadd9
];

const DORIAN: [i32; 7] = [0, 2, 3, 5, 7, 9, 10];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Amb {
    Drone(u8, u8),
    Chord(u8, bool),
    Pluck(u8, u8),
    Pair(u8),
    Bend(u8, u8),
}

pub struct Ambient {
    tonic: i32,
    dorian: bool,
    tide: u32,
    density: f64,
    rest: u32,
    since_drone: u32,
    plan: [(u8, Option<Amb>); 8],
    n: usize,
}

impl Default for Ambient {
    fn default() -> Self {
        Self::new()
    }
}

impl Ambient {
    #[must_use]
    pub const fn new() -> Self {
        Self { tonic: 4, dorian: true, tide: 0, density: 0.5, rest: 0, since_drone: 9, plan: [(0, None); 8], n: 0 }
    }

    pub fn start(&mut self) {
        self.tide = 0;
        self.rest = 0;
        self.since_drone = 9;
    }

    #[must_use]
    pub fn key_name(&self) -> &'static str {
        match (self.tonic, self.dorian) {
            (4, true) => "E DORIAN",
            (4, false) => "E AEOLIAN",
            (_, true) => "A DORIAN",
            _ => "A AEOLIAN",
        }
    }

    fn mode(&self) -> &'static [i32; 7] {
        if self.dorian {
            &DORIAN
        } else {
            &MINOR
        }
    }

    fn fits(&self, p: i32) -> bool {
        in_scale(self.tonic, self.mode(), p)
    }

    fn add(&mut self, step: u8, a: Amb) {
        if self.n < self.plan.len() {
            self.plan[self.n] = (step, Some(a));
            self.n += 1;
        }
    }

    /// A mode tone high on string 3, 4 or 5.
    fn high_note(&self, h: &mut Hand) -> Option<(u8, u8)> {
        for _ in 0..12 {
            let s = 3 + h.pick(3);
            let f = 5 + h.pick(11) as i32;
            if self.fits(pc(s, f)) {
                return Some((s as u8, f as u8));
            }
        }
        None
    }

    fn new_bar(&mut self, h: &mut Hand) {
        self.n = 0;
        self.since_drone += 1;
        if self.tide == 0 {
            self.tide = 8 + h.pick(9) as u32;
            self.density = h.between(0.3, 0.75);
            self.dorian = h.chance(0.5);
            if h.chance(0.3) {
                self.tonic = if self.tonic == 4 { 9 } else { 4 };
            }
        }
        self.tide -= 1;
        if self.rest > 0 {
            self.rest -= 1;
            return;
        }
        if h.chance(0.22 * (1.0 - self.density) + 0.05) {
            // Silence: this bar and perhaps the next one or two.
            self.rest = h.pick(3) as u32;
            return;
        }
        if self.since_drone >= 3 && h.chance(0.55) {
            self.since_drone = 0;
            let (s, f) = if self.tonic == 4 { (0, 0) } else { (1, 0) };
            self.add(0, Amb::Drone(s, f));
            if h.chance(0.3) {
                self.add(0, Amb::Drone(s + 1, 2));
            } else if h.chance(0.15) {
                self.add(8, Amb::Drone(2, 0));
            }
        }
        if h.chance(0.22 + 0.15 * self.density) {
            let mut fit = [0u8; AMB_CHORDS.len()];
            let mut k = 0;
            for (i, c) in AMB_CHORDS.iter().enumerate() {
                let pcs = pitch_classes(c);
                if (0..12).all(|p| !pcs[p] || self.fits(p as i32)) {
                    fit[k] = i as u8;
                    k += 1;
                }
            }
            if k > 0 {
                let i = fit[h.pick(k)];
                let at = [0u8, 4, 8][h.pick(3)];
                let down = h.chance(0.6);
                self.add(at, Amb::Chord(i, down));
            }
        }
        let plucks = h.weighted(&[0.6 - 0.5 * self.density, 1.0, 0.6 + self.density, 0.2 + self.density]);
        for _ in 0..plucks {
            if let Some((s, f)) = self.high_note(h) {
                let at = h.pick(16) as u8;
                self.add(at, Amb::Pluck(s, f));
            }
        }
        if h.chance(0.08) {
            for _ in 0..8 {
                let f = 5 + h.pick(10) as i32;
                if self.fits(pc(4, f)) && self.fits(pc(5, f)) {
                    let at = [2u8, 6, 10][h.pick(3)];
                    self.add(at, Amb::Pair(f as u8));
                    break;
                }
            }
        }
        if h.chance(0.05) {
            for _ in 0..8 {
                let f = 5 + h.pick(9) as i32;
                if self.fits(pc(4, f)) && self.fits(pc(4, f + 2)) {
                    self.add(4, Amb::Bend(4, f as u8));
                    break;
                }
            }
        }
    }

    pub fn step(&mut self, step: u64, h: &mut Hand) {
        let s = (step % 16) as u8;
        if s == 0 {
            self.new_bar(h);
        }
        for i in 0..self.n {
            let (at, a) = self.plan[i];
            if at != s {
                continue;
            }
            match a {
                Some(Amb::Drone(st, f)) => {
                    let v = h.between(0.4, 0.55);
                    h.note(0.0, st as usize, f, v, false);
                }
                Some(Amb::Chord(ci, down)) => {
                    let c = AMB_CHORDS[ci as usize];
                    let v = h.between(0.28, 0.42);
                    let per = h.between(0.04, 0.09);
                    let k = if down { Stroke::down(v, per) } else { Stroke::up(v, per, 0) };
                    // Let drones and plucks ring through: no damping.
                    h.strum(&c, 0.0, Stroke { damp: false, ..k });
                }
                Some(Amb::Pluck(st, f)) => {
                    let v = h.between(0.25, 0.55);
                    h.note(0.0, st as usize, f, v, false);
                    if h.chance(0.4) {
                        let d = h.between(0.6, 1.2);
                        let depth = h.between(0.08, 0.18);
                        h.vibrato(d, st as usize, depth);
                    }
                }
                Some(Amb::Pair(f)) => {
                    let v = h.between(0.3, 0.45);
                    h.note(0.0, 4, f, v, false);
                    h.note(0.0, 5, f, v, false);
                }
                Some(Amb::Bend(st, f)) => {
                    h.note(0.0, st as usize, f, 0.45, false);
                    let d = h.between(0.35, 0.7);
                    h.bend(d, st as usize, 2.0);
                    h.vibrato(d + 0.6, st as usize, 0.12);
                }
                None => {}
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Melody
// ---------------------------------------------------------------------------

struct MKey {
    name: &'static str,
    /// MIDI note of degree 0.
    tonic: i32,
    /// Hand position (fret) the melody is played around.
    pos: i32,
    lo: i32,
    hi: i32,
    /// Bass (string, fret) under each degree of the natural minor scale.
    bass: [(u8, u8); 7],
}

const MKEYS: [MKey; 3] = [
    MKey { name: "A MINOR", tonic: 69, pos: 5, lo: -5, hi: 3, bass: [(1, 0), (1, 2), (1, 3), (2, 0), (0, 0), (0, 1), (0, 3)] },
    MKey { name: "A MINOR HIGH", tonic: 69, pos: 12, lo: 0, hi: 7, bass: [(1, 0), (1, 2), (1, 3), (2, 0), (0, 0), (0, 1), (0, 3)] },
    MKey { name: "D MINOR", tonic: 74, pos: 10, lo: -4, hi: 3, bass: [(2, 0), (2, 2), (0, 1), (0, 3), (1, 0), (1, 1), (1, 3)] },
];

/// Four-bar progressions as minor-scale degrees (i III iv v VI VII).
const M_PROGS: [[i32; 4]; 7] = [[0, 5, 6, 0], [0, 3, 4, 0], [0, 6, 5, 4], [3, 0, 4, 0], [5, 6, 0, 0], [0, 2, 3, 4], [0, 3, 6, 2]];

/// Rhythm cells: note lengths in sixteenths (negative is a rest).
const CELLS: [&[i8]; 9] = [&[4], &[2, 2], &[3, 1], &[8], &[-4], &[-2, 2], &[1, 1, 2], &[6, 2], &[2, -2]];

const REST: i32 = i32::MIN;

pub struct Melody {
    key: usize,
    bar: u32,
    section_left: u32,
    harmony: [i32; 4],
    deg: i32,
    motif: [i8; 16],
    motif_n: usize,
    contour: [i32; 16],
    /// This bar: (start step, degree or REST, length).
    notes: [(u8, i32, u8); 16],
    n: usize,
    last_string: Option<usize>,
    climax: i32,
    fresh: bool,
}

impl Default for Melody {
    fn default() -> Self {
        Self::new()
    }
}

impl Melody {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            key: 0,
            bar: 0,
            section_left: 0,
            harmony: M_PROGS[0],
            deg: 0,
            motif: [0; 16],
            motif_n: 0,
            contour: [0; 16],
            notes: [(0, REST, 0); 16],
            n: 0,
            last_string: None,
            climax: i32::MIN,
            fresh: true,
        }
    }

    pub fn start(&mut self) {
        self.fresh = true;
    }

    #[must_use]
    pub fn key_name(&self) -> &'static str {
        MKEYS[self.key].name
    }

    fn midi(&self, d: i32) -> i32 {
        MKEYS[self.key].tonic + 12 * d.div_euclid(7) + MINOR[d.rem_euclid(7) as usize]
    }

    /// Where a hand at this key's position plays `midi`, on the G, B or e string.
    fn place(&self, midi: i32) -> Option<(usize, u8)> {
        let pos = MKEYS[self.key].pos;
        let mut best: Option<(usize, i32, i32)> = None;
        for s in 3..STRINGS {
            let f = midi - OPEN_MIDI[s];
            if !(0..=17).contains(&f) {
                continue;
            }
            let inside = f >= pos - 1 && f <= pos + 5;
            let cost = (f - pos - 2).abs() + if inside { 0 } else { 20 };
            if best.is_none_or(|(_, _, c)| cost < c) {
                best = Some((s, f, cost));
            }
        }
        best.map(|(s, f, _)| (s, f as u8))
    }

    fn chord_tone(&self, d: i32, harm: i32) -> bool {
        let r = (d - harm).rem_euclid(7);
        r == 0 || r == 2 || r == 4
    }

    fn clamp(&self, d: i32) -> i32 {
        let k = &MKEYS[self.key];
        if d < k.lo {
            k.lo + (k.lo - d).min(2)
        } else if d > k.hi {
            k.hi - (d - k.hi).min(2)
        } else {
            d
        }
    }

    /// The degree nearest `from` that is `target` mod 7, in range.
    fn nearest(&self, from: i32, target: i32) -> i32 {
        let k = &MKEYS[self.key];
        let mut best = from;
        let mut dist = i32::MAX;
        for d in k.lo..=k.hi {
            if (d - target).rem_euclid(7) == 0 && (d - from).abs() < dist {
                best = d;
                dist = (d - from).abs();
            }
        }
        best
    }

    /// Fill a bar with rhythm cells, reserving a final held note of `end` sixteenths.
    fn rhythm(h: &mut Hand, end: u8, out: &mut [i8; 16]) -> usize {
        let room = 16 - end as i32;
        let mut n = 0;
        let mut sum = 0;
        while sum < room {
            let cell = CELLS[h.pick(CELLS.len())];
            let len: i32 = cell.iter().map(|x| (*x as i32).abs()).sum();
            if sum + len > room {
                // What is left becomes one note.
                out[n] = (room - sum) as i8;
                n += 1;
                break;
            }
            for x in cell.iter() {
                out[n] = *x;
                n += 1;
            }
            sum += len;
        }
        if end > 0 {
            out[n] = end as i8;
            n += 1;
        }
        n
    }

    fn walk(&mut self, h: &mut Hand, strong: bool, harm: i32) -> i32 {
        let steps = [-2, -1, 1, 2, -3, 3, 0, -4, 4];
        let w = [0.15, 0.25, 0.25, 0.15, 0.06, 0.06, 0.04, 0.02, 0.02];
        let mut d = self.clamp(self.deg + steps[h.weighted(&w)]);
        if strong && !self.chord_tone(d, harm) && h.chance(0.65) {
            d = if self.chord_tone(d + 1, harm) { d + 1 } else { d - 1 };
            d = self.clamp(d);
        }
        d
    }

    fn plan(&mut self, h: &mut Hand) {
        self.n = 0;
        let harm = self.harmony[self.bar.min(3) as usize];
        let mut rhythm = [0i8; 16];
        let count = match self.bar {
            0 => {
                let n = Self::rhythm(h, 0, &mut rhythm);
                self.motif = rhythm;
                self.motif_n = n;
                n
            }
            2 => {
                rhythm = self.motif;
                if h.chance(0.35) && self.motif_n > 1 {
                    // Vary one note's length into a rest or back.
                    let i = 1 + h.pick(self.motif_n - 1);
                    rhythm[i] = -rhythm[i];
                }
                self.motif_n
            }
            1 => {
                let end = [6u8, 8][h.pick(2)];
                Self::rhythm(h, end, &mut rhythm)
            }
            _ => {
                let end = [8u8, 12][h.pick(2)];
                Self::rhythm(h, end, &mut rhythm)
            }
        };
        let mut t = 0u8;
        let mut first = None;
        let mut k = 0;
        for (i, len) in rhythm.iter().take(count).enumerate() {
            let l = len.unsigned_abs();
            if *len < 0 {
                self.notes[self.n] = (t, REST, l);
            } else {
                let last = i + 1 == count;
                let d = match (self.bar, last) {
                    (1, true) => self.nearest(self.deg, if h.chance(0.7) { 4 } else { 1 }),
                    (3, true) => self.nearest(self.deg, if h.chance(0.75) { 0 } else { 2 }),
                    (2, _) => {
                        // The motif again, sequenced from a chord tone of this bar.
                        let start = *first.get_or_insert_with(|| {
                            let s = self.nearest(self.deg, harm + 2 * (h.pick(3) as i32));
                            s
                        });
                        self.clamp(start + self.contour[k.min(15)])
                    }
                    _ => self.walk(h, t % 8 == 0, harm),
                };
                if self.bar == 0 {
                    let base = *first.get_or_insert(d);
                    self.contour[k.min(15)] = d - base;
                }
                k += 1;
                self.deg = d;
                self.climax = self.climax.max(d);
                self.notes[self.n] = (t, d, l);
            }
            self.n += 1;
            t += l;
            if self.n >= self.notes.len() {
                break;
            }
        }
    }

    pub fn step(&mut self, step: u64, h: &mut Hand) {
        let s = (step % 16) as u8;
        if s == 0 {
            if self.fresh || self.bar >= 4 {
                // A breath between phrases, sometimes.
                if !self.fresh && self.bar == 4 && h.chance(0.25) {
                    self.bar = 5;
                    self.n = 0;
                    return;
                }
                if self.fresh || self.section_left == 0 {
                    self.key = h.pick(MKEYS.len());
                    self.section_left = 2 + h.pick(2) as u32;
                }
                self.fresh = false;
                self.section_left -= 1;
                self.harmony = M_PROGS[h.pick(M_PROGS.len())];
                self.bar = 0;
                self.climax = i32::MIN;
                self.deg = self.nearest(0, 2 * h.pick(3) as i32);
            }
            if self.bar == 0 || self.bar == 2 {
                let harm = self.harmony[self.bar as usize];
                let (bs, bf) = MKEYS[self.key].bass[harm.rem_euclid(7) as usize];
                let v = h.between(0.36, 0.46);
                h.note(0.0, bs as usize, bf, v, false);
            }
            self.plan(h);
            self.bar += 1;
        }
        let bar_index = self.bar.saturating_sub(1);
        for i in 0..self.n {
            let (at, d, len) = self.notes[i];
            if at != s {
                continue;
            }
            if d == REST {
                if let Some(ls) = self.last_string {
                    if h.chance(0.6) {
                        h.mute(0.0, ls);
                        self.last_string = None;
                    }
                }
                continue;
            }
            let Some((string, fret)) = self.place(self.midi(d)) else { continue };
            if let Some(ls) = self.last_string {
                if ls != string && h.chance(0.5) {
                    h.mute(0.02, ls);
                }
            }
            let strong = at % 8 == 0;
            let last = i + 1 == self.n;
            let v = h.between(0.55, 0.68) + if strong { 0.08 } else { 0.0 };
            let below = self.midi(d) - self.midi(d - 1);
            let bend = bar_index == 2 && d == self.climax && (string == 3 || string == 4) && below == 2 && fret >= 2 && h.chance(0.45);
            if bend {
                h.note(0.0, string, fret - 2, v, false);
                h.bend(0.05, string, 2.0);
            } else {
                h.note(0.0, string, fret, v, false);
            }
            if last && (bar_index == 1 || bar_index == 3) {
                h.vibrato(0.35, string, 0.2);
            } else if len >= 6 && h.chance(0.4) {
                h.vibrato(0.4, string, 0.12);
            }
            self.last_string = Some(string);
        }
    }
}

/// Which key each generative piece is in now (for the screen).
#[must_use]
pub fn key_label(w: &Wander, t: &Travis, b: &Blues, a: &Ambient, m: &Melody, p: crate::player::Pattern) -> &'static str {
    use crate::player::Pattern;
    match p {
        Pattern::Wander => w.key_name(),
        Pattern::Travis => t.key_name(),
        Pattern::Blues => b.key_name(),
        Pattern::Ambient => a.key_name(),
        Pattern::Melody => m.key_name(),
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: [&str; 12] = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];

    /// A shape a hand can hold: at least three strings, fretted notes within
    /// four frets, nothing above the twelfth.
    fn playable(c: &Chord) -> Result<(), String> {
        let sounding = c.iter().filter(|f| **f >= 0).count();
        let fretted: Vec<i8> = c.iter().copied().filter(|f| *f > 0).collect();
        let span = fretted.iter().max().unwrap_or(&0) - fretted.iter().min().unwrap_or(&0);
        if sounding < 2 || span > 3 || c.iter().any(|f| *f > 12) {
            return Err(format!("{c:?}: {sounding} strings, span {span}"));
        }
        Ok(())
    }

    #[test]
    fn every_named_shape_is_playable_and_spells_its_name() {
        for (name, c) in SHAPES {
            playable(c).unwrap();
            let pcs = pitch_classes(c);
            let (root, quality) = if name.len() > 1 && (name.as_bytes()[1] == b'#' || name.as_bytes()[1] == b'b') {
                (&name[..2], &name[2..])
            } else {
                (&name[..1], &name[1..])
            };
            let r = NAMES.iter().position(|n| *n == root).or(match root {
                "Bb" => Some(10),
                "G#" => Some(8),
                "Ab" => Some(8),
                "Eb" => Some(3),
                _ => None,
            });
            let r = r.unwrap_or_else(|| panic!("{name}: root {root}")) as i32;
            let quality = if quality == "6/9" { quality } else { quality.split('/').next().unwrap() };
            let want: &[i32] = match quality {
                "" => &[0, 4, 7],
                "m" => &[0, 3, 7],
                "7" => &[0, 4, 7, 10],
                "maj7" => &[0, 4, 7, 11],
                "m7" => &[0, 3, 7, 10],
                "sus2" => &[0, 2, 7],
                "sus4" => &[0, 5, 7],
                "add9" => &[0, 2, 4, 7],
                "m9" => &[0, 2, 3, 7, 10],
                "6" => &[0, 4, 7, 9],
                "5" => &[0, 7],
                "6/9" => &[0, 2, 4, 7, 9],
                "m(add9)" => &[0, 2, 3, 7],
                "m11" => &[0, 3, 5, 7, 10],
                "m(add11)" => &[0, 3, 5, 7],
                "maj7#11" => &[0, 4, 6, 7, 11],
                other => panic!("{name}: unknown quality {other}"),
            };
            for p in 0..12 {
                let in_chord = want.iter().any(|i| (r + i).rem_euclid(12) == p);
                if pcs[p as usize] && !in_chord {
                    panic!("{name} {c:?} sounds {} which is not in the chord", NAMES[p as usize]);
                }
            }
            // Root and (where there is one) the third must sound; fifths may be left out.
            assert!(pcs[r as usize], "{name} lacks its root");
            for third in [3, 4, 2, 5] {
                if want.contains(&third) {
                    assert!(pcs[((r + third) % 12) as usize], "{name} {c:?} lacks its third/sus");
                    break;
                }
            }
        }
    }

    #[test]
    fn wander_voicings_are_playable_and_in_key() {
        for k in &KEYS {
            for (d, colors) in k.deg.iter().enumerate() {
                let root = (k.tonic + MAJOR[d]).rem_euclid(12);
                for c in colors {
                    playable(c).unwrap();
                    let pcs = pitch_classes(c);
                    assert!(pcs[root as usize], "key {} degree {d}: {c:?} lacks its root", k.name);
                    for p in 0..12 {
                        assert!(!pcs[p] || in_scale(k.tonic, &MAJOR, p as i32), "key {} degree {d}: {c:?} leaves the key", k.name);
                    }
                }
            }
            // V is always V, Vsus4, V7.
            let v = (k.tonic + 7).rem_euclid(12);
            let v7 = pitch_classes(&k.deg[4][2]);
            assert!(v7[((v + 10) % 12) as usize], "key {}: V7 lacks its seventh", k.name);
        }
    }

    #[test]
    fn travis_shapes_and_basses_agree() {
        for k in &TKEYS {
            for c in &k.chords {
                playable(&c.frets).unwrap();
                let pcs = pitch_classes(&c.frets);
                // The thumb plays chord tones, or the root's fifth (C7's G).
                let fifth = (pc(c.bass[0].0 as usize, c.bass[0].1 as i32) + 7) % 12;
                for (s, f) in c.bass {
                    let p = pc(s as usize, f as i32);
                    assert!(pcs[p as usize] || p == fifth, "key {}: bass {s}/{f} not in {:?}", k.name, c.frets);
                }
            }
        }
    }

    #[test]
    fn ambient_chords_and_blues_dominants_are_playable() {
        for c in &AMB_CHORDS {
            playable(c).unwrap();
        }
        for k in &BKEYS {
            for c in &k.dom {
                playable(c).unwrap();
            }
        }
    }
}
