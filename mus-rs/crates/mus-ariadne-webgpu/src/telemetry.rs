use serde::{Deserialize, Serialize};

/// Stable presentation quality levels. Scientific replay should lock one level
/// rather than use the adaptive governor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum QualityLevel {
    Safe,
    Balanced,
    High,
    Ultra,
}

impl QualityLevel {
    #[must_use]
    pub const fn lower(self) -> Self {
        match self {
            Self::Ultra => Self::High,
            Self::High => Self::Balanced,
            Self::Balanced | Self::Safe => Self::Safe,
        }
    }

    #[must_use]
    pub const fn raise(self) -> Self {
        match self {
            Self::Safe => Self::Balanced,
            Self::Balanced => Self::High,
            Self::High | Self::Ultra => Self::Ultra,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PassTiming {
    pub name: String,
    pub gpu_ms: f64,
}

/// Small aggregate receipt suitable for traces, telemetry, and performance
/// regression gates. It intentionally excludes full field readback.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceReceipt {
    pub frame_index: u64,
    pub quality: QualityLevel,
    pub gpu_ms: Option<f64>,
    pub cpu_encode_us: u64,
    pub upload_bytes: u64,
    pub readback_bytes: u64,
    pub active_fraction: f32,
    pub passes: Vec<PassTiming>,
}

impl PerformanceReceipt {
    #[must_use]
    pub fn empty(frame_index: u64, quality: QualityLevel) -> Self {
        Self {
            frame_index,
            quality,
            gpu_ms: None,
            cpu_encode_us: 0,
            upload_bytes: 0,
            readback_bytes: 0,
            active_fraction: 1.0,
            passes: Vec::new(),
        }
    }
}

/// Hysteretic quality governor for presentation-only lanes.
#[derive(Debug, Clone)]
pub struct QualityGovernor {
    level: QualityLevel,
    budget_ms: f64,
    over_budget_frames: u32,
    under_budget_frames: u32,
    lower_after: u32,
    raise_after: u32,
}

impl QualityGovernor {
    #[must_use]
    pub fn new(level: QualityLevel, budget_ms: f64) -> Self {
        Self {
            level,
            budget_ms: budget_ms.max(0.1),
            over_budget_frames: 0,
            under_budget_frames: 0,
            lower_after: 8,
            raise_after: 240,
        }
    }

    #[must_use]
    pub const fn level(&self) -> QualityLevel {
        self.level
    }

    /// Observe a complete GPU frame. Returns the current level after any
    /// hysteretic transition.
    pub fn observe(&mut self, gpu_ms: f64) -> QualityLevel {
        if !gpu_ms.is_finite() {
            return self.level;
        }
        if gpu_ms > self.budget_ms {
            self.over_budget_frames = self.over_budget_frames.saturating_add(1);
            self.under_budget_frames = 0;
            if self.over_budget_frames >= self.lower_after {
                self.level = self.level.lower();
                self.over_budget_frames = 0;
            }
        } else if gpu_ms < 0.70 * self.budget_ms {
            self.under_budget_frames = self.under_budget_frames.saturating_add(1);
            self.over_budget_frames = 0;
            if self.under_budget_frames >= self.raise_after {
                self.level = self.level.raise();
                self.under_budget_frames = 0;
            }
        } else {
            self.over_budget_frames = 0;
            self.under_budget_frames = 0;
        }
        self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn governor_uses_hysteresis() {
        let mut governor = QualityGovernor::new(QualityLevel::High, 8.0);
        for _ in 0..7 {
            assert_eq!(governor.observe(12.0), QualityLevel::High);
        }
        assert_eq!(governor.observe(12.0), QualityLevel::Balanced);
        for _ in 0..239 {
            assert_eq!(governor.observe(2.0), QualityLevel::Balanced);
        }
        assert_eq!(governor.observe(2.0), QualityLevel::High);
    }
}
