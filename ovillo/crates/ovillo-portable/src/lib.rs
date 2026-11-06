//! Every Ovillo World behind one WebAssembly module and one surface.
//!
//! Nothing here synthesizes a sound. Each *kind* (the steel-string, the
//! piano, the trombone, the vocal tract, the bench, the film's and Sunskip's
//! tab anatomies, the fart) is an Ariadne World built by the crate that owns
//! it, from the same plans, laws and constants its own renderer uses. The
//! game's sources (`ovillo-game-sound`: kart engine, race props, turbofan)
//! are standalone Worlds on that crate's own wave-network runtime, booking a
//! `mus_watsn` work ledger with the same receipt fields; the catalog says so
//! for each. This crate only decides *when* a touch reaches a World:
//!
//! - an [`Instance`] owns one World (many instances per module, each with a
//!   handle in [`abi`]);
//! - **events** (a pluck, a key, a note, a gesture) and **continuous
//!   controls** (breath, slide, tract shape, bow speed, ...) are scheduled at
//!   World frames. A render is split at every due event, so an event lands on
//!   its frame (sample-accurate); a ramped control moves every
//!   [`RAMP_STEP`] frames;
//! - a render fills `n` stereo frames: the World's own observation times one
//!   fixed listening gain per kind (`catalog: output`). No mix bus, no
//!   master, no limiter, no spatialization: those belong to the consumer
//!   (`web/ovillo/README.md`, "The boundary");
//! - the World's ledger is read back as telemetry (cheap, every block) and as
//!   a receipt (JSON, on demand), with a SHA-256 of every sample rendered.
//!
//! What each World does not model travels in the catalog (`ovillo-catalog.json`),
//! quoted from its spec, beside its events and controls.

pub mod abi;
pub mod catalog;
pub mod kinds;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub use catalog::{KindDef, Ledger};

/// Largest render request per call.
pub const MAX_FRAMES: usize = 4096;
/// Arguments an event carries (numbers; enumerations by index).
pub const MAX_ARGS: usize = 12;
/// A ramped control is re-set every this many frames.
pub const RAMP_STEP: u64 = 32;
/// The ledger is re-read into telemetry at most this often (frames).
pub const LEDGER_EVERY: u64 = 1024;

/// Common telemetry slots; a kind's own follow from [`slot::KIND`].
pub mod slot {
    pub const FRAME: usize = 0;
    pub const SECONDS: usize = 1;
    pub const SOURCE_WORK: usize = 2;
    pub const CONTROL_WORK: usize = 3;
    pub const INTERNAL_LOSS: usize = 4;
    pub const RADIATION: usize = 5;
    pub const STORED_ENERGY: usize = 6;
    pub const INITIAL_ENERGY: usize = 7;
    pub const CUMULATIVE_RESIDUAL: usize = 8;
    pub const MAX_FRAME_RESIDUAL: usize = 9;
    /// Over the last render call.
    pub const PEAK: usize = 10;
    pub const RMS: usize = 11;
    pub const ERRORS: usize = 12;
    /// Samples a kind's own listener clamped (tract, bench), cumulative.
    pub const CLAMPED: usize = 13;
    pub const EVENTS_APPLIED: usize = 14;
    pub const EVENTS_REFUSED: usize = 15;
    pub const KIND: usize = 16;
    pub const NAMES: [&str; 16] = [
        "frame",
        "seconds",
        "sourceWork",
        "controlWork",
        "internalLoss",
        "radiation",
        "storedEnergy",
        "initialEnergy",
        "cumulativeClosureResidual",
        "maximumFrameResidual",
        "peak",
        "rms",
        "errors",
        "clamped",
        "eventsApplied",
        "eventsRefused",
    ];
}

/// One World of a kind, as a kind module builds it.
pub trait Source {
    /// Render `left.len()` frames (left and right the same length): the
    /// World's observation times the kind's listening gain.
    fn render(&mut self, left: &mut [f32], right: &mut [f32]) -> Result<(), String>;
    /// Event `index` of the kind's catalog entry, with its arguments. `at` is
    /// the frame it is meant for and `now` the World's clock (`at >= now`;
    /// they differ only by the kind's [`Source::lookahead`]).
    fn event(&mut self, index: usize, args: &[f64; MAX_ARGS], at: u64, now: u64) -> Result<(), String>;
    /// Continuous control `index` of the catalog entry, now.
    fn set(&mut self, index: usize, value: f64) -> Result<(), String>;
    /// The World's own books.
    fn ledger(&self) -> Ledger;
    /// The World's receipt in its renderer's own shape.
    fn receipt(&self) -> Value;
    /// The kind's telemetry slots (catalog `telemetry`), in order.
    fn telemetry(&self, out: &mut [f64]);
    /// The World again at rest; clock and books at zero.
    fn reset(&mut self) -> Result<(), String>;
    /// Schedule a score (the kind's own format, catalog `score`) from frame
    /// `at`. Returns what reading it found.
    fn play_score(&mut self, _text: &str, _at: u64) -> Result<Value, String> {
        Err("this kind reads no score".into())
    }
    /// How many frames before its time an event is handed over (a kind that
    /// schedules inside the World: a tab fret lands before its pluck, a piano
    /// damper rises before its hammer arrives).
    fn lookahead(&self) -> u64 {
        0
    }
    /// Samples the kind's own listener clamped so far.
    fn clamped(&self) -> u64 {
        0
    }
    fn plan_digest(&self) -> String;
}

/// A World's construction parameters, from `create`.
pub struct CreateCtx<'a> {
    pub sample_rate: f64,
    pub anatomy: Option<&'a str>,
    /// The options object (kind-specific keys; see the catalog's `options`).
    pub options: &'a Value,
}

#[derive(Debug, Clone, Copy)]
enum Pending {
    Event { index: usize, args: [f64; MAX_ARGS] },
    Control { index: usize, value: f64, ramp: u64 },
}

#[derive(Debug, Clone, Copy)]
struct Ramp {
    index: usize,
    from: f64,
    to: f64,
    start: u64,
    len: u64,
}

/// One World with its schedule, buffers, telemetry and digest.
pub struct Instance {
    kind: &'static KindDef,
    anatomy: String,
    options: Value,
    source: Box<dyn Source>,
    sample_rate: f64,
    clock: u64,
    /// (due frame, intended frame, what), in due order.
    pending: Vec<(u64, u64, Pending)>,
    ramps: Vec<Ramp>,
    controls: Vec<f64>,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
    pub telemetry: Vec<f64>,
    digest: Sha256,
    applied: u64,
    refused: u64,
    errors: u64,
    last_error: String,
    last_ledger: u64,
    scores: Vec<Value>,
}

impl Instance {
    /// Build a World of `kind` (an id in the catalog).
    pub fn create(kind: &str, sample_rate: f64, anatomy: Option<&str>, options: &Value) -> Result<Self, String> {
        let def = kinds::find(kind).ok_or_else(|| {
            format!("no World kind {kind:?}; the catalog has {}", kinds::all().iter().map(|k| k.id.as_str()).collect::<Vec<_>>().join(", "))
        })?;
        if !(sample_rate.is_finite() && sample_rate > 0.0) {
            return Err(format!("sample rate {sample_rate}"));
        }
        if let Some(sr) = def.sample_rate {
            if (sr - sample_rate).abs() > 1e-6 {
                return Err(format!(
                    "{} runs at {sr} Hz (its plan's rate) and this context runs at {sample_rate} Hz: make the AudioContext at {sr} Hz",
                    def.id
                ));
            }
        }
        if let Some(a) = anatomy {
            if !def.anatomies.is_empty() && !def.anatomies.iter().any(|x| x.id == a) && !def.open_anatomies {
                return Err(format!(
                    "{} has no anatomy {a:?}; it has {}",
                    def.id,
                    def.anatomies.iter().map(|x| x.id.as_str()).collect::<Vec<_>>().join(", ")
                ));
            }
        }
        let anatomy_id = anatomy
            .map(str::to_owned)
            .or_else(|| def.anatomies.iter().find(|a| a.default).map(|a| a.id.clone()))
            .unwrap_or_default();
        let ctx = CreateCtx { sample_rate, anatomy: (!anatomy_id.is_empty()).then_some(anatomy_id.as_str()), options };
        let source = (def.build)(def, &ctx)?;
        let controls = def.controls.iter().map(|c| c.default).collect();
        let kind_slots = def.telemetry.len();
        Ok(Self {
            kind: def,
            anatomy: anatomy_id,
            options: options.clone(),
            source,
            sample_rate,
            clock: 0,
            pending: Vec::with_capacity(256),
            ramps: Vec::with_capacity(16),
            controls,
            left: vec![0.0; MAX_FRAMES],
            right: vec![0.0; MAX_FRAMES],
            telemetry: vec![0.0; slot::KIND + kind_slots],
            digest: Sha256::new(),
            applied: 0,
            refused: 0,
            errors: 0,
            last_error: String::new(),
            last_ledger: 0,
            scores: Vec::new(),
        })
    }

    pub fn kind(&self) -> &'static KindDef {
        self.kind
    }

    /// The World's clock: frames rendered.
    pub fn frame(&self) -> u64 {
        self.clock
    }

    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    pub fn last_error(&self) -> &str {
        &self.last_error
    }

    fn queue(&mut self, due: u64, at: u64, p: Pending) {
        let i = self.pending.partition_point(|x| x.0 <= due);
        self.pending.insert(i, (due, at, p));
    }

    /// Schedule event `index` at frame `at` (a frame already rendered means
    /// the next one). Unknown events are refused here; a kind's own refusal
    /// (a string it does not have) is counted when the event is due.
    pub fn event(&mut self, index: usize, at: f64, args: &[f64]) -> Result<(), String> {
        let Some(def) = self.kind.events.get(index) else {
            return Err(format!("{} has no event {index}", self.kind.id));
        };
        let mut a = [0.0; MAX_ARGS];
        for (k, arg) in def.args.iter().enumerate().take(MAX_ARGS) {
            a[k] = args.get(k).copied().filter(|v| v.is_finite()).unwrap_or(arg.default);
        }
        let at = frame_of(at).max(self.clock);
        let due = at.saturating_sub(self.source.lookahead()).max(self.clock);
        self.queue(due, at, Pending::Event { index, args: a });
        Ok(())
    }

    /// Set control `index` to `value` at frame `at`, moving there linearly
    /// over `ramp` frames (0: at once).
    pub fn set(&mut self, index: usize, value: f64, at: f64, ramp: f64) -> Result<(), String> {
        let Some(def) = self.kind.controls.get(index) else {
            return Err(format!("{} has no control {index}", self.kind.id));
        };
        if !value.is_finite() {
            return Err(format!("{}: value {value}", def.name));
        }
        let value = value.clamp(def.min, def.max);
        let at = frame_of(at).max(self.clock);
        let ramp = if def.steps { 0 } else { frame_of(ramp) };
        self.queue(at, at, Pending::Control { index, value, ramp });
        Ok(())
    }

    /// The value control `index` was last set to (or is ramping through).
    pub fn control(&self, index: usize) -> f64 {
        self.controls.get(index).copied().unwrap_or(f64::NAN)
    }

    /// Hand a score to the World from frame `at`.
    pub fn play_score(&mut self, text: &str, at: f64) -> Result<Value, String> {
        let at = frame_of(at).max(self.clock);
        let read = self.source.play_score(text, at)?;
        self.scores.push(read.clone());
        Ok(read)
    }

    fn dispatch(&mut self) {
        while let Some(&(due, at, p)) = self.pending.first() {
            if due > self.clock {
                break;
            }
            self.pending.remove(0);
            let r = match p {
                Pending::Event { index, args } => self.source.event(index, &args, at.max(self.clock), self.clock),
                Pending::Control { index, value, ramp } => {
                    self.ramps.retain(|r| r.index != index);
                    if ramp == 0 {
                        self.controls[index] = value;
                        self.source.set(index, value)
                    } else {
                        let from = self.controls[index];
                        self.ramps.push(Ramp { index, from, to: value, start: self.clock, len: ramp });
                        Ok(())
                    }
                }
            };
            match r {
                Ok(()) => self.applied += 1,
                Err(e) => {
                    self.refused += 1;
                    self.last_error = e;
                }
            }
        }
    }

    /// Set every ramped control to its value for the step `clock` is in.
    /// Steps are counted from the ramp's own start, so where a render call
    /// splits cannot change a value (block segmentation changes nothing).
    fn step_ramps(&mut self) {
        let clock = self.clock;
        let mut finished = false;
        for r in &self.ramps {
            let reached = ((clock - r.start) / RAMP_STEP + 1) * RAMP_STEP;
            let x = (reached as f64 / r.len as f64).min(1.0);
            let v = r.from + (r.to - r.from) * x;
            self.controls[r.index] = v;
            if let Err(e) = self.source.set(r.index, v) {
                self.refused += 1;
                self.last_error = e;
            }
            finished |= reached >= r.len;
        }
        if finished {
            self.ramps.retain(|r| ((clock - r.start) / RAMP_STEP + 1) * RAMP_STEP < r.len);
        }
    }

    /// The next frame at which a ramp takes its next step.
    fn next_ramp_step(&self) -> Option<u64> {
        self.ramps.iter().map(|r| r.start + ((self.clock - r.start) / RAMP_STEP + 1) * RAMP_STEP).min()
    }

    /// Render `frames` (at most [`MAX_FRAMES`]) into `left`/`right`; returns how many.
    pub fn render(&mut self, frames: usize) -> usize {
        let n = frames.min(MAX_FRAMES);
        let mut done = 0;
        while done < n {
            self.dispatch();
            if !self.ramps.is_empty() {
                self.step_ramps();
            }
            let mut end = self.clock + (n - done) as u64;
            if let Some(&(due, _, _)) = self.pending.first() {
                end = end.min(due.max(self.clock + 1));
            }
            if let Some(step) = self.next_ramp_step() {
                end = end.min(step);
            }
            let len = (end - self.clock) as usize;
            let (l, r) = (&mut self.left[done..done + len], &mut self.right[done..done + len]);
            if let Err(e) = self.source.render(l, r) {
                // A World error is a fault, not a sound: silence, counted, the World rebuilt at rest.
                l.fill(0.0);
                r.fill(0.0);
                self.errors += 1;
                self.last_error = format!("World error at frame {}: {e}", self.clock);
                let _ = self.source.reset();
            }
            self.clock += len as u64;
            done += len;
        }
        let (mut peak, mut sq) = (0.0f32, 0.0f64);
        for i in 0..n {
            let (l, r) = (self.left[i], self.right[i]);
            self.digest.update(l.to_le_bytes());
            self.digest.update(r.to_le_bytes());
            peak = peak.max(l.abs()).max(r.abs());
            sq += 0.5 * (f64::from(l) * f64::from(l) + f64::from(r) * f64::from(r));
        }
        let t = &mut self.telemetry;
        t[slot::FRAME] = self.clock as f64;
        t[slot::SECONDS] = self.clock as f64 / self.sample_rate;
        t[slot::PEAK] = f64::from(peak);
        t[slot::RMS] = if n > 0 { (sq / n as f64).sqrt() } else { 0.0 };
        t[slot::ERRORS] = self.errors as f64;
        t[slot::CLAMPED] = self.source.clamped() as f64;
        t[slot::EVENTS_APPLIED] = self.applied as f64;
        t[slot::EVENTS_REFUSED] = self.refused as f64;
        if self.clock >= self.last_ledger + LEDGER_EVERY || self.last_ledger == 0 {
            self.read_ledger();
        }
        let kind_slots = &mut self.telemetry[slot::KIND..];
        self.source.telemetry(kind_slots);
        n
    }

    fn read_ledger(&mut self) {
        self.last_ledger = self.clock.max(1);
        let l = self.source.ledger();
        let t = &mut self.telemetry;
        t[slot::SOURCE_WORK] = l.source_work;
        t[slot::CONTROL_WORK] = l.control_work;
        t[slot::INTERNAL_LOSS] = l.internal_loss;
        t[slot::RADIATION] = l.radiation;
        t[slot::STORED_ENERGY] = l.current_energy;
        t[slot::INITIAL_ENERGY] = l.initial_energy;
        t[slot::CUMULATIVE_RESIDUAL] = l.cumulative_closure_residual;
        t[slot::MAX_FRAME_RESIDUAL] = l.maximum_frame_residual;
    }

    /// Refresh every telemetry slot now (the ledger too).
    pub fn refresh(&mut self) {
        self.read_ledger();
        let kind_slots = &mut self.telemetry[slot::KIND..];
        self.source.telemetry(kind_slots);
    }

    /// The World at rest again: clock, schedule, controls, digest and books at zero.
    pub fn reset(&mut self) -> Result<(), String> {
        self.source.reset()?;
        self.clock = 0;
        self.pending.clear();
        self.ramps.clear();
        self.controls = self.kind.controls.iter().map(|c| c.default).collect();
        self.digest = Sha256::new();
        self.applied = 0;
        self.refused = 0;
        self.errors = 0;
        self.last_error.clear();
        self.last_ledger = 0;
        self.scores.clear();
        self.telemetry.fill(0.0);
        Ok(())
    }

    /// The receipt: the World's own books and receipt, what was asked of it,
    /// and the digest of every sample it rendered.
    pub fn receipt(&self) -> Value {
        let l = self.source.ledger();
        // Roundoff scales with the energy the World moves: 1e-12 of its
        // throughput, never tighter than 1e-9 J (the frame bound is 1e-15 of
        // it, never tighter than 1e-12 J). An Ariadne guitar moves 1e-8 J; a
        // kart engine's servo and flame move 1e5 J.
        let throughput = l.source_work.abs() + l.control_work.abs() + l.initial_energy.abs();
        let tolerance = (1e-12 * throughput).max(1e-9);
        let frame_tolerance = (1e-15 * throughput).max(1e-12);
        json!({
            "schema": "ovillo.portable.receipt.v1",
            "producer": format!("ovillo-portable {}", env!("CARGO_PKG_VERSION")),
            "kind": self.kind.id,
            "family": self.kind.family,
            "anatomy": self.anatomy,
            "options": redact(&self.options),
            "sampleRate": self.sample_rate,
            "frames": self.clock,
            "seconds": self.clock as f64 / self.sample_rate,
            "planDigest": self.source.plan_digest(),
            "outputDigest": format!("sha256:{}", hex(&self.digest.clone().finalize())),
            "outputDigestCovers": "every frame rendered, interleaved left/right, f32 little-endian",
            "output": &self.kind.output,
            "events": {"applied": self.applied, "refused": self.refused, "lastError": self.last_error, "errors": self.errors},
            "clamped": self.source.clamped(),
            "scores": self.scores,
            "ledger": {
                "sourceWork": l.source_work,
                "controlWork": l.control_work,
                "internalLoss": l.internal_loss,
                "radiation": l.radiation,
                "initialEnergy": l.initial_energy,
                "currentEnergy": l.current_energy,
                "cumulativeClosureResidual": l.cumulative_closure_residual,
                "maximumFrameResidual": l.maximum_frame_residual,
                "globalClosureResidual": l.global_closure_residual,
                "sampleFrames": l.sample_frames,
                "throughput": throughput,
                "closureTolerance": tolerance,
                "frameTolerance": frame_tolerance,
                "closes": l.cumulative_closure_residual.abs() <= tolerance && l.maximum_frame_residual.abs() <= frame_tolerance,
            },
            "world": self.source.receipt(),
            "doesNotModel": {"see": self.kind.does_not_model_ref, "items": self.kind.does_not_model},
            "claimBoundary": "A successful render is not by itself a theorem about transport, a biological reconstruction, intelligible speech, or listener perception.",
        })
    }

    /// What `create` built, for the page.
    pub fn info(&self) -> Value {
        json!({
            "kind": self.kind.id,
            "anatomy": self.anatomy,
            "sampleRate": self.sample_rate,
            "planDigest": self.source.plan_digest(),
            "channels": self.kind.channels,
            "lookaheadFrames": self.source.lookahead(),
            "telemetry": slot::NAMES.iter().map(|s| (*s).to_owned()).chain(self.kind.telemetry.iter().map(|s| s.name.clone())).collect::<Vec<_>>(),
        })
    }
}

fn frame_of(v: f64) -> u64 {
    if v.is_finite() && v > 0.0 {
        v.round().min(u64::MAX as f64 / 2.0) as u64
    } else {
        0
    }
}

/// A score is in the receipt by its digest, not its text.
fn redact(options: &Value) -> Value {
    let mut o = options.clone();
    if let Some(map) = o.as_object_mut() {
        for key in ["score", "plan"] {
            if let Some(Value::String(text)) = map.get(key) {
                let d = Sha256::digest(text.as_bytes());
                map.insert(key.into(), json!({"sha256": hex(&d), "bytes": text.len()}));
            }
        }
    }
    o
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Render a whole World offline: build it, hand it a score (if any), run
/// `frames` frames in blocks, and return its audio and receipt. The same path
/// a worklet runs, a block at a time.
pub fn render_offline(
    kind: &str,
    sample_rate: f64,
    anatomy: Option<&str>,
    options: &Value,
    frames: Option<usize>,
    mut schedule: impl FnMut(&mut Instance) -> Result<(), String>,
) -> Result<(Vec<f32>, Vec<f32>, Value), String> {
    let mut inst = Instance::create(kind, sample_rate, anatomy, options)?;
    let mut length = frames;
    if let Some(Value::String(score)) = options.get("score") {
        let read = inst.play_score(score, 0.0)?;
        if length.is_none() {
            let tail = options.get("tailSeconds").and_then(Value::as_f64).unwrap_or(2.0);
            length = read.get("samples").and_then(Value::as_u64).map(|s| s as usize + (tail * sample_rate) as usize);
        }
    }
    schedule(&mut inst)?;
    let n = length.ok_or("give frames (or seconds) for a render without a score")?;
    let (mut left, mut right) = (Vec::with_capacity(n), Vec::with_capacity(n));
    while left.len() < n {
        let k = inst.render((n - left.len()).min(MAX_FRAMES));
        left.extend_from_slice(&inst.left[..k]);
        right.extend_from_slice(&inst.right[..k]);
    }
    Ok((left, right, inst.receipt()))
}
