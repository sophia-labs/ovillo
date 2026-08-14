use super::World;

impl World {
    #[must_use]
    pub fn sample_frames(&self) -> u64 {
        self.sample_frames
    }

    #[must_use]
    pub fn global_closure_residual(&self) -> f64 {
        self.ledger.global_residual()
    }

    #[must_use]
    pub fn cumulative_closure_residual(&self) -> f64 {
        self.ledger.cumulative_residual
    }

    #[must_use]
    pub fn maximum_frame_residual(&self) -> f64 {
        self.ledger.max_abs_residual
    }

    #[must_use]
    pub fn maximum_local_residual(&self) -> f64 {
        self.maximum_local_residual
    }
}
