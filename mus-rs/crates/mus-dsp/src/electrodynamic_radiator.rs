//! A reciprocal electrodynamic radiator cell for Ariadne.
//!
//! This is not a cabinet filter. It is one passive region of the same physical
//! graph as the source material, strings, body, and room. Its state comprises
//! voice-coil flux linkage, cone momentum and displacement, and enclosure
//! volume displacement. Electrical, mechanical, and acoustic coordinates are
//! coupled by exact power-preserving rotations; copper, suspension, enclosure,
//! and radiation losses are explicit dissipative relations.
//!
//! The nonlinear suspension is advanced by a discrete-gradient solve, so its
//! quartic potential exchanges energy with cone momentum rather than clipping
//! an audio sample. The position-dependent force factor changes a reciprocal
//! electromechanical gyrator and therefore cannot create energy. Every sample
//! returns a local receipt intended to reconcile into Ariadne's existing
//! graph-wide ledger.

use crate::power_port::{PowerDomain, PowerPort};
use serde::{Deserialize, Serialize};

const MIN_SAMPLE_RATE: f64 = 1_000.0;
const MAX_SAMPLE_RATE: f64 = 768_000.0;
const SPRING_NEWTON_STEPS: usize = 8;

/// Physical parameters for a moving-coil radiator and sealed acoustic volume.
///
/// The default is a plausible design point for a twelve-inch guitar driver in
/// a medium sealed volume. It is not a measurement of any commercial unit.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ElectrodynamicRadiatorParameters {
    pub sample_rate_hz: f64,
    pub coil_resistance_ohm: f64,
    pub coil_inductance_h: f64,
    pub moving_mass_kg: f64,
    pub suspension_stiffness_n_per_m: f64,
    pub suspension_cubic_n_per_m3: f64,
    pub mechanical_resistance_n_s_per_m: f64,
    pub mechanical_cubic_resistance_n_s3_per_m3: f64,
    pub force_factor_t_m: f64,
    pub force_factor_drop: f64,
    pub force_factor_scale_m: f64,
    pub piston_area_m2: f64,
    pub enclosure_compliance_m5_per_n: f64,
    pub enclosure_leak_m5_per_n_s: f64,
    pub radiation_resistance_n_s_per_m: f64,
}

impl Default for ElectrodynamicRadiatorParameters {
    fn default() -> Self {
        Self {
            sample_rate_hz: 48_000.0,
            coil_resistance_ohm: 6.2,
            coil_inductance_h: 0.000_5,
            moving_mass_kg: 0.025,
            suspension_stiffness_n_per_m: 4_000.0,
            suspension_cubic_n_per_m3: 100_000_000.0,
            mechanical_resistance_n_s_per_m: 1.2,
            mechanical_cubic_resistance_n_s3_per_m3: 2.0,
            force_factor_t_m: 8.0,
            force_factor_drop: 0.35,
            force_factor_scale_m: 0.005,
            piston_area_m2: 0.052,
            enclosure_compliance_m5_per_n: 3.5e-7,
            enclosure_leak_m5_per_n_s: 1.6e-5,
            radiation_resistance_n_s_per_m: 1.5,
        }
    }
}

impl ElectrodynamicRadiatorParameters {
    pub fn validate(self) -> Result<Self, ElectrodynamicRadiatorError> {
        validate_range(
            "sample_rate_hz",
            self.sample_rate_hz,
            MIN_SAMPLE_RATE,
            MAX_SAMPLE_RATE,
        )?;
        validate_nonnegative("coil_resistance_ohm", self.coil_resistance_ohm)?;
        validate_positive("coil_inductance_h", self.coil_inductance_h)?;
        validate_positive("moving_mass_kg", self.moving_mass_kg)?;
        validate_nonnegative(
            "suspension_stiffness_n_per_m",
            self.suspension_stiffness_n_per_m,
        )?;
        validate_nonnegative(
            "suspension_cubic_n_per_m3",
            self.suspension_cubic_n_per_m3,
        )?;
        validate_nonnegative(
            "mechanical_resistance_n_s_per_m",
            self.mechanical_resistance_n_s_per_m,
        )?;
        validate_nonnegative(
            "mechanical_cubic_resistance_n_s3_per_m3",
            self.mechanical_cubic_resistance_n_s3_per_m3,
        )?;
        validate_finite("force_factor_t_m", self.force_factor_t_m)?;
        validate_nonnegative("force_factor_drop", self.force_factor_drop)?;
        validate_positive("force_factor_scale_m", self.force_factor_scale_m)?;
        validate_positive("piston_area_m2", self.piston_area_m2)?;
        validate_positive(
            "enclosure_compliance_m5_per_n",
            self.enclosure_compliance_m5_per_n,
        )?;
        validate_nonnegative(
            "enclosure_leak_m5_per_n_s",
            self.enclosure_leak_m5_per_n_s,
        )?;
        validate_nonnegative(
            "radiation_resistance_n_s_per_m",
            self.radiation_resistance_n_s_per_m,
        )?;
        Ok(self)
    }
}

/// Serializable state coordinates. All four terms participate in stored energy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ElectrodynamicRadiatorState {
    pub flux_linkage_wb_turn: f64,
    pub cone_momentum_kg_m_per_s: f64,
    pub cone_displacement_m: f64,
    pub enclosure_volume_displacement_m3: f64,
}

impl ElectrodynamicRadiatorState {
    fn is_finite(self) -> bool {
        self.flux_linkage_wb_turn.is_finite()
            && self.cone_momentum_kg_m_per_s.is_finite()
            && self.cone_displacement_m.is_finite()
            && self.enclosure_volume_displacement_m3.is_finite()
    }
}

/// Cumulative component receipt. It is deliberately local rather than a
/// competing global ledger.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ElectrodynamicRadiatorReceipt {
    pub frames: u64,
    pub electrical_port_work_j: f64,
    pub copper_loss_j: f64,
    pub mechanical_loss_j: f64,
    pub enclosure_loss_j: f64,
    pub radiated_energy_j: f64,
    pub closure_residual_j: f64,
    pub maximum_abs_closure_residual_j: f64,
    pub stored_energy_j: f64,
    pub peak_abs_current_a: f64,
    pub peak_abs_displacement_m: f64,
    pub peak_abs_volume_velocity_m3_per_s: f64,
    pub maximum_abs_spring_equation_residual: f64,
    pub non_finite_inputs: u64,
    pub non_finite_resets: u64,
}

/// One host-rate observation of the cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElectrodynamicRadiatorFrame {
    /// Electrical port points inward to the radiator.
    pub electrical_port: PowerPort,
    /// Acoustic port also points inward; outgoing radiation has negative power.
    pub radiation_port: PowerPort,
    pub current_a: f64,
    pub cone_velocity_m_per_s: f64,
    pub volume_velocity_m3_per_s: f64,
    /// Average pressure at the piston implied by the resistive radiation load.
    pub piston_pressure_pa: f64,
    pub stored_energy_j: f64,
    pub electrical_port_work_j: f64,
    pub copper_loss_j: f64,
    pub mechanical_loss_j: f64,
    pub enclosure_loss_j: f64,
    pub radiated_energy_j: f64,
    pub closure_residual_j: f64,
    pub spring_equation_residual: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElectrodynamicRadiatorError {
    InvalidParameter(&'static str),
    InvalidState,
}

impl std::fmt::Display for ElectrodynamicRadiatorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidParameter(name) => {
                write!(formatter, "invalid electrodynamic radiator parameter: {name}")
            }
            Self::InvalidState => formatter.write_str("radiator state coordinates must be finite"),
        }
    }
}

impl std::error::Error for ElectrodynamicRadiatorError {}

/// Allocation-free reciprocal moving-coil radiator.
#[derive(Debug, Clone)]
pub struct ElectrodynamicRadiator {
    parameters: ElectrodynamicRadiatorParameters,
    state: ElectrodynamicRadiatorState,
    receipt: ElectrodynamicRadiatorReceipt,
}

impl ElectrodynamicRadiator {
    pub fn try_new(
        parameters: ElectrodynamicRadiatorParameters,
    ) -> Result<Self, ElectrodynamicRadiatorError> {
        Ok(Self {
            parameters: parameters.validate()?,
            state: ElectrodynamicRadiatorState::default(),
            receipt: ElectrodynamicRadiatorReceipt::default(),
        })
    }

    /// Construct the nominal design at an arbitrary supported host rate.
    pub fn nominal(sample_rate_hz: f64) -> Result<Self, ElectrodynamicRadiatorError> {
        Self::try_new(ElectrodynamicRadiatorParameters {
            sample_rate_hz,
            ..ElectrodynamicRadiatorParameters::default()
        })
    }

    #[must_use]
    pub fn parameters(&self) -> ElectrodynamicRadiatorParameters {
        self.parameters
    }

    #[must_use]
    pub fn state(&self) -> ElectrodynamicRadiatorState {
        self.state
    }

    #[must_use]
    pub fn receipt(&self) -> ElectrodynamicRadiatorReceipt {
        self.receipt
    }

    #[must_use]
    pub fn stored_energy_j(&self) -> f64 {
        self.stored_energy_for(self.state)
    }

    pub fn reset(&mut self) {
        self.state = ElectrodynamicRadiatorState::default();
        self.receipt = ElectrodynamicRadiatorReceipt::default();
    }

    pub fn restore_state(
        &mut self,
        state: ElectrodynamicRadiatorState,
    ) -> Result<(), ElectrodynamicRadiatorError> {
        if !state.is_finite() {
            return Err(ElectrodynamicRadiatorError::InvalidState);
        }
        self.state = state;
        self.receipt.stored_energy_j = self.stored_energy_j();
        Ok(())
    }

    /// Advance one host sample from an applied terminal voltage.
    ///
    /// The split source impulses measure work by exact storage-energy change,
    /// equivalent to midpoint current over each half-step. That receipt is the
    /// authoritative discrete port work; `voltage * final_current * dt` is only
    /// an instantaneous observation.
    #[must_use]
    pub fn process_sample(&mut self, terminal_voltage_v: f64) -> ElectrodynamicRadiatorFrame {
        let voltage = if terminal_voltage_v.is_finite() {
            terminal_voltage_v
        } else {
            self.receipt.non_finite_inputs = self.receipt.non_finite_inputs.saturating_add(1);
            0.0
        };
        let dt = 1.0 / self.parameters.sample_rate_hz;
        let half_dt = 0.5 * dt;
        let old_energy = self.stored_energy_j();

        let mut port_work = self.apply_voltage_impulse(voltage, half_dt);
        let mut copper_loss = self.damp_flux(half_dt);
        let mut enclosure_loss = self.damp_enclosure(half_dt);
        let mut mechanical_loss = self.damp_mechanical(half_dt);
        let mut radiated_energy = self.damp_radiation(half_dt);

        self.rotate_electromechanical(half_dt);
        self.rotate_enclosure(half_dt);
        let spring_equation_residual = self.advance_suspension(dt);
        self.rotate_enclosure(half_dt);
        self.rotate_electromechanical(half_dt);

        radiated_energy += self.damp_radiation(half_dt);
        mechanical_loss += self.damp_mechanical(half_dt);
        enclosure_loss += self.damp_enclosure(half_dt);
        copper_loss += self.damp_flux(half_dt);
        port_work += self.apply_voltage_impulse(voltage, half_dt);

        if !self.state.is_finite() {
            self.state = ElectrodynamicRadiatorState::default();
            self.receipt.non_finite_resets = self.receipt.non_finite_resets.saturating_add(1);
            return self.zero_frame(voltage);
        }

        let stored_energy = self.stored_energy_j();
        let closure_residual = old_energy + port_work
            - copper_loss
            - mechanical_loss
            - enclosure_loss
            - radiated_energy
            - stored_energy;
        let current = self.current_a();
        let velocity = self.cone_velocity_m_per_s();
        let volume_velocity = self.parameters.piston_area_m2 * velocity;
        let piston_pressure =
            self.parameters.radiation_resistance_n_s_per_m * velocity
                / self.parameters.piston_area_m2;

        self.receipt.frames = self.receipt.frames.saturating_add(1);
        self.receipt.electrical_port_work_j += port_work;
        self.receipt.copper_loss_j += copper_loss;
        self.receipt.mechanical_loss_j += mechanical_loss;
        self.receipt.enclosure_loss_j += enclosure_loss;
        self.receipt.radiated_energy_j += radiated_energy;
        self.receipt.closure_residual_j += closure_residual;
        self.receipt.maximum_abs_closure_residual_j = self
            .receipt
            .maximum_abs_closure_residual_j
            .max(closure_residual.abs());
        self.receipt.stored_energy_j = stored_energy;
        self.receipt.peak_abs_current_a = self.receipt.peak_abs_current_a.max(current.abs());
        self.receipt.peak_abs_displacement_m = self
            .receipt
            .peak_abs_displacement_m
            .max(self.state.cone_displacement_m.abs());
        self.receipt.peak_abs_volume_velocity_m3_per_s = self
            .receipt
            .peak_abs_volume_velocity_m3_per_s
            .max(volume_velocity.abs());
        self.receipt.maximum_abs_spring_equation_residual = self
            .receipt
            .maximum_abs_spring_equation_residual
            .max(spring_equation_residual.abs());

        ElectrodynamicRadiatorFrame {
            electrical_port: PowerPort {
                domain: PowerDomain::Electrical,
                effort: voltage,
                flow: current,
            },
            radiation_port: PowerPort {
                domain: PowerDomain::Acoustic,
                effort: piston_pressure,
                flow: -volume_velocity,
            },
            current_a: current,
            cone_velocity_m_per_s: velocity,
            volume_velocity_m3_per_s: volume_velocity,
            piston_pressure_pa: piston_pressure,
            stored_energy_j: stored_energy,
            electrical_port_work_j: port_work,
            copper_loss_j: copper_loss,
            mechanical_loss_j: mechanical_loss,
            enclosure_loss_j: enclosure_loss,
            radiated_energy_j: radiated_energy,
            closure_residual_j: closure_residual,
            spring_equation_residual,
        }
    }

    fn zero_frame(&self, voltage: f64) -> ElectrodynamicRadiatorFrame {
        ElectrodynamicRadiatorFrame {
            electrical_port: PowerPort {
                domain: PowerDomain::Electrical,
                effort: voltage,
                flow: 0.0,
            },
            radiation_port: PowerPort {
                domain: PowerDomain::Acoustic,
                effort: 0.0,
                flow: 0.0,
            },
            current_a: 0.0,
            cone_velocity_m_per_s: 0.0,
            volume_velocity_m3_per_s: 0.0,
            piston_pressure_pa: 0.0,
            stored_energy_j: 0.0,
            electrical_port_work_j: 0.0,
            copper_loss_j: 0.0,
            mechanical_loss_j: 0.0,
            enclosure_loss_j: 0.0,
            radiated_energy_j: 0.0,
            closure_residual_j: 0.0,
            spring_equation_residual: 0.0,
        }
    }

    fn stored_energy_for(&self, state: ElectrodynamicRadiatorState) -> f64 {
        let electrical = state.flux_linkage_wb_turn * state.flux_linkage_wb_turn
            / (2.0 * self.parameters.coil_inductance_h);
        let kinetic = state.cone_momentum_kg_m_per_s * state.cone_momentum_kg_m_per_s
            / (2.0 * self.parameters.moving_mass_kg);
        let displacement_squared = state.cone_displacement_m * state.cone_displacement_m;
        let suspension = 0.5
            * self.parameters.suspension_stiffness_n_per_m
            * displacement_squared
            + 0.25
                * self.parameters.suspension_cubic_n_per_m3
                * displacement_squared
                * displacement_squared;
        let enclosure = state.enclosure_volume_displacement_m3
            * state.enclosure_volume_displacement_m3
            / (2.0 * self.parameters.enclosure_compliance_m5_per_n);
        electrical + kinetic + suspension + enclosure
    }

    fn current_a(&self) -> f64 {
        self.state.flux_linkage_wb_turn / self.parameters.coil_inductance_h
    }

    fn cone_velocity_m_per_s(&self) -> f64 {
        self.state.cone_momentum_kg_m_per_s / self.parameters.moving_mass_kg
    }

    fn force_factor_t_m(&self) -> f64 {
        let normalized = self.state.cone_displacement_m / self.parameters.force_factor_scale_m;
        self.parameters.force_factor_t_m
            * (-self.parameters.force_factor_drop * normalized * normalized).exp()
    }

    fn apply_voltage_impulse(&mut self, voltage: f64, duration: f64) -> f64 {
        let before = self.state.flux_linkage_wb_turn * self.state.flux_linkage_wb_turn
            / (2.0 * self.parameters.coil_inductance_h);
        self.state.flux_linkage_wb_turn += voltage * duration;
        let after = self.state.flux_linkage_wb_turn * self.state.flux_linkage_wb_turn
            / (2.0 * self.parameters.coil_inductance_h);
        after - before
    }

    fn damp_flux(&mut self, duration: f64) -> f64 {
        let before = self.state.flux_linkage_wb_turn * self.state.flux_linkage_wb_turn
            / (2.0 * self.parameters.coil_inductance_h);
        let rate = self.parameters.coil_resistance_ohm / self.parameters.coil_inductance_h;
        self.state.flux_linkage_wb_turn *= (-rate * duration).exp();
        let after = self.state.flux_linkage_wb_turn * self.state.flux_linkage_wb_turn
            / (2.0 * self.parameters.coil_inductance_h);
        before - after
    }

    fn damp_enclosure(&mut self, duration: f64) -> f64 {
        let before = self.state.enclosure_volume_displacement_m3
            * self.state.enclosure_volume_displacement_m3
            / (2.0 * self.parameters.enclosure_compliance_m5_per_n);
        let rate = self.parameters.enclosure_leak_m5_per_n_s
            / self.parameters.enclosure_compliance_m5_per_n;
        self.state.enclosure_volume_displacement_m3 *= (-rate * duration).exp();
        let after = self.state.enclosure_volume_displacement_m3
            * self.state.enclosure_volume_displacement_m3
            / (2.0 * self.parameters.enclosure_compliance_m5_per_n);
        before - after
    }

    fn damp_radiation(&mut self, duration: f64) -> f64 {
        self.damp_momentum_linear(duration, self.parameters.radiation_resistance_n_s_per_m)
    }

    fn damp_mechanical(&mut self, duration: f64) -> f64 {
        let momentum = self.state.cone_momentum_kg_m_per_s;
        if momentum == 0.0 {
            return 0.0;
        }
        let mass = self.parameters.moving_mass_kg;
        let before = momentum * momentum / (2.0 * mass);
        let linear_rate = self.parameters.mechanical_resistance_n_s_per_m / mass;
        let cubic_rate = self.parameters.mechanical_cubic_resistance_n_s3_per_m3
            / (mass * mass * mass);

        let next = if linear_rate > 0.0 {
            let decay = (-linear_rate * duration).exp();
            let decay_squared = decay * decay;
            let denominator = 1.0
                + (cubic_rate / linear_rate)
                    * momentum
                    * momentum
                    * (1.0 - decay_squared);
            momentum * decay / denominator.sqrt()
        } else if cubic_rate > 0.0 {
            momentum / (1.0 + 2.0 * cubic_rate * momentum * momentum * duration).sqrt()
        } else {
            momentum
        };
        self.state.cone_momentum_kg_m_per_s = next;
        before - next * next / (2.0 * mass)
    }

    fn damp_momentum_linear(&mut self, duration: f64, resistance: f64) -> f64 {
        let mass = self.parameters.moving_mass_kg;
        let before = self.state.cone_momentum_kg_m_per_s
            * self.state.cone_momentum_kg_m_per_s
            / (2.0 * mass);
        self.state.cone_momentum_kg_m_per_s *= (-resistance / mass * duration).exp();
        let after = self.state.cone_momentum_kg_m_per_s
            * self.state.cone_momentum_kg_m_per_s
            / (2.0 * mass);
        before - after
    }

    fn rotate_electromechanical(&mut self, duration: f64) {
        let root_inductance = self.parameters.coil_inductance_h.sqrt();
        let root_mass = self.parameters.moving_mass_kg.sqrt();
        let electrical = self.state.flux_linkage_wb_turn / root_inductance;
        let mechanical = self.state.cone_momentum_kg_m_per_s / root_mass;
        let angle = self.force_factor_t_m() * duration / (root_inductance * root_mass);
        let cosine = angle.cos();
        let sine = angle.sin();
        self.state.flux_linkage_wb_turn =
            (cosine * electrical - sine * mechanical) * root_inductance;
        self.state.cone_momentum_kg_m_per_s =
            (sine * electrical + cosine * mechanical) * root_mass;
    }

    fn rotate_enclosure(&mut self, duration: f64) {
        let root_mass = self.parameters.moving_mass_kg.sqrt();
        let root_compliance = self.parameters.enclosure_compliance_m5_per_n.sqrt();
        let mechanical = self.state.cone_momentum_kg_m_per_s / root_mass;
        let acoustic = self.state.enclosure_volume_displacement_m3 / root_compliance;
        let angle = self.parameters.piston_area_m2 * duration / (root_mass * root_compliance);
        let cosine = angle.cos();
        let sine = angle.sin();
        self.state.cone_momentum_kg_m_per_s =
            (cosine * mechanical - sine * acoustic) * root_mass;
        self.state.enclosure_volume_displacement_m3 =
            (sine * mechanical + cosine * acoustic) * root_compliance;
    }

    /// Exact discrete-gradient exchange for quadratic-plus-quartic suspension.
    fn advance_suspension(&mut self, duration: f64) -> f64 {
        let old_position = self.state.cone_displacement_m;
        let old_momentum = self.state.cone_momentum_kg_m_per_s;
        let mass = self.parameters.moving_mass_kg;
        let linear = self.parameters.suspension_stiffness_n_per_m;
        let cubic = self.parameters.suspension_cubic_n_per_m3;
        let mut next_position = old_position + duration * old_momentum / mass;

        for _ in 0..SPRING_NEWTON_STEPS {
            let sum = next_position + old_position;
            let divided_gradient = 0.5 * linear * sum
                + 0.25
                    * cubic
                    * sum
                    * (next_position * next_position + old_position * old_position);
            let equation = 2.0 * mass * (next_position - old_position) / duration
                - 2.0 * old_momentum
                + duration * divided_gradient;
            let derivative = 2.0 * mass / duration
                + duration
                    * (0.5 * linear
                        + 0.25
                            * cubic
                            * (3.0 * next_position * next_position
                                + 2.0 * old_position * next_position
                                + old_position * old_position));
            next_position -= equation / derivative;
        }

        self.state.cone_displacement_m = next_position;
        self.state.cone_momentum_kg_m_per_s =
            2.0 * mass * (next_position - old_position) / duration - old_momentum;

        let sum = next_position + old_position;
        let divided_gradient = 0.5 * linear * sum
            + 0.25
                * cubic
                * sum
                * (next_position * next_position + old_position * old_position);
        2.0 * mass * (next_position - old_position) / duration
            - 2.0 * old_momentum
            + duration * divided_gradient
    }
}

fn validate_finite(name: &'static str, value: f64) -> Result<(), ElectrodynamicRadiatorError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ElectrodynamicRadiatorError::InvalidParameter(name))
    }
}

fn validate_positive(name: &'static str, value: f64) -> Result<(), ElectrodynamicRadiatorError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(ElectrodynamicRadiatorError::InvalidParameter(name))
    }
}

fn validate_nonnegative(
    name: &'static str,
    value: f64,
) -> Result<(), ElectrodynamicRadiatorError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(ElectrodynamicRadiatorError::InvalidParameter(name))
    }
}

fn validate_range(
    name: &'static str,
    value: f64,
    minimum: f64,
    maximum: f64,
) -> Result<(), ElectrodynamicRadiatorError> {
    if value.is_finite() && (minimum..=maximum).contains(&value) {
        Ok(())
    } else {
        Err(ElectrodynamicRadiatorError::InvalidParameter(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    #[test]
    fn nominal_parameters_are_valid() {
        assert!(ElectrodynamicRadiator::nominal(48_000.0).is_ok());
        assert!(ElectrodynamicRadiator::nominal(0.0).is_err());
    }

    #[test]
    fn zero_state_cannot_create_energy() {
        let mut radiator = ElectrodynamicRadiator::nominal(48_000.0).unwrap();
        for _ in 0..4_096 {
            let frame = radiator.process_sample(0.0);
            assert!(frame.stored_energy_j.abs() < 1.0e-24);
            assert!(frame.closure_residual_j.abs() < 1.0e-24);
        }
    }

    #[test]
    fn driven_cell_closes_its_energy_ledger() {
        let mut radiator = ElectrodynamicRadiator::nominal(48_000.0).unwrap();
        for frame_index in 0..24_000 {
            let time = frame_index as f64 / 48_000.0;
            let voltage = 2.0 * (TAU * 110.0 * time).sin()
                + 0.3 * (TAU * 1_777.0 * time).sin();
            let frame = radiator.process_sample(voltage);
            assert!(frame.closure_residual_j.abs() < 1.0e-12);
            assert!(frame.copper_loss_j >= -1.0e-18);
            assert!(frame.mechanical_loss_j >= -1.0e-18);
            assert!(frame.enclosure_loss_j >= -1.0e-18);
            assert!(frame.radiated_energy_j >= -1.0e-18);
        }
        let receipt = radiator.receipt();
        assert!(receipt.maximum_abs_closure_residual_j < 1.0e-12);
        assert!(receipt.stored_energy_j > 0.0);
    }

    #[test]
    fn unforced_stored_energy_is_monotone_nonincreasing() {
        let mut radiator = ElectrodynamicRadiator::nominal(48_000.0).unwrap();
        for frame_index in 0..128 {
            let voltage = if frame_index < 16 { 8.0 } else { 0.0 };
            let _ = radiator.process_sample(voltage);
        }
        let mut previous = radiator.stored_energy_j();
        assert!(previous > 0.0);
        for _ in 0..8_192 {
            let frame = radiator.process_sample(0.0);
            assert!(frame.stored_energy_j <= previous + 1.0e-14);
            previous = frame.stored_energy_j;
        }
    }

    #[test]
    fn moving_cone_returns_energy_as_back_emf() {
        let mut radiator = ElectrodynamicRadiator::nominal(48_000.0).unwrap();
        radiator
            .restore_state(ElectrodynamicRadiatorState {
                cone_momentum_kg_m_per_s: 0.01,
                ..ElectrodynamicRadiatorState::default()
            })
            .unwrap();
        let frame = radiator.process_sample(0.0);
        assert!(frame.current_a.abs() > 1.0e-9);
    }

    #[test]
    fn outgoing_acoustic_port_has_negative_inward_power() {
        let mut radiator = ElectrodynamicRadiator::nominal(48_000.0).unwrap();
        let mut observed = false;
        for _ in 0..256 {
            let frame = radiator.process_sample(4.0);
            if frame.volume_velocity_m3_per_s.abs() > 1.0e-12 {
                observed = true;
                assert!(frame.radiation_port.power() <= 1.0e-18);
            }
        }
        assert!(observed);
    }

    #[test]
    fn non_finite_input_does_not_poison_state() {
        let mut radiator = ElectrodynamicRadiator::nominal(48_000.0).unwrap();
        let frame = radiator.process_sample(f64::NAN);
        assert!(frame.stored_energy_j.is_finite());
        assert_eq!(radiator.receipt().non_finite_inputs, 1);
        assert_eq!(radiator.receipt().non_finite_resets, 0);
    }

    #[test]
    fn invalid_dissipation_is_rejected() {
        let parameters = ElectrodynamicRadiatorParameters {
            radiation_resistance_n_s_per_m: -1.0,
            ..ElectrodynamicRadiatorParameters::default()
        };
        assert!(ElectrodynamicRadiator::try_new(parameters).is_err());
    }
}
