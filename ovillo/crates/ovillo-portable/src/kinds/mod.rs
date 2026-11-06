//! The World kinds. Each module builds its kinds' catalog entries and their
//! [`crate::Source`]s; [`all`] lists them in catalog order.
//!
//! Adding a World is one module here that returns its `KindDef`s, and one
//! line in [`all`]; a `GameWorld` (`ovillo-game-sound`) is one entry in
//! [`game::kinds`]. The JS package needs no change: it reads names, units and
//! indices from the catalog.

pub mod bench;
pub mod fart;
pub mod game;
pub mod piano;
pub mod tab;
pub mod tract;
pub mod trombone;

use std::sync::OnceLock;

use crate::catalog::KindDef;

/// Every kind, in catalog order.
pub fn all() -> &'static [KindDef] {
    static ALL: OnceLock<Vec<KindDef>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut k = Vec::new();
        k.extend(tab::kinds());
        k.push(piano::kind());
        k.push(trombone::kind());
        k.push(tract::kind());
        k.push(fart::kind());
        k.push(bench::kind());
        k.extend(game::kinds());
        k
    })
}

pub fn find(id: &str) -> Option<&'static KindDef> {
    all().iter().find(|k| k.id == id)
}

/// Option helpers shared by the kinds.
pub(crate) mod opt {
    use serde_json::Value;

    pub fn f64_or(o: &Value, key: &str, default: f64) -> Result<f64, String> {
        match o.get(key) {
            None | Some(Value::Null) => Ok(default),
            Some(v) => v.as_f64().filter(|x| x.is_finite()).ok_or_else(|| format!("option {key}: expected a number, got {v}")),
        }
    }

    pub fn str_opt<'a>(o: &'a Value, key: &str) -> Result<Option<&'a str>, String> {
        match o.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.as_str())),
            Some(v) => Err(format!("option {key}: expected a string, got {v}")),
        }
    }

}

/// MIDI of a note name (`C4` = 60; `#`, `b` accidentals; `Bb6`, `F#2`).
pub(crate) fn midi_of(name: &str) -> Option<f64> {
    let name = name.trim();
    let mut chars = name.chars();
    let letter = match chars.next()?.to_ascii_uppercase() {
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
    let mut acc = 0i32;
    let mut digits = rest.as_str();
    while let Some(c) = digits.chars().next() {
        match c {
            '#' => acc += 1,
            'b' => acc -= 1,
            _ => break,
        }
        digits = &digits[1..];
    }
    let octave: i32 = digits.parse().ok()?;
    Some(f64::from(12 * (octave + 1) + letter + acc))
}

/// The written-dynamic scale a 0..1 velocity spans (`mus_audio.DYN_DB`: ppp .. fff).
pub(crate) fn velocity_db(velocity: f64) -> f64 {
    let table = mus_engine::ariadne_live::dynamics_db();
    let db = |n: &str| table.iter().find(|(k, _)| *k == n).map_or(0.0, |(_, d)| *d);
    let (lo, hi) = (db("ppp"), db("fff"));
    lo + (hi - lo) * velocity.clamp(0.0, 1.0)
}
