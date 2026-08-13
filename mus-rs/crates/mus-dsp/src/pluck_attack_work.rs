use sha2::{Digest, Sha256};

const RECEIPTED_ATTACK_EXECUTION_STRATEGY: &str =
    "prevalidated-monotone-attack-cursor+fully-unobserved-course-kernel-v3";
const REARTICULATION_RECEIPT_SCHEMA_VERSION: &str = "string-rearticulation-receipt-v1";
const ATTACK_WORK_LAW: &str =
    "delta_E = wave_before * excitation + 0.5 * excitation^2; ordered before drive, breath, bow, and contact";
const ATTACK_WORK_UNIT: &str = "declared-string-wave-energy";

/// Exact source-work evidence for one physical attack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttackWorkReceipt {
    pub index: usize,
    pub at_sample: usize,
    pub gain: f64,
    pub active_courses: usize,
    pub nonzero_course_samples: usize,
    pub signed_work: f64,
    pub positive_work: f64,
    pub extracted_work: f64,
    pub peak_abs_course_sample_work: f64,
}

/// Render-time receipt for a fixed-topology attack schedule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RearticulationReceipt {
    pub schema_version: String,
    pub work_digest: String,
    pub schedule: AttackScheduleReceipt,
    pub work_law: String,
    pub work_unit: String,
    pub total_signed_work: f64,
    pub total_positive_work: f64,
    pub total_extracted_work: f64,
    pub peak_abs_course_sample_work: f64,
    pub attacks: Vec<AttackWorkReceipt>,
}

/// Audio plus the exact attack-work receipt measured during that render.
pub struct WorkReceiptedRender {
    pub audio: Vec<f32>,
    pub receipt: RearticulationReceipt,
}

#[derive(Debug, Clone, Copy, Default)]
struct CourseAttackWork {
    signed_work: f64,
    positive_work: f64,
    extracted_work: f64,
    peak_abs_course_sample_work: f64,
    nonzero_course_samples: usize,
}

impl CourseAttackWork {
    fn record(&mut self, work: f64) {
        self.signed_work += work;
        if work >= 0.0 {
            self.positive_work += work;
        } else {
            self.extracted_work += -work;
        }
        self.peak_abs_course_sample_work =
            self.peak_abs_course_sample_work.max(work.abs());
        self.nonzero_course_samples += 1;
    }
}

/// The production v3 wrapper around the v2 preallocated attack executor.
///
/// The inner value still owns schedule validation, persistent state, excitation
/// templates, monotone cursors, phase offsets, and normalization invariants. V3
/// adds fixed-size exact work accumulators and an explicit per-course
/// observer-off kernel. No allocation occurs after construction.
pub struct WorkReceiptedStringVoice {
    inner: RearticulatedStringVoice,
    attack_course_work: Vec<CourseAttackWork>,
}

impl WorkReceiptedStringVoice {
    pub fn new(inner: RearticulatedStringVoice) -> Result<Self, RearticulationError> {
        let work_slots = inner
            .schedule
            .frames
            .len()
            .checked_mul(inner.schedule.receipt.active_courses)
            .ok_or(RearticulationError::SampleIndexOverflow)?;
        Ok(Self {
            inner,
            attack_course_work: vec![CourseAttackWork::default(); work_slots],
        })
    }

    pub fn schedule_receipt(&self) -> &AttackScheduleReceipt {
        self.inner.receipt()
    }

    pub fn remaining_samples(&self) -> usize {
        self.inner.remaining_samples()
    }

    fn work_receipt(&self) -> RearticulationReceipt {
        let active_courses = self.inner.schedule.receipt.active_courses;
        let mut attacks = Vec::with_capacity(self.inner.schedule.frames.len());
        let mut total_signed_work = 0.0_f64;
        let mut total_positive_work = 0.0_f64;
        let mut total_extracted_work = 0.0_f64;
        let mut peak_abs_course_sample_work = 0.0_f64;

        for (index, frame) in self.inner.schedule.frames.iter().copied().enumerate() {
            let mut signed_work = 0.0_f64;
            let mut positive_work = 0.0_f64;
            let mut extracted_work = 0.0_f64;
            let mut attack_peak = 0.0_f64;
            let mut nonzero_course_samples = 0_usize;
            let start = index * active_courses;
            for account in &self.attack_course_work[start..start + active_courses] {
                // Course-major reduction deliberately matches EnergyTrace's
                // existing final `inject_burst` reduction order.
                signed_work += account.signed_work;
                positive_work += account.positive_work;
                extracted_work += account.extracted_work;
                attack_peak = attack_peak.max(account.peak_abs_course_sample_work);
                nonzero_course_samples += account.nonzero_course_samples;
            }
            total_signed_work += signed_work;
            total_positive_work += positive_work;
            total_extracted_work += extracted_work;
            peak_abs_course_sample_work = peak_abs_course_sample_work.max(attack_peak);
            attacks.push(AttackWorkReceipt {
                index,
                at_sample: frame.at_sample,
                gain: frame.gain,
                active_courses,
                nonzero_course_samples,
                signed_work,
                positive_work,
                extracted_work,
                peak_abs_course_sample_work: attack_peak,
            });
        }

        let mut schedule = self.inner.schedule.receipt.clone();
        schedule.execution_strategy = RECEIPTED_ATTACK_EXECUTION_STRATEGY.to_string();
        let work_digest = digest_attack_work(&schedule, &attacks);
        RearticulationReceipt {
            schema_version: REARTICULATION_RECEIPT_SCHEMA_VERSION.to_string(),
            work_digest,
            schedule,
            work_law: ATTACK_WORK_LAW.to_string(),
            work_unit: ATTACK_WORK_UNIT.to_string(),
            total_signed_work,
            total_positive_work,
            total_extracted_work,
            peak_abs_course_sample_work,
            attacks,
        }
    }

    /// One allocation-free sample of the work-receipted observer-off kernel.
    pub fn next_sample(&mut self) -> f32 {
        if self.inner.voice.is_finished() || self.inner.voice.strings.is_empty() {
            return 0.0;
        }

        let sample_index = self.inner.voice.sample_index;
        self.inner.update_attack_cursor(sample_index);
        for index in 0..self.inner.voice.strings.len() {
            let wave = self.inner.voice.strings[index].read_unobserved(sample_index);
            self.inner.voice.waves[index] = wave;
            self.inner.voice.incoming[index] = wave;
        }
        if self.inner.voice.mode == StringNetworkMode::Weave {
            self.inner.scatter_weave_unobserved();
        }
        let body_output = self.inner.scatter_body_unobserved();

        // External drive and backbone are refused by the v2 constructor. Keep
        // their exact +0.0 positions so the one-attack zero limit stays a bit
        // law, including signed zero.
        let drive = 0.0_f64;
        let backbone_emitted = 0.0_f64;
        let active_attack = self
            .inner
            .current_attack
            .map(|index| (index, self.inner.schedule.frames[index]));
        let active_normalization = self.inner.active_normalization;
        let network_normalization = self.inner.network_normalization;
        let excitation_templates = &self.inner.excitation_templates;
        let attack_start_offsets = &self.inner.attack_start_offsets;
        let attack_course_work = &mut self.attack_course_work;
        let work_stride = self.inner.schedule.receipt.active_courses;
        let active_count = self.inner.voice.active_count;
        let bow_pressure = self.inner.voice.patch.bow;
        let breath_amount = self.inner.voice.patch.breath;
        let buzz = self.inner.voice.patch.buzz;

        let bowing =
            bow_pressure > 1e-12 && sample_index < self.inner.voice.drive_samples;
        let bow_velocity = BOW_VELOCITY_MIN
            + (BOW_VELOCITY_MAX - BOW_VELOCITY_MIN)
                * self.inner.voice.patch.bow_speed.clamp(0.0, 1.0);
        let voice = &mut self.inner.voice;

        let mut contact_output = 0.0;
        for index in 0..voice.strings.len() {
            let coupling_scale = voice.strings[index].coupling_scale;
            let start_sample = voice.strings[index].start_sample;
            let slot_samples = voice.strings[index].slot_samples;
            let in_slot =
                sample_index >= start_sample && sample_index < start_sample + slot_samples;
            let breath = if in_slot && breath_amount > 1e-12 {
                match voice.breath.get_mut(index) {
                    Some(channel) => breath_amount * 0.25 * coupling_scale * channel.tick(),
                    None => 0.0,
                }
            } else {
                0.0
            };

            let burst = if index < active_count {
                match active_attack.and_then(|(attack_index, frame)| {
                    work_scheduled_burst(
                        frame,
                        &excitation_templates[index],
                        attack_start_offsets[index],
                        sample_index,
                    )
                    .map(|burst| (attack_index, burst))
                }) {
                    Some((attack_index, burst)) => {
                        if burst != 0.0 {
                            let wave_before = voice.waves[index];
                            let work = wave_before * burst + 0.5 * burst * burst;
                            let slot = attack_index * work_stride + index;
                            attack_course_work[slot].record(work);
                        }
                        burst
                    }
                    None => 0.0,
                }
            } else {
                0.0
            };
            let drive_here = drive * coupling_scale;
            let mut excitation = burst + drive_here + breath;
            if bowing && index < active_count {
                let relative_velocity = bow_velocity - voice.waves[index];
                let force =
                    BOW_GAIN * bow_pressure * bow_friction(relative_velocity) * coupling_scale;
                let string = &mut voice.strings[index];
                string.bow_dc += BOW_DC_POLE * (force - string.bow_dc);
                excitation += force - string.bow_dc;
            }
            let value = voice.waves[index] + excitation;
            let (feedback, contact) = voice.strings[index].apply_contact_unobserved(value, buzz);
            voice.contact[index] = contact;
            contact_output += contact;
            voice.strings[index].write(feedback);
        }

        let string_direct = voice
            .incoming
            .iter()
            .take(active_count)
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
        voice.orbit_phase = (voice.orbit_phase
            + 2.0 * PI * voice.patch.orbit_hz / SR_F64)
            .rem_euclid(2.0 * PI);
        voice.sample_index += 1;
        (0.72 * output) as f32
    }

    pub fn render_with_receipt(mut self) -> WorkReceiptedRender {
        let total_samples = self.remaining_samples();
        let mut audio = Vec::with_capacity(total_samples);
        for _ in 0..total_samples {
            audio.push(self.next_sample());
        }
        let receipt = self.work_receipt();
        WorkReceiptedRender { audio, receipt }
    }

    pub fn render_controlled_with_receipt(
        mut self,
        control_frames: &[ControlFrame],
    ) -> Result<WorkReceiptedRender, RearticulationError> {
        validate_work_control_frames(control_frames)?;
        let total_samples = self.remaining_samples();
        let mut audio = Vec::with_capacity(total_samples);
        let mut next_frame = 0_usize;
        for sample_index in 0..total_samples {
            while next_frame < control_frames.len()
                && control_frames[next_frame].at_sample <= sample_index
            {
                for &(target, value) in &control_frames[next_frame].sets {
                    self.inner.voice.apply_control(target, value);
                }
                next_frame += 1;
            }
            audio.push(self.next_sample());
        }
        let receipt = self.work_receipt();
        Ok(WorkReceiptedRender { audio, receipt })
    }
}

fn digest_attack_work(
    schedule: &AttackScheduleReceipt,
    attacks: &[AttackWorkReceipt],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"mus-string-rearticulation-work-v1\0");
    for value in [
        schedule.attack_count as u64,
        schedule.active_courses as u64,
        schedule.attack_span_samples as u64,
        schedule.drive_samples as u64,
        schedule.first_attack_sample as u64,
        schedule.last_attack_sample as u64,
    ] {
        hasher.update(value.to_le_bytes());
    }
    hasher.update(schedule.maximum_gain.to_bits().to_le_bytes());
    for attack in attacks {
        for value in [
            attack.index as u64,
            attack.at_sample as u64,
            attack.active_courses as u64,
            attack.nonzero_course_samples as u64,
        ] {
            hasher.update(value.to_le_bytes());
        }
        for value in [
            attack.gain,
            attack.signed_work,
            attack.positive_work,
            attack.extracted_work,
            attack.peak_abs_course_sample_work,
        ] {
            hasher.update(value.to_bits().to_le_bytes());
        }
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn validate_work_control_frames(
    control_frames: &[ControlFrame],
) -> Result<(), RearticulationError> {
    for (index, pair) in control_frames.windows(2).enumerate() {
        if pair[1].at_sample < pair[0].at_sample {
            return Err(RearticulationError::ControlFramesOutOfOrder {
                index: index + 1,
                previous: pair[0].at_sample,
                current: pair[1].at_sample,
            });
        }
    }
    Ok(())
}

fn work_scheduled_burst(
    attack: AttackFrame,
    template: &[f64],
    course_start_offset: usize,
    sample_index: usize,
) -> Option<f64> {
    let start = attack.at_sample.checked_add(course_start_offset)?;
    let age = sample_index.checked_sub(start)?;
    let &sample = template.get(age)?;
    Some(if attack.gain.to_bits() == 1.0_f64.to_bits() {
        sample
    } else {
        sample * attack.gain
    })
}

/// A no-observer copy of the delay-line hot path. The numerical write order is
/// identical to `DelayString::{update_loop,read}` when `observe=false`; only
/// ledger reads, branches, and writes are removed.
impl DelayString {
    fn update_loop_unobserved(&mut self, sample_index: usize) {
        let frequency = self.instantaneous_frequency(sample_index).max(20.0);
        let omega = 2.0 * PI * frequency / SR_F64;
        let (damping_magnitude, damping_delay) = one_zero_response(self.damp, omega);
        let total_delay = SR_F64 / frequency;
        let mut effective_dispersion = self.dispersion_coefficient;
        let available_dispersion_delay = (total_delay - damping_delay - 2.25).max(0.0);
        for _ in 0..12 {
            let candidate_delay = if effective_dispersion.abs() > 1e-12 {
                2.0 * allpass_phase_delay(effective_dispersion, omega)
            } else {
                0.0
            };
            if candidate_delay <= available_dispersion_delay + 1e-9 {
                break;
            }
            effective_dispersion *= 0.72;
        }
        let retune = self.retune;
        for section in &mut self.dispersion {
            let _ = section.retune_coefficient(effective_dispersion, retune, false);
        }
        let dispersion_delay = if effective_dispersion.abs() > 1e-12 {
            2.0 * allpass_phase_delay(effective_dispersion, omega)
        } else {
            0.0
        };
        let remaining = (total_delay - damping_delay - dispersion_delay).max(2.25);
        let mut integer_delay = (remaining - 0.20).floor().max(2.0) as usize;
        integer_delay = integer_delay.min(self.line.len().saturating_sub(2));
        let mut fractional_delay = remaining - integer_delay as f64;
        if fractional_delay > 1.95 && integer_delay + 1 < self.line.len() {
            integer_delay += 1;
            fractional_delay -= 1.0;
        }
        if fractional_delay < 0.02 && integer_delay > 2 {
            integer_delay -= 1;
            fractional_delay += 1.0;
        }
        self.integer_delay = integer_delay;
        let _ = self.fractional.retune_coefficient(
            allpass_coefficient_for_delay(fractional_delay, omega),
            retune,
            false,
        );
        let target_circulation_gain = 10_f64.powf(-3.0 / (self.t60 * frequency));
        self.loop_gain = (target_circulation_gain / damping_magnitude).min(0.999_999_5);
    }

    fn read_unobserved(&mut self, sample_index: usize) -> f64 {
        if sample_index.is_multiple_of(16) {
            self.update_loop_unobserved(sample_index);
        }
        let read_index =
            (self.write_index + self.line.len() - self.integer_delay) % self.line.len();
        let delayed = self.line[read_index];
        let mut value = (1.0 - self.damp) * delayed + self.damp * self.damping_x1;
        self.damping_x1 = delayed;
        if self.dispersion[0].coefficient.abs() > 1e-12 {
            value = self.dispersion[0].tick(value);
            value = self.dispersion[1].tick(value);
        }
        value = self.fractional.tick(value);
        value *= self.loop_gain;
        self.energy_smooth = 0.999 * self.energy_smooth + 0.001 * value * value;
        value
    }

    fn apply_contact_unobserved(&mut self, value: f64, buzz: f64) -> (f64, f64) {
        if buzz <= 1e-12 {
            return (value, 0.0);
        }
        let clearance = 1.22 - 0.88 * buzz;
        let magnitude = value.abs();
        if magnitude <= clearance {
            self.contact_memory *= 0.72;
            return (value, self.contact_memory * 0.08 * buzz);
        }
        let over = magnitude - clearance;
        let hardness = 1.0 + 11.0 * buzz;
        let compressed_magnitude = clearance + (hardness * over).tanh() / hardness;
        let feedback = value.signum() * compressed_magnitude.min(magnitude);
        let removed = value - feedback;
        let radiation = removed - 0.78 * self.contact_memory;
        self.contact_memory = removed;
        (feedback, radiation)
    }
}

#[cfg(test)]
mod work_tests {
    use super::*;

    fn work_params() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("synth".to_string(), "weave".to_string()),
            ("courses".to_string(), "3".to_string()),
            ("strum".to_string(), "0".to_string()),
            ("sus".to_string(), "0.12".to_string()),
        ])
    }

    fn work_schedule(
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
    fn work_kernel_one_attack_is_byte_identical_to_the_historical_voice() {
        let params = work_params();
        let attacks = [AttackFrame {
            at_sample: 0,
            gain: 1.0,
        }];
        let expected =
            StringNetworkVoice::weave(&params, &[220.0], 0.1, 1.0).render();
        let inner = RearticulatedStringVoice::new(
            StringNetworkVoice::weave(&params, &[220.0], 0.1, 1.0),
            work_schedule(&params, &attacks, 0.1),
        )
        .expect("scheduled voice");
        let actual = WorkReceiptedStringVoice::new(inner)
            .expect("work voice")
            .render_with_receipt();
        assert_eq!(actual.audio, expected);
    }

    #[test]
    fn exact_one_attack_work_matches_the_phase0_observer() {
        let params = work_params();
        let attacks = [AttackFrame {
            at_sample: 0,
            gain: 1.0,
        }];
        let (_, _, observed) = StringNetworkVoice::weave(&params, &[220.0], 0.1, 1.0)
            .render_traced_observed(1, &[]);
        let expected_work = observed.last("inject_burst").expect("final burst work");
        let inner = RearticulatedStringVoice::new(
            StringNetworkVoice::weave(&params, &[220.0], 0.1, 1.0),
            work_schedule(&params, &attacks, 0.1),
        )
        .expect("scheduled voice");
        let rendered = WorkReceiptedStringVoice::new(inner)
            .expect("work voice")
            .render_with_receipt();
        assert_eq!(
            rendered.receipt.total_signed_work.to_bits(),
            expected_work.to_bits(),
            "course-major reduction must reproduce the existing exact observer"
        );
        assert_eq!(rendered.receipt.attacks.len(), 1);
        assert!(rendered.receipt.work_digest.starts_with("sha256:"));
        assert_eq!(rendered.receipt.work_digest.len(), 71);
    }

    #[test]
    fn controlled_work_kernel_is_byte_identical_to_the_traced_executor() {
        let params = work_params();
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
        let inner = RearticulatedStringVoice::new(
            StringNetworkVoice::weave(&params, &[220.0], 0.1, 1.0),
            work_schedule(&params, &attacks, 0.1),
        )
        .expect("scheduled voice");
        let actual = WorkReceiptedStringVoice::new(inner)
            .expect("work voice")
            .render_controlled_with_receipt(&frames)
            .expect("controlled render");
        assert_eq!(actual.audio, expected);
    }

    #[test]
    fn second_attack_changes_only_the_suffix_and_receipts_its_gain() {
        let params = work_params();
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
                gain: 0.5,
            },
        ];
        let single = RearticulatedStringVoice::new(
            StringNetworkVoice::weave(&params, &[220.0], 0.1, 1.0),
            work_schedule(&params, &one, 0.1),
        )
        .expect("single attack")
        .render();
        let inner = RearticulatedStringVoice::new(
            StringNetworkVoice::weave(&params, &[220.0], 0.1, 1.0),
            work_schedule(&params, &two, 0.1),
        )
        .expect("two attacks");
        let repeated = WorkReceiptedStringVoice::new(inner)
            .expect("work voice")
            .render_with_receipt();
        assert_eq!(&repeated.audio[..2400], &single[..2400]);
        assert_ne!(&repeated.audio[2400..], &single[2400..]);
        assert_eq!(repeated.receipt.attacks[1].gain, 0.5);
        assert!(repeated.receipt.attacks[1].nonzero_course_samples > 0);
    }
}
