//! Measured construction profiles for the first shared string/brass campaign.
//!
//! These values do not define separate renderers. Each profile configures the
//! same [`MorphingWaveField`](crate::ariadne_morphing_field::MorphingWaveField)
//! and [`CompliantApertureBoundary`](crate::ariadne_compliant_aperture::CompliantApertureBoundary).
//! They are scientific fixtures, not product presets or claims about a named
//! guitar or trombone.

use crate::ariadne_compliant_aperture::CompliantApertureConfiguration;
use crate::ariadne_morphing_field::WaveFieldConfiguration;
use std::f64::consts::PI;

pub const STRING_BRASS_MODES: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedBodyRegime {
    ElectricGuitarClean,
    TromboneSustain,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SharedBodyConfiguration {
    pub field: WaveFieldConfiguration,
    pub boundary: CompliantApertureConfiguration,
    pub source_direction: [f64; STRING_BRASS_MODES],
    pub radiation_direction: [f64; STRING_BRASS_MODES],
    /// External source-wave magnitude used by the controlled reference probe.
    /// Performance trajectories may replace it; it is not stored body anatomy.
    pub reference_source_wave: f64,
}

impl SharedBodyConfiguration {
    #[must_use]
    pub fn for_regime(regime: SharedBodyRegime) -> Self {
        match regime {
            SharedBodyRegime::ElectricGuitarClean => Self {
                field: WaveFieldConfiguration::guitar(),
                boundary: guitar_boundary_v1(),
                source_direction: guitar_source_direction(),
                radiation_direction: guitar_radiation_direction(),
                reference_source_wave: 0.0,
            },
            SharedBodyRegime::TromboneSustain => Self {
                field: WaveFieldConfiguration::trombone(),
                boundary: trombone_boundary_v1(),
                source_direction: trombone_source_direction(),
                radiation_direction: trombone_radiation_direction(),
                reference_source_wave: 0.264,
            },
        }
    }
}

#[must_use]
pub fn guitar_boundary_v1() -> CompliantApertureConfiguration {
    CompliantApertureConfiguration {
        natural_frequency_hz: 540.0,
        quartic_stiffness: 4.0e6,
        asymmetry: 0.012,
        equilibrium_displacement: 0.0,
        opening_bias: 0.025,
        damping_per_second: 48.0,
        source_to_aperture_rate_hz: 1_100.0,
        source_to_field_rate_hz: 1_800.0,
        sweep_rate_hz: 420.0,
        active_rate_per_second: 0.0,
        active_target_momentum: 0.25,
    }
}

/// Fitted controlled-probe configuration from `RESULTS.json`. The relatively
/// high scattering rates are evaluated on the fixed 96 kHz material clock;
/// they must be retested before host-rate-native use.
#[must_use]
pub fn trombone_boundary_v1() -> CompliantApertureConfiguration {
    CompliantApertureConfiguration {
        natural_frequency_hz: 110.0,
        quartic_stiffness: 4.1e6,
        asymmetry: 0.028,
        equilibrium_displacement: 0.0,
        opening_bias: 0.055,
        damping_per_second: 7.9,
        source_to_aperture_rate_hz: 12_000.0,
        source_to_field_rate_hz: 2_400.0,
        sweep_rate_hz: 3_500.0,
        active_rate_per_second: 38.0,
        active_target_momentum: 0.334,
    }
}

#[must_use]
pub fn guitar_source_direction() -> [f64; STRING_BRASS_MODES] {
    normalized(std::array::from_fn(|index| {
        let mode = index as f64 + 1.0;
        (PI * mode * 0.18).sin() / mode
    }))
}

#[must_use]
pub fn guitar_radiation_direction() -> [f64; STRING_BRASS_MODES] {
    normalized(std::array::from_fn(|index| {
        let mode = index as f64 + 1.0;
        (PI * mode * 0.18).sin() / (1.0 + 0.003 * (index as f64).powi(2))
    }))
}

#[must_use]
pub fn trombone_source_direction() -> [f64; STRING_BRASS_MODES] {
    normalized(std::array::from_fn(|index| {
        let parity = if index % 2 == 0 { 1.0 } else { 0.85 };
        parity / (1.0 + 0.08 * (index as f64).powi(2))
    }))
}

#[must_use]
pub fn trombone_radiation_direction() -> [f64; STRING_BRASS_MODES] {
    normalized(std::array::from_fn(|index| {
        let position = index as f64 / (STRING_BRASS_MODES - 1) as f64;
        (1.0 + 0.97 * position) / (1.0 + 0.02 * (index as f64).powi(2))
    }))
}

fn normalized(mut values: [f64; STRING_BRASS_MODES]) -> [f64; STRING_BRASS_MODES] {
    let norm = values.iter().map(|value| value * value).sum::<f64>().sqrt();
    for value in &mut values {
        *value /= norm;
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ariadne_compliant_aperture::CompliantApertureBoundary;
    use crate::ariadne_morphing_field::MorphingWaveField;

    #[test]
    fn both_regimes_construct_the_same_rust_types_and_state_dimension() {
        for regime in [
            SharedBodyRegime::ElectricGuitarClean,
            SharedBodyRegime::TromboneSustain,
        ] {
            let profile = SharedBodyConfiguration::for_regime(regime);
            MorphingWaveField::<STRING_BRASS_MODES>::try_new(
                96_000.0,
                116.5,
                profile.field,
            )
            .unwrap();
            CompliantApertureBoundary::<STRING_BRASS_MODES>::try_new(
                96_000.0,
                profile.boundary,
                profile.source_direction,
            )
            .unwrap();
        }
    }

    #[test]
    fn profile_directions_are_unit_norm_and_distinct() {
        let directions = [
            guitar_source_direction(),
            guitar_radiation_direction(),
            trombone_source_direction(),
            trombone_radiation_direction(),
        ];
        for direction in directions {
            let norm = direction.iter().map(|value| value * value).sum::<f64>();
            assert!((norm - 1.0).abs() < 2.0e-14);
        }
        assert_ne!(guitar_source_direction(), trombone_source_direction());
        assert_ne!(guitar_radiation_direction(), trombone_radiation_direction());
    }

    #[test]
    fn sustained_source_belongs_to_the_gesture_not_the_body() {
        let guitar = SharedBodyConfiguration::for_regime(SharedBodyRegime::ElectricGuitarClean);
        let trombone = SharedBodyConfiguration::for_regime(SharedBodyRegime::TromboneSustain);
        assert_eq!(guitar.reference_source_wave, 0.0);
        assert!(trombone.reference_source_wave > 0.0);
        assert_eq!(guitar.boundary.active_rate_per_second, 0.0);
        assert!(trombone.boundary.active_rate_per_second > 0.0);
    }
}
