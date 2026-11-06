//! The streaming core: one World, her voice in, the listener's pressure out,
//! and the books.

use std::time::Instant;

use mus_ariadne::{
    BodyControls, ConstitutiveBodyControls, CouplingControls, FlowValveControls, SharedSpaceControls, SpaceBodyControls,
    SpaceTotals, World, WorldControls, MODE_COUNT,
};
use mus_ariadne::law::active::ActiveInclusionControls;
use mus_ariadne::law::transport::ResonantTransportControls;
use serde::Serialize;

use crate::build::{self, Layout, AIR_DENSITY_KG_M3, SPEED_OF_SOUND_M_S};
use crate::surface::BodySpec;

/// One source the listener hears: a delay line and, per ear, a gain and a delay.
struct Emitter {
    ring: Vec<f64>,
    pos: usize,
    delay: [f64; 2],
    gain: [f64; 2],
}

impl Emitter {
    fn new(x_m: f64, ears: &[[f64; 2]; 2], rate: f64, gain_at_1m: impl Fn(f64) -> f64) -> Self {
        let mut delay = [0.0; 2];
        let mut gain = [0.0; 2];
        for e in 0..2 {
            let dx = ears[e][0] - x_m;
            let dy = ears[e][1];
            let r = (dx * dx + dy * dy).sqrt().max(1.0e-3);
            delay[e] = r / SPEED_OF_SOUND_M_S * rate;
            gain[e] = gain_at_1m(r);
        }
        let len = (delay[0].max(delay[1]).ceil() as usize) + 4;
        Self { ring: vec![0.0; len], pos: 0, delay, gain }
    }

    #[inline]
    fn push(&mut self, v: f64) {
        self.ring[self.pos] = v;
        self.pos = (self.pos + 1) % self.ring.len();
    }

    /// The value `delay` samples ago (after `push`), linearly interpolated.
    #[inline]
    fn read(&self, ear: usize) -> f64 {
        let d = self.delay[ear];
        let i = d.floor();
        let frac = d - i;
        let len = self.ring.len();
        let idx = |back: usize| self.ring[(self.pos + len - 1 - back % len) % len];
        let a = idx(i as usize);
        let b = idx(i as usize + 1);
        a + (b - a) * frac
    }
}

/// Running output statistics.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct LevelStats {
    pub peak: f64,
    pub rms: f64,
    #[serde(skip)]
    sum_sq: f64,
    #[serde(skip)]
    count: u64,
}

impl LevelStats {
    #[inline]
    fn add(&mut self, v: f64) {
        self.peak = self.peak.max(v.abs());
        self.sum_sq += v * v;
        self.count += 1;
    }
    fn finish(&mut self) {
        self.rms = if self.count > 0 { (self.sum_sq / self.count as f64).sqrt() } else { 0.0 };
    }
    pub fn peak_dbfs(&self) -> f64 {
        20.0 * self.peak.max(1.0e-12).log10()
    }
    pub fn rms_dbfs(&self) -> f64 {
        20.0 * self.rms.max(1.0e-12).log10()
    }
}

/// The ledger and the readouts after a run.
#[derive(Debug, Clone, Serialize)]
pub struct Receipt {
    pub schema: &'static str,
    pub body: String,
    pub plan_digest: String,
    pub sample_rate_hz: f64,
    pub frames: u64,
    pub seconds: f64,
    pub channels: usize,
    pub input_peak_dbfs: f64,
    pub input_rms_dbfs: f64,
    pub output_peak_dbfs: f64,
    pub output_rms_dbfs: f64,
    pub clipped_samples: u64,
    pub faults: u64,
    /// Work her voice did on the air (J): the `node_source`'s exact `pbar U dt`, summed.
    pub voice_work_j: f64,
    /// Share of samples in which the air pushed back harder than she pushed (negative instantaneous work).
    pub voice_pushed_back_fraction: f64,
    /// The powered material's reservoir refill (J), the only other source work.
    pub supply_work_j: f64,
    pub ledger: Ledger,
    pub air: AirBooks,
    pub bodies: Vec<BodyBooks>,
    pub bandwidth_hz: f64,
    pub modes: Vec<Mode>,
    pub layout: Layout,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Ledger {
    pub source_work_j: f64,
    pub control_work_j: f64,
    pub internal_loss_j: f64,
    pub radiation_j: f64,
    pub stored_j: f64,
    pub initial_energy_j: f64,
    pub current_energy_j: f64,
    pub global_closure_residual_j: f64,
    pub cumulative_closure_residual_j: f64,
    pub maximum_frame_residual_j: f64,
    pub maximum_local_residual_j: f64,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct AirBooks {
    pub wall_loss_j: f64,
    pub radiation_by_end_j: [f64; 4],
    pub physical_radiation_by_end_j: [f64; 4],
    pub jet_loss_j: f64,
    pub turbulence_work_j: f64,
    pub turbulence_injected_j: f64,
    pub flap_loss_j: f64,
    pub flap_leak_loss_j: f64,
    pub flap_reservoir_draw_j: f64,
    pub field_energy_j: f64,
    pub max_reynolds: f64,
    pub max_jet_speed_m_s: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BodyBooks {
    pub id: String,
    pub source_work_j: f64,
    pub internal_loss_j: f64,
    pub stored_j: f64,
    pub reservoir_j: f64,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Mode {
    pub hz: f64,
    pub wall_t60_s: f64,
    pub radiation_sigma_per_s: [f64; 4],
    pub valve_shape: f64,
    pub mouth_shape: f64,
}

/// A Body: build it once, stream her voice through it.
pub struct Body {
    spec: BodySpec,
    rate: f64,
    world: World,
    layout: Layout,
    controls: Vec<ConstitutiveBodyControls>,
    world_controls: WorldControls,
    drive_gain: f64,
    /// Leaky integrator from mic pressure to flow: `y = y (1 - wc dt) + x wref dt`.
    integrate: bool,
    int_leak: f64,
    int_step: f64,
    int_state: f64,
    pa_full_scale: f64,
    dry: f64,
    room_level: f64,
    channels: usize,
    ears: [[f64; 2]; 2],
    end_emitters: Vec<Emitter>,
    radiator_emitters: Vec<Emitter>,
    dry_emitter: Emitter,
    frames: u64,
    input: LevelStats,
    output: LevelStats,
    clipped: u64,
    faults: u64,
    voice_work: f64,
    pushed_back: u64,
    supply_work: f64,
    body_source: Vec<f64>,
    body_loss: Vec<f64>,
    max_reynolds: f64,
    max_jet_speed: f64,
    render_seconds: f64,
    notes: Vec<String>,
}

impl Body {
    /// Build the World for `spec` at `rate` Hz.
    pub fn new(spec: &BodySpec, rate: f64) -> Result<Self, String> {
        let mut notes = spec.validate()?;
        let mouth = spec.get("mouth.open");
        let (plan, mut layout) = build::plan(spec, rate, mouth)?;
        let mut world = World::from_constitutive_plan(plan).map_err(|e| build::describe_error(&e.to_string()))?;
        notes.append(&mut layout.notes);
        // The air's end columns, from its current areas.
        let air = world.space_body(0).ok_or("no air body")?;
        let areas = air.network_areas_m2().ok_or("no areas")?.to_vec();
        for end in &mut layout.ends {
            let a = areas[end.node];
            let r = (a / std::f64::consts::PI).sqrt();
            end.column_m = 0.5 * end.dx_m + end.end_correction_ratio * r;
        }
        // The listener's two ears, in the plane of the openings: the mouth
        // at the origin facing +y; the listener at distance d, azimuth az.
        let d = spec.get("listener.distance_m");
        let az = spec.get("listener.azimuth_deg").to_radians();
        let s = spec.get("listener.ear_spacing_m");
        let centre = [d * az.sin(), d * az.cos()];
        let across = [az.cos(), -az.sin()];
        let ears = [
            [centre[0] - 0.5 * s * across[0], centre[1] - 0.5 * s * across[1]],
            [centre[0] + 0.5 * s * across[0], centre[1] + 0.5 * s * across[1]],
        ];
        let channels = if s > 0.0 { 2 } else { 1 };
        let power_to_pa = (AIR_DENSITY_KG_M3 * SPEED_OF_SOUND_M_S / (4.0 * std::f64::consts::PI)).sqrt();
        let end_emitters: Vec<Emitter> = layout
            .ends
            .iter()
            .map(|end| {
                let a = areas[end.node];
                let l = end.column_m;
                Emitter::new(end.x_m, &ears, rate, |r| a / (4.0 * std::f64::consts::PI * r * l))
            })
            .collect();
        let radiator_emitters: Vec<Emitter> = layout
            .radiators
            .iter()
            .map(|rad| {
                let level = rad.level;
                Emitter::new(rad.x_m, &ears, rate, |r| level * power_to_pa / r)
            })
            .collect();
        let dry_emitter = Emitter::new(0.0, &ears, rate, |_| 1.0);
        // Controls: the air (her voice as node_source, no supply, no refill),
        // the resonators (nothing), the powered material (its supply and inclusion).
        let mut controls = Vec::with_capacity(world.body_count());
        controls.push(ConstitutiveBodyControls::ModalSpace(SpaceBodyControls {
            body: BodyControls { work_input: 0.0, ..Default::default() },
            valve: FlowValveControls { supply: 0.0 },
            radiation_scale: 1.0,
            node_source: None,
        }));
        for rad in &layout.radiators {
            if rad.powered {
                controls.push(ConstitutiveBodyControls::Modal(BodyControls {
                    source_wave: 0.0,
                    work_input: spec.get("powered.supply_w"),
                    active_coupling: spec.get("powered.coupling"),
                    active: ActiveInclusionControls {
                        drive: spec.get("powered.drive"),
                        headroom: spec.get("powered.headroom"),
                        sustain: spec.get("powered.sustain"),
                        sag: spec.get("powered.sag"),
                        bloom: spec.get("powered.bloom"),
                        feedback: spec.get("powered.feedback"),
                        space: 0.0,
                    },
                    transport: ResonantTransportControls { interlace: spec.get("powered.interlace"), memory: spec.get("powered.memory") },
                    radiation_fraction: spec.get("powered.radiation"),
                }));
                world
                    .set_body_configuration_index(rad.body, spec.get("powered.organization"))
                    .map_err(|e| e.to_string())?;
            } else {
                controls.push(ConstitutiveBodyControls::ModalSpace(SpaceBodyControls {
                    body: BodyControls { work_input: 0.0, ..Default::default() },
                    valve: FlowValveControls { supply: 0.0 },
                    radiation_scale: 1.0,
                    node_source: None,
                }));
            }
        }
        let world_controls = WorldControls {
            space: SharedSpaceControls {
                coupling: spec.get("room.coupling"),
                decay: spec.get("room.decay"),
                radiation: spec.get("room.radiation"),
            },
            direct_coupling: CouplingControls { scale: 1.0 },
        };
        let n = world.body_count();
        Ok(Self {
            spec: spec.clone(),
            rate,
            world,
            layout,
            controls,
            world_controls,
            drive_gain: spec.get("drive.gain_m3s"),
            integrate: spec.on("drive.integrate"),
            int_leak: 1.0 - std::f64::consts::TAU * spec.get("drive.corner_hz") / rate,
            int_step: std::f64::consts::TAU * 200.0 / rate,
            int_state: 0.0,
            pa_full_scale: spec.get("listener.pa_full_scale"),
            dry: spec.get("listener.dry"),
            room_level: spec.get("room.level") * power_to_pa,
            channels,
            ears,
            end_emitters,
            radiator_emitters,
            dry_emitter,
            frames: 0,
            input: LevelStats::default(),
            output: LevelStats::default(),
            clipped: 0,
            faults: 0,
            voice_work: 0.0,
            pushed_back: 0,
            supply_work: 0.0,
            body_source: vec![0.0; n],
            body_loss: vec![0.0; n],
            max_reynolds: 0.0,
            max_jet_speed: 0.0,
            render_seconds: 0.0,
            notes,
        })
    }

    #[must_use]
    pub fn spec(&self) -> &BodySpec {
        &self.spec
    }

    #[must_use]
    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    #[must_use]
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Output channels (2 when the ears are apart, else 1); `process` writes
    /// them interleaved.
    #[must_use]
    pub fn channels(&self) -> usize {
        self.channels
    }

    #[must_use]
    pub fn sample_rate_hz(&self) -> f64 {
        self.rate
    }

    /// The ears' positions (m) in the plane of the openings.
    #[must_use]
    pub fn ears(&self) -> [[f64; 2]; 2] {
        self.ears
    }

    /// Move the live coordinate (the mouth's opening) now: one receipted World move.
    pub fn set_mouth(&mut self, open: f64) -> Result<(), String> {
        let open = open.clamp(0.02, 1.0);
        self.world.set_body_configuration_point(0, &[open]).map_err(|e| e.to_string())?;
        self.spec.knobs.insert("mouth.open".into(), open);
        Ok(())
    }

    /// One sample in, one frame (1 or 2 samples) out.
    #[inline]
    pub fn tick(&mut self, x: f32) -> [f64; 2] {
        let x = if x.is_finite() { f64::from(x) } else { 0.0 };
        self.input.add(x);
        // A mic hears pressure; a mouth's far field is rho dU/dt / 4 pi r: the
        // flow is the recording's leaky integral (unity at 200 Hz).
        let flow_coordinate = if self.integrate {
            self.int_state = self.int_state * self.int_leak + x * self.int_step;
            self.int_state
        } else {
            x
        };
        let flow = flow_coordinate * self.drive_gain;
        if let ConstitutiveBodyControls::ModalSpace(c) = &mut self.controls[0] {
            c.node_source = (flow != 0.0).then_some((self.layout.drive_node, flow));
        }
        let frame = match self.world.process_constitutive_sample(&self.controls, self.world_controls) {
            Ok(frame) => frame,
            Err(_) => {
                // A fault is not a sound: the World is rebuilt silent and counted.
                self.faults += 1;
                let _ = self.world.reset();
                return [0.0; 2];
            }
        };
        let voice = self.world.body_last_source_work(0).unwrap_or(0.0);
        self.voice_work += voice;
        if voice < 0.0 {
            self.pushed_back += 1;
        }
        for i in 0..self.body_source.len() {
            let s = self.world.body_last_source_work(i).unwrap_or(0.0);
            self.body_source[i] += s;
            if i > 0 {
                self.supply_work += s;
            }
            self.body_loss[i] += self.world.body_last_internal_loss(i).unwrap_or(0.0);
        }
        // The air's ends: the radiating volumes' pressures (Pa).
        let ends = self.world.body_radiation(0).unwrap_or(&[]);
        for (e, emitter) in self.end_emitters.iter_mut().enumerate() {
            emitter.push(ends.get(e).copied().unwrap_or(0.0));
        }
        // Resonators: the sum of their modes' radiated coordinates (sqrt W);
        // the powered material: its one emitted coordinate.
        for (k, rad) in self.layout.radiators.iter().enumerate() {
            let emitted = self.world.body_radiation(rad.body).unwrap_or(&[]);
            let v: f64 = if rad.powered { emitted.first().copied().unwrap_or(0.0) } else { emitted.iter().sum() };
            self.radiator_emitters[k].push(v);
        }
        self.dry_emitter.push(x);
        if self.layout.air_on {
            if let Some(obs) = self.world.body_space_observation(0) {
                self.max_reynolds = self.max_reynolds.max(obs.aero.reynolds);
                self.max_jet_speed = self.max_jet_speed.max(obs.aero.jet_speed_m_s.abs());
            }
        }
        let mut out = [0.0; 2];
        let room = [frame.space_emitted_left, frame.space_emitted_right];
        for ear in 0..self.channels {
            let mut p = 0.0;
            for em in &self.end_emitters {
                p += em.gain[ear] * em.read(ear);
            }
            for em in &self.radiator_emitters {
                p += em.gain[ear] * em.read(ear);
            }
            p += self.room_level * room[ear];
            let mut v = p / self.pa_full_scale + self.dry * self.dry_emitter.read(ear);
            if !v.is_finite() {
                v = 0.0;
            }
            if v.abs() > 1.0 {
                self.clipped += 1;
                v = v.clamp(-1.0, 1.0);
            }
            self.output.add(v);
            out[ear] = v;
        }
        self.frames += 1;
        out
    }

    /// Stream: `output.len()` must be `input.len() * channels()` (interleaved).
    pub fn process(&mut self, input: &[f32], output: &mut [f32]) {
        let t0 = Instant::now();
        let ch = self.channels;
        assert!(output.len() >= input.len() * ch, "output buffer too small");
        for (i, &x) in input.iter().enumerate() {
            let y = self.tick(x);
            for c in 0..ch {
                output[i * ch + c] = y[c] as f32;
            }
        }
        self.render_seconds += t0.elapsed().as_secs_f64();
    }

    /// Wall-clock seconds spent in `process` so far.
    #[must_use]
    pub fn render_seconds(&self) -> f64 {
        self.render_seconds
    }

    #[must_use]
    pub fn air_totals(&self) -> SpaceTotals {
        self.world.space_body(0).and_then(|b| b.totals()).unwrap_or_default()
    }

    /// The air's current modes.
    #[must_use]
    pub fn modes(&self) -> Vec<Mode> {
        let Some(air) = self.world.space_body(0) else { return Vec::new() };
        let Some(m) = air.network_modes() else { return Vec::new() };
        let mouth = air.node_shape(self.layout.mouth_node).unwrap_or([0.0; MODE_COUNT]);
        (0..MODE_COUNT)
            .map(|k| {
                let mut sig = [0.0; 4];
                for e in 0..m.ends.min(4) {
                    sig[e] = m.radiation_sigma[e][k];
                }
                Mode {
                    hz: m.frequency_hz[k],
                    wall_t60_s: if m.wall_sigma[k] > 0.0 { 6.907_755_278_982_137 / m.wall_sigma[k] } else { f64::INFINITY },
                    radiation_sigma_per_s: sig,
                    valve_shape: m.valve_shape[k],
                    mouth_shape: mouth[k],
                }
            })
            .collect()
    }

    /// The books.
    #[must_use]
    pub fn receipt(&self) -> Receipt {
        let r = self.world.receipt();
        let mut input = self.input;
        input.finish();
        let mut output = self.output;
        output.finish();
        let t = self.air_totals();
        let air_body = self.world.space_body(0);
        let modes = self.modes();
        let bodies = (0..self.world.body_count())
            .map(|i| BodyBooks {
                id: self.world.body_id(i).unwrap_or("?").to_owned(),
                source_work_j: self.body_source[i],
                internal_loss_j: self.body_loss[i],
                stored_j: self.world.body_material_energy(i).unwrap_or(0.0),
                reservoir_j: self.world.body_reservoir_energy(i).unwrap_or(0.0),
            })
            .collect();
        Receipt {
            schema: "ovillo-body.receipt.v1",
            body: self.spec.name.clone(),
            plan_digest: r.plan_digest.clone(),
            sample_rate_hz: self.rate,
            frames: self.frames,
            seconds: self.frames as f64 / self.rate,
            channels: self.channels,
            input_peak_dbfs: input.peak_dbfs(),
            input_rms_dbfs: input.rms_dbfs(),
            output_peak_dbfs: output.peak_dbfs(),
            output_rms_dbfs: output.rms_dbfs(),
            clipped_samples: self.clipped,
            faults: self.faults,
            voice_work_j: self.voice_work,
            voice_pushed_back_fraction: if self.frames > 0 { self.pushed_back as f64 / self.frames as f64 } else { 0.0 },
            supply_work_j: self.supply_work,
            ledger: Ledger {
                source_work_j: r.source_work,
                control_work_j: r.control_work,
                internal_loss_j: r.internal_loss,
                radiation_j: r.radiation,
                stored_j: r.current_energy - r.initial_energy,
                initial_energy_j: r.initial_energy,
                current_energy_j: r.current_energy,
                global_closure_residual_j: r.global_closure_residual,
                cumulative_closure_residual_j: r.cumulative_closure_residual,
                maximum_frame_residual_j: r.maximum_frame_residual,
                maximum_local_residual_j: r.maximum_local_residual,
            },
            air: AirBooks {
                wall_loss_j: t.wall_loss,
                radiation_by_end_j: t.radiation_by_end,
                physical_radiation_by_end_j: t.physical_radiation_by_end,
                jet_loss_j: t.aero.jet_loss,
                turbulence_work_j: t.aero.turbulence_work,
                turbulence_injected_j: t.aero.turbulence_injected,
                flap_loss_j: t.fold_loss,
                flap_leak_loss_j: t.valve_jet_loss,
                flap_reservoir_draw_j: t.reservoir_draw,
                field_energy_j: air_body.map_or(0.0, |b| b.field_energy()),
                max_reynolds: self.max_reynolds,
                max_jet_speed_m_s: self.max_jet_speed,
            },
            bodies,
            bandwidth_hz: modes.last().map_or(0.0, |m| m.hz),
            modes,
            layout: self.layout.clone(),
            notes: self.notes.clone(),
        }
    }
}

/// The Body's transfer, measured: a deterministic white-noise burst at
/// `level` full scale through a fresh World, its long-term spectrum against
/// the burst's own, in third-octave bands and at 1/12-octave resolution.
/// Small-signal unless `level` is high; with air.on or flap.on the answer
/// depends on the level (say so when quoting it).
#[derive(Debug, Clone, Serialize)]
pub struct Transfer {
    pub noise_rms_full_scale: f64,
    pub seconds: f64,
    /// 1/12-octave centres (Hz) and gain (dB, mic to output).
    pub frequencies_hz: Vec<f64>,
    pub gain_db: Vec<f64>,
    /// Third-octave band centres (Hz) and gain (dB).
    pub band_hz: Vec<f64>,
    pub band_db: Vec<f64>,
    pub mean_db_150_8000: f64,
    pub ripple_db_150_8000: f64,
    pub peaks_hz: Vec<(f64, f64)>,
    pub notches_hz: Vec<(f64, f64)>,
}

/// Deterministic Gaussian-ish white noise (sum of four xorshift uniforms).
pub fn noise(n: usize, rms: f64, seed: u64) -> Vec<f32> {
    let mut state = seed | 1;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state >> 11) as f64 / (1u64 << 53) as f64 - 0.5
    };
    (0..n).map(|_| ((next() + next() + next() + next()) * rms * 1.7320508) as f32).collect()
}

impl Body {
    /// Measure the transfer with a fresh World (this one is not touched).
    pub fn transfer(spec: &BodySpec, rate: f64, level: f64, seconds: f64) -> Result<Transfer, String> {
        let mut body = Body::new(spec, rate)?;
        let n = (seconds * rate) as usize;
        let x = noise(n, level, 0x2545_F491_4F6C_DD1D);
        let mut y = Vec::with_capacity(n);
        for &v in &x {
            let out = body.tick(v);
            y.push(if body.channels == 2 { 0.5 * (out[0] + out[1]) } else { out[0] } as f32);
        }
        let (dry, _) = power_spectrum(&x, 4096, false);
        let (wet, _) = power_spectrum(&y, 4096, false);
        let bands = |per_octave: f64, lo: f64, hi: f64| -> (Vec<f64>, Vec<f64>) {
            let mut centres = Vec::new();
            let mut db = Vec::new();
            let mut c = lo;
            while c < hi {
                let (a, b) = (c / 2f64.powf(0.5 / per_octave), c * 2f64.powf(0.5 / per_octave));
                let (mut pw, mut pd) = (0.0, 0.0);
                for k in 1..dry.len() {
                    let f = k as f64 * rate / 4096.0;
                    if (a..b).contains(&f) {
                        pw += wet[k];
                        pd += dry[k];
                    }
                }
                if pd > 0.0 {
                    centres.push(c);
                    db.push(10.0 * (pw / pd).max(1.0e-20).log10());
                }
                c *= 2f64.powf(1.0 / per_octave);
            }
            (centres, db)
        };
        let (frequencies_hz, gain_db) = bands(12.0, 40.0, 20_000.0);
        let (band_hz, band_db) = bands(3.0, 50.0, 20_000.0);
        let band: Vec<f64> = band_hz.iter().zip(&band_db).filter(|(f, _)| (150.0..=8000.0).contains(*f)).map(|(_, g)| *g).collect();
        let mean = band.iter().sum::<f64>() / band.len().max(1) as f64;
        let ripple = band.iter().fold(0.0f64, |m, g| m.max((g - mean).abs()));
        let mut peaks = Vec::new();
        let mut notches = Vec::new();
        for i in 1..frequencies_hz.len().saturating_sub(1) {
            let (a, b, c) = (gain_db[i - 1], gain_db[i], gain_db[i + 1]);
            if b > a && b >= c && b > mean - 20.0 && frequencies_hz[i] < 12_000.0 {
                peaks.push((frequencies_hz[i], b));
            }
            if b < a && b <= c && frequencies_hz[i] < 12_000.0 {
                notches.push((frequencies_hz[i], b));
            }
        }
        Ok(Transfer {
            noise_rms_full_scale: level,
            seconds,
            frequencies_hz,
            gain_db,
            band_hz,
            band_db,
            mean_db_150_8000: mean,
            ripple_db_150_8000: ripple,
            peaks_hz: peaks,
            notches_hz: notches,
        })
    }
}

/// Averaged Hann-windowed power spectrum (bins 0..=n/2), and the frame count.
/// With `skip_quiet`, frames under -60 dBFS mean square are left out.
pub fn power_spectrum(x: &[f32], n: usize, skip_quiet: bool) -> (Vec<f64>, usize) {
    let hop = n / 2;
    let mut power = vec![0.0f64; n / 2 + 1];
    let window: Vec<f64> = (0..n).map(|i| 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos()).collect();
    let mut frames = 0usize;
    let mut re = vec![0.0; n];
    let mut im = vec![0.0; n];
    let mut start = 0;
    while start + n <= x.len() {
        let e: f64 = x[start..start + n].iter().map(|v| f64::from(*v) * f64::from(*v)).sum::<f64>() / n as f64;
        if !skip_quiet || e > 1.0e-6 {
            for i in 0..n {
                re[i] = f64::from(x[start + i]) * window[i];
                im[i] = 0.0;
            }
            fft(&mut re, &mut im);
            for k in 0..=n / 2 {
                power[k] += re[k] * re[k] + im[k] * im[k];
            }
            frames += 1;
        }
        start += hop;
    }
    let norm = 1.0 / (frames.max(1) as f64 * (n as f64 / 2.0).powi(2));
    for p in &mut power {
        *p *= norm;
    }
    (power, frames)
}

/// Radix-2 FFT, iterative, in place.
fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -std::f64::consts::TAU / len as f64;
        let (wr, wi) = (ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for k in 0..len / 2 {
                let (ar, ai) = (re[start + k], im[start + k]);
                let (br, bi) = (re[start + k + len / 2], im[start + k + len / 2]);
                let (tr, ti) = (br * cr - bi * ci, br * ci + bi * cr);
                re[start + k] = ar + tr;
                im[start + k] = ai + ti;
                re[start + k + len / 2] = ar - tr;
                im[start + k + len / 2] = ai - ti;
                let ncr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = ncr;
            }
        }
        len <<= 1;
    }
}

/// Long-term third-octave spectrum (dB re full scale, power), spectral
/// centroid of the 200-5000 Hz power (Hz), and rms. A measurement aid.
pub fn ltas(x: &[f32], rate: f64) -> (Vec<f64>, Vec<f64>, f64, f64) {
    let n = 4096usize;
    let (power, _) = power_spectrum(x, n, true);
    let mut sum_sq = 0.0;
    for &v in x {
        sum_sq += f64::from(v) * f64::from(v);
    }
    let rms = if x.is_empty() { 0.0 } else { (sum_sq / x.len() as f64).sqrt() };
    let mut centres = Vec::new();
    let mut bands = Vec::new();
    let mut c = 50.0f64;
    let (mut cw, mut cp) = (0.0, 0.0);
    for k in 1..=n / 2 {
        let f = k as f64 * rate / n as f64;
        if (200.0..=5000.0).contains(&f) {
            cw += f * power[k];
            cp += power[k];
        }
    }
    while c < 16_000.0 {
        let (lo, hi) = (c / 2f64.powf(1.0 / 6.0), c * 2f64.powf(1.0 / 6.0));
        let mut p = 0.0;
        for k in 1..=n / 2 {
            let f = k as f64 * rate / n as f64;
            if (lo..hi).contains(&f) {
                p += power[k];
            }
        }
        centres.push(c);
        bands.push(10.0 * p.max(1.0e-20).log10());
        c *= 2f64.powf(1.0 / 3.0);
    }
    (centres, bands, if cp > 0.0 { cw / cp } else { 0.0 }, rms)
}
