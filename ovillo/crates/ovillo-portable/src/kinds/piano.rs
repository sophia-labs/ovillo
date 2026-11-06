//! The piano: `anatomy=piano-v1` (`SPEC-ARIADNE-PIANO.md`), its World
//! composed for a set of keys, played live (`mus_engine::ariadne_live::LivePiano`):
//! a key lifts its damper, throws its hammer and lets the damper fall when
//! released, unless the pedal holds it.

use std::collections::BTreeSet;

use mus_engine::ariadne_live::{piano_key, piano_velocities, LivePiano, SAMPLE_RATE};
use serde_json::{json, Value};

use super::{midi_of, opt};
use crate::catalog::*;
use crate::{CreateCtx, Source, MAX_ARGS};

/// The default keys: a C chord and its octave (C4 E4 G4 C5). The World's
/// cost grows with every key strung; four keys is about what plays live on a
/// 2019 laptop (the catalog's `cost`); `keys` changes it.
const DEFAULT_KEYS: [&str; 4] = ["C4", "E4", "G4", "C5"];

pub fn kind() -> KindDef {
    KindDef {
        id: "piano".into(),
        title: "Grand piano".into(),
        family: "piano".into(),
        summary: "anatomy=piano-v1: a World shaped like a 190 cm grand, its soundboard with ribs and bridges and, for each key strung, its stiff wires, felt hammer and damper. A note is a key going down.".into(),
        spec: "SPEC-ARIADNE-PIANO.md".into(),
        sample_rate: Some(SAMPLE_RATE),
        channels: 2,
        anatomies: vec![anatomy("piano-v1", "Estonia-scaled parlour grand (piano-shaped, not a model of it)", "profiles/ariadne-piano-v1.anatomy.json", true)],
        open_anatomies: false,
        options: vec![
            option("keys", "array|string", json!(DEFAULT_KEYS), "the keys to string: MIDI numbers, note names, or a range like \"C3-C6\" (\"A0-C8\" is all 88). Only these keys sound (SPEC-ARIADNE-PIANO 5); the World's cost grows with every key (see cost)"),
            option("score", "string", Value::Null, "MUS text with an anatomy=piano-v1 instrument: the World is composed for the score's keys and plays its presses and pedal as mus render times them"),
            option("instrument", "string", Value::Null, "which of the score's pianos (abbrev; default the first)"),
        ],
        events: vec![
            event("key", "A key goes down: its damper lifts 15 ms before the hammer reaches the strings at `at` (a key pressed now sounds 15 ms later: the action's travel). The hammer is thrown at the velocity, its felt squeezed against the strings.", vec![
                arg("pitch", "MIDI", 21.0, 108.0, 60.0, "the key (it must be strung: option keys)"),
                arg("velocity", "0..1", 0.0, 1.0, 0.6, "the dynamic: ppp 0.34 m/s (0) to fff 5.1 m/s (1), geometric (the score's table); 0.6 is about mf"),
                arg("hammer", "m/s", 0.0, 6.0, 0.0, "hammer velocity at the string, overriding velocity when > 0"),
            ]),
            event("release", "The key comes up: its damper falls 8 ms later and settles over 12 ms, unless the pedal holds it.", vec![
                arg("pitch", "MIDI", 21.0, 108.0, 60.0, "the key"),
            ]),
            event("note", "key, then release after `duration`.", vec![
                arg("pitch", "MIDI", 21.0, 108.0, 60.0, "the key"),
                arg("velocity", "0..1", 0.0, 1.0, 0.6, "as key"),
                arg("duration", "s", 0.0, 60.0, 1.0, "how long the key is held"),
                arg("hammer", "m/s", 0.0, 6.0, 0.0, "as key"),
            ]),
        ],
        controls: vec![switch("pedal", Some(&["up", "down"]), 1.0, 0.0, "the sustain pedal: down holds every damper up")],
        telemetry: vec![
            slot("pedalDown", "0/1", "the pedal is down"),
            slot("hammersActive", "count", "hammers in flight or on the strings"),
            slot("strikes", "count", "throws that reached the strings"),
            slot("refusedStrikes", "count", "throws refused while the felt still pressed"),
            slot("damperFalls", "count", ""),
            slot("keysStrung", "count", ""),
        ],
        score: Some(ScoreDef { format: "mus".into(), about: "MUS staves and a sustain-pedal voice (SPEC-ARIADNE-PIANO 4): each note is a key, its dynamic the hammer's velocity.".into() }),
        output: OutputDef {
            observes: "far-field pressure (Pa) at two points 1 m from the board, 30 degrees off its normal, toward the bass and the treble".into(),
            gain: LivePiano::OBSERVATION_GAIN,
            channels: "two listening points".into(),
            about: "mus render mixes the same pressure, times the same gain, into its bus (with its 28 Hz high-pass and master); here there is neither".into(),
        },
        lookahead_seconds: 0.016,
        cost: "the heaviest World: 14 board bodies plus about 2.5 wires and a hammer per key, each wire with 14 bridge edges. Measured in node (wasm, 128-frame quanta, a 2019 8-core i9 laptop): 1 key 2.4x real time, 4 keys 1.1x, 8 keys 0.63x, 13 keys 0.41x, 25 keys 0.22x, 37 keys 0.15x. Live play wants few keys (or a bigger machine); scores render offline".into(),
        does_not_model: lines(&[
            "One transverse polarization: no horizontal string motion, no longitudinal waves or phantom partials, no tension modulation (the strings are linear).",
            "No sympathetic strings beyond the keys strung: with the pedal down the rest of the scale does not ring along.",
            "The action is a throw: no key, wippen, escapement dynamics, repetition lever, key bounce or action noise; a key cannot be re-thrown while its felt still presses (refused and counted).",
            "The board is flat and the frame rigid: no crown, downbearing, taper, rim, lid or frame resonances; board modes above 3 kHz are sampled every 4th.",
            "Radiation is baffled and diagonal in the modes; heard at two far-field points, dry, with no room.",
            "The felt is a power law with relaxation; its parameters away from C4 are design-hypotheses, as is the damper away from A#4.",
            "Pitch is set per string, alone: where a partial sits on a board resonance the coupled pitch moves (C4 -5.6 cents); the piano is not re-tuned after it is built.",
            "It is a piano-shaped Ariadne world, not a model of the Estonia or of any particular piano.",
        ]),
        does_not_model_ref: "SPEC-ARIADNE-PIANO.md#7-what-it-does-not-model-said-plainly".into(),
        build,
    }
}

/// `"C3-C6"`, `[60, 64, "G4"]`, `"A0-C8"`.
fn parse_keys(v: &Value) -> Result<BTreeSet<usize>, String> {
    let midi = |v: &Value| -> Result<f64, String> {
        match v {
            Value::Number(n) => n.as_f64().ok_or_else(|| "a key number".to_string()),
            Value::String(s) => midi_of(s).ok_or_else(|| format!("key {s:?}")),
            other => Err(format!("key {other}")),
        }
    };
    let key = |m: f64| piano_key(m).ok_or_else(|| format!("midi {m} is not a piano key (21..108)"));
    match v {
        Value::String(s) if s.contains('-') => {
            let (a, b) = s.split_once('-').ok_or("range")?;
            let (a, b) = (midi_of(a).ok_or(format!("key {a:?}"))?, midi_of(b).ok_or(format!("key {b:?}"))?);
            let (lo, hi) = (key(a.min(b))?, key(a.max(b))?);
            Ok((lo..=hi).collect())
        }
        Value::String(_) | Value::Number(_) => Ok([key(midi(v)?)?].into()),
        Value::Array(list) => list.iter().map(|x| key(midi(x)?)).collect(),
        other => Err(format!("option keys: {other}")),
    }
}

fn velocity_m_s(v: f64, hammer: f64) -> f64 {
    if hammer > 0.0 {
        return hammer;
    }
    let table = piano_velocities();
    let get = |n: &str| table.iter().find(|(k, _)| *k == n).map_or(1.6, |(_, x)| *x);
    let (lo, hi) = (get("ppp"), get("fff"));
    lo * (hi / lo).powf(v.clamp(0.0, 1.0))
}

struct PianoSource {
    live: LivePiano,
    keys: BTreeSet<usize>,
    origin: u64,
    instrument: Option<String>,
}

fn build(_def: &KindDef, ctx: &CreateCtx) -> Result<Box<dyn Source>, String> {
    let keys = parse_keys(ctx.options.get("keys").unwrap_or(&json!(DEFAULT_KEYS)))?;
    Ok(Box::new(PianoSource {
        live: LivePiano::new(&keys)?,
        keys,
        origin: 0,
        instrument: opt::str_opt(ctx.options, "instrument")?.map(str::to_owned),
    }))
}

impl PianoSource {
    fn local(&self, frame: u64) -> usize {
        frame.saturating_sub(self.origin) as usize
    }
}

impl Source for PianoSource {
    fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> Result<(), String> {
        self.live.process(left, right)?;
        let g = LivePiano::OBSERVATION_GAIN as f32;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            *l *= g;
            *r *= g;
        }
        Ok(())
    }

    fn event(&mut self, index: usize, a: &[f64; MAX_ARGS], at: u64, _now: u64) -> Result<(), String> {
        let t = self.local(at);
        match index {
            0 | 2 => {
                let hammer = if index == 0 { a[2] } else { a[3] };
                let v = velocity_m_s(a[1], hammer);
                if !self.live.key_down(t, a[0], v)? {
                    return Err(format!("key {} is not strung in this World (option keys)", a[0]));
                }
                if index == 2 {
                    let lead = (0.015 * SAMPLE_RATE).round() as usize;
                    let onset = t.max(self.live.now() + lead);
                    self.live.key_up(onset + (a[2].max(0.0) * SAMPLE_RATE) as usize, a[0])?;
                }
                Ok(())
            }
            1 => self.live.key_up(t, a[0]),
            _ => Err(format!("piano: no event {index}")),
        }
    }

    fn set(&mut self, index: usize, value: f64) -> Result<(), String> {
        match index {
            0 => {
                let now = self.live.now();
                self.live.pedal(now, value >= 0.5);
                Ok(())
            }
            _ => Err(format!("piano: no control {index}")),
        }
    }

    fn ledger(&self) -> Ledger {
        Ledger::of(self.live.world())
    }

    fn receipt(&self) -> Value {
        let mut r = self.live.receipt();
        r["originFrame"] = json!(self.origin);
        r
    }

    fn telemetry(&self, out: &mut [f64]) {
        let (strikes, refused, falls) = self.live.counts();
        out[0] = f64::from(u8::from(self.live.pedal_down()));
        out[1] = self.live.hammers_active() as f64;
        out[2] = strikes as f64;
        out[3] = refused as f64;
        out[4] = falls as f64;
        out[5] = self.live.key_count() as f64;
    }

    fn reset(&mut self) -> Result<(), String> {
        self.live = LivePiano::new(&self.keys)?;
        self.origin = 0;
        Ok(())
    }

    fn play_score(&mut self, text: &str, at: u64) -> Result<Value, String> {
        let now = self.origin + self.live.now() as u64;
        let (live, read) = LivePiano::from_score_at(text, self.instrument.as_deref(), at.saturating_sub(now) as usize)?;
        self.live = live;
        self.origin = now;
        Ok(json!({"format": "mus", "instrument": read.instrument, "samples": read.samples, "notes": read.notes, "badTokens": read.bad_tokens, "keys": self.live.strung_midi(), "startFrame": at}))
    }

    fn lookahead(&self) -> u64 {
        (0.016 * SAMPLE_RATE) as u64
    }

    fn plan_digest(&self) -> String {
        self.live.world().plan_digest().to_owned()
    }
}
