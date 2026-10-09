//! The rig: guitarist → guitar → Ariadne World → stereo out, one sample at a
//! time, on whichever thread owns it (the audio callback, or an offline render).
//!
//! The control thread talks to it through lock-free rings made in
//! [`Rig::new`]: commands in, a scope tap out (every sample: dry, wet L, wet R),
//! a telemetry tap out (every [`TELEMETRY_EVERY`] samples: levels, the
//! World's own readings, the 24 modes, load), the guitarist's events out, and
//! a garbage ring that returns replaced scores to the control thread to be
//! freed there. Nothing on the sample path allocates, frees or locks; a full
//! tap drops frames and counts them.
//!
//! The master stage ([`Command::Master`]) sits after Ariadne and before the
//! device: a smoothed gain and a mute for the monitor. It is not part of the
//! Ariadne patch, and the taps see Ariadne's output before it, so the screens
//! and takes are the same whatever the monitor level.

use crate::ariadne::{AriadneControls, AriadneEngine, WorldReadings, PARAMS};
use crate::compose::Chord;
use crate::guitar::{Guitar, GuitarSettings, StringTruth, STRINGS};
use crate::player::{Event, Pattern, Player};
use crate::score::Score;
use mus_ariadne::MODE_COUNT;
use std::time::Instant;

pub const TELEMETRY_EVERY: usize = 256;
const FAST_SMOOTHING_S: f64 = 0.007;
const SLOW_SMOOTHING_S: f64 = 0.020;
/// Time constant of the master gain (a 0 dB → mute change settles in ~0.15 s).
pub const MASTER_SMOOTHING_S: f64 = 0.010;
/// Within this (linear, −80 dB) of its target the master snaps there: an f32
/// one-pole stalls short of it otherwise, and mute must reach exact silence.
/// The snap is far below the ramp's own per-sample step.
const MASTER_SNAP: f32 = 1e-4;
/// The master stage's range in dB (below it, use mute).
pub const MASTER_MIN_DB: f64 = -60.0;
pub const MASTER_MAX_DB: f64 = 0.0;

#[derive(Debug)]
pub enum Command {
    Ariadne(AriadneControls),
    Guitar(GuitarSettings),
    Pattern(Pattern),
    Tempo(f64),
    Pluck { string: u8, fret: u8, velocity: f32 },
    /// Any guitar event, `delay_s` from now.
    Event { event: Event, delay_s: f32 },
    /// Strum a shape now: `spread_s` from the first string to the last.
    Strum { frets: Chord, down: bool, velocity: f32, spread_s: f32 },
    MuteStrings,
    ResetWorld,
    /// The monitor: linear gain (0..1) and mute, smoothed.
    Master { gain: f32, mute: bool },
    /// Play this score now; the one it replaces comes back on the garbage ring.
    Score(Box<Score>),
    /// Reseed the guitarist.
    Seed(u32),
    /// Where the dry signal comes from.
    Source(Source),
    /// Linear gain on the live input.
    InputGain(f32),
}

/// dB to the master stage's linear gain, clamped to its range.
#[must_use]
pub fn master_gain(db: f64) -> f32 {
    10f64.powf(db.clamp(MASTER_MIN_DB, MASTER_MAX_DB) / 20.0) as f32
}

/// The dry signal: the simulated guitarist, or the audio interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Sim,
    Live,
}

/// How full the live-input ring is kept (samples): enough to ride out the
/// input and output callbacks running out of step, little enough to feel
/// immediate. Above `INPUT_MAX_FILL` the oldest samples are dropped, which is
/// also how drift between two devices' clocks is absorbed.
pub const INPUT_TARGET_FILL: usize = 256;
pub const INPUT_MAX_FILL: usize = 2048;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ScopeFrame {
    pub dry: f32,
    pub left: f32,
    pub right: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Telemetry {
    /// Sample index at the end of this block.
    pub sample: u64,
    pub sample_rate: f64,
    pub dry_peak: f32,
    pub dry_rms: f32,
    pub wet_peak: f32,
    pub wet_rms: f32,
    /// World state at the end of the block; work, loss and radiation summed over it.
    pub world: WorldReadings,
    pub modes: [f32; MODE_COUNT],
    pub mode_hz: [f32; MODE_COUNT],
    pub string_hz: [f32; STRINGS],
    /// What each string is doing, from inside the simulated guitar.
    pub strings: [StringTruth; STRINGS],
    pub world_errors: u64,
    pub safety_limited_frames: u64,
    pub dropped_scope: u64,
    /// Fraction of real time the last processed buffer took.
    pub load: f32,
    /// The master stage's current (smoothed) gain, and whether it is muted.
    pub master: f32,
    pub master_muted: bool,
    /// What the guitarist is playing, and where a score is.
    pub pattern: Pattern,
    /// The key the piece is in now (it may modulate).
    pub key: &'static str,
    pub bpm: f32,
    pub score_beat: f32,
    pub score_length: f32,
    pub score_loops: u32,
    /// One-shot scores finished so far (the control thread follows the
    /// guitarist back to its previous composition when this moves).
    pub score_ends: u32,
    /// Guitarist events dropped because its queue was full.
    pub queue_overflow: u64,
    pub source: Source,
    /// Live input samples the output wanted but the input had not delivered.
    pub input_starved: u64,
    /// Live input samples dropped to keep latency down.
    pub input_dropped: u64,
}

pub struct RigEnds {
    pub commands: rtrb::Producer<Command>,
    pub scope: rtrb::Consumer<ScopeFrame>,
    pub telemetry: rtrb::Consumer<Telemetry>,
    /// Every event the guitarist applied, with its sample index.
    pub events: rtrb::Consumer<(u64, Event)>,
    /// Scores the rig has finished with: drop them here, off the audio thread.
    pub garbage: rtrb::Consumer<Box<Score>>,
}

pub struct Rig {
    sr: f64,
    player: Player,
    guitar: Guitar,
    ariadne: AriadneEngine,
    target: AriadneControls,
    current: [f64; 20],
    coef: [f64; 20],
    commands: rtrb::Consumer<Command>,
    scope: rtrb::Producer<ScopeFrame>,
    telemetry: rtrb::Producer<Telemetry>,
    events: rtrb::Producer<(u64, Event)>,
    garbage: rtrb::Producer<Box<Score>>,
    /// Replaced scores waiting for room on the garbage ring.
    limbo: [Option<Box<Score>>; 4],
    master_target: f32,
    master_mute: bool,
    master: f32,
    master_coef: f32,
    sample: u64,
    block: usize,
    dry_peak: f32,
    dry_sq: f64,
    wet_peak: f32,
    wet_sq: f64,
    work: f64,
    loss: f64,
    radiation: f64,
    dropped_scope: u64,
    load: f32,
    source: Source,
    input: Option<rtrb::Consumer<f32>>,
    input_gain: f32,
    input_starved: u64,
    input_dropped: u64,
}

impl Rig {
    pub fn new(sr: f64) -> Result<(Self, RigEnds), String> {
        let ariadne = AriadneEngine::new(sr)?;
        let (cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
        let (scope_tx, scope_rx) = rtrb::RingBuffer::new((sr * 2.0) as usize);
        let (tele_tx, tele_rx) = rtrb::RingBuffer::new(4096);
        let (ev_tx, ev_rx) = rtrb::RingBuffer::new(4096);
        let (gb_tx, gb_rx) = rtrb::RingBuffer::new(64);
        let target = AriadneControls::default();
        let coef = std::array::from_fn(|i| {
            let tau = if PARAMS[i].slow { SLOW_SMOOTHING_S } else { FAST_SMOOTHING_S };
            1.0 - (-1.0 / (tau * sr)).exp()
        });
        let rig = Self {
            sr,
            player: Player::new(sr),
            guitar: Guitar::new(sr),
            ariadne,
            target,
            current: target.as_array(),
            coef,
            commands: cmd_rx,
            scope: scope_tx,
            telemetry: tele_tx,
            events: ev_tx,
            garbage: gb_tx,
            limbo: [None, None, None, None],
            master_target: 1.0,
            master_mute: false,
            master: 1.0,
            master_coef: (1.0 - (-1.0 / (MASTER_SMOOTHING_S * sr)).exp()) as f32,
            sample: 0,
            block: 0,
            dry_peak: 0.0,
            dry_sq: 0.0,
            wet_peak: 0.0,
            wet_sq: 0.0,
            work: 0.0,
            loss: 0.0,
            radiation: 0.0,
            dropped_scope: 0,
            load: 0.0,
            source: Source::Sim,
            input: None,
            input_gain: 1.0,
            input_starved: 0,
            input_dropped: 0,
        };
        Ok((rig, RigEnds { commands: cmd_tx, scope: scope_rx, telemetry: tele_rx, events: ev_rx, garbage: gb_rx }))
    }

    #[must_use]
    pub fn sample_rate(&self) -> f64 {
        self.sr
    }

    #[must_use]
    pub fn plan_digest(&self) -> &str {
        self.ariadne.plan_digest()
    }

    /// Give the rig a live input (samples pushed by an input stream at the
    /// same sample rate) and switch to it.
    pub fn attach_input(&mut self, input: rtrb::Consumer<f32>) {
        self.input = Some(input);
        self.source = Source::Live;
    }

    /// Reseed the guitarist directly (before the stream starts).
    pub fn seed(&mut self, seed: u32) {
        self.player.seed(seed);
    }

    /// Set the master stage at once, without a ramp (before the stream starts).
    pub fn set_master_now(&mut self, gain: f32, mute: bool) {
        self.master_target = gain.clamp(0.0, 1.0);
        self.master_mute = mute;
        self.master = if mute { 0.0 } else { self.master_target };
    }

    /// The master stage's largest possible change in gain per sample.
    #[must_use]
    pub fn master_max_step(&self) -> f32 {
        self.master_coef
    }

    fn drain_commands(&mut self) {
        while let Ok(c) = self.commands.pop() {
            match c {
                Command::Ariadne(a) => self.target = a.sanitized(),
                Command::Guitar(g) => self.guitar.set(g),
                Command::Pattern(p) => self.player.set_pattern(p),
                Command::Tempo(b) => self.player.set_bpm(b),
                Command::Pluck { string, fret, velocity } => {
                    self.player.push(Event::Pluck { string, fret, velocity, palm: false })
                }
                Command::Event { event, delay_s } => self.player.push_after(event, delay_s as f64),
                Command::Strum { frets, down, velocity, spread_s } => {
                    self.player.strum_now(&frets, down, velocity as f64, spread_s as f64)
                }
                Command::MuteStrings => self.guitar.mute_all(),
                Command::ResetWorld => self.ariadne.reset(),
                Command::Master { gain, mute } => {
                    self.master_target = gain.clamp(0.0, 1.0);
                    self.master_mute = mute;
                }
                Command::Score(s) => {
                    if let Some(old) = self.player.load_score(s) {
                        self.retire(old);
                    }
                }
                Command::Seed(seed) => self.player.seed(seed),
                Command::Source(s) => {
                    self.source = if self.input.is_some() { s } else { Source::Sim };
                    if self.source == Source::Live {
                        self.guitar.mute_all();
                    }
                }
                Command::InputGain(g) => self.input_gain = g.clamp(0.0, 64.0),
            }
        }
        // Return anything still waiting for room on the garbage ring.
        for slot in &mut self.limbo {
            if let Some(old) = slot.take() {
                if let Err(rtrb::PushError::Full(old)) = self.garbage.push(old) {
                    *slot = Some(old);
                }
            }
        }
    }

    /// Hand a finished score back to the control thread. Never frees here: if
    /// the ring and the waiting slots are all full, the score is leaked.
    fn retire(&mut self, old: Box<Score>) {
        if let Err(rtrb::PushError::Full(old)) = self.garbage.push(old) {
            if let Some(slot) = self.limbo.iter_mut().find(|s| s.is_none()) {
                *slot = Some(old);
            } else {
                std::mem::forget(old);
            }
        }
    }

    /// One sample of the master stage.
    #[inline]
    fn master_tick(&mut self) -> f32 {
        let target = if self.master_mute { 0.0 } else { self.master_target };
        let d = target - self.master;
        self.master = if d.abs() < MASTER_SNAP { target } else { self.master + d * self.master_coef };
        self.master
    }

    /// One sample: (dry DI, wet left, wet right).
    #[inline]
    pub fn frame(&mut self) -> (f32, f32, f32) {
        let dry = match (self.source, &mut self.input) {
            (Source::Live, Some(input)) => match input.pop() {
                Ok(x) if x.is_finite() => x * self.input_gain,
                _ => {
                    self.input_starved += 1;
                    0.0
                }
            },
            _ => {
                let events = &mut self.events;
                self.player.tick_with(&mut self.guitar, |t, e| {
                    let _ = events.push((t, e));
                });
                self.guitar.tick()
            }
        };
        let target = self.target.as_array();
        for i in 0..20 {
            self.current[i] += (target[i] - self.current[i]) * self.coef[i];
        }
        let controls = AriadneControls::from_array(self.current);
        let out = self.ariadne.process_sample(dry, dry, controls);
        let r = self.ariadne.readings();
        self.work += r.source_work;
        self.loss += r.internal_loss;
        self.radiation += r.radiation;

        self.dry_peak = self.dry_peak.max(dry.abs());
        self.dry_sq += (dry as f64).powi(2);
        let wet = 0.5 * (out.left + out.right);
        self.wet_peak = self.wet_peak.max(out.left.abs()).max(out.right.abs());
        self.wet_sq += (wet as f64).powi(2);
        if self.scope.push(ScopeFrame { dry, left: out.left, right: out.right }).is_err() {
            self.dropped_scope += 1;
        }
        self.sample += 1;
        self.block += 1;
        if self.block == TELEMETRY_EVERY {
            self.publish();
        }
        (dry, out.left, out.right)
    }

    fn publish(&mut self) {
        let n = self.block.max(1) as f64;
        let mut world = self.ariadne.readings();
        world.source_work = self.work;
        world.internal_loss = self.loss;
        world.radiation = self.radiation;
        let mut modes = [0.0; MODE_COUNT];
        let mut mode_hz = [0.0; MODE_COUNT];
        self.ariadne.mode_energies(&mut modes, &mut mode_hz);
        let mut hz = [0.0; STRINGS];
        self.guitar.frequencies(&mut hz);
        let mut strings = [StringTruth::default(); STRINGS];
        self.guitar.truth(&mut strings);
        let (score_beat, score_length, score_loops, score_ends) = self.player.score_position();
        let t = Telemetry {
            sample: self.sample,
            sample_rate: self.sr,
            dry_peak: self.dry_peak,
            dry_rms: (self.dry_sq / n).sqrt() as f32,
            wet_peak: self.wet_peak,
            wet_rms: (self.wet_sq / n).sqrt() as f32,
            world,
            modes,
            mode_hz,
            string_hz: hz.map(|x| x as f32),
            strings,
            world_errors: self.ariadne.world_errors,
            safety_limited_frames: self.ariadne.safety_limited_frames,
            dropped_scope: self.dropped_scope,
            load: self.load,
            master: self.master,
            master_muted: self.master_mute,
            pattern: self.player.pattern(),
            key: self.player.key_name(),
            bpm: self.player.bpm() as f32,
            score_beat: score_beat as f32,
            score_length: score_length as f32,
            score_loops,
            score_ends,
            queue_overflow: self.player.overflow(),
            source: self.source,
            input_starved: self.input_starved,
            input_dropped: self.input_dropped,
        };
        let _ = self.telemetry.push(t);
        self.block = 0;
        self.dry_peak = 0.0;
        self.dry_sq = 0.0;
        self.wet_peak = 0.0;
        self.wet_sq = 0.0;
        self.work = 0.0;
        self.loss = 0.0;
        self.radiation = 0.0;
    }

    /// Fill an interleaved output buffer of `channels` channels (wet L, wet R
    /// through the master stage; further channels silent; mono gets the wet sum).
    pub fn process(&mut self, out: &mut [f32], channels: usize) {
        let start = Instant::now();
        self.drain_commands();
        let channels = channels.max(1);
        if let Some(input) = &mut self.input {
            // Keep the input ring near its target fill: drop the oldest
            // samples when it has run ahead (latency, clock drift).
            let frames = out.len() / channels;
            let fill = input.slots();
            if fill > INPUT_MAX_FILL + frames {
                let excess = fill - (INPUT_TARGET_FILL + frames);
                if let Ok(chunk) = input.read_chunk(excess) {
                    chunk.commit_all();
                    self.input_dropped += excess as u64;
                }
            }
        }
        for frame in out.chunks_mut(channels) {
            let (_, l, r) = self.frame();
            let m = self.master_tick();
            let (l, r) = (l * m, r * m);
            match frame.len() {
                1 => frame[0] = 0.5 * (l + r),
                _ => {
                    frame[0] = l;
                    frame[1] = r;
                    for x in &mut frame[2..] {
                        *x = 0.0;
                    }
                }
            }
        }
        let frames = out.len() / channels;
        if frames > 0 {
            let budget = frames as f64 / self.sr;
            self.load = (start.elapsed().as_secs_f64() / budget) as f32;
        }
    }

    /// Apply queued commands now (offline renders call this between frames).
    pub fn poll(&mut self) {
        self.drain_commands();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::MAX_FRET;
    use crate::score;
    use serde_json::json;

    const SR: f64 = 48_000.0;

    /// Ovillo's own default patch: the levels Vera hears.
    fn ovillo_patch() -> AriadneControls {
        serde_json::from_str(include_str!("../../../patches/ovillo-default.json")).unwrap()
    }

    #[test]
    fn a_strummed_bar_comes_out_of_ariadne_finite_and_audible() {
        let (mut rig, mut ends) = Rig::new(SR).unwrap();
        let mut wet_peak = 0.0f32;
        let mut dry_peak = 0.0f32;
        for _ in 0..(SR as usize * 2) {
            let (d, l, r) = rig.frame();
            assert!(d.is_finite() && l.is_finite() && r.is_finite());
            dry_peak = dry_peak.max(d.abs());
            wet_peak = wet_peak.max(l.abs()).max(r.abs());
            let _ = ends.scope.pop();
        }
        let mut last = None;
        while let Ok(t) = ends.telemetry.pop() {
            last = Some(t);
        }
        let t = last.expect("telemetry");
        assert_eq!(t.world_errors, 0);
        assert!(dry_peak > 0.01, "guitar is silent: {dry_peak}");
        assert!(wet_peak > 0.001, "Ariadne is silent: {wet_peak}");
        assert!(t.world.material_energy > 0.0);
    }

    struct Rendered {
        dry_peak: f32,
        wet_peak: f32,
        wet_rms: f64,
        events: Vec<(u64, Event)>,
        last: Telemetry,
    }

    fn render(pattern: Pattern, seconds: f64) -> Rendered {
        let (mut rig, mut ends) = Rig::new(SR).unwrap();
        ends.commands.push(Command::Ariadne(ovillo_patch())).unwrap();
        ends.commands.push(Command::Pattern(pattern)).unwrap();
        rig.poll();
        let n = (seconds * SR) as usize;
        let (mut dry_peak, mut wet_peak, mut sq) = (0.0f32, 0.0f32, 0.0f64);
        let mut events = Vec::new();
        let mut last = None;
        for i in 0..n {
            let (d, l, r) = rig.frame();
            assert!(d.is_finite() && l.is_finite() && r.is_finite(), "{pattern:?}: non-finite at {i}");
            dry_peak = dry_peak.max(d.abs());
            wet_peak = wet_peak.max(l.abs()).max(r.abs());
            sq += 0.5 * ((l as f64).powi(2) + (r as f64).powi(2));
            if i % 1024 == 0 {
                while ends.scope.pop().is_ok() {}
                while let Ok(e) = ends.events.pop() {
                    events.push(e);
                }
                while let Ok(t) = ends.telemetry.pop() {
                    last = Some(t);
                }
            }
        }
        while let Ok(e) = ends.events.pop() {
            events.push(e);
        }
        while let Ok(t) = ends.telemetry.pop() {
            last = Some(t);
        }
        Rendered { dry_peak, wet_peak, wet_rms: (sq / n as f64).sqrt(), events, last: last.expect("telemetry") }
    }

    fn check_events_in_range(p: Pattern, events: &[(u64, Event)]) {
        for (t, e) in events {
            assert!((e.string() as usize) < STRINGS, "{p:?} at {t}: {e:?}");
            match *e {
                Event::Pluck { fret, velocity, .. } => {
                    assert!(fret <= MAX_FRET, "{p:?} at {t}: fret {fret}");
                    assert!(velocity > 0.0 && velocity <= 1.0, "{p:?} at {t}: velocity {velocity}");
                }
                Event::Bend { semitones, .. } => assert!((0.0..=3.0).contains(&semitones), "{p:?}: {e:?}"),
                Event::Vibrato { depth, .. } => assert!((0.0..=1.0).contains(&depth), "{p:?}: {e:?}"),
                Event::Mute { .. } => {}
            }
        }
    }

    /// A minute of each composition through the real rig at Ovillo's default
    /// patch: finite, audible, under full scale without the safety limiter,
    /// every event on the fretboard, nothing dropped.
    fn sixty_seconds(p: Pattern) {
        let r = render(p, 60.0);
        let db = |v: f64| 20.0 * v.max(1e-12).log10();
        eprintln!(
            "{p:?}: {} events, dry peak {:.1} dBFS, wet peak {:.1} dBFS, wet rms {:.1} dBFS",
            r.events.len(),
            db(r.dry_peak as f64),
            db(r.wet_peak as f64),
            db(r.wet_rms)
        );
        assert_eq!(r.last.world_errors, 0, "{p:?}");
        assert!(r.dry_peak > 0.01, "{p:?}: the guitar is silent ({})", r.dry_peak);
        assert!(r.wet_peak > 0.01, "{p:?}: Ariadne is silent ({})", r.wet_peak);
        assert!(r.wet_rms > 1e-3, "{p:?}: Ariadne is too quiet ({})", r.wet_rms);
        assert!(r.dry_peak < 1.0 && r.wet_peak < 1.0, "{p:?}: peaks {} {}", r.dry_peak, r.wet_peak);
        assert_eq!(r.last.safety_limited_frames, 0, "{p:?}: the output safety limiter engaged");
        assert_eq!(r.last.queue_overflow, 0, "{p:?}: the guitarist dropped events");
        assert!(r.events.len() > 20, "{p:?}: only {} events in a minute", r.events.len());
        check_events_in_range(p, &r.events);
    }

    #[test]
    fn wander_plays_a_minute() {
        sixty_seconds(Pattern::Wander);
    }
    #[test]
    fn travis_plays_a_minute() {
        sixty_seconds(Pattern::Travis);
    }
    #[test]
    fn blues_plays_a_minute() {
        sixty_seconds(Pattern::Blues);
    }
    #[test]
    fn ambient_plays_a_minute() {
        sixty_seconds(Pattern::Ambient);
    }
    #[test]
    fn melody_plays_a_minute() {
        sixty_seconds(Pattern::Melody);
    }
    #[test]
    fn the_original_loops_play_a_minute() {
        for p in [Pattern::Strum, Pattern::Arpeggio, Pattern::Riff, Pattern::Notes] {
            sixty_seconds(p);
        }
    }

    /// What the guitarist did, bar by bar: (sixteenth within the window,
    /// string, fret, kind), ignoring touch.
    fn windows(p: Pattern, bars: usize, per: usize) -> Vec<Vec<(u32, u8, u8, u8)>> {
        let bpm = 92.0;
        let mut player = crate::player::Player::new(SR);
        let mut g = Guitar::new(SR);
        player.set_bpm(bpm);
        player.set_pattern(p);
        let sixteenth = SR * 60.0 / bpm / 4.0;
        let n = (bars as f64 * 16.0 * sixteenth) as usize;
        let mut out = vec![Vec::new(); bars / per];
        let mut events = Vec::new();
        for _ in 0..n {
            player.tick_with(&mut g, |t, e| events.push((t, e)));
            let _ = g.tick();
        }
        check_events_in_range(p, &events);
        assert_eq!(player.overflow(), 0, "{p:?}: queue overflow");
        for (t, e) in events {
            let step = (t as f64 / sixteenth).floor() as usize;
            let w = step / (16 * per);
            if w >= out.len() {
                continue;
            }
            let (kind, fret) = match e {
                Event::Pluck { fret, palm, .. } => (if palm { 1 } else { 0 }, fret),
                Event::Mute { .. } => (2, 0),
                Event::Bend { semitones, .. } => (3, (semitones * 4.0) as u8),
                Event::Vibrato { .. } => (4, 0),
            };
            out[w].push(((step % (16 * per)) as u32, e.string(), fret, kind));
        }
        for w in &mut out {
            w.sort_unstable();
        }
        out
    }

    #[test]
    fn the_generative_pieces_never_repeat_a_four_bar_window() {
        for p in [Pattern::Wander, Pattern::Travis, Pattern::Blues, Pattern::Ambient, Pattern::Melody] {
            let ws = windows(p, 128, 4);
            let busy: Vec<&Vec<(u32, u8, u8, u8)>> = ws.iter().filter(|w| w.len() >= 3).collect();
            let mut distinct = busy.clone();
            distinct.sort();
            distinct.dedup();
            eprintln!("{p:?}: {} four-bar windows with music, {} distinct", busy.len(), distinct.len());
            assert!(busy.len() >= 24, "{p:?}: only {} of 32 windows have music", busy.len());
            assert_eq!(distinct.len(), busy.len(), "{p:?} repeated a four-bar window");
        }
    }

    #[test]
    fn the_old_strum_loop_does_repeat_so_the_window_test_can_see_it() {
        let ws = windows(Pattern::Strum, 32, 4);
        let mut distinct = ws.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), 1, "Strum is a fixed four-bar loop");
    }

    #[test]
    fn master_mute_silences_the_output_and_ramps_without_a_click() {
        let (mut rig, mut ends) = Rig::new(SR).unwrap();
        ends.commands.push(Command::Ariadne(ovillo_patch())).unwrap();
        ends.commands.push(Command::Pattern(Pattern::Strum)).unwrap();
        let mut buf = [0.0f32; 2];
        let step_bound = rig.master_max_step() + 1e-6;
        // Unity gain, steady: the stage passes Ariadne through.
        for _ in 0..(SR as usize / 2) {
            rig.process(&mut buf, 2);
        }
        assert_eq!(rig.master, 1.0);
        let mut prev_gain = rig.master;
        let mut prev_out = buf;
        ends.commands.push(Command::Master { gain: 1.0, mute: true }).unwrap();
        let mut max_gain_step = 0.0f32;
        let mut max_out_step = 0.0f32;
        let mut silent_from = None;
        let mut scope_peak_while_muted = 0.0f32;
        for i in 0..(SR as usize) {
            rig.process(&mut buf, 2);
            max_gain_step = max_gain_step.max((rig.master - prev_gain).abs());
            max_out_step = max_out_step.max((buf[0] - prev_out[0]).abs()).max((buf[1] - prev_out[1]).abs());
            prev_gain = rig.master;
            prev_out = buf;
            if buf == [0.0, 0.0] && silent_from.is_none() && rig.master == 0.0 {
                silent_from = Some(i);
            }
            if silent_from.is_some() {
                assert_eq!(buf, [0.0, 0.0], "sound after silence at sample {i}");
            }
            while let Ok(f) = ends.scope.pop() {
                if silent_from.is_some() {
                    scope_peak_while_muted = scope_peak_while_muted.max(f.left.abs());
                }
            }
        }
        let at = silent_from.expect("mute never reached silence");
        eprintln!("mute: silent after {:.1} ms, largest gain step {max_gain_step:.5} (bound {step_bound:.5}), largest output step {max_out_step:.4}", at as f64 / SR * 1000.0);
        assert!(max_gain_step <= step_bound, "gain jumped {max_gain_step}");
        assert!(at as f64 / SR < 0.25, "mute took {at} samples");
        assert!(at as f64 / SR > 0.05, "mute was abrupt: {at} samples");
        // The instrument still plays (the screens see it); only the monitor is silent.
        assert!(scope_peak_while_muted > 0.01, "the taps went quiet too: {scope_peak_while_muted}");
        // Unmute at -12 dB: back up, smoothly, to that level.
        ends.commands.push(Command::Master { gain: master_gain(-12.0), mute: false }).unwrap();
        prev_gain = rig.master;
        let mut max_up = 0.0f32;
        for _ in 0..(SR as usize / 2) {
            rig.process(&mut buf, 2);
            max_up = max_up.max((rig.master - prev_gain).abs());
            prev_gain = rig.master;
        }
        assert!(max_up <= step_bound, "gain jumped {max_up} on the way up");
        assert!((rig.master - master_gain(-12.0)).abs() < 1e-6, "{}", rig.master);
    }

    #[test]
    fn the_master_stage_mutes_a_live_input_too() {
        let (mut rig, mut ends) = Rig::new(SR).unwrap();
        let (mut tx, rx) = rtrb::RingBuffer::<f32>::new(INPUT_MAX_FILL * 4);
        rig.attach_input(rx);
        ends.commands.push(Command::Ariadne(ovillo_patch())).unwrap();
        let mut phase = 0.0f32;
        let mut buf = [0.0f32; 2 * 64];
        let mut run = |rig: &mut Rig, blocks: usize, out_peak: &mut f32, last_block_peak: &mut f32| {
            for _ in 0..blocks {
                for _ in 0..64 {
                    // A plucked-string-ish input: a decaying 110 Hz tone, restruck every half second.
                    let env = (-((phase / (std::f32::consts::TAU * 110.0 / SR as f32)) % (SR as f32 / 2.0)) / 6000.0).exp();
                    let _ = tx.push(0.3 * env * phase.sin());
                    phase += std::f32::consts::TAU * 110.0 / SR as f32;
                }
                rig.process(&mut buf, 2);
                *last_block_peak = buf.iter().fold(0.0f32, |m, x| m.max(x.abs()));
                *out_peak = out_peak.max(*last_block_peak);
            }
        };
        let (mut before, mut last) = (0.0f32, 0.0f32);
        run(&mut rig, 750, &mut before, &mut last);
        assert!(before > 1e-3, "the live input reaches the output: {before}");
        ends.commands.push(Command::Master { gain: 1.0, mute: true }).unwrap();
        let (mut during, mut tail) = (0.0f32, 0.0f32);
        run(&mut rig, 300, &mut during, &mut tail);
        assert_eq!(tail, 0.0, "muted live output is silent");
        let mut t = None;
        while let Ok(x) = ends.telemetry.pop() {
            t = Some(x);
        }
        let t = t.unwrap();
        assert_eq!(t.source, Source::Live);
        assert!(t.master_muted && t.master == 0.0);
        assert_eq!((t.input_starved, t.input_dropped), (0, 0), "fed in step: nothing starved or dropped");
        let mut dry = 0.0f32;
        while let Ok(f) = ends.scope.pop() {
            dry = dry.max(f.dry.abs());
        }
        assert!(dry > 0.01, "the screens still see the input: {dry}");
    }

    #[test]
    fn a_rig_started_muted_is_silent_from_its_first_sample() {
        let (mut rig, mut ends) = Rig::new(SR).unwrap();
        rig.set_master_now(1.0, true);
        ends.commands.push(Command::Pattern(Pattern::Strum)).unwrap();
        let mut buf = vec![0.0f32; 2 * 512];
        let mut heard = 0.0f32;
        for _ in 0..200 {
            rig.process(&mut buf, 2);
            assert!(buf.iter().all(|x| *x == 0.0));
            while let Ok(f) = ends.scope.pop() {
                heard = heard.max(f.dry.abs());
            }
        }
        assert!(heard > 0.01, "the guitar played: {heard}");
    }

    #[test]
    fn a_score_plays_on_time_then_hands_back_to_the_composition() {
        let (mut rig, mut ends) = Rig::new(SR).unwrap();
        ends.commands.push(Command::Pattern(Pattern::Travis)).unwrap();
        rig.poll();
        for _ in 0..(SR as usize) {
            rig.frame();
        }
        while ends.events.pop().is_ok() {}
        while ends.telemetry.pop().is_ok() {}
        let sc = score::parse(
            &json!({"tempo": 120, "events": [
                {"type": "pluck", "beat": 0, "string": 0, "fret": 5},
                {"type": "pluck", "beat": 1, "string": "e", "fret": 7, "velocity": 0.5},
                {"type": "strum", "beat": 2, "chord": "Em", "spread_ms": 20}
            ]}),
            92.0,
        )
        .unwrap();
        let start = 48_000u64;
        ends.commands.push(Command::Score(Box::new(sc))).unwrap();
        rig.poll();
        let mut evs = Vec::new();
        let mut patterns = Vec::new();
        for _ in 0..(SR as usize * 3) {
            rig.frame();
            while let Ok(e) = ends.events.pop() {
                evs.push(e);
            }
            while let Ok(t) = ends.telemetry.pop() {
                patterns.push((t.pattern, t.score_ends));
            }
        }
        // Beat 0 at once, beat 1 half a second later (120 BPM), then the strum.
        assert_eq!(evs[0], (start, Event::Pluck { string: 0, fret: 5, velocity: 0.75, palm: false }));
        assert_eq!(evs[1].1, Event::Pluck { string: 5, fret: 7, velocity: 0.5, palm: false });
        assert!((evs[1].0 as i64 - (start + 24_000) as i64).abs() <= 1, "{:?}", evs[1]);
        let strum: Vec<u8> = evs[2..8].iter().map(|(_, e)| e.string()).collect();
        assert_eq!(strum, vec![0, 1, 2, 3, 4, 5]);
        assert!((evs[7].0 - evs[2].0) as f64 / SR > 0.019);
        // The score is a bar long (2 s at 120 BPM); then Travis resumes and its events follow.
        assert_eq!(patterns.first().map(|p| p.0), Some(Pattern::Score));
        let back = patterns.iter().position(|p| p.0 == Pattern::Travis).expect("Travis resumed");
        assert_eq!(patterns[back].1, 1);
        assert!(evs.iter().any(|(t, _)| *t > start + 96_000), "Travis plays again after the score");

        // A looping score replacing this one: the old box comes back on the garbage ring.
        let looped = score::parse(&json!({"loop": true, "tempo": 200, "length_beats": 1, "events": [{"type": "pluck", "beat": 0, "string": 2}]}), 92.0).unwrap();
        let second = score::parse(&json!({"loop": true, "tempo": 200, "length_beats": 1, "events": [{"type": "pluck", "beat": 0, "string": 3}]}), 92.0).unwrap();
        ends.commands.push(Command::Score(Box::new(looped))).unwrap();
        ends.commands.push(Command::Score(Box::new(second))).unwrap();
        rig.poll();
        let mut returned = 0;
        while ends.garbage.pop().is_ok() {
            returned += 1;
        }
        assert_eq!(returned, 2, "the first score and the looped one came back to be freed here");
        while ends.events.pop().is_ok() {}
        for _ in 0..(SR as usize) {
            rig.frame();
        }
        let mut plucks = 0;
        while let Ok((_, e)) = ends.events.pop() {
            if let Event::Pluck { string: 3, .. } = e {
                plucks += 1;
            }
        }
        // One beat at 200 BPM is 0.3 s: a second of looping plays it three or four times.
        assert!((3..=4).contains(&plucks), "{plucks}");
    }
}
