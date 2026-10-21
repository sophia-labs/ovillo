use serde::Serialize;
use std::fmt::{Display, Formatter};

const MAX_ATTACK_GAIN: f64 = 4.0;
const ATTACK_EXECUTION_STRATEGY: &str =
    "prevalidated-monotone-attack-cursor+network-observer-off-v2";

/// One physical re-excitation of an already-constructed fixed-topology body.
///
/// V2 keeps the active course set, excitation identity, and pitch topology fixed.
/// `gain` scales the constructor's content-keyed excitation template. The score
/// adapter currently emits unity gains; the field exists so the physical attack
/// schedule has an honest amplitude coordinate before score dynamics are mapped
/// onto it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttackFrame {
    pub at_sample: usize,
    pub gain: f64,
}

/// Derived, deterministic evidence for one accepted attack schedule.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttackScheduleReceipt {
    pub attack_count: usize,
    pub active_courses: usize,
    pub attack_span_samples: usize,
    pub drive_samples: usize,
    pub first_attack_sample: usize,
    pub last_attack_sample: usize,
    pub maximum_gain: f64,
    pub overlap_policy: String,
    pub excitation_identity: String,
    pub execution_strategy: String,
}

/// A prevalidated, fixed-capacity schedule. Construction performs every
/// ordering, overlap, topology, and domain check before a physical body exists.
#[derive(Debug, Clone)]
pub struct AttackSchedule {
    frames: Box<[AttackFrame]>,
    drive_samples: usize,
    receipt: AttackScheduleReceipt,
}

impl AttackSchedule {
    pub fn compile(
        params: &BTreeMap<String, String>,
        frequencies_hz: &[f64],
        drive_samples: usize,
        frames: &[AttackFrame],
    ) -> Result<Self, RearticulationError> {
        let Some(first_frame) = frames.first() else {
            return Err(RearticulationError::EmptySchedule);
        };
        if drive_samples == 0 {
            return Err(RearticulationError::EmptyDriveWindow);
        }
        if first_frame.at_sample != 0 {
            return Err(RearticulationError::FirstAttackNotAtZero {
                at_sample: first_frame.at_sample,
            });
        }

        let (active_courses, attack_span_samples) =
            planned_active_topology(params, frequencies_hz)?;
        let mut maximum_gain = 0.0_f64;
        for (index, frame) in frames.iter().enumerate() {
            if !frame.gain.is_finite() {
                return Err(RearticulationError::NonFiniteGain {
                    index,
                    gain: frame.gain,
                });
            }
            if frame.gain <= 0.0 || frame.gain > MAX_ATTACK_GAIN {
                return Err(RearticulationError::GainOutOfRange {
                    index,
                    gain: frame.gain,
                    maximum: MAX_ATTACK_GAIN,
                });
            }
            if frame.at_sample >= drive_samples {
                return Err(RearticulationError::AttackOutsideDriveWindow {
                    index,
                    at_sample: frame.at_sample,
                    drive_samples,
                });
            }
            frame
                .at_sample
                .checked_add(attack_span_samples)
                .ok_or(RearticulationError::SampleIndexOverflow)?;
            maximum_gain = maximum_gain.max(frame.gain);
        }

        for (index, pair) in frames.windows(2).enumerate() {
            let previous = pair[0];
            let current = pair[1];
            if current.at_sample <= previous.at_sample {
                return Err(RearticulationError::NonIncreasingSchedule {
                    index: index + 1,
                    previous: previous.at_sample,
                    current: current.at_sample,
                });
            }
            let previous_end = previous
                .at_sample
                .checked_add(attack_span_samples)
                .ok_or(RearticulationError::SampleIndexOverflow)?;
            if current.at_sample < previous_end {
                return Err(RearticulationError::OverlappingAttacks {
                    index: index + 1,
                    previous_at: previous.at_sample,
                    previous_end,
                    current_at: current.at_sample,
                });
            }
        }

        let receipt = AttackScheduleReceipt {
            attack_count: frames.len(),
            active_courses,
            attack_span_samples,
            drive_samples,
            first_attack_sample: first_frame.at_sample,
            last_attack_sample: frames.last().unwrap_or(first_frame).at_sample,
            maximum_gain,
            overlap_policy: "refuse-v1".to_string(),
            excitation_identity: "constructor-content-keyed-template".to_string(),
            execution_strategy: ATTACK_EXECUTION_STRATEGY.to_string(),
        };
        Ok(Self {
            frames: frames.to_vec().into_boxed_slice(),
            drive_samples,
            receipt,
        })
    }

    pub fn frames(&self) -> &[AttackFrame] {
        &self.frames
    }

    pub fn receipt(&self) -> &AttackScheduleReceipt {
        &self.receipt
    }
}

/// A `StringNetworkVoice` with several physical attacks compiled against one
/// persistent body.
///
/// All allocations happen in [`Self::new`]. The sample kernel advances a
/// monotone attack cursor, reads a preallocated excitation template, and keeps
/// the delay lines, allpass memories, body modes, Weave waves, orbit phase, and
/// control state alive. It also specializes the network-level observer-off path:
/// no gesture, external-drive, backbone, trace, edge-telemetry, or body-energy
/// bookkeeping branch runs in the ordinary lifetime render.
pub struct RearticulatedStringVoice {
    voice: StringNetworkVoice,
    schedule: AttackSchedule,
    excitation_templates: Vec<Vec<f64>>,
    attack_start_offsets: Vec<usize>,
    phase_offsets: Vec<f64>,
    next_attack: usize,
    current_attack: Option<usize>,
    active_normalization: f64,
    network_normalization: f64,
    course_count_f64: f64,
}

impl RearticulatedStringVoice {
    pub fn new(
        mut voice: StringNetworkVoice,
        schedule: AttackSchedule,
    ) -> Result<Self, RearticulationError> {
        if voice.sample_index != 0 {
            return Err(RearticulationError::VoiceAlreadyStarted {
                sample_index: voice.sample_index,
            });
        }
        if voice.observe || voice.strings.iter().any(|string| string.observe) {
            return Err(RearticulationError::ObserverUnsupported);
        }
        if voice.external.is_some() || voice.patch.sing > 1e-12 {
            return Err(RearticulationError::ExternalDriveUnsupported);
        }
        if !voice.gestures.is_empty() || voice.next_gesture != 0 {
            return Err(RearticulationError::GestureWordUnsupported);
        }
        if voice.drive_samples != schedule.drive_samples {
            return Err(RearticulationError::DriveWindowMismatch {
                planned: schedule.drive_samples,
                actual: voice.drive_samples,
            });
        }
        if voice.active_count != schedule.receipt.active_courses {
            return Err(RearticulationError::ActiveTopologyMismatch {
                planned: schedule.receipt.active_courses,
                actual: voice.active_count,
            });
        }

        let active_count = voice.active_count;
        let mut excitation_templates = Vec::with_capacity(active_count);
        let mut attack_start_offsets = Vec::with_capacity(active_count);
        let mut actual_span = 0_usize;
        for string in voice.strings.iter_mut().take(active_count) {
            let template = std::mem::take(&mut string.excitation);
            let end = string
                .start_sample
                .checked_add(template.len())
                .ok_or(RearticulationError::SampleIndexOverflow)?;
            actual_span = actual_span.max(end);
            attack_start_offsets.push(string.start_sample);
            excitation_templates.push(template);
        }
        if actual_span != schedule.receipt.attack_span_samples {
            return Err(RearticulationError::AttackSpanMismatch {
                planned: schedule.receipt.attack_span_samples,
                actual: actual_span,
            });
        }

        let course_count = voice.strings.len();
        let course_count_f64 = course_count.max(1) as f64;
        let phase_offsets = (0..course_count)
            .map(|index| 2.0 * PI * index as f64 / course_count_f64)
            .collect();
        let active_normalization = (active_count.max(1) as f64).sqrt();
        let network_normalization = course_count_f64.sqrt();

        Ok(Self {
            voice,
            schedule,
            excitation_templates,
            attack_start_offsets,
            phase_offsets,
            next_attack: 0,
            current_attack: None,
            active_normalization,
            network_normalization,
            course_count_f64,
        })
    }

    pub fn receipt(&self) -> &AttackScheduleReceipt {
        self.schedule.receipt()
    }

    pub fn remaining_samples(&self) -> usize {
        self.voice.remaining_samples()
    }

    fn update_attack_cursor(&mut self, sample_index: usize) {
        while self.next_attack < self.schedule.frames.len()
            && self.schedule.frames[self.next_attack].at_sample <= sample_index
        {
            self.current_attack = Some(self.next_attack);
            self.next_attack += 1;
        }
    }

    // The ring index addresses the waves, the strings and the phase offsets.
    #[allow(clippy::needless_range_loop)]
    fn scatter_weave_unobserved(&mut self) {
        let phase_offsets = &self.phase_offsets;
        let course_count_f64 = self.course_count_f64;
        let voice = &mut self.voice;
        let count = voice.waves.len();
        if count < 2 || voice.patch.couple <= 1e-12 {
            return;
        }
        let phase = voice.orbit_phase;
        let total_energy = voice
            .strings
            .iter()
            .map(|string| string.energy_smooth)
            .sum::<f64>()
            .max(1e-14);
        let forward_weight = 0.5 * (1.0 + voice.patch.chirality);
        let backward_weight = 0.5 * (1.0 - voice.patch.chirality);

        for index in 0..count {
            let next = (index + 1) % count;
            let travelling =
                1.0 + voice.patch.orbit_depth * (phase + phase_offsets[index]).sin();
            let previous_energy = voice.strings[(index + count - 1) % count].energy_smooth;
            let next_energy = voice.strings[next].energy_smooth;
            let gradient =
                ((previous_energy - next_energy) * course_count_f64 / total_energy).tanh();
            let metric = 1.0 + 0.72 * voice.patch.curvature * gradient;
            let angle =
                (voice.patch.couple * travelling * metric * forward_weight).clamp(-0.48, 0.48);
            let (a, b) = givens(voice.waves[index], voice.waves[next], angle);
            voice.waves[index] = a;
            voice.waves[next] = b;
        }
        for reverse_index in (0..count).rev() {
            let next = (reverse_index + 1) % count;
            let travelling = 1.0
                + voice.patch.orbit_depth * (phase + phase_offsets[reverse_index]).sin();
            let previous_energy =
                voice.strings[(reverse_index + count - 1) % count].energy_smooth;
            let next_energy = voice.strings[next].energy_smooth;
            let gradient =
                ((previous_energy - next_energy) * course_count_f64 / total_energy).tanh();
            let metric = 1.0 + 0.72 * voice.patch.curvature * gradient;
            let angle =
                (voice.patch.couple * travelling * metric * backward_weight).clamp(-0.48, 0.48);
            let (a, b) = givens(voice.waves[reverse_index], voice.waves[next], angle);
            voice.waves[reverse_index] = a;
            voice.waves[next] = b;
        }
    }

    fn scatter_body_unobserved(&mut self) -> f64 {
        let voice = &mut self.voice;
        let count = voice.waves.len();
        if count == 0 || voice.patch.body <= 1e-12 {
            for mode in &mut voice.body {
                mode.free_tick();
            }
            return 0.0;
        }
        let chirality = if voice.mode == StringNetworkMode::Weave {
            voice.patch.chirality
        } else {
            0.0
        };
        let forward_weight = 0.5 * (1.0 + chirality);
        let backward_weight = 0.5 * (1.0 - chirality);
        let mut body_output = 0.0;
        for mode in &mut voice.body {
            mode.free_tick();
            let base_angle = voice.patch.body * mode.coupling;
            for index in 0..count {
                let angle = base_angle * voice.strings[index].coupling_scale * forward_weight;
                let (wave, momentum) = givens(voice.waves[index], mode.p, angle);
                voice.waves[index] = wave;
                mode.p = momentum;
            }
            for index in (0..count).rev() {
                let angle = base_angle * voice.strings[index].coupling_scale * backward_weight;
                let (wave, momentum) = givens(voice.waves[index], mode.p, angle);
                voice.waves[index] = wave;
                mode.p = momentum;
            }
            body_output += mode.radiation * mode.q;
        }
        body_output
    }

    /// One allocation-free sample of the multi-attack observer-off kernel.
    pub fn next_sample(&mut self) -> f32 {
        if self.voice.is_finished() || self.voice.strings.is_empty() {
            return 0.0;
        }

        let sample_index = self.voice.sample_index;
        self.update_attack_cursor(sample_index);
        for index in 0..self.voice.strings.len() {
            let wave = self.voice.strings[index].read(sample_index);
            self.voice.waves[index] = wave;
            self.voice.incoming[index] = wave;
        }
        if self.voice.mode == StringNetworkMode::Weave {
            self.scatter_weave_unobserved();
        }
        let body_output = self.scatter_body_unobserved();

        // External-drive and backbone terms are refused at construction. Keep
        // their exact +0.0 positions in the arithmetic expression so the
        // single-attack zero-limit remains a bit law, including signed zero.
        let drive = 0.0_f64;
        let backbone_emitted = 0.0_f64;
        let active_attack = self.current_attack.map(|index| self.schedule.frames[index]);
        let active_normalization = self.active_normalization;
        let network_normalization = self.network_normalization;
        let excitation_templates = &self.excitation_templates;
        let attack_start_offsets = &self.attack_start_offsets;
        let voice = &mut self.voice;

        let bowing = voice.patch.bow > 1e-12 && sample_index < voice.drive_samples;
        let bow_velocity = BOW_VELOCITY_MIN
            + (BOW_VELOCITY_MAX - BOW_VELOCITY_MIN)
                * voice.patch.bow_speed.clamp(0.0, 1.0);

        let mut contact_output = 0.0;
        for index in 0..voice.strings.len() {
            let coupling_scale = voice.strings[index].coupling_scale;
            let start_sample = voice.strings[index].start_sample;
            let slot_samples = voice.strings[index].slot_samples;
            let in_slot =
                sample_index >= start_sample && sample_index < start_sample + slot_samples;
            let breath = if in_slot && voice.patch.breath > 1e-12 {
                match voice.breath.get_mut(index) {
                    Some(channel) => voice.patch.breath * 0.25 * coupling_scale * channel.tick(),
                    None => 0.0,
                }
            } else {
                0.0
            };
            let burst = if index < voice.active_count {
                scheduled_burst(
                    active_attack,
                    &excitation_templates[index],
                    attack_start_offsets[index],
                    sample_index,
                )
            } else {
                0.0
            };
            let drive_here = drive * coupling_scale;
            let mut excitation = burst + drive_here + breath;
            if bowing && index < voice.active_count {
                let relative_velocity = bow_velocity - voice.waves[index];
                let force =
                    BOW_GAIN * voice.patch.bow * bow_friction(relative_velocity) * coupling_scale;
                let string = &mut voice.strings[index];
                string.bow_dc += BOW_DC_POLE * (force - string.bow_dc);
                excitation += force - string.bow_dc;
            }
            let value = voice.waves[index] + excitation;
            let (feedback, contact) = voice.strings[index].apply_contact(value, voice.patch.buzz);
            voice.contact[index] = contact;
            contact_output += contact;
            voice.strings[index].write(feedback);
        }

        let string_direct = voice
            .incoming
            .iter()
            .take(voice.active_count)
            .sum::<f64>()
            / active_normalization;
        let bridge_force = voice
            .incoming
            .iter()
            .zip(voice.waves.iter())
            .map(|(before, after)| before - after)
            .sum::<f64>()
            / network_normalization;
        contact_output /= network_normalization;

        let raw = 0.18 * string_direct
            + (0.92 + 0.52 * voice.patch.body) * body_output
            + 0.18 * bridge_force
            + 0.20 * contact_output
            + backbone_emitted;
        let output = raw - voice.previous_input + 0.995 * voice.previous_output;
        voice.previous_input = raw;
        voice.previous_output = output;
        voice.orbit_phase =
            (voice.orbit_phase + 2.0 * PI * voice.patch.orbit_hz / SR_F64)
                .rem_euclid(2.0 * PI);
        voice.sample_index += 1;
        (0.72 * output) as f32
    }

    pub fn render(mut self) -> Vec<f32> {
        let total_samples = self.remaining_samples();
        let mut output = Vec::with_capacity(total_samples);
        for _ in 0..total_samples {
            output.push(self.next_sample());
        }
        output
    }

    pub fn render_controlled(
        mut self,
        control_frames: &[ControlFrame],
    ) -> Result<Vec<f32>, RearticulationError> {
        for (index, pair) in control_frames.windows(2).enumerate() {
            if pair[1].at_sample < pair[0].at_sample {
                return Err(RearticulationError::ControlFramesOutOfOrder {
                    index: index + 1,
                    previous: pair[0].at_sample,
                    current: pair[1].at_sample,
                });
            }
        }
        let total_samples = self.remaining_samples();
        let mut output = Vec::with_capacity(total_samples);
        let mut next_frame = 0_usize;
        for sample_index in 0..total_samples {
            while next_frame < control_frames.len()
                && control_frames[next_frame].at_sample <= sample_index
            {
                for &(target, value) in &control_frames[next_frame].sets {
                    self.voice.apply_control(target, value);
                }
                next_frame += 1;
            }
            output.push(self.next_sample());
        }
        Ok(output)
    }
}

fn scheduled_burst(
    attack: Option<AttackFrame>,
    template: &[f64],
    course_start_offset: usize,
    sample_index: usize,
) -> f64 {
    let Some(attack) = attack else {
        return 0.0;
    };
    let Some(start) = attack.at_sample.checked_add(course_start_offset) else {
        return 0.0;
    };
    let Some(age) = sample_index.checked_sub(start) else {
        return 0.0;
    };
    let Some(&sample) = template.get(age) else {
        return 0.0;
    };
    if attack.gain.to_bits() == 1.0_f64.to_bits() {
        sample
    } else {
        sample * attack.gain
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RearticulationError {
    EmptySchedule,
    EmptyDriveWindow,
    FirstAttackNotAtZero {
        at_sample: usize,
    },
    NonIncreasingSchedule {
        index: usize,
        previous: usize,
        current: usize,
    },
    NonFiniteGain {
        index: usize,
        gain: f64,
    },
    GainOutOfRange {
        index: usize,
        gain: f64,
        maximum: f64,
    },
    AttackOutsideDriveWindow {
        index: usize,
        at_sample: usize,
        drive_samples: usize,
    },
    OverlappingAttacks {
        index: usize,
        previous_at: usize,
        previous_end: usize,
        current_at: usize,
    },
    NoActiveCourses,
    SampleIndexOverflow,
    VoiceAlreadyStarted {
        sample_index: usize,
    },
    ObserverUnsupported,
    ExternalDriveUnsupported,
    GestureWordUnsupported,
    DriveWindowMismatch {
        planned: usize,
        actual: usize,
    },
    ActiveTopologyMismatch {
        planned: usize,
        actual: usize,
    },
    AttackSpanMismatch {
        planned: usize,
        actual: usize,
    },
    ControlFramesOutOfOrder {
        index: usize,
        previous: usize,
        current: usize,
    },
}

impl RearticulationError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptySchedule => "rearticulation-empty-schedule",
            Self::EmptyDriveWindow => "rearticulation-empty-drive-window",
            Self::FirstAttackNotAtZero { .. } => "rearticulation-first-attack-not-zero",
            Self::NonIncreasingSchedule { .. } => "rearticulation-nonincreasing-schedule",
            Self::NonFiniteGain { .. } => "rearticulation-nonfinite-gain",
            Self::GainOutOfRange { .. } => "rearticulation-gain-out-of-range",
            Self::AttackOutsideDriveWindow { .. } => "rearticulation-attack-outside-drive",
            Self::OverlappingAttacks { .. } => "rearticulation-overlapping-attacks",
            Self::NoActiveCourses => "rearticulation-no-active-courses",
            Self::SampleIndexOverflow => "rearticulation-sample-index-overflow",
            Self::VoiceAlreadyStarted { .. } => "rearticulation-voice-already-started",
            Self::ObserverUnsupported => "rearticulation-observer-unsupported",
            Self::ExternalDriveUnsupported => "rearticulation-external-drive-unsupported",
            Self::GestureWordUnsupported => "rearticulation-gesture-word-unsupported",
            Self::DriveWindowMismatch { .. } => "rearticulation-drive-window-mismatch",
            Self::ActiveTopologyMismatch { .. } => "rearticulation-active-topology-mismatch",
            Self::AttackSpanMismatch { .. } => "rearticulation-attack-span-mismatch",
            Self::ControlFramesOutOfOrder { .. } => "rearticulation-control-frames-out-of-order",
        }
    }
}

impl Display for RearticulationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let code = self.code();
        match self {
            Self::EmptySchedule => write!(formatter, "{code}: at least one attack is required"),
            Self::EmptyDriveWindow => write!(formatter, "{code}: drive window is empty"),
            Self::FirstAttackNotAtZero { at_sample } => write!(
                formatter,
                "{code}: first attack is at sample {at_sample}, not sample 0"
            ),
            Self::NonIncreasingSchedule {
                index,
                previous,
                current,
            } => write!(
                formatter,
                "{code}: attack {index} at {current} does not follow previous sample {previous}"
            ),
            Self::NonFiniteGain { index, gain } => {
                write!(formatter, "{code}: attack {index} gain {gain} is not finite")
            }
            Self::GainOutOfRange {
                index,
                gain,
                maximum,
            } => write!(
                formatter,
                "{code}: attack {index} gain {gain} is outside (0, {maximum}]"
            ),
            Self::AttackOutsideDriveWindow {
                index,
                at_sample,
                drive_samples,
            } => write!(
                formatter,
                "{code}: attack {index} at {at_sample} is outside drive window 0..{drive_samples}"
            ),
            Self::OverlappingAttacks {
                index,
                previous_at,
                previous_end,
                current_at,
            } => write!(
                formatter,
                "{code}: attack {index} at {current_at} overlaps previous excitation [{previous_at}, {previous_end})"
            ),
            Self::NoActiveCourses => write!(formatter, "{code}: body has no playable courses"),
            Self::SampleIndexOverflow => write!(formatter, "{code}: sample index overflow"),
            Self::VoiceAlreadyStarted { sample_index } => write!(
                formatter,
                "{code}: voice has already advanced to sample {sample_index}"
            ),
            Self::ObserverUnsupported => {
                write!(formatter, "{code}: schedule requires an observer-off voice")
            }
            Self::ExternalDriveUnsupported => write!(
                formatter,
                "{code}: external sing drive is not admitted by the rearticulation kernel"
            ),
            Self::GestureWordUnsupported => write!(
                formatter,
                "{code}: compiled gesture words are not yet fused with rearticulation"
            ),
            Self::DriveWindowMismatch { planned, actual } => write!(
                formatter,
                "{code}: planned drive window {planned} differs from voice window {actual}"
            ),
            Self::ActiveTopologyMismatch { planned, actual } => write!(
                formatter,
                "{code}: planned {planned} active courses but voice constructed {actual}"
            ),
            Self::AttackSpanMismatch { planned, actual } => write!(
                formatter,
                "{code}: planned excitation span {planned} differs from constructed span {actual}"
            ),
            Self::ControlFramesOutOfOrder {
                index,
                previous,
                current,
            } => write!(
                formatter,
                "{code}: control frame {index} at {current} precedes {previous}"
            ),
        }
    }
}

impl std::error::Error for RearticulationError {}

fn planned_active_topology(
    params: &BTreeMap<String, String>,
    frequencies_hz: &[f64],
) -> Result<(usize, usize), RearticulationError> {
    let patch = PluckPatch::from(params);
    let strum_samples = (patch.strum_ms.abs() * SR_F64 / 1000.0).round() as usize;
    let mut starts = vec![0_usize; frequencies_hz.len()];
    if patch.strum_ms >= 0.0 {
        for (order, start) in starts.iter_mut().enumerate() {
            *start = order * strum_samples;
        }
    } else {
        let len = starts.len();
        for (index, start) in starts.iter_mut().enumerate() {
            *start = (len - 1 - index) * strum_samples;
        }
    }

    let period = |frequency: f64| (SR_F64 / frequency.max(20.0)).ceil().max(8.0) as usize;
    let mut active_courses = 0_usize;
    let mut span = 0_usize;
    for (pitch_index, &frequency) in frequencies_hz.iter().enumerate() {
        if frequency < 20.0 {
            continue;
        }
        active_courses += 1;
        span = span.max(
            starts[pitch_index]
                .checked_add(period(frequency))
                .ok_or(RearticulationError::SampleIndexOverflow)?,
        );
        if patch.detune > 0.0 {
            active_courses += 1;
            let detuned = frequency * 2_f64.powf(patch.detune / 1200.0);
            span = span.max(
                starts[pitch_index]
                    .checked_add(period(detuned))
                    .ok_or(RearticulationError::SampleIndexOverflow)?,
            );
        }
    }
    if active_courses == 0 {
        return Err(RearticulationError::NoActiveCourses);
    }
    Ok((active_courses, span))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("synth".to_string(), "weave".to_string()),
            ("courses".to_string(), "3".to_string()),
            ("strum".to_string(), "0".to_string()),
            ("sus".to_string(), "0.12".to_string()),
        ])
    }

    fn schedule(
        params: &BTreeMap<String, String>,
        attacks: &[AttackFrame],
        duration_seconds: f64,
    ) -> AttackSchedule {
        AttackSchedule::compile(
            params,
            &[220.0],
            (duration_seconds * SR_F64).round() as usize,
            attacks,
        )
        .expect("attack schedule")
    }

    #[test]
    fn one_attack_is_byte_identical_to_the_historical_voice() {
        let params = params();
        let attacks = [AttackFrame {
            at_sample: 0,
            gain: 1.0,
        }];
        let expected =
            StringNetworkVoice::weave(&params, &[220.0], 0.1, 1.0).render();
        let actual = RearticulatedStringVoice::new(
            StringNetworkVoice::weave(&params, &[220.0], 0.1, 1.0),
            schedule(&params, &attacks, 0.1),
        )
        .expect("scheduled voice")
        .render();
        assert_eq!(actual, expected);
    }

    #[test]
    fn controlled_one_attack_is_byte_identical_to_the_traced_executor() {
        let params = params();
        let attacks = [AttackFrame {
            at_sample: 0,
            gain: 1.0,
        }];
        let frames = vec![
            ControlFrame {
                at_sample: 0,
                sets: vec![(ControlTarget::Couple, 0.1)],
            },
            ControlFrame {
                at_sample: 2400,
                sets: vec![(ControlTarget::Couple, 0.3)],
            },
        ];
        let expected = StringNetworkVoice::weave(&params, &[220.0], 0.1, 1.0)
            .render_traced_with(128, &frames)
            .0;
        let actual = RearticulatedStringVoice::new(
            StringNetworkVoice::weave(&params, &[220.0], 0.1, 1.0),
            schedule(&params, &attacks, 0.1),
        )
        .expect("scheduled voice")
        .render_controlled(&frames)
        .expect("controlled render");
        assert_eq!(actual, expected);
    }

    #[test]
    fn second_attack_changes_only_the_audio_at_and_after_its_onset() {
        let params = params();
        let one = [AttackFrame {
            at_sample: 0,
            gain: 1.0,
        }];
        let two = [
            AttackFrame {
                at_sample: 0,
                gain: 1.0,
            },
            AttackFrame {
                at_sample: 2400,
                gain: 1.0,
            },
        ];
        let single = RearticulatedStringVoice::new(
            StringNetworkVoice::weave(&params, &[220.0], 0.1, 1.0),
            schedule(&params, &one, 0.1),
        )
        .expect("single attack")
        .render();
        let repeated = RearticulatedStringVoice::new(
            StringNetworkVoice::weave(&params, &[220.0], 0.1, 1.0),
            schedule(&params, &two, 0.1),
        )
        .expect("two attacks")
        .render();
        assert_eq!(&repeated[..2400], &single[..2400]);
        assert_ne!(&repeated[2400..], &single[2400..]);
    }

    #[test]
    fn overlap_refuses_before_voice_construction() {
        let params = params();
        let error = AttackSchedule::compile(
            &params,
            &[220.0],
            4800,
            &[
                AttackFrame {
                    at_sample: 0,
                    gain: 1.0,
                },
                AttackFrame {
                    at_sample: 100,
                    gain: 1.0,
                },
            ],
        )
        .expect_err("one-period excitation overlaps at 100 samples");
        assert!(matches!(
            error,
            RearticulationError::OverlappingAttacks { .. }
        ));
    }
}
