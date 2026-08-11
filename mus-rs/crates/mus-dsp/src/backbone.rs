//! The persistent protected backbone (unified-instrument stage U3).
//!
//! One sixteen-complex-mode conservative core per weave track, surviving
//! every event boundary — their N14 (no fresh body per note) made
//! structural. Region-major state: string 0-3, body 4-7, near 8-11,
//! room 12-15. Compiled complex words arrive **as data**
//! (`mus_watsn::ComplexOperatorProgram`, the gesture-seam doctrine); the
//! backbone schedules their gates across absolute sample time and executes
//! them through the one gate kernel `mus_watsn::apply_gate` — no second
//! executor.
//!
//! Frozen per-sample stage order (`UNIFIED-U3-persistent-backbone.md` §2):
//! transport → bridge → radiate → ledger. Any change is a version change.
//!
//! # The bridge-register law
//!
//! A course wave variable is real; a backbone mode is a complex
//! positive-frequency amplitude. Rotating a real sample directly against
//! one quadrature is exactly the N1 violation, so each coupled course owns
//! a persistent complex **bridge register** whose real part *is* the ring
//! sample while its imaginary part lives in the backbone. Per sample the
//! register loads the ring value, a real beam-splitter rotation (real
//! coefficients acting on complex amplitudes — automatically J-commuting)
//! exchanges it with its string-region channel, and the register's real
//! part is stored back to the ring. The conserved quantity per bridge step
//! is `w² + Im(b)² + |z|²` exactly; the signed backbone-side energy delta
//! is the ledger's injection line, and its sign witnesses bidirectionality
//! (their N3).
//!
//! # Radiation
//!
//! The four room channels radiate through a passive colligation with
//! per-sample energy fraction `eta`: each retains `sqrt(1-eta)` and emits
//! `sqrt(eta) z`; the audible contribution is the sum of the real parts of
//! the outgoing port amplitudes (theory §14.1 — the waveform IS the port
//! amplitude, no separate oscillator bank). Removed energy is the ledger's
//! radiation line. Media/master gain stays outside the ledger.

use mus_watsn::{apply_gate, state_energy, Complex64, ComplexGate, ComplexOperatorProgram};

pub const BACKBONE_MODES: usize = 16;
pub const STRING_CHANNELS: core::ops::Range<usize> = 0..4;
pub const ROOM_CHANNELS: core::ops::Range<usize> = 12..16;

#[derive(Debug, Clone, PartialEq)]
pub enum BackboneError {
    /// Two armed words overlap in sample time — eager rendering cannot
    /// interleave concurrent words over shared state, and a wrong answer
    /// must not be representable (contract refusal BlockScheduleAmbiguous).
    OverlappingWords { previous_end: u64, requested_start: u64 },
    /// A word's dimension does not match the backbone.
    DimensionMismatch { expected: usize, actual: usize },
    /// More bridge courses requested than string-region channels exist.
    TooManyBridges { requested: usize, available: usize },
    /// An empty span cannot schedule a non-empty word.
    EmptySpan,
}

/// Per-event ledger frame (contract §13, reduced to the stages this
/// backbone executes; `control_work` is zero for unitary words and is
/// still *measured*, never asserted).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BackboneLedgerFrame {
    pub energy_before: f64,
    pub energy_after: f64,
    /// Signed backbone-side energy delta through the bridge. Positive =
    /// ring→backbone; negative = backbone→ring (the bidirectionality
    /// witness).
    pub bridge_injected: f64,
    /// Energy removed from room channels by the radiation port.
    pub radiated: f64,
    /// Energy removed by explicit retirement (reset receipts).
    pub retired: f64,
    /// Measured state-energy change across all transport gates — zero to
    /// float residual for lawful words.
    pub transport_work: f64,
}

impl BackboneLedgerFrame {
    pub fn closure_error(&self) -> f64 {
        self.energy_after
            - (self.energy_before + self.bridge_injected + self.transport_work
                - self.radiated
                - self.retired)
    }
}

#[derive(Debug, Clone)]
pub struct ProtectedBackbone {
    state: [Complex64; BACKBONE_MODES],
    /// Persistent imaginary halves of the bridge registers, one per
    /// coupled course (the real halves live in the ring).
    bridge_imag: Vec<f64>,
    /// (absolute_sample, gate), monotone by sample; executed by cursor.
    schedule: Vec<(u64, ComplexGate)>,
    cursor: usize,
    /// End sample of the last armed word, for overlap refusal.
    armed_until: u64,
    /// Beam-splitter angle per sample. Zero is the exact-absence law.
    pub coupling: f64,
    /// Per-sample radiated energy fraction from each room channel.
    pub radiation_eta: f64,
    absolute_sample: u64,
    frame: BackboneLedgerFrame,
}

impl ProtectedBackbone {
    pub fn new(bridge_courses: usize, coupling: f64, radiation_eta: f64) -> Result<Self, BackboneError> {
        if bridge_courses > STRING_CHANNELS.len() {
            return Err(BackboneError::TooManyBridges {
                requested: bridge_courses,
                available: STRING_CHANNELS.len(),
            });
        }
        Ok(Self {
            state: [Complex64::ZERO; BACKBONE_MODES],
            bridge_imag: vec![0.0; bridge_courses],
            schedule: Vec::new(),
            cursor: 0,
            armed_until: 0,
            coupling,
            radiation_eta,
            absolute_sample: 0,
            frame: BackboneLedgerFrame::default(),
        })
    }

    pub fn state(&self) -> &[Complex64; BACKBONE_MODES] {
        &self.state
    }

    pub fn absolute_sample(&self) -> u64 {
        self.absolute_sample
    }

    /// Fixed preparation (theory §13.2): load a state directly, as the
    /// calibrated `U_pre` stage does. Preparation is part of the
    /// instrument, not a test backdoor; it appears in the ledger as
    /// injected energy.
    pub fn prepare(&mut self, state: &[Complex64; BACKBONE_MODES]) {
        let before = state_energy(&self.state);
        self.state = *state;
        self.frame.bridge_injected += state_energy(&self.state) - before;
    }

    /// Explicit state retirement with a receipt — never a side effect.
    pub fn reset(&mut self) -> f64 {
        let retired = state_energy(&self.state) + self.bridge_imag.iter().map(|v| v * v).sum::<f64>();
        self.state = [Complex64::ZERO; BACKBONE_MODES];
        for value in &mut self.bridge_imag {
            *value = 0.0;
        }
        self.frame.retired += retired;
        retired
    }

    /// Arm a validated word: its gates are spread evenly across
    /// `[start_sample, start_sample + span_samples)` in listed order.
    /// Words must be armed in nondecreasing, non-overlapping sample order.
    pub fn arm_word(
        &mut self,
        word: &ComplexOperatorProgram,
        start_sample: u64,
        span_samples: u64,
    ) -> Result<(), BackboneError> {
        if word.dimension() != BACKBONE_MODES {
            return Err(BackboneError::DimensionMismatch {
                expected: BACKBONE_MODES,
                actual: word.dimension(),
            });
        }
        if start_sample < self.armed_until {
            return Err(BackboneError::OverlappingWords {
                previous_end: self.armed_until,
                requested_start: start_sample,
            });
        }
        let gates = word.gates();
        if gates.is_empty() {
            return Ok(());
        }
        if span_samples == 0 {
            return Err(BackboneError::EmptySpan);
        }
        let count = gates.len() as u64;
        for (index, gate) in gates.iter().enumerate() {
            let offset = (index as u64 * span_samples) / count;
            self.schedule.push((start_sample + offset, *gate));
        }
        self.armed_until = start_sample + span_samples;
        Ok(())
    }

    fn run_due_gates(&mut self) {
        while self.cursor < self.schedule.len() && self.schedule[self.cursor].0 <= self.absolute_sample
        {
            let before = state_energy(&self.state);
            let gate = self.schedule[self.cursor].1;
            apply_gate(&mut self.state, &gate);
            self.frame.transport_work += state_energy(&self.state) - before;
            self.cursor += 1;
        }
    }

    fn radiate(&mut self) -> f64 {
        if self.radiation_eta <= 0.0 {
            return 0.0;
        }
        let retain = (1.0 - self.radiation_eta).sqrt();
        let emit = self.radiation_eta.sqrt();
        let mut output = 0.0;
        for channel in ROOM_CHANNELS {
            let amplitude = self.state[channel];
            output += emit * amplitude.re;
            self.frame.radiated += self.radiation_eta * amplitude.abs_squared();
            self.state[channel] = amplitude.scale(retain);
        }
        output
    }

    /// One sample of the frozen stage order while a note is sounding.
    /// `ring_taps` are the coupled course wave variables (mutated in
    /// place — the bridge is bidirectional). Returns the radiated audio
    /// contribution for this sample.
    pub fn step(&mut self, ring_taps: &mut [f64]) -> f64 {
        // 1. transport
        self.run_due_gates();
        // 2. bridge
        if self.coupling != 0.0 {
            let (sin, cos) = self.coupling.sin_cos();
            let count = ring_taps.len().min(self.bridge_imag.len());
            for index in 0..count {
                let register = Complex64::new(ring_taps[index], self.bridge_imag[index]);
                let channel = STRING_CHANNELS.start + index;
                let mode = self.state[channel];
                let backbone_before = mode.abs_squared() + register.im * register.im;
                let new_register = register.scale(cos) - mode.scale(sin);
                let new_mode = register.scale(sin) + mode.scale(cos);
                self.state[channel] = new_mode;
                self.bridge_imag[index] = new_register.im;
                ring_taps[index] = new_register.re;
                self.frame.bridge_injected +=
                    new_mode.abs_squared() + new_register.im * new_register.im - backbone_before;
            }
        }
        // 3. radiate
        let output = self.radiate();
        // 4. ledger position advances with the sample clock
        self.absolute_sample += 1;
        output
    }

    /// Advance through a silent gap: due gates execute, room channels
    /// keep radiating into `out` (the braid rings between notes), the
    /// bridge is quiescent (no ring to couple).
    pub fn advance_gap(&mut self, samples: u64, out: &mut [f32]) {
        for slot in out.iter_mut().take(samples as usize) {
            self.run_due_gates();
            *slot += self.radiate() as f32;
            self.absolute_sample += 1;
        }
        // remaining samples beyond the provided buffer still advance time
        let remaining = (samples as usize).saturating_sub(out.len());
        for _ in 0..remaining {
            self.run_due_gates();
            self.radiate();
            self.absolute_sample += 1;
        }
    }

    /// Snapshot and clear the accumulated ledger frame, stamping the
    /// current energies. `energy_before` must have been captured by the
    /// caller at frame open via [`Self::open_frame`].
    pub fn open_frame(&mut self) {
        self.frame = BackboneLedgerFrame {
            energy_before: state_energy(&self.state)
                + self.bridge_imag.iter().map(|v| v * v).sum::<f64>(),
            ..Default::default()
        };
    }

    pub fn close_frame(&mut self) -> BackboneLedgerFrame {
        self.frame.energy_after =
            state_energy(&self.state) + self.bridge_imag.iter().map(|v| v * v).sum::<f64>();
        self.frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mus_watsn::{ComplexPhaseGate, ComplexTwoModeGate};

    fn beam_splitter_word(left: usize, right: usize, angle: f64) -> ComplexOperatorProgram {
        let (sin, cos) = angle.sin_cos();
        ComplexOperatorProgram::new(
            BACKBONE_MODES,
            vec![ComplexGate::TwoMode(ComplexTwoModeGate {
                left,
                right,
                matrix: [
                    Complex64::new(cos, 0.0),
                    Complex64::new(-sin, 0.0),
                    Complex64::new(sin, 0.0),
                    Complex64::new(cos, 0.0),
                ],
            })],
            None,
            ComplexOperatorProgram::DEFAULT_UNITARITY_TOLERANCE,
        )
        .unwrap()
    }

    #[test]
    fn zero_coupling_never_touches_the_ring_or_emits() {
        let mut backbone = ProtectedBackbone::new(2, 0.0, 0.01).unwrap();
        backbone.open_frame();
        let mut ring = [0.25_f64, -0.5];
        let original = ring;
        let mut emitted = 0.0;
        for _ in 0..512 {
            emitted += backbone.step(&mut ring).abs();
        }
        assert_eq!(ring, original);
        assert_eq!(emitted, 0.0);
        let frame = backbone.close_frame();
        assert_eq!(frame.bridge_injected, 0.0);
        assert!(frame.closure_error().abs() < 1.0e-15);
    }

    #[test]
    fn bridge_is_bidirectional() {
        // ring -> backbone
        let mut backbone = ProtectedBackbone::new(1, 0.05, 0.0).unwrap();
        backbone.open_frame();
        let mut ring = [0.8_f64];
        for _ in 0..64 {
            backbone.step(&mut ring);
        }
        let forward = backbone.close_frame();
        assert!(forward.bridge_injected > 1.0e-4, "{forward:?}");

        // backbone -> ring: a charged string channel excites a silent ring
        let mut backbone = ProtectedBackbone::new(1, 0.05, 0.0).unwrap();
        let mut charged = [Complex64::ZERO; BACKBONE_MODES];
        charged[STRING_CHANNELS.start] = Complex64::new(0.9, 0.1);
        backbone.prepare(&charged);
        backbone.open_frame();
        let mut ring = [0.0_f64];
        let mut ring_energy = 0.0;
        for _ in 0..64 {
            backbone.step(&mut ring);
            ring_energy += ring[0] * ring[0];
        }
        let backward = backbone.close_frame();
        assert!(ring_energy > 1.0e-4);
        assert!(backward.bridge_injected < 0.0, "{backward:?}");
    }

    #[test]
    fn bridge_step_conserves_the_declared_total() {
        let mut backbone = ProtectedBackbone::new(1, 0.11, 0.0).unwrap();
        backbone.open_frame();
        let mut ring = [0.7_f64];
        let mut previous_total = {
            let frame_energy = state_energy(backbone.state()) + backbone.bridge_imag[0].powi(2);
            ring[0] * ring[0] + frame_energy
        };
        for _ in 0..256 {
            backbone.step(&mut ring);
            let frame_energy = state_energy(backbone.state()) + backbone.bridge_imag[0].powi(2);
            let total = ring[0] * ring[0] + frame_energy;
            assert!((total - previous_total).abs() < 1.0e-14);
            previous_total = total;
        }
    }

    #[test]
    fn ledger_closes_under_drive_transport_and_radiation() {
        let mut backbone = ProtectedBackbone::new(2, 0.08, 0.004).unwrap();
        let word = beam_splitter_word(0, 12, 0.6); // string -> room
        backbone.open_frame();
        backbone.arm_word(&word, 32, 16).unwrap();
        let mut ring = [0.5_f64, -0.3];
        for sample in 0..2048 {
            // a decaying synthetic drive so energy actually moves
            let drive = 0.4 * (-(sample as f64) / 400.0).exp();
            ring[0] += drive * ((sample as f64) * 0.11).sin();
            backbone.step(&mut ring);
        }
        let frame = backbone.close_frame();
        assert!(frame.radiated > 1.0e-6, "{frame:?}");
        assert!(
            frame.closure_error().abs() < 1.0e-12,
            "closure {} on {frame:?}",
            frame.closure_error()
        );
        // unitary transport did measurable-zero work
        assert!(frame.transport_work.abs() < 1.0e-13);
    }

    #[test]
    fn words_execute_in_composition_order_across_events() {
        // Two single-gate words on disjoint samples: the composition is
        // gate2 * gate1 regardless of the gap between them.
        let word_a = beam_splitter_word(0, 1, 0.4);
        let word_b = beam_splitter_word(1, 2, -0.7);
        let mut backbone = ProtectedBackbone::new(0, 0.0, 0.0).unwrap();
        let mut prepared = [Complex64::ZERO; BACKBONE_MODES];
        prepared[0] = Complex64::new(1.0, 0.0);
        backbone.prepare(&prepared);
        backbone.arm_word(&word_a, 10, 5).unwrap();
        backbone.arm_word(&word_b, 100, 5).unwrap();
        let mut sink = [0.0_f32; 0];
        backbone.advance_gap(200, &mut sink);

        let mut expected = prepared;
        word_a.apply(&mut expected).unwrap();
        word_b.apply(&mut expected).unwrap();
        for (actual, want) in backbone.state().iter().zip(expected.iter()) {
            assert!((*actual - *want).abs() < 1.0e-15);
        }
    }

    #[test]
    fn overlapping_words_are_refused() {
        let word = beam_splitter_word(0, 1, 0.3);
        let mut backbone = ProtectedBackbone::new(0, 0.0, 0.0).unwrap();
        backbone.arm_word(&word, 0, 100).unwrap();
        let refusal = backbone.arm_word(&word, 50, 100).unwrap_err();
        assert!(matches!(refusal, BackboneError::OverlappingWords { .. }));
    }

    #[test]
    fn reset_is_a_receipted_retirement() {
        let mut backbone = ProtectedBackbone::new(0, 0.0, 0.0).unwrap();
        let mut prepared = [Complex64::ZERO; BACKBONE_MODES];
        prepared[3] = Complex64::new(0.6, -0.8);
        backbone.open_frame();
        backbone.prepare(&prepared);
        let retired = backbone.reset();
        assert!((retired - 1.0).abs() < 1.0e-14);
        let frame = backbone.close_frame();
        assert!(frame.closure_error().abs() < 1.0e-14);
    }

    #[test]
    fn gap_radiation_rings_between_notes() {
        let mut backbone = ProtectedBackbone::new(0, 0.0, 0.01).unwrap();
        let mut charged = [Complex64::ZERO; BACKBONE_MODES];
        charged[ROOM_CHANNELS.start] = Complex64::new(1.0, 0.0);
        backbone.prepare(&charged);
        backbone.open_frame();
        let mut tail = vec![0.0_f32; 512];
        backbone.advance_gap(512, &mut tail);
        let tail_energy: f64 = tail.iter().map(|v| (*v as f64) * (*v as f64)).sum();
        assert!(tail_energy > 1.0e-4, "the braid must ring after note-off");
        // and the emission decays monotonically at the colligation rate
        assert!(tail[0].abs() > tail[511].abs());
        let frame = backbone.close_frame();
        assert!(frame.closure_error().abs() < 1.0e-12);
    }
}
