//! Continuous trajectory binding for the production string-network voice.
//!
//! [`mus_trajectory::TrajectoryProgram`] is body-neutral. This module is the
//! Weave/Guitar adapter: it binds validated lane names to the existing
//! [`ControlTarget`] surface and applies them immediately before each audio
//! sample. Because the program clock advances with [`StringNetworkVoice`], not
//! with an offline render loop, splitting a render into arbitrary host blocks
//! cannot change the path.
//!
//! Pitch transport is deliberately not faked here. The existing string kernel
//! has no `ControlTarget::PitchRatio`; its endpoint gliss rewrites delay state
//! under the still-open variable-rank transport problem. A lane named
//! `pitch_ratio` therefore returns a typed `PitchTransportPending` refusal.

use std::collections::BTreeMap;
use std::fmt;

use mus_trajectory::{
    Curve, LaneTemplate, TrajectoryError, TrajectoryProgram, TrajectoryReceipt, ValueSpace,
    FORMAT_V1,
};

use crate::backbone::ProtectedBackbone;
use crate::pluck::{ControlTarget, StringNetworkVoice};
use crate::SR;

/// A static patch plus the continuous program extracted from arrow-valued
/// physical controls. Arrow lanes are replaced by their first value in the
/// static patch so construction and sample zero agree exactly.
#[derive(Debug, Clone)]
pub struct StringTrajectoryPlan {
    pub static_params: BTreeMap<String, String>,
    pub program: TrajectoryProgram,
    pub receipt: TrajectoryReceipt,
}

/// Real-time wrapper around one persistent string-network voice.
pub struct StringTrajectoryVoice {
    voice: StringNetworkVoice,
    program: TrajectoryProgram,
    bindings: Vec<ControlTarget>,
    values: Vec<f64>,
    sample_index: u64,
    receipt: TrajectoryReceipt,
}

impl StringTrajectoryVoice {
    /// Bind a body-neutral program to an already-constructed voice.
    pub fn new(
        voice: StringNetworkVoice,
        program: TrajectoryProgram,
    ) -> Result<Self, StringTrajectoryError> {
        let receipt = program.receipt()?;
        let bindings = program
            .lanes
            .iter()
            .map(|lane| bind_target(&lane.target))
            .collect::<Result<Vec<_>, _>>()?;
        let values = vec![0.0; bindings.len()];
        Ok(Self {
            voice,
            program,
            bindings,
            values,
            sample_index: 0,
            receipt,
        })
    }

    /// Build a guitar from ordinary scalar params plus arrow-valued lanes.
    pub fn guitar(
        params: &BTreeMap<String, String>,
        frequencies_hz: &[f64],
        slot_s: f64,
        gliss_ratio: f64,
    ) -> Result<Self, StringTrajectoryError> {
        let plan = compile_string_trajectory(params, slot_samples(slot_s))?;
        let voice =
            StringNetworkVoice::guitar(&plan.static_params, frequencies_hz, slot_s, gliss_ratio);
        Self::new(voice, plan.program)
    }

    /// Build a Weave voice from ordinary scalar params plus arrow-valued lanes.
    pub fn weave(
        params: &BTreeMap<String, String>,
        frequencies_hz: &[f64],
        slot_s: f64,
        gliss_ratio: f64,
    ) -> Result<Self, StringTrajectoryError> {
        let plan = compile_string_trajectory(params, slot_samples(slot_s))?;
        let voice =
            StringNetworkVoice::weave(&plan.static_params, frequencies_hz, slot_s, gliss_ratio);
        Self::new(voice, plan.program)
    }

    /// Build a sample-driven guitar with the same trajectory semantics.
    pub fn guitar_sung(
        params: &BTreeMap<String, String>,
        frequencies_hz: &[f64],
        slot_s: f64,
        gliss_ratio: f64,
        external: &[f32],
    ) -> Result<Self, StringTrajectoryError> {
        let plan = compile_string_trajectory(params, slot_samples(slot_s))?;
        let voice = StringNetworkVoice::guitar_sung(
            &plan.static_params,
            frequencies_hz,
            slot_s,
            gliss_ratio,
            external,
        );
        Self::new(voice, plan.program)
    }

    /// Build a sample-driven Weave voice with the same trajectory semantics.
    pub fn weave_sung(
        params: &BTreeMap<String, String>,
        frequencies_hz: &[f64],
        slot_s: f64,
        gliss_ratio: f64,
        external: &[f32],
    ) -> Result<Self, StringTrajectoryError> {
        let plan = compile_string_trajectory(params, slot_samples(slot_s))?;
        let voice = StringNetworkVoice::weave_sung(
            &plan.static_params,
            frequencies_hz,
            slot_s,
            gliss_ratio,
            external,
        );
        Self::new(voice, plan.program)
    }

    fn apply_current_controls(&mut self) {
        if self.bindings.is_empty() {
            return;
        }
        self.program
            .write_values(self.sample_index, &mut self.values)
            .expect("program was validated when the voice was constructed");
        for (&target, &value) in self.bindings.iter().zip(&self.values) {
            self.voice.apply_control(target, value);
        }
    }

    /// Render one sample. No allocation occurs on this path.
    pub fn next_sample(&mut self) -> f32 {
        if self.voice.is_finished() {
            return 0.0;
        }
        self.apply_current_controls();
        self.sample_index += 1;
        self.voice.next_sample()
    }

    /// Render one sample through a persistent protected backbone.
    pub fn next_sample_bridged(&mut self, backbone: &mut ProtectedBackbone) -> f32 {
        if self.voice.is_finished() {
            return 0.0;
        }
        self.apply_current_controls();
        self.sample_index += 1;
        self.voice.next_sample_bridged(Some(backbone))
    }

    /// Fill a host-provided block. Path timing is independent of block size.
    pub fn render_block(&mut self, output: &mut [f32]) -> usize {
        let mut written = 0;
        for sample in output {
            if self.voice.is_finished() {
                *sample = 0.0;
            } else {
                *sample = self.next_sample();
                written += 1;
            }
        }
        written
    }

    /// Fill a host-provided block through a persistent protected backbone.
    pub fn render_block_bridged(
        &mut self,
        output: &mut [f32],
        backbone: &mut ProtectedBackbone,
    ) -> usize {
        let mut written = 0;
        for sample in output {
            if self.voice.is_finished() {
                *sample = 0.0;
            } else {
                *sample = self.next_sample_bridged(backbone);
                written += 1;
            }
        }
        written
    }

    /// Render all remaining samples.
    pub fn render(mut self) -> Vec<f32> {
        let mut output = vec![0.0; self.voice.remaining_samples()];
        self.render_block(&mut output);
        output
    }

    pub fn is_finished(&self) -> bool {
        self.voice.is_finished()
    }

    pub fn remaining_samples(&self) -> usize {
        self.voice.remaining_samples()
    }

    pub fn sample_index(&self) -> u64 {
        self.sample_index
    }

    pub fn program(&self) -> &TrajectoryProgram {
        &self.program
    }

    pub fn receipt(&self) -> &TrajectoryReceipt {
        &self.receipt
    }

    pub fn inner(&self) -> &StringNetworkVoice {
        &self.voice
    }

    pub fn into_inner(self) -> StringNetworkVoice {
        self.voice
    }
}

fn slot_samples(slot_s: f64) -> u64 {
    (slot_s.max(0.0) * SR as f64).round().max(1.0) as u64
}

/// Extract every arrow-valued string-network control into one program.
/// Ordinary scalar values remain ordinary patch parameters.
pub fn compile_string_trajectory(
    params: &BTreeMap<String, String>,
    duration_samples: u64,
) -> Result<StringTrajectoryPlan, StringTrajectoryError> {
    if params
        .get("pitch_ratio")
        .is_some_and(|value| value.contains("->"))
    {
        return Err(StringTrajectoryError::PitchTransportPending);
    }

    let mut static_params = params.clone();
    let mut lanes = Vec::new();
    let curve = params
        .get("trajectory_curve")
        .map(String::as_str)
        .map(|name| {
            Curve::from_name(name).ok_or_else(|| StringTrajectoryError::UnknownCurve {
                name: name.to_string(),
            })
        })
        .transpose()?
        .unwrap_or(Curve::Linear);

    for name in STRING_TRAJECTORY_TARGETS {
        let Some(raw) = params.get(*name) else {
            continue;
        };
        let Some(values) = mus_trajectory::parse_arrow_values(name, raw)? else {
            continue;
        };
        let template = target_template(name, curve).expect("name came from target registry");
        let lane = template
            .compile_arrow(raw, duration_samples)?
            .expect("arrow was already detected");
        static_params.insert((*name).to_string(), values[0].to_string());
        lanes.push(lane);
    }

    let program = TrajectoryProgram {
        format: FORMAT_V1.to_string(),
        sample_rate_hz: SR as u32,
        duration_samples,
        hold_final: true,
        lanes,
    };
    let receipt = program.receipt()?;
    Ok(StringTrajectoryPlan {
        static_params,
        program,
        receipt,
    })
}

/// Directly render a trajectory-enabled guitar.
pub fn guitar_note_trajectory(
    params: &BTreeMap<String, String>,
    frequencies_hz: &[f64],
    slot_s: f64,
    gliss_ratio: f64,
) -> Result<Vec<f32>, StringTrajectoryError> {
    Ok(StringTrajectoryVoice::guitar(params, frequencies_hz, slot_s, gliss_ratio)?.render())
}

/// Directly render a trajectory-enabled Weave voice.
pub fn weave_note_trajectory(
    params: &BTreeMap<String, String>,
    frequencies_hz: &[f64],
    slot_s: f64,
    gliss_ratio: f64,
) -> Result<Vec<f32>, StringTrajectoryError> {
    Ok(StringTrajectoryVoice::weave(params, frequencies_hz, slot_s, gliss_ratio)?.render())
}

const STRING_TRAJECTORY_TARGETS: &[&str] = &[
    "couple",
    "chirality",
    "orbit",
    "orbit_depth",
    "curvature",
    "body",
    "buzz",
    "sing",
    "bow",
    "bow_speed",
    "sus",
    "damp",
    "breath",
];

fn bind_target(name: &str) -> Result<ControlTarget, StringTrajectoryError> {
    if name == "pitch_ratio" {
        return Err(StringTrajectoryError::PitchTransportPending);
    }
    ControlTarget::from_name(name).ok_or_else(|| StringTrajectoryError::UnsupportedTarget {
        target: name.to_string(),
    })
}

fn target_template(name: &str, curve: Curve) -> Option<LaneTemplate> {
    let (unit, minimum, maximum) = match name {
        "couple" => ("radian", 0.0, 0.45),
        "chirality" => ("unitless", -1.0, 1.0),
        "orbit" => ("hertz", 0.0, 20.0),
        "orbit_depth" | "curvature" | "body" | "buzz" | "sing" | "bow" | "bow_speed" | "breath" => {
            ("unitless", 0.0, 1.0)
        }
        "sus" => ("second", 0.05, 20.0),
        "damp" => ("unitless", 0.0, 0.95),
        _ => return None,
    };
    Some(LaneTemplate {
        target: name.to_string(),
        unit: unit.to_string(),
        curve,
        value_space: ValueSpace::Linear,
        min_value: Some(minimum),
        max_value: Some(maximum),
        // We do not invent actuator-rate limits. When measured limits exist,
        // the adapter can fill this field and the generic validator enforces
        // them without changing the executor.
        max_rate_per_second: None,
    })
}

/// Typed body-adapter refusal.
#[derive(Debug, Clone, PartialEq)]
pub enum StringTrajectoryError {
    Program(TrajectoryError),
    UnknownCurve {
        name: String,
    },
    UnsupportedTarget {
        target: String,
    },
    /// The first unified seam deliberately left open. A physical pitch path
    /// needs state transport for changing delay length, not output resampling.
    PitchTransportPending,
}

impl StringTrajectoryError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Program(error) => error.code(),
            Self::UnknownCurve { .. } => "string-trajectory-unknown-curve",
            Self::UnsupportedTarget { .. } => "string-trajectory-unsupported-target",
            Self::PitchTransportPending => "string-trajectory-pitch-transport-pending",
        }
    }
}

impl From<TrajectoryError> for StringTrajectoryError {
    fn from(value: TrajectoryError) -> Self {
        Self::Program(value)
    }
}

impl fmt::Display for StringTrajectoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Program(error) => error.fmt(formatter),
            Self::UnknownCurve { name } => {
                write!(formatter, "unknown string trajectory curve {name:?}")
            }
            Self::UnsupportedTarget { target } => {
                write!(
                    formatter,
                    "string voice does not bind trajectory target {target:?}"
                )
            }
            Self::PitchTransportPending => write!(
                formatter,
                "pitch_ratio trajectory refused: lawful delay-state transport is still open"
            ),
        }
    }
}

impl std::error::Error for StringTrajectoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Program(error) => Some(error),
            _ => None,
        }
    }
}
