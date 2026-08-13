//! Content-addressed construction IR for one Ariadne world.
//!
//! The IR describes anatomy and fixed capacities. It is compiled before audio
//! processing; string, brass, voice, percussion, synthetic, and spatial
//! behavior must arrive here as configurations of the same law vocabulary.

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::field::ModalMetricConfiguration;
use crate::law::aperture::CompliantApertureConfiguration;

pub const WORLD_SPEC_SCHEMA_VERSION: &str = "ariadne-world-v1";
pub const MODE_COUNT: usize = 24;
pub const ACTIVE_RANKS: usize = 3;
pub const TRANSPORT_PAIRS: usize = 12;
pub const SPACE_COURSES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MetricSpec {
    pub frequency_hz: [f64; MODE_COUNT],
    pub t60_seconds: [f64; MODE_COUNT],
}

impl MetricSpec {
    pub(crate) fn runtime(self) -> ModalMetricConfiguration<MODE_COUNT> {
        ModalMetricConfiguration {
            frequency_hz: self.frequency_hz,
            t60_seconds: self.t60_seconds,
        }
    }

    fn validate(self, sample_rate_hz: f64) -> Result<(), SpecError> {
        for index in 0..MODE_COUNT {
            let frequency = self.frequency_hz[index];
            let t60 = self.t60_seconds[index];
            if !frequency.is_finite() || frequency <= 0.0 || frequency >= 0.48 * sample_rate_hz {
                return Err(SpecError::new(
                    "invalid-mode-frequency",
                    format!(
                        "mode {index} frequency {frequency} is outside the positive runtime band"
                    ),
                ));
            }
            if !t60.is_finite() || t60 <= 0.0 {
                return Err(SpecError::new(
                    "invalid-mode-loss",
                    format!("mode {index} T60 {t60} must be finite and positive"),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ApertureSpec {
    pub natural_frequency_hz: f64,
    pub quartic_stiffness: f64,
    pub asymmetry: f64,
    pub equilibrium_displacement: f64,
    pub opening_bias: f64,
    pub damping_per_second: f64,
    pub source_to_aperture_rate_hz: f64,
    pub source_to_field_rate_hz: f64,
    pub sweep_rate_hz: f64,
    pub active_rate_per_second: f64,
    pub active_target_momentum: f64,
}

impl ApertureSpec {
    pub(crate) fn runtime(self) -> CompliantApertureConfiguration {
        CompliantApertureConfiguration {
            natural_frequency_hz: self.natural_frequency_hz,
            quartic_stiffness: self.quartic_stiffness,
            asymmetry: self.asymmetry,
            equilibrium_displacement: self.equilibrium_displacement,
            opening_bias: self.opening_bias,
            damping_per_second: self.damping_per_second,
            source_to_aperture_rate_hz: self.source_to_aperture_rate_hz,
            source_to_field_rate_hz: self.source_to_field_rate_hz,
            sweep_rate_hz: self.sweep_rate_hz,
            active_rate_per_second: self.active_rate_per_second,
            active_target_momentum: self.active_target_momentum,
        }
    }

    fn validate(self) -> Result<(), SpecError> {
        let values = [
            self.natural_frequency_hz,
            self.quartic_stiffness,
            self.asymmetry,
            self.equilibrium_displacement,
            self.opening_bias,
            self.damping_per_second,
            self.source_to_aperture_rate_hz,
            self.source_to_field_rate_hz,
            self.sweep_rate_hz,
            self.active_rate_per_second,
            self.active_target_momentum,
        ];
        if values.iter().any(|value| !value.is_finite())
            || !(5.0..=8_000.0).contains(&self.natural_frequency_hz)
            || self.quartic_stiffness < 0.0
            || self.opening_bias < 0.0
            || self.damping_per_second < 0.0
            || self.source_to_aperture_rate_hz < 0.0
            || self.source_to_field_rate_hz < 0.0
            || self.sweep_rate_hz < 0.0
            || self.active_rate_per_second < 0.0
            || self.active_target_momentum <= 0.0
        {
            return Err(SpecError::new(
                "invalid-aperture",
                "compliant aperture coefficients are outside the supported finite domain",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WorldConfiguration {
    pub metric: MetricSpec,
    pub aperture: ApertureSpec,
    pub radiation_direction: [f64; MODE_COUNT],
}

impl WorldConfiguration {
    pub fn validate(self, sample_rate_hz: f64) -> Result<(), SpecError> {
        self.metric.validate(sample_rate_hz)?;
        self.aperture.validate()?;
        validate_direction("radiation_direction", &self.radiation_direction)?;
        Ok(())
    }

    pub fn digest(self) -> Result<String, SpecError> {
        digest_json(&self)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigurationPath {
    pub coordinate: String,
    pub left_name: String,
    pub right_name: String,
    pub left: WorldConfiguration,
    pub right: WorldConfiguration,
    pub initial_amount: f64,
}

impl ConfigurationPath {
    pub fn configuration_at(&self, amount: f64) -> WorldConfiguration {
        interpolate_configuration(self.left, self.right, amount)
    }

    fn validate(&self, sample_rate_hz: f64) -> Result<(), SpecError> {
        if self.coordinate.trim().is_empty()
            || self.left_name.trim().is_empty()
            || self.right_name.trim().is_empty()
            || !self.initial_amount.is_finite()
            || !(0.0..=1.0).contains(&self.initial_amount)
        {
            return Err(SpecError::new(
                "invalid-configuration-path",
                "configuration path names must be nonempty and initial_amount must be in [0,1]",
            ));
        }
        self.left.validate(sample_rate_hz)?;
        self.right.validate(sample_rate_hz)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TransportSpec {
    pub pairs: [(usize, usize); TRANSPORT_PAIRS],
    pub weights: [f64; TRANSPORT_PAIRS],
}

impl TransportSpec {
    fn validate(self) -> Result<(), SpecError> {
        for pair_index in 0..TRANSPORT_PAIRS {
            let (left, right) = self.pairs[pair_index];
            if left == right || left >= MODE_COUNT || right >= MODE_COUNT {
                return Err(SpecError::new(
                    "invalid-transport-pair",
                    format!(
                        "transport pair {pair_index} is not an edge in the {MODE_COUNT}-mode body"
                    ),
                ));
            }
            if !self.weights[pair_index].is_finite() {
                return Err(SpecError::new(
                    "invalid-transport-weight",
                    format!("transport pair {pair_index} has a non-finite weight"),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpaceSpec {
    pub delays_seconds: [f64; SPACE_COURSES],
    pub body_directions: [[f64; MODE_COUNT]; SPACE_COURSES],
    pub coupling_weights: [f64; SPACE_COURSES],
    pub scattering_direction: [f64; SPACE_COURSES],
    pub radiation_left: [f64; SPACE_COURSES],
    pub radiation_right: [f64; SPACE_COURSES],
}

impl SpaceSpec {
    fn validate(self) -> Result<(), SpecError> {
        for course in 0..SPACE_COURSES {
            let delay = self.delays_seconds[course];
            if !delay.is_finite() || !(0.000_25..=2.0).contains(&delay) {
                return Err(SpecError::new(
                    "invalid-space-delay",
                    format!("space course {course} delay {delay} is outside [0.25 ms, 2 s]"),
                ));
            }
            validate_direction("space body direction", &self.body_directions[course])?;
            if !self.coupling_weights[course].is_finite() {
                return Err(SpecError::new(
                    "invalid-space-coupling",
                    format!("space course {course} has a non-finite coupling weight"),
                ));
            }
        }
        validate_direction("space scattering direction", &self.scattering_direction)?;
        validate_direction("space left radiation", &self.radiation_left)?;
        validate_direction("space right radiation", &self.radiation_right)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RadiationSpec {
    pub observation_scale: f64,
}

impl RadiationSpec {
    fn validate(self) -> Result<(), SpecError> {
        if !self.observation_scale.is_finite() || !(0.01..=64.0).contains(&self.observation_scale) {
            return Err(SpecError::new(
                "invalid-observation-scale",
                "observation_scale must be finite in [0.01, 64]",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReservoirSpec {
    pub capacity: f64,
}

impl ReservoirSpec {
    fn validate(self) -> Result<(), SpecError> {
        if !self.capacity.is_finite() || self.capacity <= 0.0 {
            return Err(SpecError::new(
                "invalid-reservoir",
                "work reservoir capacity must be finite and positive",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldSpec {
    pub schema_version: String,
    pub anatomy_name: String,
    pub internal_sample_rate_hz: f64,
    pub configuration_path: ConfigurationPath,
    pub source_direction: [f64; MODE_COUNT],
    pub active_directions: [[f64; MODE_COUNT]; ACTIVE_RANKS],
    pub transport: TransportSpec,
    pub space: SpaceSpec,
    pub radiation: RadiationSpec,
    pub reservoir: ReservoirSpec,
}

impl WorldSpec {
    pub fn validate(&self) -> Result<(), SpecError> {
        if self.schema_version != WORLD_SPEC_SCHEMA_VERSION {
            return Err(SpecError::new(
                "unsupported-world-schema",
                format!(
                    "expected {WORLD_SPEC_SCHEMA_VERSION}, found {}",
                    self.schema_version
                ),
            ));
        }
        if self.anatomy_name.trim().is_empty() {
            return Err(SpecError::new(
                "empty-anatomy-name",
                "anatomy_name must be nonempty",
            ));
        }
        if !self.internal_sample_rate_hz.is_finite()
            || !(8_000.0..=384_000.0).contains(&self.internal_sample_rate_hz)
        {
            return Err(SpecError::new(
                "invalid-internal-clock",
                "internal_sample_rate_hz must be finite in [8 kHz, 384 kHz]",
            ));
        }
        self.configuration_path
            .validate(self.internal_sample_rate_hz)?;
        validate_direction("source_direction", &self.source_direction)?;
        for rank in 0..ACTIVE_RANKS {
            validate_direction("active_direction", &self.active_directions[rank])?;
        }
        self.transport.validate()?;
        self.space.validate()?;
        self.radiation.validate()?;
        self.reservoir.validate()?;
        Ok(())
    }

    pub fn digest(&self) -> Result<String, SpecError> {
        self.validate()?;
        digest_json(self)
    }

    /// First canonical anatomy: six string-course modal families and one
    /// compliant-bore family represented on the same 24 persistent pairs.
    pub fn string_brass_v1() -> Self {
        let left = WorldConfiguration {
            metric: six_course_string_metric(),
            aperture: string_contact_aperture(),
            radiation_direction: string_radiation_direction(),
        };
        let right = WorldConfiguration {
            metric: brass_bore_metric(),
            aperture: brass_compliant_aperture(),
            radiation_direction: brass_radiation_direction(),
        };
        Self {
            schema_version: WORLD_SPEC_SCHEMA_VERSION.to_owned(),
            anatomy_name: "shared-wave-material-v1".to_owned(),
            internal_sample_rate_hz: 96_000.0,
            configuration_path: ConfigurationPath {
                coordinate: "material_organization".to_owned(),
                left_name: "six_course_string".to_owned(),
                right_name: "compliant_bore".to_owned(),
                left,
                right,
                initial_amount: 0.0,
            },
            source_direction: shared_source_direction(),
            active_directions: active_directions(),
            transport: TransportSpec {
                pairs: [
                    (0, 2),
                    (1, 3),
                    (2, 5),
                    (3, 7),
                    (4, 9),
                    (5, 11),
                    (6, 13),
                    (7, 15),
                    (8, 17),
                    (9, 19),
                    (10, 21),
                    (11, 23),
                ],
                weights: [
                    1.0, 0.92, 0.84, 0.77, 0.70, 0.64, 0.59, 0.54, 0.49, 0.44, 0.40, 0.36,
                ],
            },
            space: default_space(),
            radiation: RadiationSpec {
                observation_scale: 6.0,
            },
            reservoir: ReservoirSpec { capacity: 0.002 },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecError {
    pub code: &'static str,
    pub message: String,
}

impl SpecError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl Display for SpecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for SpecError {}

pub fn interpolate_configuration(
    left: WorldConfiguration,
    right: WorldConfiguration,
    amount: f64,
) -> WorldConfiguration {
    let amount = finite(amount).clamp(0.0, 1.0);
    WorldConfiguration {
        metric: MetricSpec {
            frequency_hz: std::array::from_fn(|index| {
                let left_log = left.metric.frequency_hz[index].ln();
                let right_log = right.metric.frequency_hz[index].ln();
                (left_log + amount * (right_log - left_log)).exp()
            }),
            t60_seconds: std::array::from_fn(|index| {
                left.metric.t60_seconds[index]
                    + amount * (right.metric.t60_seconds[index] - left.metric.t60_seconds[index])
            }),
        },
        aperture: interpolate_aperture(left.aperture, right.aperture, amount),
        radiation_direction: normalized(std::array::from_fn(|index| {
            (1.0 - amount) * left.radiation_direction[index]
                + amount * right.radiation_direction[index]
        })),
    }
}

fn six_course_string_metric() -> MetricSpec {
    let fundamentals = [
        82.406_889_228,
        110.0,
        146.832_383_959,
        195.997_717_991,
        246.941_650_628,
        329.627_556_913,
    ];
    let mut frequency_hz = [0.0; MODE_COUNT];
    let mut t60_seconds = [0.0; MODE_COUNT];
    for (course, fundamental) in fundamentals.iter().enumerate() {
        for partial in 0..4 {
            let index = 4 * course + partial;
            let harmonic = partial as f64 + 1.0;
            frequency_hz[index] =
                fundamental * harmonic * (1.0 + 1.5e-5 * harmonic * harmonic).sqrt();
            t60_seconds[index] =
                (2.8 - 0.30 * course as f64) * (1.0 - 0.18 * partial as f64).max(0.25);
        }
    }
    MetricSpec {
        frequency_hz,
        t60_seconds,
    }
}

fn brass_bore_metric() -> MetricSpec {
    let fundamental = 116.540_940_380;
    MetricSpec {
        frequency_hz: std::array::from_fn(|index| {
            let harmonic = index as f64 + 1.0;
            fundamental * harmonic * (1.0 + 1.9e-5 * harmonic * harmonic).sqrt()
        }),
        t60_seconds: std::array::from_fn(|index| {
            let position = index as f64 / (MODE_COUNT - 1) as f64;
            3.2 + position * (0.58 - 3.2)
        }),
    }
}

fn string_contact_aperture() -> ApertureSpec {
    ApertureSpec {
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

fn brass_compliant_aperture() -> ApertureSpec {
    ApertureSpec {
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

fn shared_source_direction() -> [f64; MODE_COUNT] {
    let string = string_source_direction();
    let brass = brass_source_direction();
    normalized(std::array::from_fn(|index| {
        0.5 * (string[index] + brass[index])
    }))
}

fn string_source_direction() -> [f64; MODE_COUNT] {
    normalized(std::array::from_fn(|index| {
        let mode = index as f64 + 1.0;
        (std::f64::consts::PI * mode * 0.18).sin() / mode
    }))
}

fn string_radiation_direction() -> [f64; MODE_COUNT] {
    normalized(std::array::from_fn(|index| {
        let mode = index as f64 + 1.0;
        (std::f64::consts::PI * mode * 0.18).sin() / (1.0 + 0.003 * (index as f64).powi(2))
    }))
}

fn brass_source_direction() -> [f64; MODE_COUNT] {
    normalized(std::array::from_fn(|index| {
        let parity = if index % 2 == 0 { 1.0 } else { 0.85 };
        parity / (1.0 + 0.08 * (index as f64).powi(2))
    }))
}

fn brass_radiation_direction() -> [f64; MODE_COUNT] {
    normalized(std::array::from_fn(|index| {
        let position = index as f64 / (MODE_COUNT - 1) as f64;
        (1.0 + 0.97 * position) / (1.0 + 0.02 * (index as f64).powi(2))
    }))
}

fn active_directions() -> [[f64; MODE_COUNT]; ACTIVE_RANKS] {
    std::array::from_fn(|rank| {
        normalized(std::array::from_fn(|index| {
            let x = index as f64 / (MODE_COUNT - 1) as f64;
            match rank {
                0 => 0.35 + 1.65 * x,
                1 => {
                    if index % 2 == 0 {
                        1.0 - 0.35 * x
                    } else {
                        -0.82 + 0.25 * x
                    }
                }
                _ => (std::f64::consts::PI * (2.0 * x + 0.15)).sin(),
            }
        }))
    })
}

fn default_space() -> SpaceSpec {
    SpaceSpec {
        delays_seconds: [
            0.0073, 0.0111, 0.0137, 0.0179, 0.0233, 0.0297, 0.0521, 0.0899,
        ],
        body_directions: std::array::from_fn(|course| {
            let mut direction = [0.0; MODE_COUNT];
            let first = (course * 3 + 1) % MODE_COUNT;
            direction[first] = 1.0;
            direction[(first + 5) % MODE_COUNT] = if course % 2 == 0 { 0.35 } else { -0.35 };
            normalized(direction)
        }),
        coupling_weights: [1.0, 0.91, 0.82, 0.74, 0.67, 0.60, 0.54, 0.48],
        scattering_direction: normalized([1.0, -0.73, 0.88, -0.61, 0.79, -0.52, 0.69, -0.44]),
        radiation_left: normalized([1.0, 0.62, 0.18, -0.31, -0.70, -0.54, 0.08, 0.48]),
        radiation_right: normalized([0.17, -0.51, -0.72, -0.24, 0.36, 0.61, 0.31, -0.22]),
    }
}

fn interpolate_aperture(left: ApertureSpec, right: ApertureSpec, amount: f64) -> ApertureSpec {
    macro_rules! linear {
        ($field:ident) => {
            left.$field + amount * (right.$field - left.$field)
        };
    }
    ApertureSpec {
        natural_frequency_hz: linear!(natural_frequency_hz),
        quartic_stiffness: linear!(quartic_stiffness),
        asymmetry: linear!(asymmetry),
        equilibrium_displacement: linear!(equilibrium_displacement),
        opening_bias: linear!(opening_bias),
        damping_per_second: linear!(damping_per_second),
        source_to_aperture_rate_hz: linear!(source_to_aperture_rate_hz),
        source_to_field_rate_hz: linear!(source_to_field_rate_hz),
        sweep_rate_hz: linear!(sweep_rate_hz),
        active_rate_per_second: linear!(active_rate_per_second),
        active_target_momentum: linear!(active_target_momentum),
    }
}

fn validate_direction<const N: usize>(
    name: &'static str,
    direction: &[f64; N],
) -> Result<(), SpecError> {
    if direction.iter().any(|value| !value.is_finite()) {
        return Err(SpecError::new(
            "nonfinite-direction",
            format!("{name} contains NaN or infinity"),
        ));
    }
    let norm = direction
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    if !norm.is_finite() || norm <= 1.0e-12 {
        return Err(SpecError::new(
            "degenerate-direction",
            format!("{name} has no finite spatial extent"),
        ));
    }
    Ok(())
}

fn normalized<const N: usize>(mut values: [f64; N]) -> [f64; N] {
    let norm = values.iter().map(|value| value * value).sum::<f64>().sqrt();
    for value in &mut values {
        *value /= norm;
    }
    values
}

fn digest_json<T: Serialize>(value: &T) -> Result<String, SpecError> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        SpecError::new(
            "world-serialization",
            format!("could not serialize world IR: {error}"),
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn finite(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_spec_is_valid_and_content_addressed() {
        let spec = WorldSpec::string_brass_v1();
        spec.validate().unwrap();
        assert_eq!(spec.digest().unwrap(), spec.digest().unwrap());
    }

    #[test]
    fn interpolation_stays_inside_positive_metric_cone() {
        let spec = WorldSpec::string_brass_v1();
        for step in 0..=100 {
            spec.configuration_path
                .configuration_at(step as f64 / 100.0)
                .validate(spec.internal_sample_rate_hz)
                .unwrap();
        }
    }
}
