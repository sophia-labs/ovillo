//! The aigua/telar string-network voices (`mus_dsp`'s `synth=weave`/`pluck`) as
//! a polyphonic note-buffer voice.
//!
//! A note is rendered once, in full, into a slot ([`WeaveEngine::note_on`]),
//! and the engine mixes the active slots one sample at a time
//! ([`WeaveEngine::frame`]). Offline this is a convenience; on the audio thread
//! the same shape can be made allocation-free by rendering into a preallocated
//! pool, which is a later step (Phase 1b in the rig).

use std::collections::BTreeMap;

/// How many notes may sound at once before a new note steals the oldest.
pub const MAX_VOICES: usize = 8;

/// Per-note gain headroom: a full-velocity weave note already peaks near
/// 0 dBFS on its own, so several at once are scaled down here.
const HEADROOM: f32 = 0.35;

/// A `synth=weave`/`pluck` patch: the exact key/value pairs `mus_dsp::pluck`
/// reads (`bodyp`, `courses`, `dimension`, `chirality`, `orbit`, `bow`,
/// `breath`, `tension`, …) plus `synth` to choose the voice.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WeavePatch {
    pub params: BTreeMap<String, String>,
}

impl WeavePatch {
    fn put(mut self, key: &str, value: &str) -> Self {
        self.params.insert(key.to_string(), value.to_string());
        self
    }

    /// The "ml" (stated tune) instrument from the aigua/Gaily scores: a bowed,
    /// violin-ish string network.
    #[must_use]
    pub fn violin() -> Self {
        Self::default()
            .put("synth", "weave")
            .put("bodyp", "violin")
            .put("courses", "7")
            .put("dimension", "1.72")
            .put("couple", "0.065")
            .put("chirality", "0.52")
            .put("orbit", "0.13")
            .put("orbit_depth", "0.30")
            .put("curvature", "0.10")
            .put("bow", "0.18")
            .put("bow_speed", "0.42")
            .put("sus", "3.4")
            .put("damp", "0.25")
            .put("pos", "0.34")
            .put("pick", "0.34")
            .put("body", "0.68")
            .put("body_size", "0.96")
            .put("stiff", "0.08")
            .put("tension", "4")
            .put("detune", "3")
    }

    /// A plucked, guitar-ish string network (the aigua "gt" instrument).
    #[must_use]
    pub fn pluck_guitar() -> Self {
        Self::default()
            .put("synth", "pluck")
            .put("bodyp", "guitar")
            .put("sus", "2.8")
            .put("damp", "0.31")
            .put("pos", "0.24")
            .put("pick", "0.46")
            .put("body", "0.74")
            .put("body_size", "1.13")
            .put("stiff", "0.07")
            .put("tension", "4")
            .put("symp", "0.40")
            .put("detune", "2")
    }

    /// Override one key (e.g. `set("bodyp", "cello")`).
    pub fn set(&mut self, key: &str, value: &str) {
        self.params.insert(key.to_string(), value.to_string());
    }
}

#[derive(Default)]
struct Voice {
    active: bool,
    buf: Vec<f32>,
    pos: usize,
    gain: f32,
}

impl Voice {
    #[inline]
    fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }
}

/// A polyphonic `synth=weave`/`pluck` voice. Notes are rendered whole and
/// mixed per sample.
pub struct WeaveEngine {
    sr: f64,
    patch: WeavePatch,
    voices: [Voice; MAX_VOICES],
}

impl WeaveEngine {
    #[must_use]
    pub fn new(sr: f64) -> Self {
        Self {
            sr,
            patch: WeavePatch::violin(),
            voices: std::array::from_fn(|_| Voice::default()),
        }
    }

    #[must_use]
    pub fn sample_rate(&self) -> f64 {
        self.sr
    }

    #[must_use]
    pub fn patch(&self) -> &WeavePatch {
        &self.patch
    }

    pub fn set_patch(&mut self, patch: WeavePatch) {
        self.patch = patch;
    }

    /// Render `freqs` (Hz) as one note through the current patch and start it
    /// sounding. `velocity` is 0..1; `dur_s` is the note's slot length, after
    /// which the string network rings on by its own `sus`/`damp`.
    pub fn note_on(&mut self, freqs: &[f64], velocity: f32, dur_s: f64) {
        let synth = self.patch.params.get("synth").map(String::as_str).unwrap_or("weave");
        let buf = if synth == "pluck" {
            mus_dsp::pluck::pluck_note(&self.patch.params, freqs, dur_s, 1.0)
        } else {
            mus_dsp::pluck::weave_note(&self.patch.params, freqs, dur_s, 1.0)
        };
        let gain = velocity.clamp(0.0, 1.0) * HEADROOM;

        // Prefer a free slot; otherwise steal the one nearest the end of its note.
        let idx = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, v)| !v.active)
            .map(|(i, _)| i)
            .next()
            .unwrap_or_else(|| {
                (0..MAX_VOICES)
                    .min_by_key(|&i| self.voices[i].remaining())
                    .unwrap_or(0)
            });
        self.voices[idx] = Voice { active: true, buf, pos: 0, gain };
    }

    /// Stop everything (as a hand laid across the strings).
    pub fn all_off(&mut self) {
        for v in &mut self.voices {
            v.active = false;
        }
    }

    /// One mono sample: the sum of the notes sounding now.
    #[inline]
    pub fn frame(&mut self) -> f32 {
        let mut sum = 0.0_f32;
        for v in &mut self.voices {
            if !v.active {
                continue;
            }
            if v.pos < v.buf.len() {
                sum += v.buf[v.pos] * v.gain;
                v.pos += 1;
            } else {
                v.active = false;
            }
        }
        sum
    }

    /// How many notes are sounding.
    #[must_use]
    pub fn active(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chord_sounds_and_two_notes_mix() {
        let mut e = WeaveEngine::new(48_000.0);
        // A G minor-ish chord, two notes together, then measure.
        e.note_on(&[196.0], 0.7, 0.5);
        e.note_on(&[233.08, 293.66], 0.6, 0.5);
        assert_eq!(e.active(), 2);

        let mut peak = 0.0_f32;
        let mut non_silent = 0usize;
        let mut samples = 0usize;
        for _ in 0..48_000 {
            let s = e.frame();
            peak = peak.max(s.abs());
            if s.abs() > 1e-4 {
                non_silent += 1;
            }
            samples += 1;
        }
        assert!(peak > 0.01, "weave chord is silent: {peak}");
        assert!(non_silent > 1000, "only {non_silent}/{samples} samples non-silent");
        assert!(peak <= 1.0, "weave chord clips: {peak}");
    }

    #[test]
    fn notes_ring_out_then_fall_silent() {
        let mut e = WeaveEngine::new(48_000.0);
        e.note_on(&[440.0], 0.8, 0.2);
        // Drain past the note's natural decay (sus=3.4s in the violin patch).
        let mut last_peak = 0.0_f32;
        for i in 0..(48_000 * 5) {
            let s = e.frame();
            if i > 48_000 * 4 {
                last_peak = last_peak.max(s.abs());
            }
        }
        assert_eq!(e.active(), 0);
        assert!(last_peak < 1e-3, "note did not decay: {last_peak}");
    }
}
