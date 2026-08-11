//! `ariadne-clap` — the PW9 §3 P1 skeleton: a CLAP instrument that is a
//! *window* onto [`StringNetworkVoice`], never a second implementation of
//! it.
//!
//! Three laws this file is written to obey, in order of importance:
//!
//! 1. **The kernel is the product.** Not one line of synthesis lives
//!    here. Everything below is voice allocation, parameter plumbing,
//!    sample-rate bookkeeping and summing. Every sample the host hears
//!    came out of [`StringNetworkVoice::render_block`].
//! 2. **The vocabulary IS the parameter schema** (design doc §8: "the
//!    plugin should be a client of the declared vocabulary, not a second
//!    hand-maintained parameter registry"). Ranges and defaults are read
//!    from `mus_vocab::param_specs()` at instantiation, so an automation
//!    range cannot drift from the score language. The engine's own clamps
//!    (in `apply_control`) are the backstop, and a test in this file
//!    fails loudly if the two registries ever disagree.
//! 3. **Honest scope.** What P1 does not do is written down, in the
//!    README and in comments at the exact place the shortcut is taken —
//!    never smoothed over.
//!
//! ## The sample-rate situation (read this before believing the pitch)
//!
//! `mus-dsp` is compiled for a fixed 48 kHz (`mus_dsp::SR_F64`): delay
//! lengths, body-mode frequencies and damping coefficients are all built
//! against that constant. A host running at 44.1 or 96 kHz gets the 48 kHz
//! render *reinterpreted* at its own rate, which is a pure time-scale by
//! `R = 48000 / host_sr`.
//!
//! P1 undoes that scale where it can, uniformly: note frequencies and
//! `orbit` are multiplied by `R`, slot length and `sus` are divided by it.
//! Pitch, note length, decay time and crank speed then come out correct in
//! wall-clock terms at any host rate. The residual is spectral and cannot
//! be fixed from out here: the body resonances and the loop filter's
//! corner frequencies scale with `R` too, so at 44.1 kHz the instrument's
//! *body* is transposed down about 1.5 semitones. The real fix is a
//! sample-rate-parametric kernel, which is engine work, not wrapper work.

use std::collections::BTreeMap;
use std::sync::Arc;

use mus_dsp::pluck::{ControlTarget, StringNetworkVoice};
use mus_dsp::SR_F64;
use mus_vocab::param_specs::{param_specs, ParamKind};
use nih_plug::prelude::*;

/// Fixed polyphony. Small and constant on purpose: a Weave voice is
/// `courses` coupled delay lines, so voices are expensive and an
/// unbounded pool would be an unbounded CPU promise.
const MAX_VOICES: usize = 6;

/// P1 events are slot-shaped, exactly as MUS score events are: a note is
/// a fixed 2.0 s excitation plus the kernel's own decay tail. Note-off is
/// ignored (see [`AriadneClap::process`]).
const SLOT_SECONDS: f64 = 2.0;

/// The nine controls the engine declares block-automatable (PW9 §4;
/// `mus_vocab::automation_class` returns "block" for each). `sing` is the
/// tenth and is deliberately absent from P1 — it needs a sidechain input,
/// which is the FX shape, not the instrument shape.
const AUTOMATABLE: [(&str, ControlTarget); 9] = [
    ("couple", ControlTarget::Couple),
    ("chirality", ControlTarget::Chirality),
    ("orbit", ControlTarget::OrbitHz),
    ("orbit_depth", ControlTarget::OrbitDepth),
    ("curvature", ControlTarget::Curvature),
    ("body", ControlTarget::Body),
    ("buzz", ControlTarget::Buzz),
    ("sus", ControlTarget::Sus),
    ("damp", ControlTarget::Damp),
];

// ---------------------------------------------------------------- params

/// `(min, max, default)` for a float-valued parameter, read out of the
/// declared vocabulary.
///
/// Panics if the name is absent or not float-shaped. That is deliberate:
/// a plugin that cannot find its own schema has nothing honest to expose,
/// and `vocab_covers_every_declared_param` below makes the panic
/// unreachable in any build that passes its tests.
fn float_spec(name: &str, specs: &[mus_vocab::param_specs::ParamSpec]) -> (f32, f32, f32) {
    let spec = specs
        .iter()
        .find(|spec| spec.name == name)
        .unwrap_or_else(|| panic!("ariadne-clap: '{name}' is not in the mus vocabulary"));
    match spec.kind {
        ParamKind::Ratio { min, max, default }
        | ParamKind::Seconds { min, max, default }
        | ParamKind::Hz {
            min, max, default, ..
        } => (min as f32, max as f32, default.unwrap_or(min) as f32),
        _ => panic!("ariadne-clap: '{name}' is not a float-shaped vocabulary parameter"),
    }
}

/// `(min, max, default)` for a count-valued parameter. Same contract.
fn count_spec(name: &str, specs: &[mus_vocab::param_specs::ParamSpec]) -> (i32, i32, i32) {
    let spec = specs
        .iter()
        .find(|spec| spec.name == name)
        .unwrap_or_else(|| panic!("ariadne-clap: '{name}' is not in the mus vocabulary"));
    match spec.kind {
        ParamKind::Count { min, max, default } => {
            (min as i32, max as i32, default.unwrap_or(min) as i32)
        }
        _ => panic!("ariadne-clap: '{name}' is not a count-shaped vocabulary parameter"),
    }
}

/// Build a linear [`FloatParam`] straight from the vocabulary entry.
fn vocab_float(
    display: &str,
    name: &str,
    specs: &[mus_vocab::param_specs::ParamSpec],
) -> FloatParam {
    let (min, max, default) = float_spec(name, specs);
    // No smoother: these are applied to live voices once per block via
    // `apply_control`, which is the engine's declared Block automation
    // class. A per-sample smoother would produce values nothing reads.
    FloatParam::new(display, default, FloatRange::Linear { min, max })
}

#[derive(Params)]
pub struct AriadneParams {
    // --- block-automatable (PW9 §4): retargeted on live voices ---------
    #[id = "couple"]
    pub couple: FloatParam,
    #[id = "chiral"]
    pub chirality: FloatParam,
    #[id = "orbit"]
    pub orbit: FloatParam,
    #[id = "orbdep"]
    pub orbit_depth: FloatParam,
    #[id = "curvat"]
    pub curvature: FloatParam,
    #[id = "body"]
    pub body: FloatParam,
    #[id = "buzz"]
    pub buzz: FloatParam,
    #[id = "sus"]
    pub sus: FloatParam,
    #[id = "damp"]
    pub damp: FloatParam,

    // --- construction-time: read only when a voice is woven ------------
    // These change the network's topology, so they cannot be retargeted
    // mid-note. Turning them affects the *next* note, not the ringing
    // one — which is the honest hardware behaviour anyway (you cannot
    // add a rod to a loom that is already sounding).
    #[id = "course"]
    pub courses: IntParam,
    #[id = "dimens"]
    pub dimension: FloatParam,
}

impl Default for AriadneParams {
    fn default() -> Self {
        let specs = param_specs();
        let (course_min, course_max, course_default) = count_spec("courses", &specs);
        Self {
            couple: vocab_float("Couple", "couple", &specs),
            chirality: vocab_float("Chirality", "chirality", &specs),
            orbit: vocab_float("Orbit", "orbit", &specs).with_unit(" Hz"),
            orbit_depth: vocab_float("Orbit Depth", "orbit_depth", &specs),
            curvature: vocab_float("Curvature", "curvature", &specs),
            body: vocab_float("Body", "body", &specs),
            buzz: vocab_float("Buzz", "buzz", &specs),
            sus: vocab_float("Sus", "sus", &specs).with_unit(" s"),
            damp: vocab_float("Damp", "damp", &specs),
            courses: IntParam::new(
                "Courses",
                course_default,
                IntRange::Linear {
                    min: course_min,
                    max: course_max,
                },
            ),
            dimension: vocab_float("Dimension", "dimension", &specs),
        }
    }
}

impl AriadneParams {
    /// Current host-time value of one automatable control.
    fn automatable_value(&self, target: ControlTarget) -> f64 {
        match target {
            ControlTarget::Couple => self.couple.value() as f64,
            ControlTarget::Chirality => self.chirality.value() as f64,
            ControlTarget::OrbitHz => self.orbit.value() as f64,
            ControlTarget::OrbitDepth => self.orbit_depth.value() as f64,
            ControlTarget::Curvature => self.curvature.value() as f64,
            ControlTarget::Body => self.body.value() as f64,
            ControlTarget::Buzz => self.buzz.value() as f64,
            ControlTarget::Sus => self.sus.value() as f64,
            ControlTarget::Damp => self.damp.value() as f64,
            // Not exposed at P1 — no sidechain, so no drive to mix in.
            ControlTarget::Sing => 0.0,
            // Not exposed at P1 either (PW10 §C, landed after this
            // skeleton): no `AriadneParams` field yet, so there is
            // nothing to read. Wiring it in is P2 surface work, not an
            // engine question.
            ControlTarget::Breath => 0.0,
        }
    }
}

// ---------------------------------------------------------------- engine

/// One sounding note.
struct VoiceSlot {
    voice: StringNetworkVoice,
    /// MIDI key, kept so a future note-off / choke can find its voice.
    note: u8,
    /// Monotonic spawn order; the smallest is the oldest, and the oldest
    /// is what gets stolen.
    age: u64,
    /// Velocity as a plain output gain. This is a mixer decision, not a
    /// synthesis one: the kernel takes no velocity, so playing harder is
    /// currently louder but not brighter. Velocity→excitation level (the
    /// `level` argument `physical_excitation` already takes) is an engine
    /// seam to open in P2, not something to fake out here.
    gain: f32,
}

/// Voice allocation, sample-rate mapping, and summing. Deliberately free
/// of any nih-plug type so the unit tests can drive it without a host.
pub struct AriadneEngine {
    voices: Vec<VoiceSlot>,
    /// Per-voice render destination, sized once in [`Self::prepare`].
    scratch: Vec<f32>,
    /// `48000 / host_sr`. See the module docs.
    rate_ratio: f64,
    spawned: u64,
}

impl Default for AriadneEngine {
    fn default() -> Self {
        Self {
            voices: Vec::with_capacity(MAX_VOICES),
            scratch: Vec::new(),
            rate_ratio: 1.0,
            spawned: 0,
        }
    }
}

impl AriadneEngine {
    /// Allocate everything the audio thread will need. Called from
    /// `initialize()` and from the tests, never from `process()`.
    pub fn prepare(&mut self, sample_rate: f32, max_block_size: usize) {
        self.rate_ratio = SR_F64 / (sample_rate as f64).max(1.0);
        self.scratch.clear();
        self.scratch.resize(max_block_size.max(1), 0.0);
        self.voices.clear();
        self.voices.reserve(MAX_VOICES);
    }

    pub fn reset(&mut self) {
        self.voices.clear();
    }

    pub fn voice_count(&self) -> usize {
        self.voices.len()
    }

    /// Map a host-time control value into engine time. Unitless controls
    /// pass straight through; seconds shrink and hertz grow by the same
    /// ratio, which is exactly the inverse of the playback time-scale.
    fn to_engine(&self, target: ControlTarget, host_value: f64) -> f64 {
        match target {
            ControlTarget::Sus => host_value / self.rate_ratio,
            ControlTarget::OrbitHz => host_value * self.rate_ratio,
            _ => host_value,
        }
    }

    /// Weave a new voice for `note`, stealing the oldest if the pool is
    /// full.
    ///
    /// **P1 is not allocation-clean.** `StringNetworkVoice::weave` builds
    /// its delay lines, body modes and excitation buffers on the spot, and
    /// the parameter map it reads is a freshly built `BTreeMap<String,
    /// String>` — so a note-on allocates several times on the audio
    /// thread, and retiring a voice frees on it. That is a real (if rare
    /// and bounded) xrun risk and it is stated here rather than hidden.
    /// The P2 fix is a voice pool: `MAX_VOICES` voices built at
    /// `prepare()` for the worst-case `courses`, with a `re_weave(f0,
    /// slot)` on the engine that re-excites the existing buffers in
    /// place. That is an engine-side seam (mus-dsp), which is why P1 does
    /// not try to fake it from the wrapper.
    pub fn note_on(&mut self, note: u8, velocity: f32, params: &AriadneParams) {
        if self.voices.len() >= MAX_VOICES {
            // Oldest-steal: the voice with the smallest age loses.
            if let Some(index) = self
                .voices
                .iter()
                .enumerate()
                .min_by_key(|(_, slot)| slot.age)
                .map(|(index, _)| index)
            {
                self.voices.remove(index);
            }
        }

        let f0_host = util::midi_note_to_freq(note) as f64;
        // Engine time: ask for a higher note over a shorter slot, because
        // the 48 kHz render is played back slower (or faster) than that.
        let f0_engine = f0_host * self.rate_ratio;
        let slot_engine = SLOT_SECONDS / self.rate_ratio;

        let mut construction: BTreeMap<String, String> = BTreeMap::new();
        // Construction-time topology.
        construction.insert("courses".into(), params.courses.value().to_string());
        construction.insert("dimension".into(), params.dimension.value().to_string());
        // The block-automatable controls also have to be *initially* set,
        // otherwise the first block of every note would sound at the
        // engine defaults before `push_params` catches up.
        for (name, target) in AUTOMATABLE {
            let value = self.to_engine(target, params.automatable_value(target));
            construction.insert(name.into(), value.to_string());
        }

        // `gliss_ratio = 1.0`: no glissando. MIDI pitch bend is a P2 seam
        // (it maps onto the same argument, per-voice).
        let voice = StringNetworkVoice::weave(&construction, &[f0_engine], slot_engine, 1.0);
        self.spawned += 1;
        self.voices.push(VoiceSlot {
            voice,
            note,
            age: self.spawned,
            gain: velocity.clamp(0.0, 1.0),
        });
    }

    /// Retarget one control on every live voice.
    pub fn apply_control_all(&mut self, target: ControlTarget, host_value: f64) {
        let value = self.to_engine(target, host_value);
        for slot in &mut self.voices {
            slot.voice.apply_control(target, value);
        }
    }

    /// Sum every live voice into `channels`. Channel 0 is rendered and the
    /// rest are copies of it: the kernel emits one signal, and inventing a
    /// stereo image out here would be exactly the "wrapper reimplements
    /// synthesis" mistake §8 forbids. Real stereo is a bridge/pickup pair
    /// in the engine (P2).
    pub fn render(&mut self, channels: &mut [&mut [f32]]) {
        let Some((first, rest)) = channels.split_first_mut() else {
            return;
        };
        let mono: &mut [f32] = first;
        mono.fill(0.0);

        if !self.scratch.is_empty() {
            let frames = mono.len();
            let mut start = 0;
            while start < frames {
                let count = (frames - start).min(self.scratch.len());
                let scratch = &mut self.scratch[..count];
                for slot in &mut self.voices {
                    // The one call that makes sound. Everything else in
                    // this crate is bookkeeping around it.
                    slot.voice.render_block(scratch);
                    let gain = slot.gain;
                    for (out, sample) in mono[start..start + count].iter_mut().zip(scratch.iter()) {
                        *out += sample * gain;
                    }
                }
                start += count;
            }
        }

        for channel in rest {
            let count = channel.len().min(mono.len());
            channel[..count].copy_from_slice(&mono[..count]);
        }

        // Retiring a finished voice frees on the audio thread — the other
        // half of the allocation debt documented on `note_on`.
        self.voices.retain(|slot| !slot.voice.is_finished());
    }

    /// Notes currently sounding, for tests and for a future voice display.
    pub fn sounding_notes(&self) -> Vec<u8> {
        self.voices.iter().map(|slot| slot.note).collect()
    }
}

// ---------------------------------------------------------------- plugin

pub struct AriadneClap {
    params: Arc<AriadneParams>,
    engine: AriadneEngine,
}

impl Default for AriadneClap {
    fn default() -> Self {
        Self {
            params: Arc::new(AriadneParams::default()),
            engine: AriadneEngine::default(),
        }
    }
}

impl AriadneClap {
    /// Everything `initialize()` does, reachable without a host.
    pub fn prepare(&mut self, sample_rate: f32, max_block_size: usize) {
        self.engine.prepare(sample_rate, max_block_size);
    }

    /// Push the current value of every block-automatable parameter into
    /// every live voice. Unconditional: `apply_control` is a clamped store
    /// (plus a walk over at most 24 courses for `sus`/`damp`), so change
    /// detection would buy nothing and could go stale against host
    /// automation.
    pub fn push_params(&mut self) {
        for (_, target) in AUTOMATABLE {
            let value = self.params.automatable_value(target);
            self.engine.apply_control_all(target, value);
        }
    }

    /// The body of `process()`, minus the host event queue: retarget, then
    /// render. The unit tests drive exactly this, so what they exercise is
    /// what the host gets.
    pub fn render_block(&mut self, channels: &mut [&mut [f32]]) {
        self.push_params();
        self.engine.render(channels);
    }

    pub fn note_on(&mut self, note: u8, velocity: f32) {
        self.engine.note_on(note, velocity, &self.params);
    }
}

impl Plugin for AriadneClap {
    const NAME: &'static str = "Ariadne";
    const VENDOR: &'static str = "Sophia Labs";
    const URL: &'static str = "https://github.com/sophia-labs/mus";
    const EMAIL: &'static str = "noreply@sophia-labs.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // P1 is an instrument: no main input. The FX shape PW9 §3 actually
    // wants to lead with — a main input routed into the `sing` seam — is
    // the next step, and needs `AUDIO_IO_LAYOUTS` to grow a stereo input
    // plus a `sing` parameter.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    // Block granularity, honestly declared: the engine's automation class
    // is Block (PW9 §4), so asking the host to split buffers on every
    // parameter change would buy resolution nothing downstream can use.
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        if buffer_config.sample_rate != mus_dsp::SR_F {
            nih_log!(
                "ariadne-clap: host is at {} Hz; the mus-dsp kernel is fixed at {} Hz. \
                 Pitch and timing are rescaled, but body resonances are transposed by {:.3}x.",
                buffer_config.sample_rate,
                mus_dsp::SR_F,
                buffer_config.sample_rate / mus_dsp::SR_F
            );
        }
        self.engine.prepare(
            buffer_config.sample_rate,
            buffer_config.max_buffer_size as usize,
        );
        true
    }

    fn reset(&mut self) {
        self.engine.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // P1 handles events at block granularity: every note-on in this
        // block starts at the block boundary, so onset timing is quantised
        // to the buffer size. Sample-accurate onset needs per-voice start
        // offsets in the render loop — P2, together with the voice pool.
        while let Some(event) = context.next_event() {
            if let NoteEvent::NoteOn { note, velocity, .. } = event {
                self.note_on(note, velocity);
            }
            // Note-off is deliberately ignored. A MUS event is a slot: the
            // note rings for `SLOT_SECONDS` plus the kernel's decay tail
            // and then retires itself. Making note-off mean something
            // requires a release seam in the engine (damp the courses, or
            // shorten `sus` into a release ramp) — an engine ruling, not a
            // wrapper improvisation.
        }

        // Exactly the entry points the unit tests drive — the host path
        // and the tested path are the same two calls.
        self.render_block(buffer.as_slice());

        // Voices outlive their note-ons by seconds, so the host must keep
        // calling us even with no input.
        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for AriadneClap {
    const CLAP_ID: &'static str = "com.sophia-labs.ariadne";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("A woven string network: coupled courses with chirality, orbit and curvature");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];
}

nih_export_clap!(AriadneClap);

// VST3 is intentionally NOT exported. nih-plug's VST3 wrapper links
// `vst3-sys` (GPLv3); this workspace is MIT, so shipping VST3 from this
// codebase would relicense it. Revisit if a permissive VST3 binding lands.

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK: usize = 512;

    fn stereo_render(plugin: &mut AriadneClap, left: &mut [f32], right: &mut [f32]) {
        let mut channels: [&mut [f32]; 2] = [left, right];
        plugin.render_block(&mut channels);
    }

    /// The headline test: a note-on, four blocks, real audio out.
    #[test]
    fn four_blocks_after_a_note_on_are_finite_and_audible() {
        let mut plugin = AriadneClap::default();
        plugin.prepare(mus_dsp::SR_F, BLOCK);
        // A2 = 110 Hz, the Weave's own base register.
        plugin.note_on(45, 1.0);

        let mut left = [0.0_f32; BLOCK];
        let mut right = [0.0_f32; BLOCK];
        let mut peak = 0.0_f32;
        let mut rendered = 0_usize;

        for _ in 0..4 {
            stereo_render(&mut plugin, &mut left, &mut right);
            for (&l, &r) in left.iter().zip(right.iter()) {
                assert!(l.is_finite(), "left channel produced {l}");
                assert!(r.is_finite(), "right channel produced {r}");
                assert_eq!(l, r, "P1 renders one signal into both channels");
                peak = peak.max(l.abs());
                rendered += 1;
            }
        }

        assert_eq!(rendered, 4 * BLOCK);
        assert!(
            peak > 1.0e-3,
            "four blocks were effectively silent (peak {peak})"
        );
        assert!(peak < 8.0, "output ran away (peak {peak})");
        assert_eq!(
            plugin.engine.voice_count(),
            1,
            "the voice should still be ringing"
        );
    }

    #[test]
    fn silence_in_silence_out() {
        let mut plugin = AriadneClap::default();
        plugin.prepare(mus_dsp::SR_F, BLOCK);

        let mut left = [1.0_f32; BLOCK];
        let mut right = [1.0_f32; BLOCK];
        stereo_render(&mut plugin, &mut left, &mut right);

        assert!(left.iter().all(|&sample| sample == 0.0));
        assert!(right.iter().all(|&sample| sample == 0.0));
    }

    #[test]
    fn voice_pool_is_bounded_and_steals_the_oldest() {
        let mut plugin = AriadneClap::default();
        plugin.prepare(mus_dsp::SR_F, BLOCK);

        for note in 40..(40 + MAX_VOICES as u8 + 2) {
            plugin.note_on(note, 0.8);
            assert!(plugin.engine.voice_count() <= MAX_VOICES);
        }

        let sounding = plugin.engine.sounding_notes();
        assert_eq!(sounding.len(), MAX_VOICES);
        // The two oldest (40, 41) were stolen; the newest survive.
        assert!(!sounding.contains(&40));
        assert!(!sounding.contains(&41));
        assert!(sounding.contains(&(40 + MAX_VOICES as u8 + 1)));
    }

    /// Renders past the end of a voice's slot + tail and checks it retires
    /// itself without leaving NaNs behind. Default `sus` is 2.5 s, so the
    /// total is 2.0 + min(2.5*0.78, 4.0) ≈ 3.95 s.
    #[test]
    fn a_voice_retires_after_its_slot_and_tail() {
        let mut plugin = AriadneClap::default();
        plugin.prepare(mus_dsp::SR_F, BLOCK);
        plugin.note_on(45, 1.0);

        let mut left = [0.0_f32; BLOCK];
        let mut right = [0.0_f32; BLOCK];
        let blocks = (5.0 * mus_dsp::SR_F64 / BLOCK as f64).ceil() as usize;
        for _ in 0..blocks {
            stereo_render(&mut plugin, &mut left, &mut right);
            assert!(left.iter().all(|sample| sample.is_finite()));
        }

        assert_eq!(plugin.engine.voice_count(), 0);
    }

    /// A host at 44.1 kHz must still be finite and audible, and the
    /// engine-time mapping must be the one the module docs claim.
    #[test]
    fn a_non_engine_sample_rate_still_renders() {
        let mut plugin = AriadneClap::default();
        plugin.prepare(44_100.0, BLOCK);
        plugin.note_on(45, 1.0);

        let ratio = plugin.engine.rate_ratio;
        assert!((ratio - 48_000.0 / 44_100.0).abs() < 1e-12);
        // Seconds shrink, hertz grow.
        assert!(plugin.engine.to_engine(ControlTarget::Sus, 2.5) < 2.5);
        assert!(plugin.engine.to_engine(ControlTarget::OrbitHz, 0.31) > 0.31);
        assert_eq!(plugin.engine.to_engine(ControlTarget::Couple, 0.11), 0.11);

        let mut left = [0.0_f32; BLOCK];
        let mut right = [0.0_f32; BLOCK];
        let mut peak = 0.0_f32;
        for _ in 0..4 {
            stereo_render(&mut plugin, &mut left, &mut right);
            for &sample in left.iter() {
                assert!(sample.is_finite());
                peak = peak.max(sample.abs());
            }
        }
        assert!(peak > 1.0e-3, "44.1 kHz render was silent (peak {peak})");
    }

    /// A block larger than the scratch buffer must still render (the
    /// chunking path), not panic or go silent.
    ///
    /// Four blocks, not one: a 110 Hz course is a ~436-sample delay line,
    /// so the first 300 samples out of a fresh voice are honestly silent —
    /// the excitation has not reached the pickup yet. That measured fact
    /// is why the headline test above reads four blocks too.
    #[test]
    fn blocks_larger_than_the_prepared_maximum_still_render() {
        let mut plugin = AriadneClap::default();
        plugin.prepare(mus_dsp::SR_F, 64);
        plugin.note_on(45, 1.0);

        let mut left = [0.0_f32; 300];
        let mut right = [0.0_f32; 300];
        let mut peak = 0.0_f32;
        for _ in 0..4 {
            stereo_render(&mut plugin, &mut left, &mut right);
            for &sample in left.iter() {
                assert!(sample.is_finite());
                peak = peak.max(sample.abs());
            }
        }
        assert!(peak > 1.0e-3, "chunked render was silent (peak {peak})");
    }

    // ---- the vocabulary really is the schema, not a copy of it --------

    #[test]
    fn vocab_covers_every_declared_param() {
        let specs = param_specs();
        for (name, _) in AUTOMATABLE {
            let _ = float_spec(name, &specs);
            assert_eq!(
                mus_vocab::automation_class(name),
                "block",
                "{name} is exposed as automatable but the vocabulary says otherwise"
            );
        }
        let _ = float_spec("dimension", &specs);
        let _ = count_spec("courses", &specs);
    }

    /// Drift alarm. These literals are the clamps inside
    /// `StringNetworkVoice::apply_control` / `PluckPatch::from`. If the
    /// engine widens a range and the vocabulary does not follow (or the
    /// reverse), this fails and somebody has to decide which one is right
    /// — which is the point.
    #[test]
    fn vocab_ranges_match_the_engine_clamps() {
        let specs = param_specs();
        let expected: [(&str, f32, f32, f32); 10] = [
            ("couple", 0.0, 0.45, 0.11),
            ("chirality", -1.0, 1.0, 0.72),
            ("orbit", 0.0, 20.0, 0.31),
            ("orbit_depth", 0.0, 1.0, 0.62),
            ("curvature", 0.0, 1.0, 0.28),
            ("body", 0.0, 1.0, 0.25),
            ("buzz", 0.0, 1.0, 0.0),
            ("sus", 0.05, 20.0, 2.5),
            ("damp", 0.0, 0.95, 0.35),
            ("dimension", 0.55, 3.0, 1.35),
        ];
        for (name, min, max, default) in expected {
            assert_eq!(float_spec(name, &specs), (min, max, default), "{name}");
        }
        assert_eq!(count_spec("courses", &specs), (3, 24, 11));
    }

    #[test]
    fn params_default_to_the_vocabulary_defaults() {
        let params = AriadneParams::default();
        assert_eq!(params.couple.value(), 0.11);
        assert_eq!(params.chirality.value(), 0.72);
        assert_eq!(params.orbit.value(), 0.31);
        assert_eq!(params.courses.value(), 11);
        assert_eq!(params.dimension.value(), 1.35);
    }
}
