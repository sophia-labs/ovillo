use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Integer-rational scheduler from Ariadne material ticks to a fixed optical
/// constitutive cadence.
///
/// Scheduling and completion are deliberately separate. A caller may request a
/// distant material tick, encode only a bounded batch of optical steps, and
/// continue in a later presentation frame. Failed GPU encoding therefore never
/// advances the completed optical clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpticalClock {
    material_rate_hz: u32,
    optical_rate_hz: u32,
    completed_steps: u64,
    target_steps: u64,
    last_material_tick: u64,
    maximum_steps_per_batch: u32,
}

impl OpticalClock {
    pub fn new(
        material_rate_hz: u32,
        optical_rate_hz: u32,
        maximum_steps_per_batch: u32,
    ) -> Result<Self, OpticalClockError> {
        if material_rate_hz == 0 || optical_rate_hz == 0 {
            return Err(OpticalClockError::ZeroRate);
        }
        if optical_rate_hz > material_rate_hz {
            return Err(OpticalClockError::OpticalRateExceedsMaterial {
                material_rate_hz,
                optical_rate_hz,
            });
        }
        if maximum_steps_per_batch == 0 {
            return Err(OpticalClockError::ZeroBatchCapacity);
        }
        Ok(Self {
            material_rate_hz,
            optical_rate_hz,
            completed_steps: 0,
            target_steps: 0,
            last_material_tick: 0,
            maximum_steps_per_batch,
        })
    }

    #[must_use]
    pub fn fixed_dt_seconds(&self) -> f32 {
        1.0 / self.optical_rate_hz as f32
    }

    #[must_use]
    pub const fn material_rate_hz(&self) -> u32 {
        self.material_rate_hz
    }

    #[must_use]
    pub const fn optical_rate_hz(&self) -> u32 {
        self.optical_rate_hz
    }

    #[must_use]
    pub const fn completed_steps(&self) -> u64 {
        self.completed_steps
    }

    #[must_use]
    pub const fn target_steps(&self) -> u64 {
        self.target_steps
    }

    #[must_use]
    pub const fn last_material_tick(&self) -> u64 {
        self.last_material_tick
    }

    #[must_use]
    pub const fn pending_steps(&self) -> u64 {
        self.target_steps.saturating_sub(self.completed_steps)
    }

    #[must_use]
    pub const fn is_caught_up(&self) -> bool {
        self.completed_steps == self.target_steps
    }

    #[must_use]
    pub fn steps_due_in_batch(&self) -> u32 {
        self.pending_steps()
            .min(u64::from(self.maximum_steps_per_batch)) as u32
    }

    #[must_use]
    pub fn next_step_number(&self) -> Option<u64> {
        (!self.is_caught_up()).then_some(self.completed_steps.saturating_add(1))
    }

    #[must_use]
    pub fn next_step_tick(&self) -> Option<u64> {
        self.next_step_number()
            .map(|step| self.material_tick_for_step(step))
    }

    pub fn reset(&mut self) {
        self.completed_steps = 0;
        self.target_steps = 0;
        self.last_material_tick = 0;
    }

    /// Request progress toward one absolute material tick. The returned value
    /// is the bounded number of steps the caller may encode in its next batch.
    /// Repeating the same target is valid while a long seek is being replayed.
    pub fn schedule_to_tick(&mut self, material_tick: u64) -> Result<u32, OpticalClockError> {
        if material_tick < self.last_material_tick {
            return Err(OpticalClockError::MovedBackward {
                previous: self.last_material_tick,
                next: material_tick,
            });
        }
        self.target_steps = self.steps_at_material_tick(material_tick);
        self.last_material_tick = material_tick;
        Ok(self.steps_due_in_batch())
    }

    /// Commit exactly one successfully encoded optical step.
    pub fn complete_next_step(
        &mut self,
        observation_tick: u64,
    ) -> Result<u64, OpticalClockError> {
        let Some(step) = self.next_step_number() else {
            return Err(OpticalClockError::NoScheduledStep);
        };
        let expected_tick = self.material_tick_for_step(step);
        if observation_tick != expected_tick {
            return Err(OpticalClockError::UnexpectedObservationTick {
                step,
                expected: expected_tick,
                actual: observation_tick,
            });
        }
        self.completed_steps = step;
        Ok(step)
    }

    #[must_use]
    pub fn material_tick_for_step(&self, step: u64) -> u64 {
        (((step as u128) * (self.material_rate_hz as u128)
            + (self.optical_rate_hz as u128 - 1))
            / self.optical_rate_hz as u128)
            .min(u64::MAX as u128) as u64
    }

    #[must_use]
    fn steps_at_material_tick(&self, material_tick: u64) -> u64 {
        ((material_tick as u128) * (self.optical_rate_hz as u128)
            / (self.material_rate_hz as u128))
            .min(u64::MAX as u128) as u64
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OpticalClockError {
    #[error("material and optical rates must be positive")]
    ZeroRate,
    #[error("optical rate {optical_rate_hz} exceeds material rate {material_rate_hz}")]
    OpticalRateExceedsMaterial {
        material_rate_hz: u32,
        optical_rate_hz: u32,
    },
    #[error("optical batch capacity must be positive")]
    ZeroBatchCapacity,
    #[error("optical clock moved backward from material tick {previous} to {next}")]
    MovedBackward { previous: u64, next: u64 },
    #[error("no optical step is currently scheduled")]
    NoScheduledStep,
    #[error(
        "optical step {step} requires observation tick {expected}, received {actual}"
    )]
    UnexpectedObservationTick {
        step: u64,
        expected: u64,
        actual: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_batch(clock: &mut OpticalClock) -> u32 {
        let due = clock.steps_due_in_batch();
        for _ in 0..due {
            let tick = clock.next_step_tick().unwrap();
            clock.complete_next_step(tick).unwrap();
        }
        due
    }

    #[test]
    fn optical_clock_requires_distinct_monotone_material_ticks() {
        assert!(matches!(
            OpticalClock::new(120, 121, 8),
            Err(OpticalClockError::OpticalRateExceedsMaterial { .. })
        ));
    }

    #[test]
    fn presentation_grouping_does_not_change_completed_steps() {
        let mut single = OpticalClock::new(48_000, 120, 1_000).unwrap();
        assert_eq!(single.schedule_to_tick(48_000).unwrap(), 120);
        assert_eq!(complete_batch(&mut single), 120);

        let mut segmented = OpticalClock::new(48_000, 120, 1_000).unwrap();
        let mut completed = 0;
        for tick in (400..=48_000).step_by(400) {
            segmented.schedule_to_tick(tick).unwrap();
            completed += complete_batch(&mut segmented);
        }
        assert_eq!(completed, 120);
        assert_eq!(single.completed_steps(), segmented.completed_steps());
        assert!(single.is_caught_up() && segmented.is_caught_up());
    }

    #[test]
    fn long_seek_is_replayed_in_bounded_batches() {
        let mut clock = OpticalClock::new(48_000, 120, 32).unwrap();
        assert_eq!(clock.schedule_to_tick(48_000).unwrap(), 32);
        assert_eq!(clock.pending_steps(), 120);
        assert_eq!(complete_batch(&mut clock), 32);
        assert_eq!(clock.schedule_to_tick(48_000).unwrap(), 32);
        while !clock.is_caught_up() {
            complete_batch(&mut clock);
        }
        assert_eq!(clock.completed_steps(), 120);
    }

    #[test]
    fn completion_is_transactional_and_tick_exact() {
        let mut clock = OpticalClock::new(44_100, 144, 8).unwrap();
        clock.schedule_to_tick(44_100).unwrap();
        let expected = clock.next_step_tick().unwrap();
        assert!(matches!(
            clock.complete_next_step(expected + 1),
            Err(OpticalClockError::UnexpectedObservationTick { .. })
        ));
        assert_eq!(clock.completed_steps(), 0);
        assert_eq!(clock.complete_next_step(expected).unwrap(), 1);
    }

    #[test]
    fn non_divisible_rates_do_not_drift() {
        let mut clock = OpticalClock::new(44_100, 144, 1_000).unwrap();
        clock.schedule_to_tick(44_100).unwrap();
        while !clock.is_caught_up() {
            complete_batch(&mut clock);
        }
        assert_eq!(clock.completed_steps(), 144);
        assert_eq!(clock.material_tick_for_step(144), 44_100);
    }
}
