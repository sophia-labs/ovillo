//! The Ovillo rig for the browser: the simulated guitar, its guitarist and the
//! canonical Ariadne World (`ovillo_dsp::rig::Rig`), behind a plain C ABI so an
//! `AudioWorkletProcessor` can `WebAssembly.instantiate` the module directly,
//! with no bindgen glue.
//!
//! One rig per instance (`ovillo_init`). The worklet calls `ovillo_render(n)`
//! for each quantum and reads the wet left/right (through the master), the
//! World's own output before it, and the dry DI from the exported buffers; controls go in as rig [`Command`]s, applied at the start
//! of the next render, exactly as the native app sends them. Telemetry is a
//! fixed table of f64 slots ([`slot`]) refreshed every render.
//!
//! Two stages sit after Ariadne and before the speakers, neither part of the
//! World or of any patch: the master (a smoothed gain and mute, the app's own
//! monitor stage) and a runaway guard. The guard watches the World's safety
//! limiter; when it is limiting most of the time it fades out, rests the World
//! (`Command::ResetWorld`), and fades back in, and says so (`slot::GUARD`).
//! The World's physics are untouched.

use ovillo_dsp::ariadne::{AriadneControls, PARAMS};
use ovillo_dsp::guitar::{GuitarSettings, Pickup, STRINGS};
use ovillo_dsp::player::Pattern;
use ovillo_dsp::rig::{Command, Rig, RigEnds, Telemetry, MASTER_SMOOTHING_S};
use ovillo_dsp::score;

/// The largest quantum `ovillo_render` fills in one call.
pub const MAX_FRAMES: usize = 4096;

/// Ovillo's own default patch (the levels the bench is tuned for).
pub const DEFAULT_PATCH: &str = include_str!("../../../patches/ovillo-default.json");

/// The parameters the MAP square's Y axis can carry (the app's `MAP_Y`).
pub const MAP_Y: [usize; 6] = [4, 2, 3, 13, 11, 7];

/// Within this of its target a gain snaps there (the app's master snap).
const SNAP: f32 = 1e-4;

/// Telemetry slots, in the f64 table `ovillo_telemetry_ptr` points at.
pub mod slot {
    pub const MATERIAL: usize = 0;
    pub const RESERVOIR: usize = 1;
    pub const SPACE: usize = 2;
    pub const CONFIGURATION: usize = 3;
    /// Summed over the last telemetry block.
    pub const SOURCE_WORK: usize = 4;
    pub const INTERNAL_LOSS: usize = 5;
    pub const RADIATION: usize = 6;
    pub const CLOSURE_RESIDUAL: usize = 7;
    pub const SAFETY_LIMITED_FRAMES: usize = 8;
    pub const WORLD_ERRORS: usize = 9;
    /// 0 ok, 1 fading out, 2 resting, 3 fading in.
    pub const GUARD: usize = 10;
    pub const GUARD_TRIPS: usize = 11;
    pub const WET_PEAK: usize = 12;
    pub const WET_RMS: usize = 13;
    pub const DRY_PEAK: usize = 14;
    pub const DRY_RMS: usize = 15;
    /// The master stage's smoothed gain, times the guard's.
    pub const MASTER: usize = 16;
    pub const SAMPLE: usize = 17;
    pub const BPM: usize = 18;
    pub const SCORE_BEAT: usize = 19;
    pub const SCORE_LENGTH: usize = 20;
    /// Index into the pattern list `ovillo_info` gives.
    pub const PATTERN: usize = 21;
    /// Bit i set: string i (0 = low E) is sounding.
    pub const SOUNDING: usize = 22;
    /// Fraction of the guard's window the limiter was on.
    pub const LIMITING: usize = 23;
    pub const TELEMETRY_BLOCKS: usize = 24;
    /// Six string levels, then six sounding frequencies (Hz), then six frets.
    pub const STRING_LEVEL: usize = 32;
    pub const STRING_HZ: usize = 38;
    pub const STRING_FRET: usize = 44;
    /// The World's 24 modes: energies, then frequencies (Hz).
    pub const MODE_ENERGY: usize = 64;
    pub const MODE_HZ: usize = 88;
    pub const COUNT: usize = 112;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Guard {
    Ok,
    Fading,
    Resting,
    Returning,
}

pub struct Web {
    sr: f64,
    rig: Rig,
    ends: RigEnds,
    controls: AriadneControls,
    guitar: GuitarSettings,
    pattern: Pattern,
    /// The pattern START comes back to.
    last_pattern: Pattern,
    master_target: f32,
    master_mute: bool,
    master: f32,
    master_coef: f32,
    guard: Guard,
    guard_gain: f32,
    guard_until: u64,
    guard_trips: u64,
    limiting: f64,
    last_limited: u64,
    sample: u64,
    tele: Option<Telemetry>,
    blocks: u64,
    pub left: Box<[f32; MAX_FRAMES]>,
    pub right: Box<[f32; MAX_FRAMES]>,
    pub dry: Box<[f32; MAX_FRAMES]>,
    /// The World's output, mono, before the master and the guard (what the screens draw).
    pub wet: Box<[f32; MAX_FRAMES]>,
    pub slots: Box<[f64; slot::COUNT]>,
    /// Text out (info JSON, errors) and text in (scores).
    text: Vec<u8>,
    input: Vec<u8>,
}

impl Web {
    pub fn new(sr: f64, seed: u32) -> Result<Self, String> {
        let (mut rig, mut ends) = Rig::new(sr)?;
        rig.seed(seed.max(1));
        let controls: AriadneControls = serde_json::from_str(DEFAULT_PATCH).map_err(|e| e.to_string())?;
        let guitar = GuitarSettings::default();
        let pattern = Pattern::Off;
        for c in [Command::Ariadne(controls), Command::Guitar(guitar), Command::Pattern(pattern), Command::Tempo(92.0)] {
            ends.commands.push(c).map_err(|_| "command ring full")?;
        }
        rig.poll();
        let master_target = 10f32.powf(-12.0 / 20.0);
        Ok(Self {
            sr,
            rig,
            ends,
            controls,
            guitar,
            pattern,
            last_pattern: Pattern::Wander,
            master_target,
            master_mute: false,
            master: master_target,
            master_coef: (1.0 - (-1.0 / (MASTER_SMOOTHING_S * sr)).exp()) as f32,
            guard: Guard::Ok,
            guard_gain: 1.0,
            guard_until: 0,
            guard_trips: 0,
            limiting: 0.0,
            last_limited: 0,
            sample: 0,
            tele: None,
            blocks: 0,
            left: Box::new([0.0; MAX_FRAMES]),
            right: Box::new([0.0; MAX_FRAMES]),
            dry: Box::new([0.0; MAX_FRAMES]),
            wet: Box::new([0.0; MAX_FRAMES]),
            slots: Box::new([0.0; slot::COUNT]),
            text: Vec::with_capacity(16 * 1024),
            input: Vec::new(),
        })
    }

    fn send(&mut self, c: Command) {
        // The ring holds 1024; renders drain it every quantum.
        let _ = self.ends.commands.push(c);
    }

    pub fn pluck(&mut self, string: u8, fret: u8, velocity: f32) {
        if (string as usize) < STRINGS {
            self.send(Command::Pluck { string, fret: fret.min(22), velocity: velocity.clamp(0.0, 1.0) });
        }
    }

    pub fn set_pattern(&mut self, p: Pattern) {
        if p != Pattern::Off {
            self.last_pattern = p;
        }
        self.pattern = p;
        self.send(Command::Pattern(p));
    }

    /// START (the last pattern, WANDER the first time) or STOP (OFF: what rings, rings out).
    pub fn transport(&mut self, on: bool) {
        self.set_pattern(if on { self.last_pattern } else { Pattern::Off });
    }

    /// Set one Ariadne parameter (the `PARAMS` index), clamped as the plug-in clamps; returns the value set.
    pub fn set_param(&mut self, i: usize, v: f64) -> f64 {
        if i >= PARAMS.len() {
            return f64::NAN;
        }
        self.controls.set(i, v);
        self.controls = self.controls.sanitized();
        self.send(Command::Ariadne(self.controls));
        self.controls.get(i)
    }

    pub fn set_pickup(&mut self, i: usize) {
        if let Some(p) = Pickup::ALL.get(i) {
            self.guitar.pickup = *p;
            self.send(Command::Guitar(self.guitar));
        }
    }

    pub fn set_guitar(&mut self, tone: f64, volume: f64, hum: bool) {
        self.guitar.tone = tone.clamp(0.0, 1.0);
        self.guitar.volume = volume.clamp(0.0, 1.0);
        self.guitar.hum = hum;
        self.send(Command::Guitar(self.guitar));
    }

    pub fn load_score(&mut self, json: &str) -> Result<String, String> {
        let s = score::parse_str(json, 92.0)?;
        let summary = s.summary();
        self.pattern = Pattern::Score;
        self.send(Command::Score(Box::new(s)));
        Ok(summary.to_string())
    }

    pub fn reset_world(&mut self) {
        self.send(Command::ResetWorld);
    }

    pub fn set_master(&mut self, gain: f32, mute: bool) {
        self.master_target = if gain.is_finite() { gain.clamp(0.0, 1.0) } else { 0.0 };
        self.master_mute = mute;
    }

    /// The master at once, no ramp: before the first quantum, so a muted start is silent from sample one.
    pub fn set_master_now(&mut self, gain: f32, mute: bool) {
        self.set_master(gain, mute);
        self.master = if mute { 0.0 } else { self.master_target };
    }

    /// Fill `left`, `right` and `dry` with `n` frames.
    pub fn render(&mut self, n: usize) -> usize {
        let n = n.min(MAX_FRAMES);
        self.rig.poll();
        for i in 0..n {
            let (d, l, r) = self.rig.frame();
            let target = if self.master_mute { 0.0 } else { self.master_target };
            let dm = target - self.master;
            self.master = if dm.abs() < SNAP { target } else { self.master + dm * self.master_coef };
            let g = self.master * self.guard_tick();
            self.dry[i] = d;
            self.wet[i] = 0.5 * (l + r);
            self.left[i] = l * g;
            self.right[i] = r * g;
        }
        self.drain();
        self.write_slots();
        n
    }

    /// One sample of the runaway guard's gain.
    fn guard_tick(&mut self) -> f32 {
        self.sample += 1;
        let (target, rate) = match self.guard {
            Guard::Ok => (1.0, 0.0),
            Guard::Fading => (0.0, 60e-3),
            Guard::Resting => (0.0, 0.0),
            Guard::Returning => (1.0, 300e-3),
        };
        if rate > 0.0 {
            let step = (1.0 / (rate * self.sr)) as f32;
            self.guard_gain = if target > self.guard_gain {
                (self.guard_gain + step).min(1.0)
            } else {
                (self.guard_gain - step).max(0.0)
            };
        }
        match self.guard {
            Guard::Fading if self.guard_gain <= 0.0 => {
                let _ = self.ends.commands.push(Command::ResetWorld);
                self.limiting = 0.0;
                self.guard = Guard::Resting;
                self.guard_until = self.sample + (0.25 * self.sr) as u64;
            }
            Guard::Resting if self.sample >= self.guard_until => self.guard = Guard::Returning,
            Guard::Returning if self.guard_gain >= 1.0 => self.guard = Guard::Ok,
            _ => {}
        }
        self.guard_gain
    }

    /// Empty the rig's taps (the scope is read from `render`'s own buffers),
    /// keep the newest telemetry, and free replaced scores.
    fn drain(&mut self) {
        let k = self.ends.scope.slots();
        if let Ok(chunk) = self.ends.scope.read_chunk(k) {
            chunk.commit_all();
        }
        let k = self.ends.events.slots();
        if let Ok(chunk) = self.ends.events.read_chunk(k) {
            chunk.commit_all();
        }
        while self.ends.garbage.pop().is_ok() {}
        while let Ok(t) = self.ends.telemetry.pop() {
            self.blocks += 1;
            // The guard's window: an exponential average of the fraction of
            // frames the limiter held, over about half a second.
            let limited = t.safety_limited_frames.saturating_sub(self.last_limited);
            self.last_limited = t.safety_limited_frames;
            let block = ovillo_dsp::rig::telemetry_every(self.sr) as f64;
            let a = block / (0.5 * self.sr);
            self.limiting += ((limited as f64 / block).min(1.0) - self.limiting) * a;
            if self.guard == Guard::Ok && self.limiting > 0.35 {
                self.guard = Guard::Fading;
                self.guard_trips += 1;
            }
            self.tele = Some(t);
        }
    }

    fn write_slots(&mut self) {
        let s = &mut self.slots;
        s[slot::GUARD] = match self.guard {
            Guard::Ok => 0.0,
            Guard::Fading => 1.0,
            Guard::Resting => 2.0,
            Guard::Returning => 3.0,
        };
        s[slot::GUARD_TRIPS] = self.guard_trips as f64;
        s[slot::MASTER] = (self.master * self.guard_gain) as f64;
        s[slot::LIMITING] = self.limiting;
        s[slot::TELEMETRY_BLOCKS] = self.blocks as f64;
        s[slot::PATTERN] = Pattern::ALL.iter().position(|p| *p == self.pattern).unwrap_or(0) as f64;
        let Some(t) = &self.tele else { return };
        s[slot::MATERIAL] = t.world.material_energy;
        s[slot::RESERVOIR] = t.world.reservoir_energy;
        s[slot::SPACE] = t.world.space_energy;
        s[slot::CONFIGURATION] = t.world.configuration;
        s[slot::SOURCE_WORK] = t.world.source_work;
        s[slot::INTERNAL_LOSS] = t.world.internal_loss;
        s[slot::RADIATION] = t.world.radiation;
        s[slot::CLOSURE_RESIDUAL] = t.world.global_closure_residual;
        s[slot::SAFETY_LIMITED_FRAMES] = t.safety_limited_frames as f64;
        s[slot::WORLD_ERRORS] = t.world_errors as f64;
        s[slot::WET_PEAK] = t.wet_peak as f64;
        s[slot::WET_RMS] = t.wet_rms as f64;
        s[slot::DRY_PEAK] = t.dry_peak as f64;
        s[slot::DRY_RMS] = t.dry_rms as f64;
        s[slot::SAMPLE] = t.sample as f64;
        s[slot::BPM] = t.bpm as f64;
        s[slot::SCORE_BEAT] = t.score_beat as f64;
        s[slot::SCORE_LENGTH] = t.score_length as f64;
        if t.pattern != Pattern::Score || self.pattern == Pattern::Score {
            // A one-shot score hands the guitarist back to what it played before.
            s[slot::PATTERN] = Pattern::ALL.iter().position(|p| *p == t.pattern).unwrap_or(0) as f64;
        }
        let mut bits = 0u32;
        for (i, st) in t.strings.iter().enumerate() {
            if st.sounding {
                bits |= 1 << i;
            }
            s[slot::STRING_LEVEL + i] = st.level as f64;
            s[slot::STRING_HZ + i] = st.hz as f64;
            s[slot::STRING_FRET + i] = st.fret as f64;
        }
        s[slot::SOUNDING] = bits as f64;
        for i in 0..mus_ariadne::MODE_COUNT.min(24) {
            s[slot::MODE_ENERGY + i] = t.modes[i] as f64;
            s[slot::MODE_HZ + i] = t.mode_hz[i] as f64;
        }
    }

    /// Everything the page needs to draw its menus, as JSON.
    pub fn info(&self) -> String {
        let patterns: Vec<_> = Pattern::ALL
            .iter()
            .map(|p| serde_json::json!({"id": p.id(), "label": p.label(), "about": p.about()}))
            .collect();
        let params: Vec<_> = PARAMS
            .iter()
            .enumerate()
            .map(|(i, p)| {
                serde_json::json!({"index": i, "id": p.id, "name": p.name, "label": p.label,
                    "min": p.min, "max": p.max, "slow": p.slow, "value": self.controls.get(i)})
            })
            .collect();
        let pickups: Vec<_> = Pickup::ALL.iter().map(|p| p.label()).collect();
        serde_json::json!({
            "sampleRate": self.sr,
            "planDigest": self.rig.plan_digest(),
            "maxFrames": MAX_FRAMES,
            "patterns": patterns,
            "params": params,
            "pickups": pickups,
            "pickup": Pickup::ALL.iter().position(|p| *p == self.guitar.pickup),
            "mapY": MAP_Y,
            "slots": {
                "material": slot::MATERIAL, "reservoir": slot::RESERVOIR, "space": slot::SPACE,
                "configuration": slot::CONFIGURATION, "sourceWork": slot::SOURCE_WORK,
                "internalLoss": slot::INTERNAL_LOSS, "radiation": slot::RADIATION,
                "closureResidual": slot::CLOSURE_RESIDUAL, "safetyLimitedFrames": slot::SAFETY_LIMITED_FRAMES,
                "worldErrors": slot::WORLD_ERRORS, "guard": slot::GUARD, "guardTrips": slot::GUARD_TRIPS,
                "wetPeak": slot::WET_PEAK, "wetRms": slot::WET_RMS, "dryPeak": slot::DRY_PEAK,
                "dryRms": slot::DRY_RMS, "master": slot::MASTER, "sample": slot::SAMPLE, "bpm": slot::BPM,
                "scoreBeat": slot::SCORE_BEAT, "scoreLength": slot::SCORE_LENGTH, "pattern": slot::PATTERN,
                "sounding": slot::SOUNDING, "limiting": slot::LIMITING, "telemetryBlocks": slot::TELEMETRY_BLOCKS,
                "stringLevel": slot::STRING_LEVEL, "stringHz": slot::STRING_HZ, "stringFret": slot::STRING_FRET,
                "modeEnergy": slot::MODE_ENERGY, "modeHz": slot::MODE_HZ, "count": slot::COUNT
            }
        })
        .to_string()
    }
}

// ------------------------------------------------------------------ C ABI
//
// A worklet owns exactly one instance and calls in from one thread, so the
// rig lives in one static. Every export is a no-op (or returns an error
// value) before `ovillo_init`.

static mut WEB: Option<Web> = None;

#[allow(static_mut_refs)]
fn web() -> Option<&'static mut Web> {
    // SAFETY: one instance per WebAssembly module, called from one thread.
    unsafe { WEB.as_mut() }
}

fn put_text(w: &mut Web, s: &str) -> u32 {
    w.text.clear();
    w.text.extend_from_slice(s.as_bytes());
    w.text.len() as u32
}

/// Build the rig at `sample_rate`; 0 on success, else the length of an error in the text buffer.
#[no_mangle]
pub extern "C" fn ovillo_init(sample_rate: f64, seed: u32) -> i32 {
    match Web::new(sample_rate, seed) {
        Ok(w) => {
            // SAFETY: as in `web`.
            unsafe { WEB = Some(w) };
            0
        }
        Err(_) => -1,
    }
}

#[no_mangle]
pub extern "C" fn ovillo_render(frames: u32) -> u32 {
    web().map_or(0, |w| w.render(frames as usize) as u32)
}

#[no_mangle]
pub extern "C" fn ovillo_max_frames() -> u32 {
    MAX_FRAMES as u32
}

#[no_mangle]
pub extern "C" fn ovillo_left_ptr() -> *const f32 {
    web().map_or(std::ptr::null(), |w| w.left.as_ptr())
}

#[no_mangle]
pub extern "C" fn ovillo_right_ptr() -> *const f32 {
    web().map_or(std::ptr::null(), |w| w.right.as_ptr())
}

#[no_mangle]
pub extern "C" fn ovillo_dry_ptr() -> *const f32 {
    web().map_or(std::ptr::null(), |w| w.dry.as_ptr())
}

#[no_mangle]
pub extern "C" fn ovillo_wet_ptr() -> *const f32 {
    web().map_or(std::ptr::null(), |w| w.wet.as_ptr())
}

#[no_mangle]
pub extern "C" fn ovillo_telemetry_ptr() -> *const f64 {
    web().map_or(std::ptr::null(), |w| w.slots.as_ptr())
}

#[no_mangle]
pub extern "C" fn ovillo_text_ptr() -> *const u8 {
    web().map_or(std::ptr::null(), |w| w.text.as_ptr())
}

/// Write the info JSON into the text buffer; returns its length.
#[no_mangle]
pub extern "C" fn ovillo_info() -> u32 {
    web().map_or(0, |w| {
        let s = w.info();
        put_text(w, &s)
    })
}

/// `string` 0 = low E … 5 = high e.
#[no_mangle]
pub extern "C" fn ovillo_pluck(string: u32, fret: u32, velocity: f32) {
    if let Some(w) = web() {
        w.pluck(string.min(255) as u8, fret.min(255) as u8, velocity);
    }
}

/// An index into the info JSON's `patterns`.
#[no_mangle]
pub extern "C" fn ovillo_set_pattern(index: u32) -> i32 {
    match (web(), Pattern::ALL.get(index as usize)) {
        (Some(w), Some(p)) if *p != Pattern::Score => {
            w.set_pattern(*p);
            0
        }
        _ => -1,
    }
}

#[no_mangle]
pub extern "C" fn ovillo_transport(on: u32) {
    if let Some(w) = web() {
        w.transport(on != 0);
    }
}

#[no_mangle]
pub extern "C" fn ovillo_set_tempo(bpm: f64) {
    if let Some(w) = web() {
        w.send(Command::Tempo(bpm));
    }
}

/// Returns the clamped value set, or NaN for an unknown index.
#[no_mangle]
pub extern "C" fn ovillo_set_param(index: u32, value: f64) -> f64 {
    web().map_or(f64::NAN, |w| w.set_param(index as usize, value))
}

#[no_mangle]
pub extern "C" fn ovillo_get_param(index: u32) -> f64 {
    web().map_or(f64::NAN, |w| if (index as usize) < PARAMS.len() { w.controls.get(index as usize) } else { f64::NAN })
}

#[no_mangle]
pub extern "C" fn ovillo_set_pickup(index: u32) {
    if let Some(w) = web() {
        w.set_pickup(index as usize);
    }
}

#[no_mangle]
pub extern "C" fn ovillo_set_guitar(tone: f64, volume: f64, hum: u32) {
    if let Some(w) = web() {
        w.set_guitar(tone, volume, hum != 0);
    }
}

/// Room for `len` bytes of input (a score's JSON); write there, then call `ovillo_load_score(len)`.
#[no_mangle]
pub extern "C" fn ovillo_input_ptr(len: u32) -> *mut u8 {
    web().map_or(std::ptr::null_mut(), |w| {
        w.input.clear();
        w.input.resize(len as usize, 0);
        w.input.as_mut_ptr()
    })
}

/// Play the score in the input buffer. 0 and a summary in the text buffer, or
/// -1 and the parser's error there; `ovillo_text_len` gives the length.
#[no_mangle]
pub extern "C" fn ovillo_load_score(len: u32) -> i32 {
    let Some(w) = web() else { return -1 };
    let n = (len as usize).min(w.input.len());
    let json = String::from_utf8_lossy(&w.input[..n]).into_owned();
    match w.load_score(&json) {
        Ok(summary) => {
            put_text(w, &summary);
            0
        }
        Err(e) => {
            put_text(w, &e);
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn ovillo_text_len() -> u32 {
    web().map_or(0, |w| w.text.len() as u32)
}

#[no_mangle]
pub extern "C" fn ovillo_reset_world() {
    if let Some(w) = web() {
        w.reset_world();
    }
}

#[no_mangle]
pub extern "C" fn ovillo_mute_strings() {
    if let Some(w) = web() {
        w.send(Command::MuteStrings);
    }
}

/// The monitor: linear gain 0..1 and mute, smoothed (10 ms).
#[no_mangle]
pub extern "C" fn ovillo_set_master(gain: f32, mute: u32) {
    if let Some(w) = web() {
        w.set_master(gain, mute != 0);
    }
}

#[no_mangle]
pub extern "C" fn ovillo_set_master_now(gain: f32, mute: u32) {
    if let Some(w) = web() {
        w.set_master_now(gain, mute != 0);
    }
}

/// Restore Ovillo's default patch; the page re-reads `ovillo_info` after.
#[no_mangle]
pub extern "C" fn ovillo_default_patch() {
    if let Some(w) = web() {
        if let Ok(c) = serde_json::from_str::<AriadneControls>(DEFAULT_PATCH) {
            w.controls = c;
            w.send(Command::Ariadne(c));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f64 = 48_000.0;

    fn render_seconds(w: &mut Web, s: f64) -> (f32, f64) {
        let mut peak = 0.0f32;
        let mut sq = 0.0f64;
        let mut n = 0usize;
        let total = (s * SR) as usize;
        while n < total {
            let k = w.render(128);
            for i in 0..k {
                assert!(w.left[i].is_finite() && w.right[i].is_finite() && w.dry[i].is_finite());
                peak = peak.max(w.left[i].abs()).max(w.right[i].abs());
                sq += (w.left[i] as f64).powi(2);
            }
            n += k;
        }
        (peak, (sq / n as f64).sqrt())
    }

    #[test]
    fn a_pattern_plays_through_the_world_finite_audible_and_balanced() {
        let mut w = Web::new(SR, 7).unwrap();
        w.transport(true);
        let (peak, rms) = render_seconds(&mut w, 6.0);
        assert!(peak > 1e-3, "wet peak {peak}");
        assert!(rms > 1e-5, "wet rms {rms}");
        let res = w.slots[slot::CLOSURE_RESIDUAL];
        assert!(res.abs() < 1e-9, "closure residual {res}");
        assert_eq!(w.slots[slot::WORLD_ERRORS], 0.0);
        assert!(w.slots[slot::MATERIAL] > 0.0);
        assert_eq!(w.slots[slot::PATTERN] as usize, 0, "WANDER");
        assert_eq!(w.slots[slot::GUARD], 0.0);
    }

    #[test]
    fn a_muted_start_is_silent_from_the_first_sample() {
        let mut w = Web::new(SR, 5).unwrap();
        w.set_master_now(1.0, true);
        w.pluck(0, 0, 1.0);
        let (peak, _) = render_seconds(&mut w, 0.25);
        assert_eq!(peak, 0.0);
        assert!(w.wet.iter().any(|x| *x != 0.0), "the screens still see the World");
    }

    #[test]
    fn mute_reaches_exact_silence_and_a_pluck_is_heard_unmuted() {
        let mut w = Web::new(SR, 3).unwrap();
        w.set_master(1.0, true);
        w.pluck(0, 0, 0.9);
        let (peak, _) = render_seconds(&mut w, 0.5);
        assert!(peak > 0.0, "the ramp to mute starts from -12 dB");
        let (peak, _) = render_seconds(&mut w, 0.5);
        assert_eq!(peak, 0.0);
        w.set_master(1.0, false);
        w.pluck(5, 3, 0.9);
        let (peak, _) = render_seconds(&mut w, 0.5);
        assert!(peak > 1e-3, "{peak}");
    }

    #[test]
    fn a_world_held_in_the_limiter_trips_the_guard_rests_and_returns() {
        let mut w = Web::new(SR, 9).unwrap();
        // Everything that turns the World up, as far as it goes.
        for (i, v) in [(1, 4.0), (17, 64.0), (19, 8.0), (4, 1.5), (7, 1.0), (9, 1.0)] {
            w.set_param(i, v);
        }
        w.set_pattern(Pattern::Strum);
        let mut states = Vec::new();
        for _ in 0..(SR as usize * 8 / 128) {
            w.render(128);
            let g = w.slots[slot::GUARD] as u8;
            if states.last() != Some(&g) {
                states.push(g);
            }
        }
        assert!(w.slots[slot::GUARD_TRIPS] >= 1.0, "no trip; limiting {}", w.slots[slot::LIMITING]);
        // ok → fading → resting → returning, in that order, at least once.
        let s: String = states.iter().map(|g| g.to_string()).collect();
        assert!(s.contains("0123"), "guard states {s}");
    }

    #[test]
    fn params_clamp_like_the_plugin_and_scores_load() {
        let mut w = Web::new(SR, 1).unwrap();
        assert_eq!(w.set_param(0, 7.0), 1.0);
        assert_eq!(w.set_param(5, 0.0), 0.02);
        assert!(w.set_param(99, 0.0).is_nan());
        let s = include_str!("../../../scores/first-light.json");
        assert!(w.load_score(s).is_ok());
        assert!(w.load_score("{\"events\": [{\"type\": \"pluck\"}]}").is_err());
        let info: serde_json::Value = serde_json::from_str(&w.info()).unwrap();
        assert_eq!(info["params"].as_array().unwrap().len(), 20);
        assert_eq!(info["patterns"][0]["id"], "wander");
    }
}
