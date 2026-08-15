//! Sparse reciprocal coupling graph between persistent Ariadne bodies.
//!
//! Couplings act on every constitution's fixed-size boundary momentum. Modal
//! bodies expose their native momentum; local media expose the storage-bearing
//! boundary transducer defined by `LocalMediumBoundarySpec`.

use std::collections::BTreeMap;
use std::f64::consts::TAU;

use mus_watsn::givens;

use crate::body::Body;
use crate::constitutive_body::WorldBody;
use crate::ir::MODE_COUNT;
use crate::plan::{ConstitutiveWorldPlan, DirectCouplingSpec, PlanError, WorldPlan};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CouplingControls {
    pub scale: f64,
}

impl Default for CouplingControls {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}

impl CouplingControls {
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            scale: finite(self.scale).clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CouplingFrame {
    pub exchange_into_left: f64,
    pub metric_residual: f64,
    pub maximum_abs_angle: f64,
    pub active_edges: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CouplingError {
    Plan(String),
    UnknownBody(String),
    InvalidDirection(String),
    RepeatedBody(String),
    BodyCountMismatch { expected: usize, actual: usize },
    NonFiniteState,
}

impl std::fmt::Display for CouplingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plan(message) => write!(formatter, "invalid coupling plan: {message}"),
            Self::UnknownBody(id) => write!(formatter, "coupling references unknown body {id:?}"),
            Self::InvalidDirection(id) => {
                write!(formatter, "coupling {id:?} has a degenerate aperture direction")
            }
            Self::RepeatedBody(id) => write!(formatter, "coupling {id:?} joins a body to itself"),
            Self::BodyCountMismatch { expected, actual } => write!(
                formatter,
                "coupling network expected {expected} bodies, received {actual}"
            ),
            Self::NonFiniteState => formatter.write_str("coupling encountered non-finite body state"),
        }
    }
}

impl std::error::Error for CouplingError {}

impl From<PlanError> for CouplingError {
    fn from(error: PlanError) -> Self {
        Self::Plan(error.to_string())
    }
}

#[derive(Debug, Clone)]
struct CompiledDirectCoupling {
    id: String,
    left_body: usize,
    right_body: usize,
    left_direction: [f64; MODE_COUNT],
    right_direction: [f64; MODE_COUNT],
    rate_hz: f64,
    weight: f64,
}

#[derive(Debug, Clone, Default)]
pub struct CouplingNetwork {
    body_count: usize,
    edges: Vec<CompiledDirectCoupling>,
    last_exchanges_into_left: Vec<f64>,
}

impl CouplingNetwork {
    pub fn compile(plan: &WorldPlan) -> Result<Self, CouplingError> {
        plan.validate()?;
        Self::compile_parts(
            plan.bodies.iter().map(|body| body.id.as_str()),
            &plan.direct_couplings,
        )
    }

    pub fn compile_constitutive(plan: &ConstitutiveWorldPlan) -> Result<Self, CouplingError> {
        plan.validate()?;
        Self::compile_parts(
            plan.bodies.iter().map(|body| body.id.as_str()),
            &plan.direct_couplings,
        )
    }

    fn compile_parts<'a>(
        body_ids: impl Iterator<Item = &'a str>,
        specs: &[DirectCouplingSpec],
    ) -> Result<Self, CouplingError> {
        let body_indices = body_ids
            .enumerate()
            .map(|(index, id)| (id, index))
            .collect::<BTreeMap<_, _>>();
        let mut edges = Vec::with_capacity(specs.len());
        for spec in specs {
            edges.push(compile_edge(spec, &body_indices)?);
        }
        Ok(Self {
            body_count: body_indices.len(),
            last_exchanges_into_left: vec![0.0; edges.len()],
            edges,
        })
    }

    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    #[must_use]
    pub fn edge_id(&self, index: usize) -> Option<&str> {
        self.edges.get(index).map(|edge| edge.id.as_str())
    }

    #[must_use]
    pub fn last_exchange_into_left(&self, index: usize) -> Option<f64> {
        self.last_exchanges_into_left.get(index).copied()
    }

    #[must_use]
    pub fn last_exchanges_into_left(&self) -> &[f64] {
        &self.last_exchanges_into_left
    }

    /// Legacy public coupling surface for callers that operate directly on
    /// authored modal bodies.
    pub fn process(
        &mut self,
        sample_rate_hz: f64,
        bodies: &mut [Body],
        controls: CouplingControls,
    ) -> Result<CouplingFrame, CouplingError> {
        self.process_impl(sample_rate_hz, bodies, controls)
    }

    pub(crate) fn process_constitutive(
        &mut self,
        sample_rate_hz: f64,
        bodies: &mut [WorldBody],
        controls: CouplingControls,
    ) -> Result<CouplingFrame, CouplingError> {
        self.process_impl(sample_rate_hz, bodies, controls)
    }

    fn process_impl<T: CoupledBody>(
        &mut self,
        sample_rate_hz: f64,
        bodies: &mut [T],
        controls: CouplingControls,
    ) -> Result<CouplingFrame, CouplingError> {
        if bodies.len() != self.body_count {
            return Err(CouplingError::BodyCountMismatch {
                expected: self.body_count,
                actual: bodies.len(),
            });
        }
        self.last_exchanges_into_left.fill(0.0);
        let controls = controls.sanitized();
        if controls.scale == 0.0 || self.edges.is_empty() {
            return Ok(CouplingFrame::default());
        }
        let dt = sample_rate_hz.recip();
        let mut frame = CouplingFrame::default();
        for (index, edge) in self.edges.iter().enumerate() {
            let (left_body, right_body) = two_mut(bodies, edge.left_body, edge.right_body)
                .ok_or_else(|| CouplingError::RepeatedBody(edge.id.clone()))?;
            let left_projection = dot(left_body.coupled_momentum(), &edge.left_direction);
            let right_projection = dot(right_body.coupled_momentum(), &edge.right_direction);
            if !left_projection.is_finite() || !right_projection.is_finite() {
                return Err(CouplingError::NonFiniteState);
            }
            let angle = TAU * edge.rate_hz * dt * edge.weight * controls.scale;
            let (next_left, next_right) = givens(left_projection, right_projection, angle);
            add_projection(
                left_body.coupled_momentum_mut(),
                &edge.left_direction,
                next_left - left_projection,
            );
            add_projection(
                right_body.coupled_momentum_mut(),
                &edge.right_direction,
                next_right - right_projection,
            );
            let left_exchange = 0.5 * dt * (next_left * next_left - left_projection * left_projection);
            let residual = 0.5
                * dt
                * (next_left * next_left + next_right * next_right
                    - left_projection * left_projection
                    - right_projection * right_projection);
            self.last_exchanges_into_left[index] = left_exchange;
            frame.exchange_into_left += left_exchange;
            frame.metric_residual += residual;
            frame.maximum_abs_angle = frame.maximum_abs_angle.max(angle.abs());
            if angle != 0.0 {
                frame.active_edges += 1;
            }
        }
        Ok(frame)
    }
}

trait CoupledBody {
    fn coupled_momentum(&self) -> &[f64; MODE_COUNT];
    fn coupled_momentum_mut(&mut self) -> &mut [f64; MODE_COUNT];
}

impl CoupledBody for Body {
    fn coupled_momentum(&self) -> &[f64; MODE_COUNT] {
        self.momentum()
    }

    fn coupled_momentum_mut(&mut self) -> &mut [f64; MODE_COUNT] {
        self.momentum_mut()
    }
}

impl CoupledBody for WorldBody {
    fn coupled_momentum(&self) -> &[f64; MODE_COUNT] {
        self.momentum()
    }

    fn coupled_momentum_mut(&mut self) -> &mut [f64; MODE_COUNT] {
        self.momentum_mut()
    }
}

fn compile_edge(
    spec: &DirectCouplingSpec,
    body_indices: &BTreeMap<&str, usize>,
) -> Result<CompiledDirectCoupling, CouplingError> {
    let left_body = body_indices
        .get(spec.left_body.as_str())
        .copied()
        .ok_or_else(|| CouplingError::UnknownBody(spec.left_body.clone()))?;
    let right_body = body_indices
        .get(spec.right_body.as_str())
        .copied()
        .ok_or_else(|| CouplingError::UnknownBody(spec.right_body.clone()))?;
    if left_body == right_body {
        return Err(CouplingError::RepeatedBody(spec.id.clone()));
    }
    Ok(CompiledDirectCoupling {
        id: spec.id.clone(),
        left_body,
        right_body,
        left_direction: normalize(spec.left_direction)
            .ok_or_else(|| CouplingError::InvalidDirection(spec.id.clone()))?,
        right_direction: normalize(spec.right_direction)
            .ok_or_else(|| CouplingError::InvalidDirection(spec.id.clone()))?,
        rate_hz: spec.rate_hz,
        weight: spec.weight,
    })
}

fn two_mut<T>(slice: &mut [T], left: usize, right: usize) -> Option<(&mut T, &mut T)> {
    if left == right || left >= slice.len() || right >= slice.len() {
        return None;
    }
    if left < right {
        let (before, after) = slice.split_at_mut(right);
        Some((&mut before[left], &mut after[0]))
    } else {
        let (before, after) = slice.split_at_mut(left);
        Some((&mut after[0], &mut before[right]))
    }
}

fn normalize<const N: usize>(mut direction: [f64; N]) -> Option<[f64; N]> {
    let norm = direction.iter().map(|value| value * value).sum::<f64>().sqrt();
    if !norm.is_finite() || norm <= 1.0e-12 {
        return None;
    }
    for value in &mut direction {
        *value /= norm;
    }
    Some(direction)
}

#[inline]
fn dot<const N: usize>(left: &[f64; N], right: &[f64; N]) -> f64 {
    left.iter().zip(right).map(|(left, right)| left * right).sum()
}

#[inline]
fn add_projection<const N: usize>(state: &mut [f64; N], direction: &[f64; N], amount: f64) {
    for (value, basis) in state.iter_mut().zip(direction) {
        *value += amount * basis;
    }
}

#[inline]
fn finite(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BodyControls;
    use crate::plan::{BodyInstanceSpec, DirectCouplingSpec, WorldPlan};

    fn coupled_plan() -> WorldPlan {
        let mut plan = WorldPlan::string_brass_v1(96_000.0);
        let mut second: BodyInstanceSpec = plan.bodies[0].clone();
        second.id = "body/1".to_owned();
        plan.bodies.push(second);
        let direction = std::array::from_fn(|index| if index == 0 { 1.0 } else { 0.0 });
        plan.direct_couplings.push(DirectCouplingSpec {
            id: "bridge/0".to_owned(),
            left_body: "body/0".to_owned(),
            right_body: "body/1".to_owned(),
            left_direction: direction,
            right_direction: direction,
            rate_hz: 400.0,
            weight: 1.0,
        });
        plan
    }

    #[test]
    fn direct_exchange_is_metric_compatible_and_observable() {
        let plan = coupled_plan();
        let mut network = CouplingNetwork::compile(&plan).unwrap();
        let mut bodies = plan
            .bodies
            .iter()
            .map(|instance| {
                Body::try_new(plan.internal_sample_rate_hz, instance.body.clone()).unwrap()
            })
            .collect::<Vec<_>>();
        bodies[0]
            .process_local(
                BodyControls {
                    source_wave: 0.8,
                    ..Default::default()
                },
                0.0,
            )
            .unwrap();
        let energy_before = bodies.iter().map(Body::total_stored_energy).sum::<f64>();
        let frame = network
            .process(
                plan.internal_sample_rate_hz,
                &mut bodies,
                CouplingControls::default(),
            )
            .unwrap();
        let energy_after = bodies.iter().map(Body::total_stored_energy).sum::<f64>();
        assert!(frame.active_edges > 0);
        assert_eq!(network.edge_id(0), Some("bridge/0"));
        assert_eq!(
            network.last_exchange_into_left(0),
            Some(frame.exchange_into_left)
        );
        assert!(frame.metric_residual.abs() < 1.0e-17);
        assert!((energy_after - energy_before).abs() < 1.0e-17);
    }
}
