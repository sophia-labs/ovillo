//! Ensembles of guts. A score is a directory:
//!
//! ```text
//! score.json                  title, voices (kind, octave, legato, pan, gain, anatomy), sections (id, bpm, beats, bar, voices),
//!                             direction (the house defaults every part starts with: wobble, vibrato, jitter, rasp)
//! <section>/<voice>.notes     one part, played as written on its own gut (see "Parts")
//! out/stems/<section>/<voice>.wav|.json   one rendered part and what the World measured (cached by content)
//! out/stems/<section>/<voice>.trace.f32   the World's observers at every 10 ms hop of that part (little-endian
//!                             f32 rows; the columns are listed in the .json's `trace`), for pictures
//! out/sections/<section>.wav, out/mix.wav, out/mix.json
//! ```
//!
//! # Parts
//!
//! A part is the player's hands on one `fart::Gut`, in time order, tokens
//! separated by spaces:
//!
//! ```text
//! G2@84:1      a note: target G2 (what it is meant to be), the valve's tension commanded at 84 Hz, one beat
//! G2:1         the same with the tension at the target's own frequency (the valve runs sharp of it)
//! Bb2@101:1/3  beats as fractions (triplets) or sums (1+1/2); `~` after the beats ties to the same note next
//! r:2          a rest (no push)          X:1   a cannon shot (cheeks clenched, then released)
//! push=0.7 cheeks=0.4 tone=0.75          supply 0..1, cheek aperture 0..1, adduction 0..1 (hold until changed)
//! wobble=0.3 vibrato=5.5/0.4             slow tension wander 0..1; vibrato rate Hz / depth semitones
//! jitter=1.5 rasp=0.6                    tension jittered +-semitones every hop; a partial clench near the exit
//!                                        while sounding (the jet there sputters), 0..1
//! tongue=on|off                          clench 50 ms between consecutive notes (a release pop each) or slur
//! eat=beans eat=beans*3 eat=soda,egg     food into the stomach (fart::FOODS)
//! digest=on|off                          the stomach pays the reservoir (it holds 1.5 J and starts full)
//! pre=4                                  seconds of silence before the first beat (eat and digest there)
//! |                                      a bar line (checked against the section's `bar`)
//! # ...                                  a comment (a `#` that starts a token: C#3 is a note)
//! ```
//!
//! Each part starts on a fresh gut: reservoir full, stomach empty, not
//! digesting, push 0.7, cheeks 0.4, tone 0.75, no wobble, tongued (or slurred
//! if the voice is `legato`), then the score's `direction` settings (if any)
//! as if they were the part's first tokens: a part that sets its own wobble,
//! vibrato, jitter or rasp overrides them. Nothing is fed or tuned for the player. Every
//! part is its own World with its own ledger; parts render in parallel.
//! The mix (constant-power pan, gains, sections end to end, peak-normalized)
//! is a listening choice applied to the parts' listener copies; it books
//! nothing.

use ovillo_tract_world::fart::{self, Event, Gut, HopTrace, NoteSpan, HOP_S};
use realfft::RealFftPlanner;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub const SR: f64 = 48_000.0;
/// Peak of the normalized mix (-1 dBFS).
pub const MIX_PEAK: f32 = 0.891;
/// The valve's commandable tension range (`fart_v1`'s FlowValveSpec).
pub const TENSION_RANGE: (f64, f64) = (25.0, 300.0);
/// Notes are measured from this far in (the attack is excluded).
const SETTLE_S: f64 = 0.06;
/// Part of every stem's cache key: change it when the engine changes.
const ENGINE: &str = concat!("ovillo-fart ", env!("CARGO_PKG_VERSION"), " fart_v1 perform-6");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Score {
    pub title: String,
    #[serde(default)]
    pub about: String,
    pub voices: BTreeMap<String, VoiceSpec>,
    pub sections: Vec<Section>,
    #[serde(default)]
    pub direction: Direction,
}

/// The house defaults every part starts with (the director's, not the
/// composers'): each is applied as a token before the part's own, so the
/// part's own settings win. Physics only: nothing is applied to the sound.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct Direction {
    /// Slow tension wander, 0..1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wobble: Option<f64>,
    /// Vibrato: rate Hz, depth semitones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vibrato: Option<[f64; 2]>,
    /// Tension jitter, +- semitones, redrawn every 10 ms hop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jitter: Option<f64>,
    /// A partial clench near the exit while sounding, 0..1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rasp: Option<f64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub about: String,
}

impl Direction {
    pub fn events(&self) -> Vec<Event> {
        let mut out = Vec::new();
        if let Some(w) = self.wobble {
            out.push(Event::Wobble(w));
        }
        if let Some([rate_hz, depth_semitones]) = self.vibrato {
            out.push(Event::Vibrato { rate_hz, depth_semitones });
        }
        if let Some(j) = self.jitter {
            out.push(Event::Jitter(j));
        }
        if let Some(r) = self.rasp {
            out.push(Event::Rasp(r));
        }
        out
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub id: String,
    pub bpm: f64,
    /// Every part of the section must last exactly this many beats (if
    /// absent: as many as each other).
    #[serde(default)]
    pub beats: Option<f64>,
    /// Beats per bar: every `|` in a part must fall on a whole bar.
    #[serde(default)]
    pub bar: Option<f64>,
    pub voices: Vec<String>,
    #[serde(default)]
    pub about: String,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceSpec {
    #[serde(default)]
    pub kind: Kind,
    /// Octaves added to every note's target and tension.
    #[serde(default)]
    pub octave: i32,
    /// Start slurred (tongue=off) instead of tongued.
    #[serde(default)]
    pub legato: bool,
    /// -1 left .. 1 right.
    #[serde(default)]
    pub pan: f64,
    /// The gut's anatomy (`fart::ANATOMIES`: soprano, alto, bass, cannon).
    #[serde(default = "alto")]
    pub anatomy: String,
    #[serde(default = "one")]
    pub gain: f64,
    #[serde(default)]
    pub about: String,
}

fn one() -> f64 {
    1.0
}

fn alto() -> String {
    "alto".into()
}

/// Informational: a cannon voice is a gut like any other (any part may fire `X`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Tune,
    Cannon,
}

/// One token of a part, where it was written.
#[derive(Debug, Clone)]
pub struct Token {
    pub line: usize,
    pub text: String,
    pub event: Event,
}

pub fn load(dir: &Path) -> Result<Score, String> {
    let path = dir.join("score.json");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let score: Score = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    for s in &score.sections {
        if !(s.bpm.is_finite() && s.bpm > 0.0) {
            return Err(format!("section {}: bpm must be positive", s.id));
        }
        for v in &s.voices {
            if !score.voices.contains_key(v) {
                return Err(format!("section {}: voice \"{v}\" is not in score.json's voices", s.id));
            }
        }
    }
    Ok(score)
}

pub fn part_path(dir: &Path, section: &str, voice: &str) -> PathBuf {
    dir.join(section).join(format!("{voice}.notes"))
}

fn number(text: &str) -> Option<f64> {
    text.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Beats: `1.5`, `3/2`, `1+1/3`.
fn beats(text: &str) -> Option<f64> {
    let total: Option<f64> = text
        .split('+')
        .map(|t| match t.split_once('/') {
            Some((n, d)) => number(n).zip(number(d)).filter(|(_, d)| *d != 0.0).map(|(n, d)| n / d),
            None => number(t),
        })
        .sum();
    total.filter(|b| *b > 0.0)
}

fn unit(name: &str, v: &str) -> Result<f64, String> {
    number(v).filter(|x| (0.0..=1.0).contains(x)).ok_or_else(|| format!("{name} takes 0..1, not \"{v}\""))
}

fn on_off(name: &str, v: &str) -> Result<bool, String> {
    match v {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(format!("{name} is on or off, not \"{v}\"")),
    }
}

/// One `key=value` token as events.
fn setting(key: &str, v: &str) -> Result<Vec<Event>, String> {
    Ok(match key {
        "push" => vec![Event::Push(unit(key, v)?)],
        "cheeks" => vec![Event::Cheeks(unit(key, v)?)],
        "tone" => vec![Event::Tone(unit(key, v)?)],
        "wobble" => vec![Event::Wobble(unit(key, v)?)],
        "vibrato" => {
            let (r, d) = v.split_once('/').ok_or("vibrato is rate/depth, e.g. 5.5/0.3")?;
            let (r, d) = number(r).zip(number(d)).ok_or("vibrato is rate/depth, e.g. 5.5/0.3")?;
            if !(0.1..=12.0).contains(&r) || !(0.0..=2.0).contains(&d) {
                return Err("vibrato: rate 0.1..12 Hz, depth 0..2 semitones".into());
            }
            vec![Event::Vibrato { rate_hz: r, depth_semitones: d }]
        }
        "jitter" => vec![Event::Jitter(number(v).filter(|x| (0.0..=12.0).contains(x)).ok_or_else(|| format!("jitter takes 0..12 semitones, not \"{v}\""))?)],
        "rasp" => vec![Event::Rasp(unit(key, v)?)],
        "tongue" => vec![Event::Tongue(on_off(key, v)?)],
        "digest" => vec![Event::Digest(on_off(key, v)?)],
        "pre" => vec![Event::Pre { seconds: number(v).filter(|s| (0.0..=30.0).contains(s)).ok_or("pre takes 0..30 seconds")? }],
        "eat" => {
            let mut out = Vec::new();
            for item in v.split(',') {
                let (name, count) = item.split_once('*').unwrap_or((item, "1"));
                let count: usize = count.parse().map_err(|_| format!("eat: \"{item}\" (food*count)"))?;
                let food = fart::food(name).ok_or_else(|| format!("eat: no food \"{name}\" ({})", fart::FOODS.map(|f| f.name).join(", ")))?;
                out.extend(std::iter::repeat(Event::Eat(food)).take(count.min(40)));
            }
            out
        }
        _ => return Err(format!("unknown setting \"{key}\" (push cheeks tone wobble vibrato jitter rasp tongue digest eat pre)")),
    })
}

/// Parse one part. Errors name the line and token. With `bar` (beats per
/// bar), every `|` must fall on a whole bar.
pub fn parse_part(text: &str, bpm: f64, spec: &VoiceSpec, bar: Option<f64>) -> Result<Vec<Token>, Vec<String>> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut errors = Vec::new();
    let shift = f64::from(spec.octave).exp2();
    let beat_s = 60.0 / bpm;
    let mut position = 0.0f64;
    let mut bars = 0usize;
    let mut tie_open = false;
    let mut started = false;
    for (i, line) in text.lines().enumerate() {
        let line_no = i + 1;
        // A comment begins at a `#` that starts a token (C#3 is a note).
        let code = line.match_indices('#').find(|(i, _)| *i == 0 || line[..*i].ends_with(char::is_whitespace)).map_or(line, |(i, _)| &line[..i]);
        for tok in code.split_whitespace() {
            if tok == "|" {
                bars += 1;
                if let Some(bar) = bar {
                    if (position - (position / bar).round() * bar).abs() > 1e-6 {
                        errors.push(format!("line {line_no}: bar line {bars} falls at beat {}, inside bar {} ({bar} beats a bar)", round(position, 3), (position / bar).floor() as usize + 1));
                    }
                }
                continue;
            }
            if let Some((key, v)) = tok.split_once('=') {
                if key == "pre" && started {
                    errors.push(format!("line {line_no}: pre= only before the first note or rest"));
                    continue;
                }
                match setting(key, v) {
                    Ok(events) => tokens.extend(events.into_iter().map(|event| Token { line: line_no, text: tok.to_owned(), event })),
                    Err(e) => errors.push(format!("line {line_no}: {e}")),
                }
                continue;
            }
            let tied = tok.ends_with('~');
            let body = tok.trim_end_matches('~');
            let Some((head, b)) = body.split_once(':') else {
                errors.push(format!("line {line_no}: \"{tok}\" needs :beats (G2@84:1, r:1, X:1, or key=value)"));
                continue;
            };
            let Some(b) = beats(b) else {
                errors.push(format!("line {line_no}: \"{tok}\": beats must be positive (1, 1/3, 1+1/2)"));
                continue;
            };
            started = true;
            let seconds = b * beat_s;
            position += b;
            let event = if head.eq_ignore_ascii_case("r") {
                Event::Rest { seconds }
            } else if head.eq_ignore_ascii_case("x") {
                Event::Note { target_hz: 0.0, tension_hz: 0.0, seconds, shot: true }
            } else {
                let (name, tension) = head.split_once('@').map_or((head, None), |(n, t)| (n, Some(t)));
                let Some(target) = fart::note_hz(name) else {
                    errors.push(format!("line {line_no}: \"{tok}\": \"{name}\" is not a note (G2, Eb2, F#3, or Hz)"));
                    continue;
                };
                let tension = match tension.map(number) {
                    None => target,
                    Some(Some(t)) => t,
                    Some(None) => {
                        errors.push(format!("line {line_no}: \"{tok}\": the tension after @ is a number of Hz"));
                        continue;
                    }
                };
                let tension = tension * shift;
                if !(TENSION_RANGE.0..=TENSION_RANGE.1).contains(&tension) {
                    errors.push(format!("line {line_no}: \"{tok}\": tension {tension:.1} Hz is outside the valve's {}-{} Hz", TENSION_RANGE.0, TENSION_RANGE.1));
                }
                Event::Note { target_hz: target * shift, tension_hz: tension, seconds, shot: false }
            };
            // A tie extends the previous note (one attack, its tension).
            let extended = match (tie_open, tokens.last_mut(), &event) {
                (true, Some(prev), Event::Note { target_hz, seconds, shot: false, .. }) => match &mut prev.event {
                    Event::Note { target_hz: t, seconds: s, shot: false, .. } if t == target_hz => {
                        *s += seconds;
                        true
                    }
                    _ => false,
                },
                _ => false,
            };
            if !extended {
                tokens.push(Token { line: line_no, text: tok.to_owned(), event });
            }
            tie_open = tied;
        }
    }
    if !tokens.iter().any(|t| matches!(t.event, Event::Note { .. } | Event::Rest { .. })) && errors.is_empty() {
        errors.push("no notes".into());
    }
    if errors.is_empty() {
        Ok(tokens)
    } else {
        Err(errors)
    }
}

fn beats_of(tokens: &[Token], bpm: f64) -> f64 {
    tokens
        .iter()
        .map(|t| match t.event {
            Event::Note { seconds, .. } | Event::Rest { seconds } => seconds,
            _ => 0.0,
        })
        .sum::<f64>()
        * bpm
        / 60.0
}

/// Which parts to touch.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub section: Option<String>,
    pub voice: Option<String>,
}

impl Filter {
    fn wants(&self, section: &str, voice: &str) -> bool {
        self.section.as_deref().map_or(true, |s| s == section) && self.voice.as_deref().map_or(true, |v| v == voice)
    }
}

/// Parse without rendering: errors by line, beats against the section, bar
/// lines, tensions inside the valve's range, and what the part eats.
pub fn check(dir: &Path, score: &Score, filter: &Filter) -> Value {
    let mut sections = Vec::new();
    let mut problems = 0usize;
    for s in &score.sections {
        if filter.section.as_deref().is_some_and(|f| f != s.id) {
            continue;
        }
        let mut parts = Vec::new();
        let mut lengths = Vec::new();
        for v in &s.voices {
            let spec = &score.voices[v];
            let path = part_path(dir, &s.id, v);
            let mut part = json!({"voice": v});
            match std::fs::read_to_string(&path) {
                Err(_) => {
                    part["error"] = json!(format!("missing {}", path.display()));
                    problems += 1;
                }
                Ok(text) => match parse_part(&text, s.bpm, spec, s.bar) {
                    Err(errs) => {
                        problems += errs.len();
                        part["errors"] = json!(errs);
                    }
                    Ok(tokens) => {
                        let b = beats_of(&tokens, s.bpm);
                        lengths.push((v.clone(), b));
                        let notes = tokens.iter().filter(|t| matches!(t.event, Event::Note { .. })).count();
                        let eaten: f64 = tokens.iter().filter_map(|t| if let Event::Eat(f) = t.event { Some(f.gas_j) } else { None }).sum();
                        let digests = tokens.iter().any(|t| matches!(t.event, Event::Digest(true)));
                        part["notes"] = json!(notes);
                        part["beats"] = json!(round(b, 3));
                        part["seconds"] = json!(round(b * 60.0 / s.bpm, 2));
                        part["eatsJ"] = json!(round(eaten, 3));
                        part["digests"] = json!(digests);
                        if eaten > 0.0 && !digests {
                            part["warning"] = json!("eats but never digest=on: the food stays in the stomach");
                        }
                    }
                },
            }
            if filter.wants(&s.id, v) {
                parts.push(part);
            }
        }
        let target = s.beats.or_else(|| lengths.iter().map(|l| l.1).fold(None, |m: Option<f64>, b| Some(m.map_or(b, |m| m.max(b)))));
        let mut misaligned = Vec::new();
        if let Some(target) = target {
            for (v, b) in &lengths {
                if (b - target).abs() > 1e-6 {
                    misaligned.push(format!("{v}: {} beats, the section has {}", round(*b, 3), round(target, 3)));
                }
            }
        }
        problems += misaligned.len();
        let mut sec = json!({"section": s.id, "bpm": s.bpm, "beats": target.map(|b| round(b, 3)), "parts": parts});
        if !misaligned.is_empty() {
            sec["misaligned"] = json!(misaligned);
        }
        sections.push(sec);
    }
    json!({"ok": problems == 0, "problems": problems, "sections": sections})
}

fn round(v: f64, digits: i32) -> f64 {
    let k = 10f64.powi(digits);
    (v * k).round() / k
}

/// FNV-1a, 64-bit: a stable cache key.
fn fnv(parts: &[&str]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for p in parts {
        for b in p.bytes().chain(std::iter::once(0)) {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{h:016x}")
}

fn stem_paths(dir: &Path, section: &str, voice: &str) -> (PathBuf, PathBuf) {
    let base = dir.join("out").join("stems").join(section);
    (base.join(format!("{voice}.wav")), base.join(format!("{voice}.json")))
}

pub fn write_wav(path: &Path, channels: u16, samples: &[f32]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let spec = hound::WavSpec { channels, sample_rate: SR as u32, bits_per_sample: 32, sample_format: hound::SampleFormat::Float };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &s in samples {
        w.write_sample(s).map_err(|e| e.to_string())?;
    }
    w.finalize().map_err(|e| e.to_string())
}

fn read_wav(path: &Path) -> Result<Vec<f32>, String> {
    let mut r = hound::WavReader::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    r.samples::<f32>().collect::<Result<_, _>>().map_err(|e| e.to_string())
}

/// Timbre of a stretch of the listener's copy.
#[derive(Debug, Clone, Copy, Default)]
pub struct Timbre {
    pub rms_db: f64,
    /// 99.9th percentile of |x[n] - x[n-1]| over its RMS: how impulsive the
    /// valve's pulses are (a sine is 1.4; held fart_v0 notes measured 6.8 to
    /// 10.3; single-sample clicks at 85 Hz would be about 17).
    pub sharpness: f64,
    /// Spectral centroid (Hz): brightness.
    pub centroid_hz: f64,
    /// Energy above 4 kHz, fraction.
    pub above_4k: f64,
}

pub fn timbre(x: &[f32]) -> Timbre {
    if x.len() < 64 {
        return Timbre::default();
    }
    let rms = (x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len() as f64).sqrt();
    let mut d: Vec<f64> = x.windows(2).map(|w| f64::from(w[1] - w[0]).abs()).collect();
    let d_rms = (d.iter().map(|v| v * v).sum::<f64>() / d.len() as f64).sqrt().max(1e-12);
    d.sort_by(f64::total_cmp);
    let p999 = d[((d.len() as f64 * 0.999) as usize).min(d.len() - 1)];
    // The largest power of two that fits (at most 8192), from the middle.
    let mut n = 1usize;
    while n * 2 <= x.len().min(8192) {
        n *= 2;
    }
    let start = (x.len() - n) / 2;
    let mut planner = RealFftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(n);
    let mut input: Vec<f64> = (0..n).map(|k| f64::from(x[start + k]) * (0.5 - 0.5 * (std::f64::consts::TAU * k as f64 / n as f64).cos())).collect();
    let mut spectrum = fft.make_output_vec();
    let _ = fft.process(&mut input, &mut spectrum);
    let (mut total, mut moment, mut high) = (0.0, 0.0, 0.0);
    for (k, c) in spectrum.iter().enumerate() {
        let p = c.norm_sqr();
        let f = k as f64 * SR / n as f64;
        total += p;
        moment += p * f;
        if f > 4000.0 {
            high += p;
        }
    }
    Timbre {
        rms_db: 20.0 * rms.max(1e-9).log10(),
        sharpness: p999 / d_rms,
        centroid_hz: if total > 0.0 { moment / total } else { 0.0 },
        above_4k: if total > 0.0 { high / total } else { 0.0 },
    }
}

/// Per note: what it was meant to be, what was commanded, what the World
/// measured (F0 after the attack, how much of it swung, the gas when it
/// started), and how it sounded (level, attack pop, sharpness, brightness).
fn note_report(tok: &Token, span: &NoteSpan, trace: &[HopTrace], audio: &[f32]) -> Value {
    let hop = (HOP_S * SR) as usize;
    let settle = (SETTLE_S / HOP_S).round() as usize;
    let body = (span.start_hop + settle).min(span.end_hop)..span.end_hop;
    let mut f0s: Vec<f64> = trace[body.clone()].iter().map(|t| t.f0_hz).filter(|f| *f > 0.0).collect();
    let swung = if body.is_empty() { 0.0 } else { f0s.len() as f64 / body.len() as f64 };
    f0s.sort_by(f64::total_cmp);
    let measured = f0s.get(f0s.len() / 2).copied();
    let all = &audio[(span.start_hop * hop).min(audio.len())..(span.end_hop * hop).min(audio.len())];
    let attack = &all[..all.len().min((0.04 * SR) as usize)];
    let rest = &all[attack.len()..];
    let t = timbre(if rest.len() > 256 { rest } else { all });
    let rms = 10f64.powf(t.rms_db / 20.0);
    let pop = attack.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    let before = span.start_hop.checked_sub(1).map_or(0, |h| trace[h].clipped);
    let clipped = trace[(span.end_hop.max(1) - 1).min(trace.len() - 1)].clipped - before;
    let gas = span.start_hop.checked_sub(1).map_or(1.0, |h| trace[h].gas_fraction);
    let mut r = json!({
        "line": tok.line, "token": tok.text, "atS": round(span.start_hop as f64 * HOP_S, 2), "gas": round(gas, 3),
        "rmsDb": round(t.rms_db, 1), "popDb": round(20.0 * (f64::from(pop) / rms.max(1e-9)).max(1e-9).log10(), 1),
        "sharpness": round(t.sharpness, 1), "centroidHz": round(t.centroid_hz, 0), "clipped": clipped,
    });
    if !span.shot {
        r["targetHz"] = json!(round(span.target_hz, 1));
        r["tensionHz"] = json!(round(span.tension_hz, 1));
        r["measuredHz"] = json!(measured.map(|m| round(m, 1)));
        r["cents"] = json!(measured.map(|m| round(1200.0 * (m / span.target_hz).log2(), 0)));
        r["swung"] = json!(round(swung, 2));
    }
    r
}

/// The trace file's columns, in order: the scalars, then one area (m²) and
/// one RMS pressure (Pa) per oral section, valve to exit.
pub const TRACE_SCALARS: [&str; 24] = [
    "f0Hz", "gasFraction", "supplyPa", "clipped", "reservoirJ", "stomachJ", "gapMinM", "gapMaxM", "flowM3S", "fieldJ", "radiatedJ", "turbulenceJ",
    "jetApertureM2", "jetPosition", "push", "cheeks", "tone", "tensionHz", "clench", "rasp", "digest", "sound", "shot", "eatenJ",
];

/// One hop as a trace row (little-endian f32).
fn trace_row(t: &HopTrace, out: &mut Vec<u8>) {
    let b = |v: f64| (v as f32).to_le_bytes();
    let flag = |x: bool| if x { 1.0 } else { 0.0 };
    for v in [
        t.f0_hz, t.gas_fraction, t.supply_pa, t.clipped as f64, t.reservoir_j, t.stomach_j, t.gap_min_m, t.gap_max_m, t.flow_m3_s, t.field_j, t.radiated_j,
        t.turbulence_j, t.jet_aperture_m2, t.jet_position, t.push, t.cheeks, t.tone, t.tension_hz, flag(t.clench), t.rasp, flag(t.digest), flag(t.sound),
        flag(t.shot), t.eaten_j,
    ] {
        out.extend_from_slice(&b(v));
    }
    for a in &t.areas_m2 {
        out.extend_from_slice(&a.to_le_bytes());
    }
    for p in &t.pressure_rms_pa {
        out.extend_from_slice(&p.to_le_bytes());
    }
}

/// Render one part to its stem; returns its report.
fn render_stem(dir: &Path, section: &Section, voice: &str, spec: &VoiceSpec, direction: &Direction, key: &str, text: &str) -> Result<Value, String> {
    let tokens = parse_part(text, section.bpm, spec, section.bar).map_err(|e| e.join("; "))?;
    let mut events = vec![Event::Tongue(!spec.legato)];
    events.extend(direction.events());
    let lead = events.len();
    events.extend(tokens.iter().map(|t| t.event));
    let anatomy = fart::anatomy(&spec.anatomy).ok_or_else(|| format!("voice {voice}: no anatomy \"{}\" ({})", spec.anatomy, fart::ANATOMIES.join(", ")))?;
    let mut gut = Gut::with_anatomy(SR, anatomy).map_err(|e| format!("{e:?}"))?;
    let p = gut.perform(&events).map_err(|e| format!("{e:?}"))?;
    let (wav, json_path) = stem_paths(dir, &section.id, voice);
    // perform() reports indices into `events`, `lead` past the tokens' (the tongue and the direction).
    let notes: Vec<Value> = p.notes.iter().map(|span| note_report(&tokens[span.event - lead], span, &p.trace, &p.audio)).collect();
    let sections = p.trace.first().map_or(0, |t| t.areas_m2.len());
    let mut trace = Vec::with_capacity(p.trace.len() * (TRACE_SCALARS.len() + 2 * sections) * 4);
    for t in &p.trace {
        trace_row(t, &mut trace);
    }
    let trace_path = wav.with_extension("trace.f32");
    let pitched: Vec<&Value> = notes.iter().filter(|n| !n["targetHz"].is_null()).collect();
    let cents: Vec<f64> = pitched.iter().map(|n| n["cents"].as_f64().map_or(1200.0, f64::abs)).collect();
    let in_tune = cents.iter().filter(|c| **c <= 50.0).count();
    let silent = pitched.iter().filter(|n| n["measuredHz"].is_null()).count();
    let dry = notes.iter().filter(|n| n["gas"].as_f64().unwrap_or(1.0) < 0.05).count();
    let mean = |key: &str| if notes.is_empty() { 0.0 } else { round(notes.iter().filter_map(|n| n[key].as_f64()).sum::<f64>() / notes.len() as f64, 1) };
    let mut by_cents: Vec<&Value> = pitched.clone();
    by_cents.sort_by(|a, b| b["cents"].as_f64().map_or(1e9, f64::abs).total_cmp(&a["cents"].as_f64().map_or(1e9, f64::abs)));
    let mut by_pop: Vec<&Value> = notes.iter().collect();
    by_pop.sort_by(|a, b| b["popDb"].as_f64().unwrap_or(0.0).total_cmp(&a["popDb"].as_f64().unwrap_or(0.0)));
    let whole = timbre(&p.audio);
    // Sharpness and brightness over sounding notes: silence has neither.
    let median_over_notes = |key: &str| {
        let mut v: Vec<f64> = notes.iter().filter(|n| n["rmsDb"].as_f64().unwrap_or(-200.0) > -60.0).filter_map(|n| n[key].as_f64()).collect();
        v.sort_by(f64::total_cmp);
        v.get(v.len() / 2).map(|x| round(*x, 1))
    };
    let r = gut.world().world().receipt();
    let report = json!({
        "key": key, "section": section.id, "voice": voice,
        "seconds": round(p.audio.len() as f64 / SR, 3),
        "direction": direction,
        "anatomy": {"name": anatomy.name, "oralLengthM": anatomy.oral_length_m, "oralSections": anatomy.oral_sections, "valveMassKg": anatomy.valve.mass_kg,
            "valveLengthM": anatomy.valve.length_m, "lungCapacityJ": anatomy.lung_capacity_j, "maxSupplyPa": anatomy.valve.max_supply_pressure_pa,
            "tuningHz": [anatomy.valve.min_tuning_hz, anatomy.valve.max_tuning_hz]},
        "trace": {"file": trace_path.file_name().unwrap().to_str(), "hopS": HOP_S, "rows": p.trace.len(), "oralSections": sections,
            "columns": TRACE_SCALARS.iter().map(|s| s.to_string()).chain((0..sections).map(|i| format!("areaM2[{i}]"))).chain((0..sections).map(|i| format!("pressureRmsPa[{i}]"))).collect::<Vec<_>>()},
        "inTune50Cents": format!("{in_tune}/{}", pitched.len()),
        "meanAbsCents": if cents.is_empty() { 0.0 } else { round(cents.iter().sum::<f64>() / cents.len() as f64, 1) },
        "silentNotes": silent, "notesStartedDry": dry, "shots": notes.len() - pitched.len(),
        "gasEnd": round(gut.gas_fraction(), 3), "stomachLeftJ": round(gut.stomach().iter().map(|(_, l)| l.max(0.0)).sum::<f64>(), 3) + 0.0,
        "rmsDb": round(whole.rms_db, 1), "sharpness": median_over_notes("sharpness"), "centroidHz": median_over_notes("centroidHz"), "above4k": round(whole.above_4k, 4),
        "meanPopDb": mean("popDb"),
        "peak": p.audio.iter().fold(0.0f32, |m, v| m.max(v.abs())),
        "clippedSamples": gut.world().observers().safety_limited,
        "worstTuning": by_cents.into_iter().take(5).cloned().collect::<Vec<_>>(),
        "worstPops": by_pop.into_iter().take(3).cloned().collect::<Vec<_>>(),
        "books": {
            "sourceWorkJ": r.source_work, "radiationJ": r.radiation, "internalLossJ": r.internal_loss,
            "maximumFrameResidualJ": r.maximum_frame_residual, "cumulativeClosureResidualJ": r.cumulative_closure_residual,
        },
        "notes": notes,
    });
    write_wav(&wav, 1, &p.audio)?;
    std::fs::write(&trace_path, &trace).map_err(|e| e.to_string())?;
    std::fs::write(&json_path, serde_json::to_string_pretty(&report).unwrap()).map_err(|e| e.to_string())?;
    Ok(report)
}

/// The short form of a stem report.
fn brief(r: &Value, cached: bool) -> Value {
    let mut b = json!({"cached": cached});
    for k in ["section", "voice", "seconds", "inTune50Cents", "meanAbsCents", "silentNotes", "notesStartedDry", "shots", "gasEnd", "stomachLeftJ",
        "rmsDb", "sharpness", "centroidHz", "meanPopDb", "clippedSamples", "worstTuning", "worstPops"]
    {
        b[k] = r[k].clone();
    }
    b["maximumFrameResidualJ"] = r["books"]["maximumFrameResidualJ"].clone();
    b
}

/// Render the parts `filter` selects (unchanged ones come from the cache
/// unless `force`), on `jobs` threads.
pub fn render(dir: &Path, score: &Score, filter: &Filter, force: bool, jobs: usize) -> Value {
    struct Job<'a> {
        section: &'a Section,
        voice: &'a str,
    }
    let mut work = Vec::new();
    for s in &score.sections {
        for v in &s.voices {
            if filter.wants(&s.id, v) {
                work.push(Job { section: s, voice: v });
            }
        }
    }
    let next = AtomicUsize::new(0);
    let results = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1).min(work.len().max(1)) {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some(job) = work.get(i) else { break };
                let spec = &score.voices[job.voice];
                let path = part_path(dir, &job.section.id, job.voice);
                let outcome = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display())).and_then(|text| {
                    let key = fnv(&[ENGINE, &job.section.bpm.to_string(), &format!("{:?}", job.section.bar), &serde_json::to_string(spec).unwrap(),
                        &serde_json::to_string(&score.direction).unwrap(), &text]);
                    let (wav, json_path) = stem_paths(dir, &job.section.id, job.voice);
                    if !force && wav.exists() {
                        if let Some(old) = std::fs::read_to_string(&json_path).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
                            if old["key"] == key {
                                return Ok(brief(&old, true));
                            }
                        }
                    }
                    render_stem(dir, job.section, job.voice, spec, &score.direction, &key, &text).map(|r| brief(&r, false))
                });
                let entry = outcome.unwrap_or_else(|e| json!({"section": job.section.id, "voice": job.voice, "error": e}));
                results.lock().unwrap().push((i, entry));
            });
        }
    });
    let mut results = results.into_inner().unwrap();
    results.sort_by_key(|r| r.0);
    let stems: Vec<Value> = results.into_iter().map(|r| r.1).collect();
    let errors = stems.iter().filter(|s| !s["error"].is_null()).count();
    json!({"ok": errors == 0, "errors": errors, "rendered": stems.iter().filter(|s| s["cached"] == false).count(), "stems": stems})
}

/// The full per-note table of a rendered part (from its stem report).
pub fn analyze_part(dir: &Path, section: &str, voice: &str) -> Result<Value, String> {
    let (_, json_path) = stem_paths(dir, section, voice);
    let text = std::fs::read_to_string(&json_path).map_err(|_| format!("{section}/{voice} is not rendered yet"))?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// Timbre of a section's mix, in half-second windows (the sharpest sounding
/// ones first), and overall.
pub fn analyze_section(dir: &Path, section: &str) -> Result<Value, String> {
    let path = dir.join("out").join("sections").join(format!("{section}.wav"));
    let stereo = read_wav(&path).map_err(|_| format!("section {section} is not mixed yet (render it without --no-mix)"))?;
    let mono: Vec<f32> = stereo.chunks(2).map(|c| 0.5 * (c[0] + c.get(1).copied().unwrap_or(c[0]))).collect();
    let win = (0.5 * SR) as usize;
    let windows: Vec<(f64, Timbre)> = mono.chunks(win).enumerate().filter(|(_, c)| c.len() > win / 2).map(|(k, c)| (k as f64 * 0.5, timbre(c))).collect();
    // The sharpest sounding windows (silence has no pulses to be sharp).
    let mut sharp: Vec<(f64, Timbre)> = windows.iter().filter(|(_, t)| t.rms_db > -45.0).copied().collect();
    sharp.sort_by(|a, b| b.1.sharpness.total_cmp(&a.1.sharpness));
    let all = timbre(&mono);
    let w = |(t, m): &(f64, Timbre)| json!({"atS": t, "rmsDb": round(m.rms_db, 1), "sharpness": round(m.sharpness, 1), "centroidHz": round(m.centroid_hz, 0)});
    Ok(json!({
        "section": section, "seconds": round(mono.len() as f64 / SR, 2),
        "rmsDb": round(all.rms_db, 1), "sharpness": round(all.sharpness, 1), "centroidHz": round(all.centroid_hz, 0), "above4k": round(all.above_4k, 4),
        "sharpestWindows": sharp.iter().take(4).map(w).collect::<Vec<_>>(),
        "windows": windows.iter().map(w).collect::<Vec<_>>(),
    }))
}

/// The instrument's answer at given settings: for tensions from `from` to
/// `to` Hz, a held note on a practice gut (fed by the practice room, its own
/// World): measured F0 and timbre. Measurement only; what to command is the
/// player's decision.
pub fn probe(push: f64, cheeks: f64, tone: f64, wobble: f64, from: f64, to: f64, steps: usize) -> Result<Value, String> {
    probe_on("alto", &Direction::default(), push, cheeks, tone, wobble, from, to, steps)
}

/// [`probe`] on a named anatomy (`fart::ANATOMIES`) with a score's direction
/// applied first (so the measurement is the instrument a part really plays).
#[allow(clippy::too_many_arguments)]
pub fn probe_on(anatomy: &str, direction: &Direction, push: f64, cheeks: f64, tone: f64, wobble: f64, from: f64, to: f64, steps: usize) -> Result<Value, String> {
    let a = fart::anatomy(anatomy).ok_or_else(|| format!("no anatomy \"{anatomy}\" ({})", fart::ANATOMIES.join(", ")))?;
    let mut gut = Gut::with_anatomy(SR, a).map_err(|e| format!("{e:?}"))?;
    gut.house_w = 1.0;
    let steps = steps.clamp(2, 64);
    let mut events = vec![Event::Digest(true), Event::Tongue(false)];
    events.extend(direction.events());
    events.extend([Event::Push(push), Event::Cheeks(cheeks), Event::Tone(tone)]);
    if wobble > 0.0 {
        events.push(Event::Wobble(wobble));
    }
    for k in 0..steps {
        let t = from * (to / from).powf(k as f64 / (steps - 1) as f64);
        events.push(Event::Note { target_hz: t, tension_hz: t, seconds: 0.6, shot: false });
    }
    let p = gut.perform(&events).map_err(|e| format!("{e:?}"))?;
    let hop = (HOP_S * SR) as usize;
    let rows: Vec<Value> = p
        .notes
        .iter()
        .map(|span| {
            let body = (span.start_hop + 25).min(span.end_hop)..span.end_hop;
            let mut f0s: Vec<f64> = p.trace[body.clone()].iter().map(|t| t.f0_hz).filter(|f| *f > 0.0).collect();
            f0s.sort_by(f64::total_cmp);
            let m = f0s.get(f0s.len() / 2).copied();
            let t = timbre(&p.audio[body.start * hop..body.end * hop]);
            json!({"tensionHz": round(span.tension_hz, 1), "measuredHz": m.map(|m| round(m, 1)),
                "note": m.map(note_name), "centsFromTension": m.map(|m| round(1200.0 * (m / span.tension_hz).log2(), 0)),
                "rmsDb": round(t.rms_db, 1), "sharpness": round(t.sharpness, 1), "centroidHz": round(t.centroid_hz, 0)})
        })
        .collect();
    Ok(json!({"anatomy": anatomy, "direction": direction, "push": push, "cheeks": cheeks, "tone": tone, "wobble": wobble, "rows": rows}))
}

/// The nearest equal-tempered note and its offset in cents, e.g. "G2+12".
pub fn note_name(hz: f64) -> String {
    const NAMES: [&str; 12] = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];
    let midi = 69.0 + 12.0 * (hz / 440.0).log2();
    let n = midi.round();
    let cents = ((midi - n) * 100.0).round();
    let n = n as i64;
    format!("{}{}{:+}", NAMES[n.rem_euclid(12) as usize], n.div_euclid(12) - 1, cents)
}

fn section_beats(dir: &Path, score: &Score, s: &Section) -> f64 {
    s.beats.unwrap_or_else(|| {
        s.voices
            .iter()
            .filter_map(|v| std::fs::read_to_string(part_path(dir, &s.id, v)).ok().and_then(|t| parse_part(&t, s.bpm, &score.voices[v], s.bar).ok()))
            .map(|t| beats_of(&t, s.bpm))
            .fold(0.0, f64::max)
    })
}

/// Mix every section whose stems all exist, end to end, into a stereo
/// `out/mix.wav` (peak-normalized to [`MIX_PEAK`]) and `out/sections/*.wav`;
/// with `only`, just that section's `out/sections/<id>.wav` (no full mix).
pub fn mix(dir: &Path, score: &Score, only: Option<&str>) -> Result<Value, String> {
    let mut sections = Vec::new();
    let mut offset = 0usize;
    let mut full: Vec<f32> = Vec::new();
    let mut missing = Vec::new();
    for s in &score.sections {
        if only.is_some_and(|o| o != s.id) {
            offset += 2 * (section_beats(dir, score, s) * 60.0 / s.bpm * SR).round() as usize;
            continue;
        }
        let mut buf: Vec<f32> = Vec::new();
        let mut complete = true;
        for v in &s.voices {
            let spec = &score.voices[v];
            let (wav, _) = stem_paths(dir, &s.id, v);
            let Ok(mono) = read_wav(&wav) else {
                complete = false;
                missing.push(format!("{}/{v}", s.id));
                continue;
            };
            if buf.len() < 2 * mono.len() {
                buf.resize(2 * mono.len(), 0.0);
            }
            let theta = (spec.pan.clamp(-1.0, 1.0) + 1.0) * std::f64::consts::FRAC_PI_4;
            let (gl, gr) = ((spec.gain * theta.cos()) as f32, (spec.gain * theta.sin()) as f32);
            for (k, x) in mono.iter().enumerate() {
                buf[2 * k] += x * gl;
                buf[2 * k + 1] += x * gr;
            }
        }
        let length_s = section_beats(dir, score, s) * 60.0 / s.bpm;
        if complete && !buf.is_empty() {
            let peak = buf.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            let norm = if peak > 0.0 { MIX_PEAK / peak } else { 1.0 };
            let normalized: Vec<f32> = buf.iter().map(|v| v * norm).collect();
            write_wav(&dir.join("out").join("sections").join(format!("{}.wav", s.id)), 2, &normalized)?;
            let end = offset + buf.len();
            if full.len() < end {
                full.resize(end, 0.0);
            }
            for (k, v) in buf.iter().enumerate() {
                full[offset + k] += v;
            }
        }
        sections.push(json!({"section": s.id, "complete": complete, "startS": round(offset as f64 / 2.0 / SR, 3), "seconds": round(length_s, 3)}));
        offset += 2 * (length_s * SR).round() as usize;
    }
    if only.is_some() {
        return Ok(json!({"title": score.title, "sections": sections, "missingStems": missing, "seconds": sections.first().map(|s| s["seconds"].clone()),
            "mix": dir.join("out").join("sections").join(format!("{}.wav", only.unwrap_or_default())).display().to_string()}));
    }
    let peak = full.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    let gain = if peak > 0.0 { MIX_PEAK / peak } else { 1.0 };
    for v in &mut full {
        *v *= gain;
    }
    let path = dir.join("out").join("mix.wav");
    if !full.is_empty() {
        write_wav(&path, 2, &full)?;
    }
    let summary = json!({"title": score.title, "mix": path.display().to_string(), "seconds": round(full.len() as f64 / 2.0 / SR, 2),
        "normalizationGain": gain, "sections": sections, "missingStems": missing});
    std::fs::write(dir.join("out").join("mix.json"), serde_json::to_string_pretty(&summary).unwrap()).map_err(|e| e.to_string())?;
    Ok(summary)
}
