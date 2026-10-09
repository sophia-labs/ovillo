//! The rig: guitarist → guitar → Ariadne World → stereo out, one sample at a
//! time, on whichever thread owns it (the audio callback, or an offline render).
//!
//! The control thread talks to it through three lock-free rings made in
//! [`Rig::new`]: commands in, a scope tap out (every sample: dry, wet L, wet R),
//! and a telemetry tap out (every [`TELEMETRY_EVERY`] samples: levels, the
//! World's own readings, the 24 modes, load). Nothing on the sample path
//! allocates or locks; a full tap drops frames and counts them.

use crate::ariadne::{AriadneControls, AriadneEngine, WorldReadings, PARAMS};
use crate::guitar::{Guitar, GuitarSettings, STRINGS};
use crate::player::{Event, Pattern, Player};
use mus_ariadne::MODE_COUNT;
use std::time::Instant;

pub const TELEMETRY_EVERY: usize = 256;
const FAST_SMOOTHING_S: f64 = 0.007;
const SLOW_SMOOTHING_S: f64 = 0.020;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    Ariadne(AriadneControls),
    Guitar(GuitarSettings),
    Pattern(Pattern),
    Tempo(f64),
    Pluck { string: u8, fret: u8, velocity: f32 },
    MuteStrings,
    ResetWorld,
}

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
    pub world_errors: u64,
    pub safety_limited_frames: u64,
    pub dropped_scope: u64,
    /// Fraction of real time the last processed buffer took.
    pub load: f32,
}

pub struct RigEnds {
    pub commands: rtrb::Producer<Command>,
    pub scope: rtrb::Consumer<ScopeFrame>,
    pub telemetry: rtrb::Consumer<Telemetry>,
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
}

impl Rig {
    pub fn new(sr: f64) -> Result<(Self, RigEnds), String> {
        let ariadne = AriadneEngine::new(sr)?;
        let (cmd_tx, cmd_rx) = rtrb::RingBuffer::new(1024);
        let (scope_tx, scope_rx) = rtrb::RingBuffer::new((sr * 2.0) as usize);
        let (tele_tx, tele_rx) = rtrb::RingBuffer::new(4096);
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
        };
        Ok((rig, RigEnds { commands: cmd_tx, scope: scope_rx, telemetry: tele_rx }))
    }

    #[must_use]
    pub fn sample_rate(&self) -> f64 {
        self.sr
    }

    #[must_use]
    pub fn plan_digest(&self) -> &str {
        self.ariadne.plan_digest()
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
                Command::MuteStrings => self.guitar.mute_all(),
                Command::ResetWorld => self.ariadne.reset(),
            }
        }
    }

    /// One sample: (dry DI, wet left, wet right).
    #[inline]
    pub fn frame(&mut self) -> (f32, f32, f32) {
        self.player.tick(&mut self.guitar);
        let dry = self.guitar.tick();
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
            world_errors: self.ariadne.world_errors,
            safety_limited_frames: self.ariadne.safety_limited_frames,
            dropped_scope: self.dropped_scope,
            load: self.load,
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

    /// Fill an interleaved output buffer of `channels` channels (wet L, wet R;
    /// further channels silent; mono gets the wet sum).
    pub fn process(&mut self, out: &mut [f32], channels: usize) {
        let start = Instant::now();
        self.drain_commands();
        let channels = channels.max(1);
        for frame in out.chunks_mut(channels) {
            let (_, l, r) = self.frame();
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

    #[test]
    fn a_strummed_bar_comes_out_of_ariadne_finite_and_audible() {
        let sr = 48_000.0;
        let (mut rig, mut ends) = Rig::new(sr).unwrap();
        let mut wet_peak = 0.0f32;
        let mut dry_peak = 0.0f32;
        for _ in 0..(sr as usize * 2) {
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
}
