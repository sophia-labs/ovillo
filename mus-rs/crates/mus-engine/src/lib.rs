//! `mus-engine` — the render orchestrator, at full semantic parity with
//! `mus_audio.render()`: headers, voices (sample or synth), the bar map,
//! the per-track event walk (dynamics, hairpins, pattern repetition,
//! gesture quotation, chords), the transform chain, sidechain-aware
//! mixing, and the reverb/highpass/master bus. Every DSP primitive is
//! `mus-dsp`'s (sample-parity-tested against the Python oracle); this
//! crate's own job is the glue that calls them in the oracle's order.
//!
//! WF1's reduced-scope engine (rubato resampling, refusing synth/vocoder/
//! sidechain/reverb/chop/ring/glow rather than rendering them) is
//! superseded here — this stage's whole point is to stop refusing those
//! constructs and render them instead.

pub mod ariadne_piano;
pub mod ariadne_tab;
mod events;
mod gestures;
mod layout;
mod mixbus;
mod pack;
mod rawbars;
pub mod receipt;
mod source;
pub mod tab_check;
mod transforms;
mod util;

use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::path::Path;

use mus_dsp::trajectory::StringTrajectoryReceipt;
use serde::Serialize;
use sha2::{Digest, Sha256};

pub const SAMPLE_RATE: u32 = 48_000;
pub const ENGINE_VERSION: &str = "mus-engine/0.2.0-wf8";

#[derive(Debug)]
pub enum RenderError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Wav(String),
    Invalid(String),
}

impl Display for RenderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Json(e) => write!(f, "manifest error: {e}"),
            Self::Wav(e) => write!(f, "audio file error: {e}"),
            Self::Invalid(e) => write!(f, "invalid score: {e}"),
        }
    }
}

impl std::error::Error for RenderError {}
impl From<std::io::Error> for RenderError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<serde_json::Error> for RenderError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderedAudio {
    /// Interleaved stereo frames, left then right.
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

impl RenderedAudio {
    pub fn frames(&self) -> usize {
        self.samples.len() / 2
    }

    pub fn peak(&self) -> f32 {
        self.samples.iter().map(|v| v.abs()).fold(0.0_f32, f32::max)
    }

    pub fn rms(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        (self.samples.iter().map(|v| v * v).sum::<f32>() / self.samples.len() as f32).sqrt()
    }

    pub fn peak_dbfs(&self) -> f32 {
        dbfs(self.peak())
    }

    pub fn rms_dbfs(&self) -> f32 {
        dbfs(self.rms())
    }
}

fn dbfs(value: f32) -> f32 {
    20.0 * value.max(1e-12).log10()
}

/// One canonical trajectory execution joined to its score placement.
///
/// The embedded adapter receipt keeps its body-neutral path digest unchanged.
/// Bar, track, onset, and slot belong to the score-event envelope rather than
/// the trajectory's executable identity.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTrajectoryReceipt {
    pub bar: u32,
    pub track: String,
    pub onset_seconds: f64,
    pub slot_seconds: f64,
    pub receipt: StringTrajectoryReceipt,
}

/// Everything the render receipt wants beyond peak/RMS: `mus_audio.py`'s
/// own `stats` dict (notes/skipped/bad, lines 874-1179) extended with the
/// per-track and sidechain counts the WF8 spec asks for.
#[derive(Debug, Clone, Default)]
pub struct RenderStats {
    pub notes: u32,
    pub skipped: u32,
    pub bad: Vec<String>,
    /// Missing-source track warnings (`mus_audio.py` line 757) — kept
    /// separate from `bad`, matching the oracle's own separate stderr
    /// stream for these.
    pub warnings: Vec<String>,
    /// Structurally empty on the historical scalar path.
    pub trajectory_receipts: Vec<EventTrajectoryReceipt>,
    pub per_track_events: BTreeMap<String, u32>,
    pub sidechain_hits: usize,
    pub voices: usize,
    pub total_seconds: f64,
    pub tuning_a: f64,
    /// One receipt per instrument played on an Ariadne world.
    pub ariadne_worlds: Vec<AriadneReceipt>,
    /// Per-body stored energy through each such render (for visualizers).
    pub ariadne_energy: Vec<AriadneTrace>,
}

/// An Ariadne world's receipt: an annotated-tab instrument's or a piano's.
/// Untagged: each serializes exactly as its own struct.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum AriadneReceipt {
    Tab(ariadne_tab::AriadneWorldReceipt),
    Piano(ariadne_piano::PianoWorldReceipt),
}

/// An Ariadne world's energy trace (untagged, as [`AriadneReceipt`]).
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum AriadneTrace {
    Tab(ariadne_tab::EnergyTrace),
    Piano(ariadne_piano::PianoEnergyTrace),
}

#[derive(Debug, Clone)]
pub struct RenderOutcome {
    pub audio: RenderedAudio,
    pub stats: RenderStats,
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Re-time bars `from..=to` to start at zero, keeping the full score's
/// per-bar durations and the standard 4 s render tail. Bars are renumbered
/// `1..=(to-from+1)` so the walk (which iterates `1..=n_bars`) sees them at
/// their original relative positions. The full range reproduces the input
/// map/content unchanged (start of bar 1 is already zero).
type BarContent = BTreeMap<u32, BTreeMap<String, String>>;

fn slice_bars(
    full: layout::BarMap,
    content: &BarContent,
    from: u32,
    to: u32,
) -> Result<(layout::BarMap, BarContent, u32), RenderError> {
    let n = full.bars.len() as u32;
    if from == 0 || to < from || to > n {
        return Err(RenderError::Invalid(format!(
            "bar range {from}..{to} is outside 1..={n}"
        )));
    }
    let start = full.bars[&from].start_s;
    let mut bars = BTreeMap::new();
    for b in from..=to {
        let t = &full.bars[&b];
        bars.insert(
            b - from + 1,
            layout::BarTiming {
                start_s: t.start_s - start,
                qlen: t.qlen,
                spq: t.spq,
            },
        );
    }
    let last = &full.bars[&to];
    let total_s = (last.start_s + last.qlen * last.spq) - start + 4.0;
    let bar_map = layout::BarMap { bars, total_s };

    let mut bar_content = BTreeMap::new();
    for (&b, tracks) in content {
        if (from..=to).contains(&b) {
            bar_content.insert(b - from + 1, tracks.clone());
        }
    }
    Ok((bar_map, bar_content, to - from + 1))
}

/// Render a score to full sample parity with `mus_audio.render()`
/// (`mus_audio.py` lines 717-1181). `source` is the score text; assets
/// (`pack`, `gestures`, `tape`, `sample=`) resolve relative to
/// `score_dir`.
pub fn render(source: &str, score_dir: &Path) -> Result<RenderOutcome, RenderError> {
    render_inner(source, score_dir, None)
}

/// Render only bars `from..=to` (1-based, inclusive), re-timed to start at
/// zero and given the same 4 s tail as a full render (`mus render --bars`).
/// The full range (`1..=n_bars`) is bit-identical to [`render`].
pub fn render_bars(
    source: &str,
    score_dir: &Path,
    from: u32,
    to: u32,
) -> Result<RenderOutcome, RenderError> {
    render_inner(source, score_dir, Some((from, to)))
}

fn render_inner(
    source: &str,
    score_dir: &Path,
    bar_range: Option<(u32, u32)>,
) -> Result<RenderOutcome, RenderError> {
    StagedRender::begin_inner(source, score_dir, bar_range)?.finish()
}

/// A render taken in stages: [`StagedRender::begin`] reads the score, walks
/// the voices and builds every Ariadne world; [`StagedRender::step`] simulates
/// the worlds a block of samples at a time; [`StagedRender::finish`] mixes,
/// runs the bus and returns what [`render`] returns. [`render`] is exactly
/// `begin` then `finish`, so a staged render is the same render, sample for
/// sample, whatever the block sizes (`mus-wasm` steps it to report progress).
pub struct StagedRender {
    buffers: mixbus::MixBuffers,
    placed: Vec<mixbus::PlacedNote>,
    sc_onsets: Vec<f64>,
    sidechain: Option<layout::Sidechain>,
    reverb_s: f64,
    master_rms_db: f64,
    n_total: usize,
    stats: RenderStats,
    worlds: Vec<ariadne_tab::WorldRun>,
    tab_instruments: usize,
    /// Pianos (SPEC-ARIADNE-PIANO.md): each plays its whole performance on
    /// one world at [`StagedRender::finish`], after the tab worlds and before
    /// the mixdown, as `render` always ran them; not stepped.
    pianos: Option<PianoPass>,
}

/// What [`ariadne_piano::render`] reads, kept from `begin` for `finish`.
struct PianoPass {
    instruments: Vec<ariadne_piano::PianoInstrument>,
    bar_map: layout::BarMap,
    bar_content: BarContent,
    n_bars: u32,
    swing_unit_ql: f64,
    swing_r: f64,
}

impl StagedRender {
    pub fn begin(source: &str, score_dir: &Path) -> Result<Self, RenderError> {
        Self::begin_inner(source, score_dir, None)
    }

    pub fn begin_bars(
        source: &str,
        score_dir: &Path,
        from: u32,
        to: u32,
    ) -> Result<Self, RenderError> {
        Self::begin_inner(source, score_dir, Some((from, to)))
    }

    fn begin_inner(
        source: &str,
        score_dir: &Path,
        bar_range: Option<(u32, u32)>,
    ) -> Result<Self, RenderError> {
        let (proposal, _diags) = mus_text::parse_score_lossy(source);
        let headers = &proposal.headers;

        // ---- headers (lines 721-733, 776-788, 844-860, 1153-1166)
        let a4 = layout::parse_tuning_a4(headers.get("tuning"))?;
        let (swing_unit_ql, swing_r) = layout::parse_swing(headers.get("swing"));
        let sidechain = layout::parse_sidechain(headers.get("sidechain"));
        let reverb_s = layout::parse_reverb_seconds(headers.get("reverb"))?;
        let master_rms_db = layout::parse_master_rms(headers.get("master"))?;

        // ---- pack / gestures / tape (lines 727-743)
        let pack = match headers.get("pack") {
            Some(rel) => Some(pack::Pack::load(&score_dir.join(rel))?),
            None => None,
        };
        let gestures = match headers.get("gestures") {
            Some(rel) => gestures::load_gestures(&score_dir.join(rel))?,
            None => BTreeMap::new(),
        };
        let cache = pack::SampleCache::new();
        let tape_audio: Option<Vec<f32>> = match headers.get("tape") {
            Some(rel) => Some((*cache.load(&score_dir.join(rel))?).clone()),
            None => None,
        };

        // ---- voices (lines 745-774)
        // Instruments that declare an `anatomy=` are played on an Ariadne world,
        // with their dot-voices as hands (SPEC-TAB.md) or, for a piano, as staves
        // and pedal (SPEC-ARIADNE-PIANO.md); everything else is a voice.
        let (tab_instruments, tab_owned) = ariadne_tab::collect(&proposal.instruments, score_dir)?;
        let (piano_instruments, piano_owned) =
            ariadne_piano::collect(&proposal.instruments, score_dir)?;
        let voice_instruments: Vec<_> = proposal
            .instruments
            .iter()
            .filter(|i| !tab_owned.contains(&i.abbrev) && !piano_owned.contains(&i.abbrev))
            .cloned()
            .collect();
        let (voices, warnings) =
            pack::build_voices(&voice_instruments, pack.as_ref(), score_dir, a4)?;

        // ---- bar map (lines 798-826). The inline tempo/time overrides come
        // from a fresh raw-text bar scan, not `proposal.bar_changes` — see
        // `rawbars`'s module doc for why.
        let abbrevs: Vec<String> = proposal
            .instruments
            .iter()
            .map(|i| i.abbrev.clone())
            .collect();
        let raw = rawbars::scan(source, &abbrevs);
        // `if not bar_count and bars: bar_count = max(...)` (mus.py line
        // 1326-1327): Python's `not bar_count` is also true for an explicit
        // `# bars: 0`, so a literal `0` header falls through to the bar-lines
        // count exactly like a missing/unparseable header does.
        let n_bars = headers
            .get("bars")
            .and_then(|v| v.trim().parse::<u32>().ok())
            .filter(|&n| n != 0)
            .unwrap_or(raw.max_bar);
        let bar_map = layout::build_bar_map(
            headers.get("tempo"),
            headers.get("time"),
            &raw.inline_tempo,
            &raw.inline_time,
            n_bars,
        )?;
        let (bar_map, bar_content, n_bars) = match bar_range {
            Some((from, to)) => slice_bars(bar_map, &raw.bar_content, from, to)?,
            None => (bar_map, raw.bar_content, n_bars),
        };
        let n_total = (bar_map.total_s * mus_dsp::SR_F64) as usize;

        // ---- the event walk, track by track (lines 862-1138)
        let eager = sidechain.is_none();
        let mut buffers = mixbus::MixBuffers::new(n_total);
        let mut placed = Vec::new();
        let mut sc_onsets = Vec::new();
        let mut stats = RenderStats {
            voices: voices.len(),
            total_seconds: bar_map.total_s,
            tuning_a: a4,
            warnings,
            ..Default::default()
        };

        for voice in &voices {
            let ctx = events::WalkContext {
                bar_map: &bar_map,
                bar_content: &bar_content,
                n_bars,
                swing_unit_ql,
                swing_r,
                a4,
                gestures: &gestures,
                tape_audio: tape_audio.as_deref(),
                cache: &cache,
                n_total,
                eager,
                sc_track: sidechain.as_ref().map(|s| s.track.as_str()),
            };
            events::walk_track(
                &ctx,
                voice,
                &mut buffers,
                &mut placed,
                &mut sc_onsets,
                &mut stats,
            )?;
        }

        // ---- annotated-tab instruments: one persistent world each
        let worlds = ariadne_tab::prepare(
            &tab_instruments,
            &bar_map,
            &bar_content,
            n_bars,
            swing_unit_ql,
            swing_r,
            mus_dsp::SR_F64,
            n_total,
            &mut stats,
        )?;
        let pianos = (!piano_instruments.is_empty()).then(|| PianoPass {
            instruments: piano_instruments,
            bar_map,
            bar_content,
            n_bars,
            swing_unit_ql,
            swing_r,
        });
        Ok(Self {
            buffers,
            placed,
            sc_onsets,
            sidechain,
            reverb_s,
            master_rms_db,
            n_total,
            stats,
            worlds,
            tab_instruments: tab_instruments.len(),
            pianos,
        })
    }

    /// What the score walk has counted so far (events, bad tokens, warnings);
    /// the worlds' receipts join it at [`StagedRender::finish`].
    pub fn stats(&self) -> &RenderStats {
        &self.stats
    }

    /// Length of the render, in sample frames.
    pub fn frames(&self) -> usize {
        self.n_total
    }

    /// World samples to simulate, all worlds together.
    pub fn world_samples(&self) -> usize {
        self.worlds.iter().map(|w| w.samples()).sum()
    }

    /// World samples simulated so far.
    pub fn world_samples_done(&self) -> usize {
        self.worlds.iter().map(|w| w.done()).sum()
    }

    /// The tab instruments played on worlds, in render order.
    pub fn world_instruments(&self) -> Vec<String> {
        self.worlds
            .iter()
            .map(|w| w.instrument().to_owned())
            .collect()
    }

    /// Simulate up to `max` more world samples (worlds run one after another);
    /// returns true once every world has run to the end.
    pub fn step(&mut self, mut max: usize) -> Result<bool, RenderError> {
        for world in self.worlds.iter_mut() {
            if max == 0 {
                break;
            }
            if !world.is_finished() {
                max -= world.step(max)?;
            }
        }
        Ok(self.worlds.iter().all(|w| w.is_finished()))
    }

    /// Run whatever is left, mix, and finish the bus (reverb, high-pass, master).
    pub fn finish(mut self) -> Result<RenderOutcome, RenderError> {
        for world in std::mem::take(&mut self.worlds) {
            world.finish(&mut self.buffers, &mut self.stats)?;
        }
        let mut stats = self.stats;
        stats.voices += self.tab_instruments;

        // ---- pianos: one persistent world each
        if let Some(p) = self.pianos.take() {
            ariadne_piano::render(
                &p.instruments,
                &p.bar_map,
                &p.bar_content,
                p.n_bars,
                p.swing_unit_ql,
                p.swing_r,
                mus_dsp::SR_F64,
                &mut self.buffers,
                &mut stats,
            )?;
            stats.voices += p.instruments.len();
        }

        // ---- deferred mixdown + sidechain (lines 1140-1148)
        let sidechain = self.sidechain.as_ref();
        let sidechain_hits = mixbus::deferred_mixdown(
            &mut self.buffers,
            &self.placed,
            sidechain.map(|s| s.track.as_str()),
            &self.sc_onsets,
            sidechain.map(|s| s.depth).unwrap_or(0.75),
            sidechain.map(|s| s.rel).unwrap_or(0.16),
            self.n_total,
        );
        stats.sidechain_hits = sidechain_hits;

        // ---- reverb + highpass + master (lines 1150-1167)
        let out = mixbus::finish_bus(
            self.buffers,
            self.reverb_s,
            self.master_rms_db,
            self.n_total,
        );

        let mut samples = Vec::with_capacity(self.n_total * 2);
        for (&l, &r) in out[0].iter().zip(out[1].iter()) {
            samples.push(l);
            samples.push(r);
        }
        Ok(RenderOutcome {
            audio: RenderedAudio {
                samples,
                sample_rate: SAMPLE_RATE,
            },
            stats,
        })
    }
}

/// One voice's dry contribution rendered alone: post-voice, post-event
/// gain/dynamics (stereo folded to mono), pre-send, no sidechain, no
/// reverb, no master. The spatial scene renderer is the intended
/// consumer — it brings its own room, placement, and loudness, so a stem
/// must not carry the score's.
#[derive(Debug, Clone)]
pub struct Stem {
    pub abbrev: String,
    /// Mono samples at [`SAMPLE_RATE`].
    pub samples: Vec<f32>,
}

/// Render a score as one dry mono stem per instrument, for spatial
/// placement. Deliberately NOT the parity path: sidechain is ignored
/// (the pump is a mix-bus phenomenon; stems are sources), the reverb and
/// master stages never run, and each voice walks into its own buffers.
/// The setup prelude mirrors [`render`]'s on purpose — the parity-locked
/// function stays untouched rather than being refactored under it.
pub fn render_stems(
    source: &str,
    score_dir: &Path,
) -> Result<(Vec<Stem>, RenderStats), RenderError> {
    let (proposal, _diags) = mus_text::parse_score_lossy(source);
    let headers = &proposal.headers;
    let a4 = layout::parse_tuning_a4(headers.get("tuning"))?;
    let (swing_unit_ql, swing_r) = layout::parse_swing(headers.get("swing"));
    let pack = match headers.get("pack") {
        Some(rel) => Some(pack::Pack::load(&score_dir.join(rel))?),
        None => None,
    };
    let gestures = match headers.get("gestures") {
        Some(rel) => gestures::load_gestures(&score_dir.join(rel))?,
        None => BTreeMap::new(),
    };
    let cache = pack::SampleCache::new();
    let tape_audio: Option<Vec<f32>> = match headers.get("tape") {
        Some(rel) => Some((*cache.load(&score_dir.join(rel))?).clone()),
        None => None,
    };
    // Tab instruments and pianos are played on Ariadne worlds, not voiced; they have no stem yet.
    let (_, tab_owned) = ariadne_tab::collect(&proposal.instruments, score_dir)?;
    let (_, piano_owned) = ariadne_piano::collect(&proposal.instruments, score_dir)?;
    let voice_instruments: Vec<_> = proposal
        .instruments
        .iter()
        .filter(|i| !tab_owned.contains(&i.abbrev) && !piano_owned.contains(&i.abbrev))
        .cloned()
        .collect();
    let (voices, warnings) = pack::build_voices(&voice_instruments, pack.as_ref(), score_dir, a4)?;
    let abbrevs: Vec<String> = proposal
        .instruments
        .iter()
        .map(|i| i.abbrev.clone())
        .collect();
    let raw = rawbars::scan(source, &abbrevs);
    let n_bars = headers
        .get("bars")
        .and_then(|v| v.trim().parse::<u32>().ok())
        .filter(|&n| n != 0)
        .unwrap_or(raw.max_bar);
    let bar_map = layout::build_bar_map(
        headers.get("tempo"),
        headers.get("time"),
        &raw.inline_tempo,
        &raw.inline_time,
        n_bars,
    )?;
    let n_total = (bar_map.total_s * mus_dsp::SR_F64) as usize;

    let mut stats = RenderStats {
        voices: voices.len(),
        total_seconds: bar_map.total_s,
        tuning_a: a4,
        warnings,
        ..Default::default()
    };
    let mut stems = Vec::with_capacity(voices.len());
    for voice in &voices {
        let ctx = events::WalkContext {
            bar_map: &bar_map,
            bar_content: &raw.bar_content,
            n_bars,
            swing_unit_ql,
            swing_r,
            a4,
            gestures: &gestures,
            tape_audio: tape_audio.as_deref(),
            cache: &cache,
            n_total,
            eager: true,
            sc_track: None,
        };
        let mut buffers = mixbus::MixBuffers::new(n_total);
        let mut placed = Vec::new();
        let mut sc_onsets = Vec::new();
        events::walk_track(
            &ctx,
            voice,
            &mut buffers,
            &mut placed,
            &mut sc_onsets,
            &mut stats,
        )?;
        // Equal-power pan folded back to mono: a centered event sums to
        // sqrt(2)·x, a hard-panned one to 1·x — a ≤1.5 dB seam accepted
        // so the stem carries the event's full energy either way. The
        // scene renderer re-places everything; score-side pan is not a
        // spatial truth worth preserving here.
        let mono: Vec<f32> = buffers.dry[0]
            .iter()
            .zip(buffers.dry[1].iter())
            .map(|(&left, &right)| left + right)
            .collect();
        stems.push(Stem {
            abbrev: voice.abbrev.clone(),
            samples: mono,
        });
    }
    Ok((stems, stats))
}

/// Write one mono stem as 32-bit FLOAT — lossless, and the format the
/// spatial scene loader (`pitch::decode_mono`, a librosa-parity subset:
/// Int16 or Float32 only) actually accepts. Stems are intermediates for
/// re-placement, not deliverables, so they skip [`write_wav`]'s PCM_24
/// oracle conversion entirely.
pub fn write_wav_mono(samples: &[f32], path: &Path) -> Result<(), RenderError> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|e| RenderError::Wav(format!("{}: {e}", path.display())))?;
    for &value in samples {
        writer
            .write_sample(value)
            .map_err(|e| RenderError::Wav(e.to_string()))?;
    }
    writer
        .finalize()
        .map_err(|e| RenderError::Wav(e.to_string()))?;
    Ok(())
}

/// `float × 2²³`, floored, clipped to the 24-bit signed range —
/// `soundfile`'s (`libsndfile`) actual `PCM_24` write conversion.
///
/// **This is `floor`, not the round-to-nearest-even
/// `SPECS/audio/WF8-orchestrator.md`'s Output section describes.** That
/// prose turned out not to match the executable oracle, which
/// `SPECS/audio/README.md` names as the actual arbiter ("the oracle is
/// executable"); flagging the discrepancy here rather than silently
/// picking a side. The previous attempt at this function also produced
/// `floor` and was rejected for it, but on inspection its supporting
/// test was unsound, not just circular: several of its per-fraction
/// cases (`0.1`, `0.25`, `0.4`, …) were built as `(target as f64 + frac)
/// as f32` at `target ≈ 1_000_000`, a magnitude where f32 has only ~3-4
/// fractional mantissa bits left — most of those fractions silently
/// round to a *different* value during that cast, before the conversion
/// under test ever sees them, so the table wasn't the evidence it
/// claimed to be. Redone here with values constructed to be *exactly*
/// representable in f32 (see `tests::half_boundary`/`tests::
/// sixteenth_above`), verified against real `soundfile` 0.14.0 /
/// `libsndfile` 1.2.2 output (both mono and interleaved-stereo array
/// shapes, matching `write_wav`'s actual layout):
///
/// - Every `m + 0.5` boundary floors to `m` on the positive side —
///   *regardless* of whether `m` or `m+1` is even, which rules out both
///   round-half-to-even and round-half-up (either would round at least
///   one of `1_000_000.5`/`1_000_001.5` up; neither does).
/// - The negative side floors toward `-infinity` (`-(m+0.5)` → `-(m+1)`),
///   not toward zero, which rules out a truncating cast.
/// - A dense sweep of every sixteenth across a whole unit interval floors
///   uniformly across the *entire* interval — not just near either end —
///   which no rounding rule, whatever its tie-breaking policy, could
///   produce.
///
/// See `tests::pcm24_matches_soundfile_oracle_bit_exact` for the pinned
/// values (hardcoded literals, not round-tripped through this function —
/// the previous attempt's circularity complaint) and the regeneration
/// command.
fn f32_to_pcm24(sample: f32) -> i32 {
    let scaled = (sample as f64 * 8_388_608.0).floor();
    scaled.clamp(-8_388_608.0, 8_388_607.0) as i32
}

/// `sf.write(str(out_path), out.T, SR, subtype="PCM_24")` (`mus_audio.py`
/// line 1169).
pub fn write_wav(audio: &RenderedAudio, path: &Path) -> Result<(), RenderError> {
    let writer = hound::WavWriter::create(path, pcm24_spec(audio))
        .map_err(|e| RenderError::Wav(format!("{}: {e}", path.display())))?;
    write_pcm24(audio, writer)
}

/// The same PCM_24 WAV [`write_wav`] writes, as bytes: what `renderDigest`
/// digests when there is no file system (the browser).
pub fn wav_bytes(audio: &RenderedAudio) -> Result<Vec<u8>, RenderError> {
    let mut cursor = std::io::Cursor::new(Vec::with_capacity(44 + audio.samples.len() * 3));
    let writer = hound::WavWriter::new(&mut cursor, pcm24_spec(audio))
        .map_err(|e| RenderError::Wav(e.to_string()))?;
    write_pcm24(audio, writer)?;
    Ok(cursor.into_inner())
}

fn pcm24_spec(audio: &RenderedAudio) -> hound::WavSpec {
    hound::WavSpec {
        channels: 2,
        sample_rate: audio.sample_rate,
        bits_per_sample: 24,
        sample_format: hound::SampleFormat::Int,
    }
}

fn write_pcm24<W: std::io::Write + std::io::Seek>(
    audio: &RenderedAudio,
    mut writer: hound::WavWriter<W>,
) -> Result<(), RenderError> {
    for &value in &audio.samples {
        writer
            .write_sample(f32_to_pcm24(value))
            .map_err(|e| RenderError::Wav(e.to_string()))?;
    }
    writer
        .finalize()
        .map_err(|e| RenderError::Wav(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A float32 exactly `(2m + 1) / 2**24` — chosen so `value * 2**23`
    /// equals `m + 0.5` with no float rounding anywhere in its
    /// construction: `2m + 1` fits f32's 24-bit mantissa exactly for
    /// every `m` used below (`2*8_000_001 + 1 = 16_000_003 <
    /// 16_777_216 = 2**24`), and dividing by a power of two only shifts
    /// the exponent. This is the fix over the previous attempt's
    /// `(target as f64 + frac) as f32` construction, which is *not*
    /// exact at these magnitudes (see `f32_to_pcm24`'s doc comment).
    fn half_boundary(m: i64) -> f32 {
        (2 * m + 1) as f32 / 16_777_216.0
    }

    /// A float32 exactly `k / (16 * 2**23)`, i.e. `value * 2**23 == k /
    /// 16` — sweeps every sixteenth of the unit interval above `base`
    /// with the same exactness guarantee as [`half_boundary`] (`base *
    /// 16 + sixteenths` stays well under `2**24` for every value used
    /// below).
    fn sixteenth_above(base: i64, sixteenths: i64) -> f32 {
        (base * 16 + sixteenths) as f32 / 134_217_728.0
    }

    /// Pinned against real `soundfile`/`libsndfile` output — see
    /// `f32_to_pcm24`'s doc comment for the full finding. Every expected
    /// value below is a hardcoded literal; none are computed by calling
    /// [`f32_to_pcm24`] or any other Rust helper (the previous attempt's
    /// circularity complaint — its second test compared `write_wav`'s
    /// output back to `f32_to_pcm24` itself, which is `pcm24_write_
    /// round_trips_byte_exact`'s job below, *not* oracle evidence).
    ///
    /// Regenerate/verify with:
    /// ```text
    /// uv run --with soundfile --with numpy python3 -c '
    /// import struct, numpy as np, soundfile as sf
    /// def half(m): return np.float32((2*m+1) / (2.0**24))
    /// ms = (1_000_000, 1_000_001, 2, 3, 0, 6, 7, 8_000_000, 8_000_001)
    /// vals = [half(m) for m in ms] + [np.float32(-float(half(m))) for m in ms]
    /// sf.write("/tmp/t.wav", np.array(vals, dtype=np.float32), 48000, subtype="PCM_24")
    /// data = open("/tmp/t.wav","rb").read()
    /// i = data.find(b"data"); n = struct.unpack("<I", data[i+4:i+8])[0]
    /// raw = data[i+8:i+8+n]
    /// print([int.from_bytes(raw[k*3:k*3+3], "little", signed=True) for k in range(n//3)])'
    /// ```
    #[test]
    fn pcm24_matches_soundfile_oracle_bit_exact() {
        let cases: &[(f32, i32)] = &[
            (0.0, 0),
            (1.0, 8_388_607),                       // clips: floor(8388608.0) > max
            (-1.0, -8_388_608), // exact: floor(-8388608.0) == min, no clip needed
            (2.0, 8_388_607),   // far over: clips
            (-2.0, -8_388_608), // far under: clips
            (1_000_000.0 / 8_388_608.0, 1_000_000), // exact integer, sanity check
            (-1_000_000.0 / 8_388_608.0, -1_000_000),
        ];
        for &(input, expected) in cases {
            assert_eq!(f32_to_pcm24(input), expected, "input {input}");
        }

        // Half-integer boundaries: floors to `m` on the positive side for
        // BOTH an even and an odd `m` (rules out round-half-to-even,
        // which would round `1_000_001.5` up to the even `1_000_002`,
        // and round-half-up, which would round `1_000_000.5` up to
        // `1_000_001` — neither happens). The negative side floors
        // toward -infinity, not zero.
        for &m in &[1_000_000i64, 1_000_001, 2, 3, 0, 6, 7, 8_000_000, 8_000_001] {
            assert_eq!(
                f32_to_pcm24(half_boundary(m)),
                m as i32,
                "+({m}+0.5) should floor to {m}"
            );
            assert_eq!(
                f32_to_pcm24(-half_boundary(m)),
                -(m as i32) - 1,
                "-({m}+0.5) should floor to -({m}+1)"
            );
        }

        // Every sixteenth across a whole unit interval floors to the
        // same base integer, not just the values near either end — no
        // rounding rule (whatever its tie-breaking policy) could
        // produce that.
        for s in 1..=15i64 {
            assert_eq!(
                f32_to_pcm24(sixteenth_above(1_000_000, s)),
                1_000_000,
                "1_000_000 + {s}/16 should floor to 1_000_000"
            );
        }
    }

    /// Byte-level round-trip: write via [`write_wav`], read the 24-bit PCM
    /// bytes straight back with `hound` (bypassing float rescaling), and
    /// confirm they match [`f32_to_pcm24`]'s own conversion sample-for-
    /// sample. This proves `write_wav`'s byte packing/interleaving is
    /// faithful to whatever `f32_to_pcm24` computes — it is deliberately
    /// *not* oracle evidence (that's `pcm24_matches_soundfile_oracle_
    /// bit_exact`'s job, against literal pinned values, above), since
    /// comparing the writer to the same helper it calls internally proves
    /// only self-consistency.
    #[test]
    fn pcm24_write_round_trips_byte_exact() {
        let samples: Vec<f32> = (0..2000)
            .map(|i| ((i as f32 / 317.0).sin()) * 0.9)
            .collect();
        let audio = RenderedAudio {
            samples: samples.clone(),
            sample_rate: 48_000,
        };
        let path = std::env::temp_dir().join(format!(
            "mus-engine-pcm24-roundtrip-{}.wav",
            std::process::id()
        ));
        write_wav(&audio, &path).unwrap();
        let mut reader = hound::WavReader::open(&path).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.bits_per_sample, 24);
        assert_eq!(spec.channels, 2);
        let ints: Vec<i32> = reader.samples::<i32>().map(|s| s.unwrap()).collect();
        assert_eq!(ints.len(), samples.len());
        for (got, &input) in ints.iter().zip(&samples) {
            assert_eq!(*got, f32_to_pcm24(input));
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn explicit_bars_zero_header_falls_back_to_the_bar_lines_count() {
        // `if not bar_count and bars: bar_count = max(...)` (mus.py line
        // 1326-1327) treats an explicit `# bars: 0` the same as a missing
        // header — Python's `not 0` is `True`. Before the fix this rendered
        // zero bars (an empty `1..=0` walk range) and produced no events at
        // all; now both notated bars render.
        let text = "# score: bars header zero\n# tempo: 120\n# time: 4/4\n\
                     # bars: 0\n# instruments:\n#   s = synth (perc, synth=sine)\n\
                     bar 1: s=A4q\nbar 2: s=A4q\n";
        let out = render(text, Path::new(".")).unwrap();
        assert_eq!(out.stats.notes, 2, "{:?}", out.stats);
        // 2 bars @ 120bpm 4/4 = 4s of notated material + the 4s render tail.
        assert!((out.stats.total_seconds - 8.0).abs() < 1e-9);
    }

    #[test]
    fn smoke_score_renders_deterministically_with_correct_stats() {
        let score = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../aigua/smoke.mus");
        let text = std::fs::read_to_string(&score).unwrap();
        let base = score.parent().unwrap();
        let first = render(&text, base).unwrap();
        let second = render(&text, base).unwrap();
        assert_eq!(first.audio, second.audio, "render must be deterministic");
        assert_eq!(first.stats.notes, second.stats.notes);
        assert!(first.stats.bad.is_empty(), "{:?}", first.stats.bad);
        assert!(
            first.stats.warnings.is_empty(),
            "{:?}",
            first.stats.warnings
        );
        assert_eq!(first.stats.voices, 5);
        assert!((first.stats.tuning_a - 445.6).abs() < 1e-9);
        assert!(first.audio.frames() > 8 * SAMPLE_RATE as usize);
        assert!(first.audio.peak() <= 10f32.powf(-1.0 / 20.0) + 1e-4);
        // smoke.mus has no `# sidechain:` header.
        assert_eq!(first.stats.sidechain_hits, 0);
    }

    /// End-to-end: a malformed numeric event param refuses the whole
    /// render with a structured `RenderError`, matching the oracle's own
    /// uncaught `ValueError` there — not a silently-defaulted note
    /// wearing a "success" receipt. Exercises the wiring through
    /// `render()` → `events::walk_track` → `transforms::apply_chain`
    /// end to end, on top of `events.rs`/`transforms.rs`'s own unit
    /// coverage of the individual sites.
    #[test]
    fn malformed_event_param_refuses_the_whole_render() {
        let text = "# score: malformed param\n# tempo: 120\n# time: 4/4\n\
                     # instruments:\n#   s = synth (perc, synth=sine)\n\
                     bar 1: s=A4q[drive=hot]\n";
        let err = render(text, Path::new(".")).unwrap_err();
        let RenderError::Invalid(msg) = err else {
            panic!("expected RenderError::Invalid, got {err:?}");
        };
        assert!(msg.contains("drive=\"hot\""), "{msg}");
    }

    /// Same end-to-end shape, for a malformed *header* value: `# time:`
    /// that isn't `N/D` aborts the oracle at the bar-map division just
    /// like a malformed event param does.
    #[test]
    fn malformed_time_signature_header_refuses_the_whole_render() {
        let text = "# score: malformed time\n# tempo: 120\n# time: five-four\n\
                     # instruments:\n#   s = synth (perc, synth=sine)\n\
                     bar 1: s=A4q\n";
        let err = render(text, Path::new(".")).unwrap_err();
        let RenderError::Invalid(msg) = err else {
            panic!("expected RenderError::Invalid, got {err:?}");
        };
        assert!(msg.contains("five-four"), "{msg}");
    }

    /// `# tempo: 96.5` reaches the header tempo-change list, where
    /// Python's `int("96.5")` fails and `coerce` keeps it poisoned rather
    /// than dropping it — using it (bar 1's `spq = 60.0 / tempo`) is where
    /// the oracle itself raises. End to end through `render()`, not just
    /// `layout::build_bar_map` in isolation.
    #[test]
    fn fractional_tempo_header_refuses_the_whole_render() {
        let text = "# score: fractional tempo\n# tempo: 96.5\n# time: 4/4\n\
                     # instruments:\n#   s = synth (perc, synth=sine)\n\
                     bar 1: s=A4q\n";
        let err = render(text, Path::new(".")).unwrap_err();
        let RenderError::Invalid(msg) = err else {
            panic!("expected RenderError::Invalid, got {err:?}");
        };
        assert!(msg.contains("96.5"), "{msg}");
    }

    /// The `lib.rs:190` allocation site this fix targets: `# tempo: 0`
    /// reaches the bar map as a perfectly valid *integer* (Python's own
    /// crash there is `ZeroDivisionError`, not a poisoned string), so
    /// unchecked it makes `total_s` infinite and `n_total = (total_s *
    /// SR) as usize` saturates to `usize::MAX` — an attempted
    /// multi-exabyte allocation rather than a refusal. `render()` must
    /// surface a structured `RenderError` promptly instead.
    #[test]
    fn zero_tempo_header_refuses_the_whole_render_instead_of_a_huge_allocation() {
        let text = "# score: zero tempo\n# tempo: 0\n# time: 4/4\n\
                     # instruments:\n#   s = synth (perc, synth=sine)\n\
                     bar 1: s=A4q\n";
        let err = render(text, Path::new(".")).unwrap_err();
        let RenderError::Invalid(msg) = err else {
            panic!("expected RenderError::Invalid, got {err:?}");
        };
        assert!(msg.contains("not finite and positive"), "{msg}");
    }

    /// `render_bars` over the whole score is bit-identical to `render`;
    /// a mid-range slice is re-timed to start at zero with the same 4 s
    /// tail, and a range outside the score is refused.
    #[test]
    fn render_bars_matches_full_render_and_slices_to_zero() {
        // 120 bpm, 2/4: each bar is exactly 1 s.
        let text = "# score: bars\n# tempo: 120\n# time: 2/4\n# bars: 4\n\
                     # instruments:\n#   s = synth (treble, synth=sine)\n\
                     bar 1: s=C4q D4q\n\
                     bar 2: s=E4q F4q\n\
                     bar 3: s=G4q A4q\n\
                     bar 4: s=B4q C5q\n";

        let full = render(text, Path::new(".")).unwrap();
        let full_bars = render_bars(text, Path::new("."), 1, 4).unwrap();
        assert_eq!(full.audio.samples, full_bars.audio.samples);
        assert_eq!(full.stats.notes, 8);

        // bars 2..=3: 2 bars (2 s) + the 4 s tail = 6 s of audio, 4 notes.
        let slice = render_bars(text, Path::new("."), 2, 3).unwrap();
        assert_eq!(
            slice.audio.samples.len(),
            (6.0 * SAMPLE_RATE as f64) as usize * 2
        );
        assert_eq!(slice.stats.notes, 4);
        assert!(slice.audio.peak() > 0.0);

        // Out of range is a structured refusal, not a panic or empty render.
        let err = render_bars(text, Path::new("."), 2, 5).unwrap_err();
        let RenderError::Invalid(msg) = err else {
            panic!("expected RenderError::Invalid, got {err:?}");
        };
        assert!(msg.contains("2..5"), "{msg}");
    }
}
