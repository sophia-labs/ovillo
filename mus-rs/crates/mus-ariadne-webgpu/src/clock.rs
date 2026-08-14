use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Integer-rational scheduler from Ariadne material ticks to a fixed optical
/// constitutive cadence. Presentation frames may group calls arbitrarily; the
/// number and identity of optical steps are unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpticalClock {
    material_rate_hz: u32,
    optical_rate_hz: u32,
    completed_steps: u64,
    last_material_tick: u64,
    maximum_catch_up_steps: u32,
}

impl OpticalClock {
    pub fn new(
        material_rate_hz: u32,
        optical_rate_hz: u32,
        maximum_catch_up_steps: u32,
    ) -> Result<Self, OpticalClockError> {
        if material_rate_hz == 0 || optical_rate_hz == 0 {
            return Err(OpticalClockError::ZeroRate);
        }
        if maximum_catch_up_steps == 0 {
            return Err(OpticalClockError::ZeroCatchUpCapacity);
        }
        Ok(Self {
            material_rate_hz,
            optical_rate_hz,
            completed_steps: 0,
            last_material_tick: 0,
            maximum_catch_up_steps,
        })
    }

    #[must_use]
    pub fn fixed_dt_seconds(&self) -> f32 {
        1.0 / self.optical_rate_hz as f32
    }

    #[must_use]
    pub const fn completed_steps(&self) -> u64 {
        self.completed_steps
    }

    #[must_use]
    pub const fn last_material_tick(&self) -> u64 {
        self.last_material_tick
    }

    pub fn reset(&mut self) {
        self.completed_steps = 0;
        self.last_material_tick = 0;
    }

    /// Advance scheduling to one absolute material tick and return the number
    /// of fixed optical steps now due. No floating-point time accumulation is
    /// used.
    pub fn advance_to_tick(&mut self, material_tick: u64) -> Result<u32, OpticalClockError> {
        if material_tick < self.last_material_tick {
            return Err(OpticalClockError::MovedBackward {
                previous: self.last_material_tick,
                next: material_tick,
            });
        }
        let target_steps = ((material_tick as u128) * (self.optical_rate_hz as u128)
            / (self.material_rate_hz as u128))
            .min(u64::MAX as u128) as u64;
        let due = target_steps.saturating_sub(self.completed_steps);
        if due > self.maximum_catch_up_steps as u64 {
            return Err(OpticalClockError::CatchUpOverflow {
                due,
                capacity: self.maximum_catch_up_steps,
            });
        }
        self.completed_steps = target_steps;
        self.last_material_tick = material_tick;
        Ok(due as u32)
    }

    #[must_use]
    pub fn material_tick_for_step(&self, step: u64) -> u64 {
        (((step as u128) * (self.material_rate_hz as u128)
            + (self.optical_rate_hz as u128 - 1))
            / self.optical_rate_hz as u128)
            .min(u64::MAX as u128) as u64
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OpticalClockError {
    #[error("material and optical rates must be positive")]
    ZeroRate,
    #[error("optical catch-up capacity must be positive")]
    ZeroCatchUpCapacity,
    #[error("optical clock moved backward from material tick {previous} to {next}")]
    MovedBackward { previous: u64, next: u64 },
    #[error("{due} optical steps are due, exceeding catch-up capacity {capacity}")]
    CatchUpOverflow { due: u64, capacity: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_grouping_does_not_change_step_count() {
        let mut single = OpticalClock::new(48_000, 120, 1_000).unwrap();
        let single_steps = single.advance_to_tick(48_000).unwrap();

        let mut segmented = OpticalClock::new(48_000, 120, 1_000).unwrap();
        let mut segmented_steps = 0;
        for tick in (400..=48_000).step_by(400) {
            segmented_steps += segmented.advance_to_tick(tick).unwrap();
        }
        assert_eq!(single_steps, 120);
        assert_eq!(segmented_steps, single_steps);
        assert_eq!(single.completed_steps(), segmented.completed_steps());
    }

    #[test]
    fn non_divisible_rates_do_not_drift() {
        let mut clock = OpticalClock::new(44_100, 144, 1_000).unwrap();
        assert_eq!(clock.advance_to_tick(44_100).unwrap(), 144);
        assert_eq!(clock.material_tick_for_step(144), 44_100);
    }
}
