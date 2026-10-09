//! Scores: music written down (by Claude, over the control socket, or in a
//! file) for the guitarist to play.
//!
//! A score is JSON:
//!
//! ```json
//! {
//!   "name": "wake",
//!   "tempo": 84,
//!   "loop": false,
//!   "then": "resume",
//!   "events": [
//!     {"type": "strum", "beat": 0, "chord": "Em9", "dir": "down", "spread_ms": 60, "velocity": 0.6},
//!     {"type": "pluck", "beat": 2, "string": 5, "fret": 7, "velocity": 0.7},
//!     {"type": "bend", "beat": 2.25, "string": 5, "semitones": 1},
//!     {"type": "vibrato", "beat": 3, "string": 5, "depth": 0.2},
//!     {"type": "mute", "beat": 4, "strings": [0, 1]},
//!     {"type": "rest", "beat": 4, "beats": 4}
//!   ]
//! }
//! ```
//!
//! Strings count from the low E: 0 = E2 … 5 = E4 (names `"E" "A" "D" "G" "B"
//! "e"` work too). A chord is a name from [`crate::compose::SHAPES`], six frets
//! low to high (`[3,2,0,0,0,3]`, with `-1`, `null` or `"x"` for a string not
//! played), or tab (`"x32010"`, `"x-3-2-0-1-0"`). Beats are quarter notes
//! from 0.
//!
//! The score is checked and compiled here, on the UI thread, into a sorted
//! boxed slice of single-string events (strums are rolled out across their
//! spread at the score's tempo). The audio thread only walks that slice; the
//! box comes back through the rig's garbage ring to be freed. A one-shot
//! score plays once and then the guitarist returns to whatever it was
//! playing before (`"then": "resume"`, the default) or stops
//! (`"then": "stop"`); `"loop": true` repeats it until something else is
//! chosen. Rings left at the end ring out.

use crate::compose::{chord_shape, Chord, MAX_FRET, SHAPES, X};
use crate::guitar::{STRINGS, STRING_NAMES};
use crate::player::Event;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// Most events a score may be written with.
pub const MAX_EVENTS: usize = 4096;
/// Latest beat an event may sit on.
pub const MAX_BEATS: f64 = 4096.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timed {
    /// When, in beats from the start (strum spread included).
    pub beat: f64,
    pub event: Event,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Then {
    /// Return to the composition playing before the score (silence if none).
    #[default]
    Resume,
    /// Stop: the guitarist falls silent.
    Stop,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Score {
    pub name: String,
    pub tempo: f64,
    pub looped: bool,
    pub then: Then,
    /// 0 plays exactly as written; 1 adds up to 12 ms of lag and ±0.08 velocity.
    pub humanize: f32,
    pub length_beats: f64,
    /// Single-string events, sorted by beat.
    pub events: Box<[Timed]>,
    /// How many events the score was written with.
    pub written: usize,
}

impl Score {
    /// Seconds one pass takes at the score's tempo.
    #[must_use]
    pub fn seconds(&self) -> f64 {
        self.length_beats * 60.0 / self.tempo
    }

    /// A short report of what was compiled.
    #[must_use]
    pub fn summary(&self) -> Value {
        json!({
            "name": self.name,
            "tempo": self.tempo,
            "loop": self.looped,
            "then": if self.then == Then::Resume { "resume" } else { "stop" },
            "humanize": self.humanize,
            "length_beats": self.length_beats,
            "seconds": (self.seconds() * 1000.0).round() / 1000.0,
            "events_written": self.written,
            "events_compiled": self.events.len(),
        })
    }

    /// The compiled score as score JSON (single-string events at exact
    /// beats). Parsing it gives back the same score.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let events: Vec<Value> = self
            .events
            .iter()
            .map(|t| match t.event {
                Event::Pluck { string, fret, velocity, palm } => json!({
                    "type": "pluck", "beat": t.beat, "string": string, "fret": fret,
                    "velocity": velocity as f64, "palm": palm
                }),
                Event::Mute { string } => json!({"type": "mute", "beat": t.beat, "strings": [string]}),
                Event::Bend { string, semitones } => {
                    json!({"type": "bend", "beat": t.beat, "string": string, "semitones": semitones as f64})
                }
                Event::Vibrato { string, depth } => {
                    json!({"type": "vibrato", "beat": t.beat, "string": string, "depth": depth as f64})
                }
            })
            .collect();
        json!({
            "name": self.name,
            "tempo": self.tempo,
            "loop": self.looped,
            "then": if self.then == Then::Resume { "resume" } else { "stop" },
            "humanize": self.humanize as f64,
            "length_beats": self.length_beats,
            "events": events,
        })
    }
}

/// Parse and compile a score given as JSON text. `bpm` is the tempo to use
/// when the score names none.
pub fn parse_str(s: &str, bpm: f64) -> Result<Score, String> {
    let v: Value = serde_json::from_str(s).map_err(|e| format!("score is not valid JSON: {e}"))?;
    parse(&v, bpm)
}

const TOP_KEYS: [&str; 8] = ["name", "tempo", "loop", "then", "humanize", "beats_per_bar", "length_beats", "events"];

/// Parse and compile a score. `bpm` is the tempo to use when the score names none.
pub fn parse(v: &Value, bpm: f64) -> Result<Score, String> {
    let o = v.as_object().ok_or("a score is a JSON object: {\"tempo\": 96, \"events\": [...]}")?;
    for k in o.keys() {
        if !TOP_KEYS.contains(&k.as_str()) {
            return Err(format!("unknown score field \"{k}\" (allowed: {})", TOP_KEYS.join(", ")));
        }
    }
    let name = match o.get("name") {
        None | Some(Value::Null) => "score".to_string(),
        Some(Value::String(s)) => s.chars().take(40).collect(),
        Some(_) => return Err("\"name\" must be a string".into()),
    };
    let tempo = num(o, "tempo", "score")?.unwrap_or(bpm);
    if !(40.0..=220.0).contains(&tempo) {
        return Err(format!("tempo {tempo} is out of range 40..=220 BPM"));
    }
    let looped = boolean(o, "loop", "score")?.unwrap_or(false);
    let then = match o.get("then") {
        None | Some(Value::Null) => Then::Resume,
        Some(Value::String(s)) if s == "resume" => Then::Resume,
        Some(Value::String(s)) if s == "stop" => Then::Stop,
        Some(other) => return Err(format!("\"then\" must be \"resume\" or \"stop\", not {other}")),
    };
    let humanize = num(o, "humanize", "score")?.unwrap_or(0.0);
    if !(0.0..=1.0).contains(&humanize) {
        return Err(format!("humanize {humanize} is out of range 0..=1"));
    }
    let bpb = num(o, "beats_per_bar", "score")?.unwrap_or(4.0);
    if !(1.0..=16.0).contains(&bpb) {
        return Err(format!("beats_per_bar {bpb} is out of range 1..=16"));
    }
    let events = o
        .get("events")
        .ok_or("a score needs \"events\": [{\"type\": \"pluck\", \"beat\": 0, \"string\": 0, \"fret\": 3}, ...]")?
        .as_array()
        .ok_or("\"events\" must be an array")?;
    if events.is_empty() {
        return Err("\"events\" is empty: give at least one event".into());
    }
    if events.len() > MAX_EVENTS {
        return Err(format!("{} events is more than the {MAX_EVENTS} a score may hold", events.len()));
    }
    let beats_per_s = tempo / 60.0;
    let mut out: Vec<Timed> = Vec::with_capacity(events.len() * 2);
    let mut end = 0.0f64;
    let mut last_beat = 0.0f64;
    for (i, e) in events.iter().enumerate() {
        let (beat, extent) = event(i, e, beats_per_s, &mut out)?;
        end = end.max(extent);
        last_beat = last_beat.max(beat);
    }
    let length_beats = match num(o, "length_beats", "score")? {
        Some(l) => {
            if !(l > 0.0 && l <= MAX_BEATS * 2.0) {
                return Err(format!("length_beats {l} must be above 0 and at most {}", MAX_BEATS * 2.0));
            }
            if looped && last_beat >= l {
                return Err(format!("length_beats {l} must be after the last event (beat {last_beat}) in a looping score"));
            }
            if !looped && end > l {
                return Err(format!("length_beats {l} ends before the last event (beat {end})"));
            }
            l
        }
        None => ((end / bpb).floor() + 1.0) * bpb,
    };
    // Strum spreads may run past the end of a looped score: keep them inside.
    for t in &mut out {
        t.beat = t.beat.min(length_beats - 1e-9).max(0.0);
    }
    out.sort_by(|a, b| a.beat.total_cmp(&b.beat));
    Ok(Score {
        name,
        tempo,
        looped,
        then,
        humanize: humanize as f32,
        length_beats,
        events: out.into_boxed_slice(),
        written: events.len(),
    })
}

fn num(o: &Map<String, Value>, k: &str, ctx: &str) -> Result<Option<f64>, String> {
    match o.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_f64()
            .filter(|x| x.is_finite())
            .map(Some)
            .ok_or_else(|| format!("{ctx}: \"{k}\" must be a number, not {v}")),
    }
}

fn boolean(o: &Map<String, Value>, k: &str, ctx: &str) -> Result<Option<bool>, String> {
    match o.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(v) => Err(format!("{ctx}: \"{k}\" must be true or false, not {v}")),
    }
}

fn in_range(ctx: &str, k: &str, x: f64, lo: f64, hi: f64) -> Result<f64, String> {
    if (lo..=hi).contains(&x) {
        Ok(x)
    } else {
        Err(format!("{ctx}: {k} {x} is out of range {lo}..={hi}"))
    }
}

/// A string by number (0 = low E … 5 = high e) or name (`E A D G B e`, or `E2 … E4`).
pub fn string_ref(v: &Value) -> Result<usize, String> {
    if let Some(n) = v.as_f64() {
        if n.fract() == 0.0 && (0.0..STRINGS as f64).contains(&n) {
            return Ok(n as usize);
        }
        return Err(format!("string {n} must be a whole number 0..=5 (0 = low E, 5 = high e)"));
    }
    if let Some(s) = v.as_str() {
        let t = s.trim();
        let by_letter = ["E", "A", "D", "G", "B", "e"].iter().position(|n| *n == t);
        let by_name = STRING_NAMES.iter().position(|n| n.eq_ignore_ascii_case(t));
        let by_word = match t.to_ascii_lowercase().as_str() {
            "low e" | "low-e" | "low_e" => Some(0),
            "high e" | "high-e" | "high_e" => Some(5),
            _ => None,
        };
        if let Some(i) = by_letter.or(by_name).or(by_word) {
            return Ok(i);
        }
        return Err(format!("unknown string \"{t}\" (use 0..=5 from the low E, or E A D G B e)"));
    }
    Err(format!("a string is a number 0..=5 or a name, not {v}"))
}

fn fret_value(v: &Value) -> Result<i8, String> {
    match v {
        Value::Null => Ok(X),
        Value::String(s) if s.eq_ignore_ascii_case("x") => Ok(X),
        Value::Number(_) => {
            let f = v.as_f64().unwrap_or(-2.0);
            if f == -1.0 {
                Ok(X)
            } else if f.fract() == 0.0 && (0.0..=MAX_FRET as f64).contains(&f) {
                Ok(f as i8)
            } else {
                Err(format!("fret {f} must be a whole number 0..={MAX_FRET} (or -1 / null / \"x\" for a string not played)"))
            }
        }
        Value::String(s) => s.parse::<i8>().map_err(|_| format!("fret \"{s}\" is not a number")).and_then(|f| fret_value(&json!(f))),
        other => Err(format!("a fret is a number, null or \"x\", not {other}")),
    }
}

/// A chord: a shape name, six frets low E to high e, or tab text.
pub fn chord_ref(v: &Value) -> Result<Chord, String> {
    match v {
        Value::String(s) => {
            if let Some(c) = chord_shape(s) {
                return Ok(c);
            }
            let t = s.trim();
            let tokens: Vec<&str> = if t.contains(['-', ' ', ',']) {
                t.split(['-', ' ', ',']).filter(|x| !x.is_empty()).collect()
            } else if t.len() == STRINGS && t.chars().all(|c| c.is_ascii_digit() || c == 'x' || c == 'X') {
                t.split("").filter(|x| !x.is_empty()).collect()
            } else {
                vec![]
            };
            if tokens.len() == STRINGS {
                let mut c = [X; STRINGS];
                for (i, tok) in tokens.iter().enumerate() {
                    c[i] = fret_value(&Value::String((*tok).to_string()))?;
                }
                return check_shape(c);
            }
            let names: Vec<&str> = SHAPES.iter().map(|(n, _)| *n).collect();
            Err(format!(
                "unknown chord \"{t}\": give six frets low E to high e ([3,2,0,0,0,3] or \"x32010\"), or one of {}",
                names.join(" ")
            ))
        }
        Value::Array(a) => {
            if a.len() != STRINGS {
                return Err(format!("a chord's frets are six values, low E to high e; got {}", a.len()));
            }
            let mut c = [X; STRINGS];
            for (i, f) in a.iter().enumerate() {
                c[i] = fret_value(f).map_err(|e| format!("string {i}: {e}"))?;
            }
            check_shape(c)
        }
        other => Err(format!("a chord is a name, six frets, or tab text, not {other}")),
    }
}

fn check_shape(c: Chord) -> Result<Chord, String> {
    if c.iter().all(|f| *f < 0) {
        return Err("that chord plays no strings".into());
    }
    Ok(c)
}

fn strings_list(v: Option<&Value>, ctx: &str) -> Result<[bool; STRINGS], String> {
    match v {
        None | Some(Value::Null) => Ok([true; STRINGS]),
        Some(Value::Array(a)) => {
            let mut sel = [false; STRINGS];
            for s in a {
                sel[string_ref(s).map_err(|e| format!("{ctx}: {e}"))?] = true;
            }
            Ok(sel)
        }
        Some(Value::String(s)) if s == "all" => Ok([true; STRINGS]),
        Some(other) => Ok({
            let mut sel = [false; STRINGS];
            sel[string_ref(other).map_err(|e| format!("{ctx}: {e}"))?] = true;
            sel
        }),
    }
}

/// Compile one event into `out`. Returns its beat and the beat it extends the score to.
fn event(i: usize, v: &Value, beats_per_s: f64, out: &mut Vec<Timed>) -> Result<(f64, f64), String> {
    let o = v
        .as_object()
        .ok_or_else(|| format!("events[{i}]: an event is an object like {{\"type\": \"pluck\", \"beat\": 0, \"string\": 0, \"fret\": 3}}"))?;
    let ty = o
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("events[{i}]: missing \"type\" (pluck, strum, mute, bend, vibrato or rest)"))?;
    let allowed: &[&str] = match ty {
        "pluck" => &["type", "beat", "string", "fret", "velocity", "palm"],
        "strum" => &["type", "beat", "chord", "dir", "velocity", "spread_ms", "strings", "palm"],
        "mute" => &["type", "beat", "strings"],
        "bend" => &["type", "beat", "string", "semitones"],
        "vibrato" => &["type", "beat", "string", "depth"],
        "rest" => &["type", "beat", "beats"],
        other => return Err(format!("events[{i}]: unknown type \"{other}\" (pluck, strum, mute, bend, vibrato or rest)")),
    };
    let ctx = format!("events[{i}] ({ty})");
    for k in o.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(format!("{ctx}: unknown field \"{k}\" (allowed: {})", allowed.join(", ")));
        }
    }
    let beat = num(o, "beat", &ctx)?.ok_or_else(|| format!("{ctx}: missing \"beat\" (quarter notes from 0)"))?;
    let beat = in_range(&ctx, "beat", beat, 0.0, MAX_BEATS)?;
    let string = |o: &Map<String, Value>| -> Result<usize, String> {
        string_ref(o.get("string").ok_or_else(|| format!("{ctx}: missing \"string\" (0 = low E … 5 = high e)"))?)
            .map_err(|e| format!("{ctx}: {e}"))
    };
    match ty {
        "pluck" => {
            let s = string(o)?;
            let fret = match o.get("fret") {
                None => 0,
                Some(f) => match fret_value(f).map_err(|e| format!("{ctx}: {e}"))? {
                    X => return Err(format!("{ctx}: a plucked string needs a fret 0..={MAX_FRET}")),
                    f => f as u8,
                },
            };
            let velocity = in_range(&ctx, "velocity", num(o, "velocity", &ctx)?.unwrap_or(0.75), 0.0, 1.0)?;
            let palm = boolean(o, "palm", &ctx)?.unwrap_or(false);
            out.push(Timed { beat, event: Event::Pluck { string: s as u8, fret, velocity: velocity as f32, palm } });
            Ok((beat, beat))
        }
        "strum" => {
            let chord = chord_ref(o.get("chord").ok_or_else(|| format!("{ctx}: missing \"chord\""))?)
                .map_err(|e| format!("{ctx}: {e}"))?;
            let down = match o.get("dir").and_then(Value::as_str) {
                None | Some("down") => true,
                Some("up") => false,
                Some(d) => return Err(format!("{ctx}: dir \"{d}\" must be \"down\" or \"up\"")),
            };
            if o.get("dir").is_some_and(|d| !d.is_string() && !d.is_null()) {
                return Err(format!("{ctx}: dir must be \"down\" or \"up\""));
            }
            let velocity = in_range(&ctx, "velocity", num(o, "velocity", &ctx)?.unwrap_or(0.8), 0.0, 1.0)?;
            let spread_ms = in_range(&ctx, "spread_ms", num(o, "spread_ms", &ctx)?.unwrap_or(30.0), 0.0, 1000.0)?;
            let sel = strings_list(o.get("strings"), &ctx)?;
            let palm = boolean(o, "palm", &ctx)?.unwrap_or(false);
            let order: Vec<usize> = if down { (0..STRINGS).collect() } else { (0..STRINGS).rev().collect() };
            let played: Vec<usize> = order.iter().copied().filter(|s| sel[*s] && chord[*s] >= 0).collect();
            if played.is_empty() {
                return Err(format!("{ctx}: the chord plays none of the chosen strings"));
            }
            let step_beats = if played.len() > 1 { spread_ms / 1000.0 * beats_per_s / (played.len() - 1) as f64 } else { 0.0 };
            for s in order.iter().copied().filter(|s| sel[*s] && chord[*s] < 0) {
                out.push(Timed { beat, event: Event::Mute { string: s as u8 } });
            }
            for (k, s) in played.iter().enumerate() {
                out.push(Timed {
                    beat: beat + k as f64 * step_beats,
                    event: Event::Pluck { string: *s as u8, fret: chord[*s] as u8, velocity: velocity as f32, palm },
                });
            }
            Ok((beat, beat + (played.len() - 1) as f64 * step_beats))
        }
        "mute" => {
            let sel = strings_list(o.get("strings"), &ctx)?;
            for (s, on) in sel.iter().enumerate() {
                if *on {
                    out.push(Timed { beat, event: Event::Mute { string: s as u8 } });
                }
            }
            Ok((beat, beat))
        }
        "bend" => {
            let s = string(o)?;
            let semis = num(o, "semitones", &ctx)?.ok_or_else(|| format!("{ctx}: missing \"semitones\" (0 releases, 2 is a whole step)"))?;
            let semis = in_range(&ctx, "semitones", semis, 0.0, 3.0)?;
            out.push(Timed { beat, event: Event::Bend { string: s as u8, semitones: semis as f32 } });
            Ok((beat, beat))
        }
        "vibrato" => {
            let s = string(o)?;
            let depth = num(o, "depth", &ctx)?.ok_or_else(|| format!("{ctx}: missing \"depth\" (semitones, 0 stops it; 0.15 is gentle)"))?;
            let depth = in_range(&ctx, "depth", depth, 0.0, 1.0)?;
            out.push(Timed { beat, event: Event::Vibrato { string: s as u8, depth: depth as f32 } });
            Ok((beat, beat))
        }
        _ => {
            let beats = in_range(&ctx, "beats", num(o, "beats", &ctx)?.unwrap_or(0.0), 0.0, MAX_BEATS)?;
            Ok((beat, beat + beats))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_kind() -> Value {
        json!({
            "name": "all kinds",
            "tempo": 120,
            "humanize": 0.25,
            "events": [
                {"type": "strum", "beat": 0, "chord": "C", "dir": "down", "spread_ms": 40, "velocity": 0.7},
                {"type": "strum", "beat": 1, "chord": [3, 2, 0, 0, 0, 3], "dir": "up", "strings": [3, 4, 5]},
                {"type": "strum", "beat": 2, "chord": "x-x-0-2-3-2", "palm": true},
                {"type": "strum", "beat": 2.5, "chord": "x02210"},
                {"type": "pluck", "beat": 3, "string": "B", "fret": 8, "velocity": 0.9},
                {"type": "bend", "beat": 3.25, "string": 4, "semitones": 2},
                {"type": "vibrato", "beat": 3.5, "string": "B3", "depth": 0.2},
                {"type": "mute", "beat": 5, "strings": ["E", "A"]},
                {"type": "mute", "beat": 5.5},
                {"type": "rest", "beat": 6, "beats": 3}
            ]
        })
    }

    #[test]
    fn every_kind_of_event_compiles_in_order() {
        let s = parse(&every_kind(), 92.0).unwrap();
        assert_eq!(s.tempo, 120.0);
        assert_eq!(s.written, 10);
        // C down: one mute (low E) + five plucks; G up on three strings; D: two mutes + four;
        // Am tab: one mute + five; pluck, bend, vibrato; two mutes; six mutes; rest adds nothing.
        assert_eq!(s.events.len(), 6 + 3 + 6 + 6 + 3 + 2 + 6);
        assert!(s.events.windows(2).all(|w| w[0].beat <= w[1].beat));
        // The rest at beat 6 lasting 3 beats ends at 9: the score runs to the next bar line, 12.
        assert_eq!(s.length_beats, 12.0);
        // A 40 ms strum at 120 BPM spans 0.08 beats over its five strings.
        let c: Vec<f64> = s.events.iter().filter(|t| t.beat < 0.5 && matches!(t.event, Event::Pluck { .. })).map(|t| t.beat).collect();
        assert_eq!(c.len(), 5);
        assert!((c[4] - 0.08).abs() < 1e-9, "{c:?}");
        // The upstroke starts on the high e.
        let up: Vec<u8> = s.events.iter().filter(|t| (1.0..1.5).contains(&t.beat)).map(|t| t.event.string()).collect();
        assert_eq!(up, vec![5, 4, 3]);
        // D with palm: every plucked string palm-muted.
        assert!(s
            .events
            .iter()
            .filter(|t| (2.0..2.4).contains(&t.beat))
            .all(|t| matches!(t.event, Event::Pluck { palm: true, .. } | Event::Mute { .. })));
    }

    #[test]
    fn a_compiled_score_round_trips_through_its_json() {
        let s = parse(&every_kind(), 92.0).unwrap();
        let again = parse(&s.to_json(), 60.0).unwrap();
        assert_eq!(again.events, s.events);
        assert_eq!((again.tempo, again.length_beats, again.looped, again.then, again.humanize), (s.tempo, s.length_beats, s.looped, s.then, s.humanize));
        // And once more through text.
        let text = serde_json::to_string(&again.to_json()).unwrap();
        assert_eq!(parse_str(&text, 60.0).unwrap().events, s.events);
    }

    #[test]
    fn defaults_fill_in_and_loops_keep_their_length() {
        let s = parse(&json!({"loop": true, "length_beats": 2, "events": [{"type": "pluck", "beat": 1.5, "string": 0}]}), 77.0).unwrap();
        assert_eq!(s.tempo, 77.0);
        assert!(s.looped);
        assert_eq!(s.then, Then::Resume);
        assert_eq!(s.length_beats, 2.0);
        assert_eq!(s.events[0].event, Event::Pluck { string: 0, fret: 0, velocity: 0.75, palm: false });
        let stop = parse(&json!({"then": "stop", "events": [{"type": "pluck", "beat": 4, "string": 1, "fret": 2}]}), 90.0).unwrap();
        assert_eq!(stop.then, Then::Stop);
        // An event on beat 4 opens the second bar: the score lasts two bars.
        assert_eq!(stop.length_beats, 8.0);
    }

    fn err(v: Value) -> String {
        parse(&v, 92.0).expect_err("should be rejected")
    }

    #[test]
    fn bad_scores_are_rejected_with_reasons() {
        let cases: Vec<(Value, &str)> = vec![
            (json!([1, 2]), "JSON object"),
            (json!({"events": []}), "empty"),
            (json!({"tempo": 300, "events": [{"type": "rest", "beat": 0}]}), "tempo 300"),
            (json!({"temp": 90, "events": []}), "unknown score field \"temp\""),
            (json!({"events": [{"type": "plunk", "beat": 0}]}), "unknown type \"plunk\""),
            (json!({"events": [{"beat": 0, "string": 1}]}), "missing \"type\""),
            (json!({"events": [{"type": "pluck", "string": 1}]}), "missing \"beat\""),
            (json!({"events": [{"type": "pluck", "beat": -1, "string": 1}]}), "beat -1"),
            (json!({"events": [{"type": "pluck", "beat": 0, "string": 6}]}), "string 6"),
            (json!({"events": [{"type": "pluck", "beat": 0, "string": "H"}]}), "unknown string \"H\""),
            (json!({"events": [{"type": "pluck", "beat": 0, "string": 1, "fret": 30}]}), "fret 30"),
            (json!({"events": [{"type": "pluck", "beat": 0, "string": 1, "fret": 2.5}]}), "fret 2.5"),
            (json!({"events": [{"type": "pluck", "beat": 0, "string": 1, "fert": 2}]}), "unknown field \"fert\""),
            (json!({"events": [{"type": "pluck", "beat": 0, "string": 1, "velocity": 2}]}), "velocity 2"),
            (json!({"events": [{"type": "strum", "beat": 0, "chord": "Hm7"}]}), "unknown chord \"Hm7\""),
            (json!({"events": [{"type": "strum", "beat": 0, "chord": [0, 2, 2]}]}), "six values"),
            (json!({"events": [{"type": "strum", "beat": 0, "chord": "C", "dir": "sideways"}]}), "dir \"sideways\""),
            (json!({"events": [{"type": "strum", "beat": 0, "chord": "D", "strings": [0, 1]}]}), "none of the chosen strings"),
            (json!({"events": [{"type": "bend", "beat": 0, "string": 3}]}), "missing \"semitones\""),
            (json!({"events": [{"type": "bend", "beat": 0, "string": 3, "semitones": 5}]}), "semitones 5"),
            (json!({"events": [{"type": "vibrato", "beat": 0, "string": 3, "depth": 3}]}), "depth 3"),
            (json!({"length_beats": 2, "events": [{"type": "pluck", "beat": 3, "string": 0}]}), "ends before"),
            (json!({"loop": true, "length_beats": 4, "events": [{"type": "pluck", "beat": 4, "string": 0}]}), "after the last event"),
            (json!({"then": "again", "events": [{"type": "rest", "beat": 0}]}), "\"then\""),
            (json!({"events": [{"type": "rest", "beat": 0}], "humanize": 2}), "humanize 2"),
        ];
        for (v, want) in cases {
            let e = err(v.clone());
            assert!(e.contains(want), "{v}: got {e:?}, wanted it to mention {want:?}");
        }
        let many: Vec<Value> = (0..MAX_EVENTS + 1).map(|i| json!({"type": "rest", "beat": i % 100})).collect();
        assert!(err(json!({"events": many})).contains("more than"));
    }
}
