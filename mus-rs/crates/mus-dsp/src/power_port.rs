//! Domain-explicit effort/flow ports for Ariadne's physical graph.
//!
//! Ariadne already treats acoustic and mechanical state as one energetic
//! space. This module makes the same statement executable across domains:
//! every boundary is an inward-oriented effort/flow pair, so instantaneous
//! power is always `effort * flow`. Electrical voltage/current, translational
//! force/velocity, acoustic pressure/volume velocity, and magnetic
//! magnetomotive-force/flux-rate ports therefore share one metric without
//! pretending that their coordinates have the same units.
//!
//! The wave normalization is power-normalized:
//!
//! ```text
//! a = (e + R f) / (2 sqrt(R))
//! b = (e - R f) / (2 sqrt(R))
//! e f = a^2 - b^2
//! ```
//!
//! Lossless transformer, gyrator, and orthogonal scattering operations are
//! supplied as the small, auditable junction vocabulary from which pickups,
//! transformers, motors, bridges, and radiators can be assembled. They do
//! not own a second energy ledger; callers reconcile their port powers in the
//! existing Ariadne receipt graph.

use serde::{Deserialize, Serialize};

/// The physical domain whose conjugate coordinates meet at a power port.
///
/// `Magnetic` uses magnetomotive force and flux rate. Flux itself is a storage
/// coordinate, not a flow variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PowerDomain {
    Normalized,
    Electrical,
    MechanicalTranslation,
    Acoustic,
    Magnetic,
}

/// An inward-oriented pair of power-conjugate variables.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PowerPort {
    pub domain: PowerDomain,
    pub effort: f64,
    pub flow: f64,
}

impl PowerPort {
    /// Construct a finite effort/flow port.
    pub fn try_new(
        domain: PowerDomain,
        effort: f64,
        flow: f64,
    ) -> Result<Self, PowerPortError> {
        if !effort.is_finite() || !flow.is_finite() {
            return Err(PowerPortError::NonFiniteCoordinate);
        }
        Ok(Self {
            domain,
            effort,
            flow,
        })
    }

    /// Instantaneous power entering the component through this port.
    #[must_use]
    pub fn power(self) -> f64 {
        self.effort * self.flow
    }

    /// Convert to power-normalized incident/reflected waves.
    pub fn to_wave(self, resistance: f64) -> Result<PowerWave, PowerPortError> {
        PowerWave::from_port(self, resistance)
    }
}

/// Power-normalized wave coordinates at one port.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PowerWave {
    pub domain: PowerDomain,
    pub incident: f64,
    pub reflected: f64,
    pub resistance: f64,
}

impl PowerWave {
    /// Convert an effort/flow port using a positive reference resistance.
    pub fn from_port(port: PowerPort, resistance: f64) -> Result<Self, PowerPortError> {
        validate_positive_finite(resistance, PowerPortError::InvalidResistance)?;
        let root = resistance.sqrt();
        Ok(Self {
            domain: port.domain,
            incident: (port.effort + resistance * port.flow) / (2.0 * root),
            reflected: (port.effort - resistance * port.flow) / (2.0 * root),
            resistance,
        })
    }

    /// Recover effort and inward flow exactly up to floating-point roundoff.
    #[must_use]
    pub fn to_port(self) -> PowerPort {
        let root = self.resistance.sqrt();
        PowerPort {
            domain: self.domain,
            effort: root * (self.incident + self.reflected),
            flow: (self.incident - self.reflected) / root,
        }
    }

    /// Net power entering the component represented by this wave port.
    #[must_use]
    pub fn net_power(self) -> f64 {
        self.incident.mul_add(self.incident, -self.reflected * self.reflected)
    }
}

/// A two-port, power-preserving transformer relation.
///
/// Port orientations point inward on both sides, so the returned secondary
/// port has the opposite flow sign and `primary.power() + secondary.power()`
/// is zero. `ratio` is secondary effort divided by primary effort.
pub fn ideal_transformer_secondary(
    primary: PowerPort,
    secondary_domain: PowerDomain,
    ratio: f64,
) -> Result<PowerPort, PowerPortError> {
    validate_nonzero_finite(ratio, PowerPortError::InvalidRatio)?;
    PowerPort::try_new(
        secondary_domain,
        ratio * primary.effort,
        -primary.flow / ratio,
    )
}

/// A two-port, power-preserving gyrator relation.
///
/// Gyrators are the natural junction for reciprocal cross-domain conversion:
/// an electrodynamic motor, for example, maps electrical current to force and
/// mechanical velocity to back electromotive force. Both ports point inward.
pub fn ideal_gyrator_secondary(
    primary: PowerPort,
    secondary_domain: PowerDomain,
    gyration_resistance: f64,
) -> Result<PowerPort, PowerPortError> {
    validate_nonzero_finite(
        gyration_resistance,
        PowerPortError::InvalidGyrationResistance,
    )?;
    PowerPort::try_new(
        secondary_domain,
        gyration_resistance * primary.flow,
        -primary.effort / gyration_resistance,
    )
}

/// A fixed lossless two-port scattering junction.
///
/// The matrix is a proper rotation and therefore preserves the squared wave
/// norm for every input. A graph compiler can compose these small rotations
/// into larger orthogonal junctions without introducing hidden gain.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LosslessRotation2 {
    cosine: f64,
    sine: f64,
}

impl LosslessRotation2 {
    #[must_use]
    pub fn from_angle(angle_radians: f64) -> Self {
        let angle = if angle_radians.is_finite() {
            angle_radians
        } else {
            0.0
        };
        Self {
            cosine: angle.cos(),
            sine: angle.sin(),
        }
    }

    /// Scatter two incident waves into two reflected waves.
    #[must_use]
    pub fn scatter(self, incident: [f64; 2]) -> [f64; 2] {
        [
            self.cosine
                .mul_add(incident[0], -self.sine * incident[1]),
            self.sine
                .mul_add(incident[0], self.cosine * incident[1]),
        ]
    }
}

/// A passive one-port wave termination.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PassiveTermination {
    reflection: f64,
}

impl PassiveTermination {
    pub fn try_new(reflection: f64) -> Result<Self, PowerPortError> {
        if !reflection.is_finite() || reflection.abs() > 1.0 {
            return Err(PowerPortError::NonPassiveReflection);
        }
        Ok(Self { reflection })
    }

    /// Return the reflected wave and nonnegative absorbed power.
    #[must_use]
    pub fn terminate(self, incident: f64) -> TerminationFrame {
        let reflected = self.reflection * incident;
        TerminationFrame {
            reflected,
            absorbed_power: incident.mul_add(incident, -reflected * reflected),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct TerminationFrame {
    pub reflected: f64,
    pub absorbed_power: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerPortError {
    NonFiniteCoordinate,
    InvalidResistance,
    InvalidRatio,
    InvalidGyrationResistance,
    NonPassiveReflection,
}

impl std::fmt::Display for PowerPortError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::NonFiniteCoordinate => "power-port coordinates must be finite",
            Self::InvalidResistance => "wave resistance must be finite and positive",
            Self::InvalidRatio => "transformer ratio must be finite and nonzero",
            Self::InvalidGyrationResistance => {
                "gyration resistance must be finite and nonzero"
            }
            Self::NonPassiveReflection => {
                "passive reflection coefficient must be finite and have magnitude at most one"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PowerPortError {}

fn validate_positive_finite(value: f64, error: PowerPortError) -> Result<(), PowerPortError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(error)
    }
}

fn validate_nonzero_finite(value: f64, error: PowerPortError) -> Result<(), PowerPortError> {
    if value.is_finite() && value != 0.0 {
        Ok(())
    } else {
        Err(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1.0e-12;

    #[test]
    fn wave_conversion_preserves_port_and_power_metric() {
        let port = PowerPort::try_new(PowerDomain::Electrical, 3.2, -0.17).unwrap();
        let wave = port.to_wave(47_000.0).unwrap();
        let recovered = wave.to_port();

        assert!((recovered.effort - port.effort).abs() < TOLERANCE);
        assert!((recovered.flow - port.flow).abs() < TOLERANCE);
        assert!((wave.net_power() - port.power()).abs() < TOLERANCE);
    }

    #[test]
    fn transformer_is_cross_domain_and_power_preserving() {
        let electrical = PowerPort::try_new(PowerDomain::Electrical, 12.0, 0.25).unwrap();
        let mechanical = ideal_transformer_secondary(
            electrical,
            PowerDomain::MechanicalTranslation,
            8.0,
        )
        .unwrap();

        assert!((electrical.power() + mechanical.power()).abs() < TOLERANCE);
        assert_eq!(mechanical.domain, PowerDomain::MechanicalTranslation);
    }

    #[test]
    fn gyrator_is_cross_domain_and_power_preserving() {
        let electrical = PowerPort::try_new(PowerDomain::Electrical, -4.0, 0.6).unwrap();
        let mechanical = ideal_gyrator_secondary(
            electrical,
            PowerDomain::MechanicalTranslation,
            7.5,
        )
        .unwrap();

        assert!((electrical.power() + mechanical.power()).abs() < TOLERANCE);
    }

    #[test]
    fn rotation_scattering_preserves_wave_norm() {
        let junction = LosslessRotation2::from_angle(0.731);
        let incident = [0.37, -1.42];
        let reflected = junction.scatter(incident);
        let before = incident[0].mul_add(incident[0], incident[1] * incident[1]);
        let after = reflected[0].mul_add(reflected[0], reflected[1] * reflected[1]);

        assert!((before - after).abs() < TOLERANCE);
    }

    #[test]
    fn passive_termination_never_creates_power() {
        for reflection in [-1.0, -0.4, 0.0, 0.7, 1.0] {
            let termination = PassiveTermination::try_new(reflection).unwrap();
            let frame = termination.terminate(1.7);
            assert!(frame.absorbed_power >= -TOLERANCE);
        }
        assert!(PassiveTermination::try_new(1.000_001).is_err());
    }
}
