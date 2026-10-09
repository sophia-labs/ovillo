//! The Ariadne World product engine, lifted from the VST3/CLAP wrapper
//! (`mus-rs/plugins/ariadne-clap/src/lib.rs` at f05473f) without its host ABI.
//!
//! `AriadneControls`, their defaults and ranges, and the lowering in
//! `process_sample` are the plug-in's, line for line, so a patch made in Ovillo
//! sounds the same in the plug-in. Keep the two in step until the engine moves
//! into a crate both depend on. What Ovillo adds is read-only: [`WorldReadings`]
//! taken from the World's own accessors, which a plug-in host never sees.

use mus_ariadne::law::active::ActiveInclusionControls;
use mus_ariadne::law::transport::ResonantTransportControls;
use mus_ariadne::{
    BodyControls, SharedSpaceControls, StereoListener, StereoListenerSpec, World, WorldControls,
    WorldFrame, WorldPlan, MODE_COUNT,
};
use serde::{Deserialize, Serialize};

const OUTPUT_SAFETY_LIMIT: f64 = 1.0;

/// The plug-in's twenty parameters, in its order. Serialized as a patch.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AriadneControls {
    pub material_organization: f64,
    pub source_coupling: f64,
    pub work_input: f64,
    pub active_coupling: f64,
    pub active_drive: f64,
    pub headroom: f64,
    pub sustain: f64,
    pub sag: f64,
    pub bloom: f64,
    pub feedback: f64,
    pub active_space: f64,
    pub interlace: f64,
    pub memory: f64,
    pub space_coupling: f64,
    pub space_decay: f64,
    pub space_radiation: f64,
    pub body_radiation: f64,
    pub listener_scale: f64,
    pub mix: f64,
    pub output_gain: f64,
}

impl Default for AriadneControls {
    fn default() -> Self {
        Self {
            material_organization: 0.06,
            source_coupling: 1.0,
            work_input: 0.004,
            active_coupling: 0.46,
            active_drive: 0.42,
            headroom: 0.72,
            sustain: 0.24,
            sag: 0.35,
            bloom: 0.36,
            feedback: 0.04,
            active_space: 0.16,
            interlace: 0.42,
            memory: 0.28,
            space_coupling: 0.16,
            space_decay: 0.42,
            space_radiation: 0.20,
            body_radiation: 0.008,
            listener_scale: 6.0,
            mix: 1.0,
            output_gain: 1.0,
        }
    }
}

/// One parameter's identity: the plug-in's id and display name, its range,
/// whether the plug-in smooths it slowly (60 ms rather than 20 ms), and a
/// short CRT label.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub label: &'static str,
    pub min: f64,
    pub max: f64,
    pub slow: bool,
}

const fn p(id: &'static str, name: &'static str, label: &'static str, min: f64, max: f64, slow: bool) -> ParamInfo {
    ParamInfo { id, name, label, min, max, slow }
}

/// The parameters in [`AriadneControls`] field order.
pub const PARAMS: [ParamInfo; 20] = [
    p("organization", "Material Organization", "ORGANIZ", 0.0, 1.0, true),
    p("source_coupling", "Source Coupling", "SRC CPL", 0.0, 4.0, false),
    p("work_input", "Work Input", "WORK IN", 0.0, 1.0, false),
    p("active_coupling", "Active Material", "ACTIVE", 0.0, 1.0, false),
    p("active_drive", "Active Drive", "DRIVE", 0.0, 1.5, false),
    p("headroom", "Headroom", "HEADROOM", 0.02, 1.0, false),
    p("sustain", "Sustain", "SUSTAIN", 0.0, 1.0, false),
    p("sag", "Hunger", "HUNGER", 0.0, 1.0, false),
    p("bloom", "Metabolism", "METABOL", 0.0, 1.0, false),
    p("feedback", "Reciprocity", "RECIPROC", 0.0, 1.0, true),
    p("active_space", "Active Space", "ACT SPACE", 0.0, 1.0, true),
    p("interlace", "Interlace", "INTERLACE", 0.0, 1.0, false),
    p("memory", "Path Memory", "PATH MEM", 0.0, 1.0, true),
    p("space_coupling", "Shared Space", "SHARED SP", 0.0, 1.0, true),
    p("space_decay", "Space Persistence", "PERSIST", 0.0, 1.0, true),
    p("space_radiation", "Space Radiation", "SPACE RAD", 0.0, 1.0, true),
    p("body_radiation", "Body Radiation", "BODY RAD", 0.0, 1.0, false),
    p("listener_scale", "Listener Scale", "LISTENER", 0.0, 64.0, false),
    p("mix", "Mix", "MIX", 0.0, 1.0, false),
    p("output", "Output", "OUTPUT", 0.0, 8.0, false),
];

impl AriadneControls {
    #[must_use]
    pub fn sanitized(self) -> Self {
        let mut a = self.as_array();
        for (value, info) in a.iter_mut().zip(PARAMS.iter()) {
            *value = finite(*value).clamp(info.min, info.max);
        }
        Self::from_array(a)
    }

    /// The value of parameter `i` in [`PARAMS`] order.
    #[must_use]
    pub fn get(&self, i: usize) -> f64 {
        self.as_array()[i]
    }

    pub fn set(&mut self, i: usize, v: f64) {
        let mut a = self.as_array();
        a[i] = v;
        *self = Self::from_array(a);
    }

    #[must_use]
    pub fn as_array(&self) -> [f64; 20] {
        [
            self.material_organization,
            self.source_coupling,
            self.work_input,
            self.active_coupling,
            self.active_drive,
            self.headroom,
            self.sustain,
            self.sag,
            self.bloom,
            self.feedback,
            self.active_space,
            self.interlace,
            self.memory,
            self.space_coupling,
            self.space_decay,
            self.space_radiation,
            self.body_radiation,
            self.listener_scale,
            self.mix,
            self.output_gain,
        ]
    }

    #[must_use]
    pub fn from_array(a: [f64; 20]) -> Self {
        Self {
            material_organization: a[0],
            source_coupling: a[1],
            work_input: a[2],
            active_coupling: a[3],
            active_drive: a[4],
            headroom: a[5],
            sustain: a[6],
            sag: a[7],
            bloom: a[8],
            feedback: a[9],
            active_space: a[10],
            interlace: a[11],
            memory: a[12],
            space_coupling: a[13],
            space_decay: a[14],
            space_radiation: a[15],
            body_radiation: a[16],
            listener_scale: a[17],
            mix: a[18],
            output_gain: a[19],
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AriadneFrame {
    pub left: f32,
    pub right: f32,
    pub observed_left: f32,
    pub observed_right: f32,
    pub world_error: bool,
    pub safety_limited: bool,
}

/// What the World says about itself after a sample: its own accessors, read,
/// never fed back. Energies are in the World's units; work and loss are per
/// sample.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WorldReadings {
    pub material_energy: f64,
    pub reservoir_energy: f64,
    pub space_energy: f64,
    pub configuration: f64,
    pub source_work: f64,
    pub internal_loss: f64,
    pub radiation: f64,
    pub global_closure_residual: f64,
}

pub struct AriadneEngine {
    world: World,
    listener: StereoListener,
    body_controls: Vec<BodyControls>,
    last_frame: WorldFrame,
    pub non_finite_input_frames: u64,
    pub world_errors: u64,
    pub safety_limited_frames: u64,
}

impl AriadneEngine {
    pub fn new(sample_rate_hz: f64) -> Result<Self, String> {
        Self::from_plan(WorldPlan::string_brass_v1(sample_rate_hz))
    }

    /// An engine over any one-body plan (e.g. one whose configuration path
    /// runs between two bore lengths, so Material Organization is a slide).
    pub fn from_plan(plan: WorldPlan) -> Result<Self, String> {
        let listener_spec =
            StereoListenerSpec::all_bodies(&plan, 1.0).map_err(|error| error.to_string())?;
        let listener =
            StereoListener::bind(&listener_spec, &plan).map_err(|error| error.to_string())?;
        let world = World::from_plan(plan).map_err(|error| error.to_string())?;
        Ok(Self {
            body_controls: vec![BodyControls::default(); world.body_count()],
            world,
            listener,
            last_frame: WorldFrame::default(),
            non_finite_input_frames: 0,
            world_errors: 0,
            safety_limited_frames: 0,
        })
    }

    #[inline]
    pub fn process_sample(&mut self, left: f32, right: f32, controls: AriadneControls) -> AriadneFrame {
        let controls = controls.sanitized();
        if !left.is_finite() || !right.is_finite() {
            self.non_finite_input_frames += 1;
        }
        let dry_left = if left.is_finite() { left as f64 } else { 0.0 };
        let dry_right = if right.is_finite() { right as f64 } else { 0.0 };
        let source = 0.5 * (dry_left + dry_right) * controls.source_coupling;

        let body = &mut self.body_controls[0];
        body.source_wave = source;
        body.work_input = controls.work_input;
        body.active_coupling = controls.active_coupling;
        body.active = ActiveInclusionControls {
            drive: controls.active_drive,
            headroom: controls.headroom,
            sustain: controls.sustain,
            sag: controls.sag,
            bloom: controls.bloom,
            feedback: controls.feedback,
            space: controls.active_space,
        };
        body.transport = ResonantTransportControls {
            interlace: controls.interlace,
            memory: controls.memory,
        };
        body.radiation_fraction = controls.body_radiation;
        let world_controls = WorldControls {
            space: SharedSpaceControls {
                coupling: controls.space_coupling,
                decay: controls.space_decay,
                radiation: controls.space_radiation,
            },
            direct_coupling: Default::default(),
        };

        let observed = self
            .world
            .set_body_configuration_index(0, controls.material_organization)
            .and_then(|_| self.world.process_sample(&self.body_controls, world_controls))
            .and_then(|frame| {
                self.last_frame = frame;
                self.listener
                    .observe(&self.world, &frame)
                    .map_err(|error| mus_ariadne::WorldError::Plan(error.to_string()))
            });
        let (observed_left, observed_right, world_error) = match observed {
            Ok(frame) => (frame.left * controls.listener_scale, frame.right * controls.listener_scale, false),
            Err(_) => {
                self.world_errors += 1;
                (0.0, 0.0, true)
            }
        };

        let output_left =
            dry_left.mul_add(1.0 - controls.mix, observed_left * controls.mix) * controls.output_gain;
        let output_right =
            dry_right.mul_add(1.0 - controls.mix, observed_right * controls.mix) * controls.output_gain;
        let (left, left_limited) = safe_output(output_left);
        let (right, right_limited) = safe_output(output_right);
        let safety_limited = left_limited || right_limited;
        if safety_limited {
            self.safety_limited_frames += 1;
        }

        AriadneFrame {
            left,
            right,
            observed_left: finite(observed_left) as f32,
            observed_right: finite(observed_right) as f32,
            world_error,
            safety_limited,
        }
    }

    pub fn reset(&mut self) {
        if self.world.reset().is_err() {
            self.world_errors += 1;
        }
        self.body_controls.fill(BodyControls::default());
        self.last_frame = WorldFrame::default();
        self.non_finite_input_frames = 0;
        self.world_errors = 0;
        self.safety_limited_frames = 0;
    }

    #[must_use]
    pub fn sample_rate(&self) -> f64 {
        self.world.sample_rate_hz()
    }

    #[must_use]
    pub fn plan_digest(&self) -> &str {
        self.world.plan_digest()
    }

    /// Cheap reads of the World's own state (no allocation).
    #[must_use]
    pub fn readings(&self) -> WorldReadings {
        WorldReadings {
            material_energy: self.world.body_material_energy(0).unwrap_or(0.0),
            reservoir_energy: self.world.body_reservoir_energy(0).unwrap_or(0.0),
            space_energy: self.world.space_stored_energy(),
            configuration: self.world.body_configuration_amount(0).unwrap_or(0.0),
            source_work: self.last_frame.source_work,
            internal_loss: self.last_frame.internal_loss,
            radiation: self.last_frame.radiation,
            global_closure_residual: self.world.global_closure_residual(),
        }
    }

    /// Energy in each of the body's 24 modes, `½(p² + (ωq)²)`, and the modes'
    /// frequencies at the body's current configuration.
    pub fn mode_energies(&self, energy: &mut [f32; MODE_COUNT], frequency_hz: &mut [f32; MODE_COUNT]) {
        let Some(state) = self.world.body_state(0) else { return };
        let amount = self.world.body_configuration_amount(0).unwrap_or(0.0);
        let configuration = self.world.plan().bodies[0].body.configuration_path.configuration_at(amount);
        for k in 0..MODE_COUNT {
            let f = configuration.metric.frequency_hz[k];
            let omega = std::f64::consts::TAU * f;
            let (q, m) = (state.displacement[k], state.momentum[k]);
            energy[k] = (0.5 * (m * m + (omega * q).powi(2))) as f32;
            frequency_hz[k] = f as f32;
        }
    }
}

#[inline]
fn finite(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

#[inline]
fn safe_output(value: f64) -> (f32, bool) {
    let value = finite(value);
    let limited = value.abs() > OUTPUT_SAFETY_LIMIT;
    (value.clamp(-OUTPUT_SAFETY_LIMIT, OUTPUT_SAFETY_LIMIT) as f32, limited)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controls_round_trip_through_the_param_table() {
        let c = AriadneControls::default();
        assert_eq!(AriadneControls::from_array(c.as_array()), c);
        assert_eq!(c.sanitized(), c);
        let mut wild = c;
        wild.active_drive = 9.0;
        wild.headroom = f64::NAN;
        let s = wild.sanitized();
        assert_eq!(s.active_drive, 1.5);
        assert_eq!(s.headroom, 0.02);
    }

    #[test]
    fn silence_in_silence_out_and_the_ledger_closes() {
        let mut e = AriadneEngine::new(48_000.0).unwrap();
        let c = AriadneControls::default();
        for _ in 0..4_800 {
            let f = e.process_sample(0.0, 0.0, c);
            assert!(f.left.is_finite() && f.right.is_finite());
        }
        assert_eq!(e.world_errors, 0);
        assert!(e.readings().global_closure_residual.abs() < 1e-9);
    }
}
