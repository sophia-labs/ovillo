//! Exact finite-rank radiation from a declared positive metric.
//!
//! Rows are expressed in metric-normalized coordinates. A loading radiation
//! step removes exactly the storage carried by the emitted amplitudes. The same
//! aperture may be used as a non-loading projection control, but that operation
//! books no radiation and leaves the state unchanged.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const METRIC_RADIATION_SCHEMA_VERSION: &str = "ariadne-metric-radiation-v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricRadiationPlan {
    pub schema_version: String,
    pub id: String,
    pub fraction: f64,
    /// Row-orthonormal aperture coordinates in `sqrt(metric) * state` space.
    pub rows: Vec<Vec<f64>>,
}

impl MetricRadiationPlan {
    pub fn validate(&self, dimension: usize) -> Result<(), MetricRadiationError> {
        if self.schema_version != METRIC_RADIATION_SCHEMA_VERSION {
            return Err(MetricRadiationError::InvalidPlan(format!(
                "expected schema {METRIC_RADIATION_SCHEMA_VERSION}, found {}",
                self.schema_version
            )));
        }
        if self.id.trim().is_empty() {
            return Err(MetricRadiationError::InvalidPlan(
                "radiation aperture id must be nonempty".to_owned(),
            ));
        }
        if !self.fraction.is_finite() || !(0.0..=1.0).contains(&self.fraction) {
            return Err(MetricRadiationError::InvalidPlan(
                "radiation fraction must be finite in [0,1]".to_owned(),
            ));
        }
        if dimension == 0 || self.rows.is_empty() || self.rows.len() > dimension {
            return Err(MetricRadiationError::InvalidPlan(
                "radiation rank must be in [1, state dimension]".to_owned(),
            ));
        }
        for (row_index, row) in self.rows.iter().enumerate() {
            if row.len() != dimension || row.iter().any(|value| !value.is_finite()) {
                return Err(MetricRadiationError::InvalidPlan(format!(
                    "radiation row {row_index} has invalid dimension or values"
                )));
            }
        }
        let tolerance = 1.0e-9 * dimension.max(1) as f64;
        for (left_index, left_row) in self.rows.iter().enumerate() {
            for (right_index, right_row) in self.rows.iter().enumerate() {
                let row_dot = dot(left_row, right_row);
                let target = if left_index == right_index { 1.0 } else { 0.0 };
                if (row_dot - target).abs() > tolerance {
                    return Err(MetricRadiationError::InvalidPlan(format!(
                        "radiation rows are not orthonormal at ({left_index},{right_index}); residual={}",
                        row_dot - target
                    )));
                }
            }
        }
        Ok(())
    }

    pub fn digest(&self, dimension: usize) -> Result<String, MetricRadiationError> {
        self.validate(dimension)?;
        let encoded = serde_json::to_vec(self)
            .map_err(|error| MetricRadiationError::Serialization(error.to_string()))?;
        Ok(format!("{:x}", Sha256::digest(encoded)))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricRadiationReceipt {
    pub aperture_id: String,
    pub aperture_digest: String,
    pub loading: bool,
    pub emitted: Vec<f64>,
    pub storage_before: f64,
    pub storage_after: f64,
    pub projected_intensity: f64,
    pub booked_radiation: f64,
    pub residual: f64,
}

#[derive(Debug, Error)]
pub enum MetricRadiationError {
    #[error("invalid metric-radiation plan: {0}")]
    InvalidPlan(String),
    #[error("metric-radiation state and metric dimensions do not agree")]
    DimensionMismatch,
    #[error("metric entries must be finite and positive and state must be finite")]
    InvalidState,
    #[error("metric-radiation serialization failed: {0}")]
    Serialization(String),
}

#[derive(Debug, Clone)]
pub struct MetricRadiationAperture {
    plan: MetricRadiationPlan,
    dimension: usize,
    digest: String,
}

impl MetricRadiationAperture {
    pub fn from_plan(
        plan: MetricRadiationPlan,
        dimension: usize,
    ) -> Result<Self, MetricRadiationError> {
        plan.validate(dimension)?;
        let digest = plan.digest(dimension)?;
        Ok(Self {
            plan,
            dimension,
            digest,
        })
    }

    #[must_use]
    pub fn plan(&self) -> &MetricRadiationPlan {
        &self.plan
    }

    #[must_use]
    pub fn rank(&self) -> usize {
        self.plan.rows.len()
    }

    #[must_use]
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn observe(
        &self,
        state: &[f64],
        metric: &[f64],
    ) -> Result<MetricRadiationReceipt, MetricRadiationError> {
        let mut copy = state.to_vec();
        self.apply(&mut copy, metric, false)
    }

    pub fn radiate(
        &self,
        state: &mut [f64],
        metric: &[f64],
    ) -> Result<MetricRadiationReceipt, MetricRadiationError> {
        self.apply(state, metric, true)
    }

    fn apply(
        &self,
        state: &mut [f64],
        metric: &[f64],
        loading: bool,
    ) -> Result<MetricRadiationReceipt, MetricRadiationError> {
        validate_state_metric(state, metric, self.dimension)?;
        let mut normalized = metric
            .iter()
            .zip(state.iter())
            .map(|(metric, state)| metric.sqrt() * state)
            .collect::<Vec<_>>();
        let storage_before = 0.5 * dot(&normalized, &normalized);
        let coefficients = self
            .plan
            .rows
            .iter()
            .map(|row| dot(row, &normalized))
            .collect::<Vec<_>>();
        let emitted_scale = self.plan.fraction.sqrt();
        let emitted = coefficients
            .iter()
            .map(|coefficient| emitted_scale * coefficient)
            .collect::<Vec<_>>();
        let projected_intensity = 0.5 * dot(&emitted, &emitted);

        let booked_radiation = if loading {
            let removal = 1.0 - (1.0 - self.plan.fraction).sqrt();
            let projected = (0..self.dimension)
                .map(|index| {
                    self.plan
                        .rows
                        .iter()
                        .zip(coefficients.iter())
                        .map(|(row, coefficient)| row[index] * coefficient)
                        .sum::<f64>()
                })
                .collect::<Vec<_>>();
            for (((state, normalized), metric), projection) in state
                .iter_mut()
                .zip(normalized.iter_mut())
                .zip(metric.iter())
                .zip(projected.iter())
            {
                *normalized -= removal * projection;
                *state = *normalized / metric.sqrt();
            }
            projected_intensity
        } else {
            0.0
        };
        let storage_after = if loading {
            0.5 * dot(&normalized, &normalized)
        } else {
            storage_before
        };
        Ok(MetricRadiationReceipt {
            aperture_id: self.plan.id.clone(),
            aperture_digest: self.digest.clone(),
            loading,
            emitted,
            storage_before,
            storage_after,
            projected_intensity,
            booked_radiation,
            residual: storage_after - storage_before + booked_radiation,
        })
    }
}

fn validate_state_metric(
    state: &[f64],
    metric: &[f64],
    dimension: usize,
) -> Result<(), MetricRadiationError> {
    if state.len() != dimension || metric.len() != dimension {
        return Err(MetricRadiationError::DimensionMismatch);
    }
    if state.iter().any(|value| !value.is_finite())
        || metric
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(MetricRadiationError::InvalidState);
    }
    Ok(())
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| left * right)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aperture(fraction: f64) -> MetricRadiationAperture {
        MetricRadiationAperture::from_plan(
            MetricRadiationPlan {
                schema_version: METRIC_RADIATION_SCHEMA_VERSION.to_owned(),
                id: "test-aperture".to_owned(),
                fraction,
                rows: vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]],
            },
            3,
        )
        .unwrap()
    }

    #[test]
    fn zero_is_an_exact_fixed_point() {
        let aperture = aperture(0.4);
        let metric = [1.0, 2.0, 3.0];
        let mut state = [0.0; 3];
        let receipt = aperture.radiate(&mut state, &metric).unwrap();
        assert_eq!(state, [0.0; 3]);
        assert_eq!(receipt.emitted, vec![0.0, 0.0]);
        assert_eq!(receipt.booked_radiation, 0.0);
        assert_eq!(receipt.residual, 0.0);
    }

    #[test]
    fn rank_two_radiation_closes_the_metric_ledger() {
        let aperture = aperture(0.37);
        let metric = [0.75, 1.5, 2.25];
        let mut state = [0.9, -0.4, 1.2];
        let receipt = aperture.radiate(&mut state, &metric).unwrap();
        assert!(receipt.booked_radiation > 0.0);
        assert!(
            (receipt.storage_before - receipt.storage_after - receipt.booked_radiation).abs()
                < 1.0e-14
        );
        assert!(receipt.residual.abs() < 1.0e-14);
        // The third metric-normalized coordinate lies outside the aperture.
        assert_eq!(state[2].to_bits(), 1.2_f64.to_bits());
    }

    #[test]
    fn non_loading_observation_does_not_change_state_or_book_radiation() {
        let aperture = aperture(0.5);
        let metric = [1.0, 2.0, 3.0];
        let state = [1.0, -0.5, 0.25];
        let receipt = aperture.observe(&state, &metric).unwrap();
        assert!(!receipt.loading);
        assert!(receipt.projected_intensity > 0.0);
        assert_eq!(receipt.booked_radiation, 0.0);
        assert_eq!(
            receipt.storage_before.to_bits(),
            receipt.storage_after.to_bits()
        );
        assert_eq!(receipt.residual, 0.0);
    }

    #[test]
    fn full_fraction_removes_the_aperture_subspace() {
        let aperture = aperture(1.0);
        let metric = [1.0, 4.0, 9.0];
        let mut state = [2.0, -3.0, 5.0];
        let receipt = aperture.radiate(&mut state, &metric).unwrap();
        assert!(state[0].abs() < 1.0e-15);
        assert!(state[1].abs() < 1.0e-15);
        assert_eq!(state[2].to_bits(), 5.0_f64.to_bits());
        assert!(receipt.residual.abs() < 1.0e-13);
    }

    #[test]
    fn nonorthogonal_rows_refuse() {
        let result = MetricRadiationAperture::from_plan(
            MetricRadiationPlan {
                schema_version: METRIC_RADIATION_SCHEMA_VERSION.to_owned(),
                id: "bad".to_owned(),
                fraction: 0.25,
                rows: vec![vec![1.0, 0.0], vec![1.0, 0.0]],
            },
            2,
        );
        assert!(matches!(result, Err(MetricRadiationError::InvalidPlan(_))));
    }
}
