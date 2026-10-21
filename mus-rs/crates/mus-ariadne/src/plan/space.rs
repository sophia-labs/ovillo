//! Plan IR for a body with an N-dimensional configuration space.
//!
//! This is an additive constitution (`kind: "modal_space"` in
//! `ariadne-world-plan-v3`). It never appears in a v2 plan, and it does not
//! change how any existing constitution serializes, so every existing plan
//! digest is unchanged.

use serde::{Deserialize, Serialize};

use crate::configuration::{ConfigurationMapSpec, ConfigurationSpaceSpec};
use crate::ir::{ReservoirSpec, TransportSpec, ACTIVE_RANKS, MODE_COUNT, TRANSPORT_PAIRS};
use crate::law::aero::AeroSpec;
use crate::law::valve::FlowValveSpec;

use super::helpers::{validate_direction, validate_independent};
use super::PlanError;

/// The local laws of an N-D body.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SpaceBodyLaws {
    /// The legacy modal body's laws (compliant aperture along a fixed source
    /// direction, reservoir-paid active inclusion, ordered transport,
    /// projection radiation), driven from an N-D point. Needs a multilinear or
    /// stiff-string map.
    Modal {
        source_direction: [f64; MODE_COUNT],
        active_directions: [[f64; MODE_COUNT]; ACTIVE_RANKS],
        transport: TransportSpec,
    },
    /// A tube network's laws: a flow valve at one volume and mode-diagonal
    /// radiation at every radiating end. Needs a tube-network map. `aero`
    /// (POC-3) adds the jet's separation loss, turbulence and a DC-correct
    /// wall loss at the network's contact column; absent, the plan
    /// serializes exactly as before and the body is linear acoustics.
    TubeNetwork {
        valve: FlowValveSpec,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        aero: Option<AeroSpec>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpaceBodySpec {
    pub anatomy_name: String,
    pub configuration_space: ConfigurationSpaceSpec,
    pub laws: SpaceBodyLaws,
    pub reservoir: ReservoirSpec,
}

impl SpaceBodySpec {
    pub(crate) fn validate(&self, sample_rate_hz: f64) -> Result<(), PlanError> {
        if self.anatomy_name.trim().is_empty() {
            return Err(PlanError::new("empty-body-anatomy", "body anatomy_name must be nonempty"));
        }
        self.configuration_space.validate(sample_rate_hz)?;
        match (&self.laws, &self.configuration_space.map) {
            (
                SpaceBodyLaws::Modal { source_direction, active_directions, transport },
                ConfigurationMapSpec::Multilinear { .. } | ConfigurationMapSpec::StiffString { .. },
            ) => {
                validate_direction("space body source direction", source_direction)?;
                validate_independent("space body active directions", active_directions)?;
                for pair in 0..TRANSPORT_PAIRS {
                    let (left, right) = transport.pairs[pair];
                    if left == right || left >= MODE_COUNT || right >= MODE_COUNT {
                        return Err(PlanError::new(
                            "invalid-body-transport-pair",
                            format!("transport pair {pair} is outside the {MODE_COUNT}-mode body"),
                        ));
                    }
                    let weight = transport.weights[pair];
                    if !weight.is_finite() || weight.abs() > 16.0 {
                        return Err(PlanError::new(
                            "invalid-body-transport-weight",
                            format!("transport weight {pair} must be finite with abs <= 16"),
                        ));
                    }
                }
            }
            (SpaceBodyLaws::TubeNetwork { valve, aero }, ConfigurationMapSpec::TubeNetwork { network }) => {
                if let Some(aero) = aero {
                    aero.validate().map_err(|message| PlanError::new("invalid-aero", message))?;
                }
                valve
                    .validate()
                    .map_err(|message| PlanError::new("invalid-flow-valve", message))?;
                if valve.tube >= network.tubes.len()
                    || valve.section >= network.tubes[valve.tube].base_log_area_m2.len()
                {
                    return Err(PlanError::new(
                        "invalid-flow-valve-node",
                        "the valve's volume is outside the network",
                    ));
                }
            }
            _ => {
                return Err(PlanError::new(
                    "space-body-law-map-mismatch",
                    "modal laws need a multilinear or stiff-string map; tube-network laws need a tube-network map",
                ))
            }
        }
        if !self.reservoir.capacity.is_finite() || self.reservoir.capacity <= 0.0 {
            return Err(PlanError::new(
                "invalid-body-reservoir",
                "body reservoir capacity must be finite and positive",
            ));
        }
        Ok(())
    }
}
