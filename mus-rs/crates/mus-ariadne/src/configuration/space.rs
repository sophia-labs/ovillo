//! Configuration-space IR: named coordinates, the map from a point to a body
//! configuration, and the connection that transports state along a move.

use serde::{Deserialize, Serialize};

use crate::ir::{ApertureSpec, MetricSpec, WorldConfiguration, MODE_COUNT};
use crate::plan::PlanError;

use super::network::TubeNetworkSpec;

/// Largest supported number of coordinates (tube networks and stiff strings).
pub const MAX_CONFIGURATION_DIMENSIONS: usize = 16;
/// A multilinear map evaluates `2^dimensions` corners per point: at most 8
/// coordinates (256 corners).
pub const MAX_MULTILINEAR_DIMENSIONS: usize = 8;
const MAX_ANCHORS: usize = 4_096;

/// One named coordinate of a configuration space with its closed bounds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationCoordinate {
    pub name: String,
    pub lower: f64,
    pub upper: f64,
    pub initial: f64,
}

/// How a point of the space becomes a body configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConfigurationMapSpec {
    /// Tensor-product grid of anchor configurations. `axes[i]` lists the grid
    /// values of coordinate `i` (strictly increasing, from `lower` to `upper`);
    /// `anchors` is row-major with the last coordinate fastest. Between grid
    /// values the legacy 1-D rules apply axis by axis: log-linear frequency,
    /// linear T60 and aperture, linear-then-normalized radiation direction.
    /// An interpolation between authored anchors: a fit, not a law.
    Multilinear {
        axes: Vec<Vec<f64>>,
        anchors: Vec<WorldConfiguration>,
    },
    /// Modes computed from a lumped acoustic tube network whose areas are a
    /// declared function of the coordinates (a constitutive law).
    TubeNetwork { network: TubeNetworkSpec },
    /// Partials computed from a pinned stiff string (a constitutive law):
    /// `f_n = n f0 sqrt(1 + B n^2)`, `f0 ~ sqrt(T)/L`, `B ~ 1/(T L^2)`.
    StiffString { string: StiffStringSpec },
    /// Partials computed from a pinned stiff wire declared by its material
    /// and geometry (a constitutive law; [`StiffWireSpec`]). Additive: the
    /// variants above serialize exactly as before.
    StiffWire { wire: StiffWireSpec },
}

/// A pinned stiff string whose stopped length (and optionally tension) are
/// coordinates. At stopped length `L = L0 2^(-frets/12)` and tension
/// `T = t T0`: `f0 = f0_open 2^(frets/12) sqrt(t)`,
/// `B = B_open 4^(frets/12) / t`, `f_n = n f0 sqrt(1 + B n^2)` (Euler-Bernoulli
/// stiff string, pinned ends; `B = pi^3 E d^4 / (64 T L^2)`). Loss
/// `sigma(omega) = a + b omega^2` per partial, T60 = ln(1000)/sigma.
/// The aperture and radiation direction are fixed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StiffStringSpec {
    pub open_fundamental_hz: f64,
    pub open_inharmonicity: f64,
    /// Coordinate holding the stopped length in frets.
    pub fret_coordinate: usize,
    /// Coordinate holding tension relative to the open string's, if any.
    #[serde(default)]
    pub tension_coordinate: Option<usize>,
    pub decay_constant_per_s: f64,
    pub decay_quadratic_s: f64,
    /// `ln(1000)` as the anatomy compiler wrote it (the steel-string compiler
    /// uses `6.91`); T60 = `t60_numerator / sigma`.
    pub t60_numerator: f64,
    pub aperture: ApertureSpec,
    pub radiation_direction: [f64; MODE_COUNT],
}

impl StiffStringSpec {
    /// The configuration at `point`: the law, evaluated.
    #[must_use]
    pub fn configuration_at(&self, point: &[f64]) -> WorldConfiguration {
        let frets = point[self.fret_coordinate];
        let tension = self.tension_coordinate.map_or(1.0, |index| point[index]);
        let length_scale = (frets / 12.0).exp2();
        let f0 = self.open_fundamental_hz * length_scale * tension.sqrt();
        let b = self.open_inharmonicity * length_scale * length_scale / tension;
        let mut frequency_hz = [0.0; MODE_COUNT];
        let mut t60_seconds = [0.0; MODE_COUNT];
        for index in 0..MODE_COUNT {
            let n = index as f64 + 1.0;
            let f = n * f0 * (1.0 + b * n * n).sqrt();
            let omega = std::f64::consts::TAU * f;
            frequency_hz[index] = f;
            t60_seconds[index] = self.t60_numerator / (self.decay_constant_per_s + self.decay_quadratic_s * omega * omega);
        }
        WorldConfiguration {
            metric: MetricSpec { frequency_hz, t60_seconds },
            aperture: self.aperture,
            radiation_direction: self.radiation_direction,
        }
    }

    fn validate(&self, coordinates: &[ConfigurationCoordinate], sample_rate_hz: f64) -> Result<(), PlanError> {
        let dims = coordinates.len();
        let tension_ok = self.tension_coordinate.is_none_or(|index| {
            index < dims && index != self.fret_coordinate && coordinates[index].lower > 0.0
        });
        if self.fret_coordinate >= dims
            || !tension_ok
            || !self.open_fundamental_hz.is_finite()
            || self.open_fundamental_hz <= 0.0
            || !self.open_inharmonicity.is_finite()
            || self.open_inharmonicity < 0.0
            || !self.decay_constant_per_s.is_finite()
            || !self.decay_quadratic_s.is_finite()
            || self.decay_constant_per_s < 0.0
            || self.decay_quadratic_s < 0.0
            || self.decay_constant_per_s + self.decay_quadratic_s <= 0.0
            || !self.t60_numerator.is_finite()
            || self.t60_numerator <= 0.0
        {
            return Err(PlanError::new(
                "invalid-stiff-string",
                "stiff string needs f0 > 0, B >= 0, positive loss, a fret coordinate and a positive tension range",
            ));
        }
        // Frequencies grow with frets and tension: the corners bound the band.
        let mut point: Vec<f64> = coordinates.iter().map(|c| c.lower).collect();
        for corner in 0..(1usize << dims) {
            for (axis, coordinate) in coordinates.iter().enumerate() {
                point[axis] = if corner >> axis & 1 == 1 { coordinate.upper } else { coordinate.lower };
            }
            self.configuration_at(&point).validate(sample_rate_hz).map_err(|error| {
                PlanError::new("invalid-stiff-string-band", format!("at {point:?}: {error}"))
            })?;
        }
        Ok(())
    }
}

/// `ln(1000)`: T60 is the time for a mode's energy to fall 60 dB.
const LN_1000: f64 = 6.907_755_278_982_137;

/// A pinned stiff wire declared by what a scale designer writes down: its
/// speaking length, the diameter of its bending core, its linear density
/// (core plus any winding), the core's Young's modulus, and its tension,
/// which one coordinate scales (`T = t T0`, the tuning pin). The partials
/// follow (Euler-Bernoulli stiff string, pinned ends):
///
/// ```text
/// f0  = sqrt(T / mu) / (2 L)
/// B   = pi^3 E d^4 / (64 T L^2)
/// f_n = n f0 sqrt(1 + B n^2)
/// ```
///
/// with mass-normalized mode shapes `sqrt(2 / (mu L)) sin(n pi x / L)` (pure
/// sines: pinned ends) and loss `sigma(omega) = a + b omega^2` per partial
/// (T60 = `ln(1000) / sigma`). A wound string's winding adds mass, not
/// bending stiffness: `B` is the core's (the usual approximation). The wire
/// radiates as a thin cylinder (a line dipole, `ka << 1`): its radiation
/// direction weighs partial `n` by `omega_n^(3/2)`, and
/// [`StiffWireSpec::dipole_radiation_sigma`] gives each partial's rate.
///
/// Only partials below `band_limit_hz` at the box's highest tension are
/// retained ([`StiffWireSpec::retained_partials`]). The slots above hold no
/// state: every direction this law hands out (mode shape, moving-end weight,
/// radiation) is zero there, and they repeat the last retained partial's
/// frequency and loss only to keep the metric valid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StiffWireSpec {
    pub speaking_length_m: f64,
    pub core_diameter_m: f64,
    pub linear_density_kg_m: f64,
    pub youngs_modulus_pa: f64,
    pub tension_n: f64,
    /// Coordinate holding tension relative to `tension_n`.
    pub tension_coordinate: usize,
    pub band_limit_hz: f64,
    pub decay_constant_per_s: f64,
    pub decay_quadratic_s: f64,
}

impl StiffWireSpec {
    /// Fundamental of the flexible string, `sqrt(T / mu) / (2 L)`, at tension ratio `t`.
    #[must_use]
    pub fn flexible_fundamental_hz(&self, t: f64) -> f64 {
        (t * self.tension_n / self.linear_density_kg_m).sqrt() / (2.0 * self.speaking_length_m)
    }

    /// `B = pi^3 E d^4 / (64 T L^2)` at tension ratio `t`.
    #[must_use]
    pub fn inharmonicity(&self, t: f64) -> f64 {
        let pi = std::f64::consts::PI;
        pi.powi(3) * self.youngs_modulus_pa * self.core_diameter_m.powi(4)
            / (64.0 * t * self.tension_n * self.speaking_length_m.powi(2))
    }

    /// Partial `n` (1-based) at tension ratio `t`.
    #[must_use]
    pub fn partial_hz(&self, n: usize, t: f64) -> f64 {
        let n = n as f64;
        n * self.flexible_fundamental_hz(t) * (1.0 + self.inharmonicity(t) * n * n).sqrt()
    }

    /// How many partials lie below `band_limit_hz` at the highest tension
    /// ratio `t_max` (at most [`MODE_COUNT`]). Partial frequencies grow with
    /// tension, so this count holds over the whole box.
    #[must_use]
    pub fn retained_partials(&self, t_max: f64) -> usize {
        (1..=MODE_COUNT).take_while(|&n| self.partial_hz(n, t_max) < self.band_limit_hz).count()
    }

    /// The configuration at `point` with `retained` partials: the law, evaluated.
    #[must_use]
    pub fn configuration_at(&self, point: &[f64], retained: usize) -> WorldConfiguration {
        let t = point[self.tension_coordinate];
        let mut frequency_hz = [0.0; MODE_COUNT];
        let mut t60_seconds = [0.0; MODE_COUNT];
        let mut radiation_direction = [0.0; MODE_COUNT];
        let kept = retained.clamp(1, MODE_COUNT);
        for index in 0..MODE_COUNT {
            let f = self.partial_hz(index.min(kept - 1) + 1, t);
            let omega = std::f64::consts::TAU * f;
            frequency_hz[index] = f;
            t60_seconds[index] = LN_1000 / (self.decay_constant_per_s + self.decay_quadratic_s * omega * omega);
            if index < kept {
                radiation_direction[index] = omega.powf(1.5);
            }
        }
        let norm = radiation_direction.iter().map(|v| v * v).sum::<f64>().sqrt();
        for value in &mut radiation_direction {
            *value /= norm;
        }
        WorldConfiguration {
            metric: MetricSpec { frequency_hz, t60_seconds },
            aperture: wire_aperture(),
            radiation_direction,
        }
    }

    /// Mass-normalized mode shapes (1/sqrt(kg)) averaged over a contact of
    /// `width_m` centred at `fraction` of the speaking length:
    /// `sqrt(2 / (mu L)) sin(n pi x / L) sinc(n pi w / (2 L))`. Zero in the
    /// slots that are not retained.
    #[must_use]
    pub fn mode_shape_at(&self, fraction: f64, width_m: f64, retained: usize) -> [f64; MODE_COUNT] {
        let length = self.speaking_length_m;
        let norm = (2.0 / (self.linear_density_kg_m * length)).sqrt();
        let half = 0.5 * width_m.max(0.0) / length;
        std::array::from_fn(|index| {
            if index >= retained {
                return 0.0;
            }
            let n = index as f64 + 1.0;
            let u = std::f64::consts::PI * n * half;
            let sinc = if u.abs() < 1.0e-12 { 1.0 } else { u.sin() / u };
            norm * (std::f64::consts::PI * n * fraction).sin() * sinc
        })
    }

    /// How strongly each partial couples to a moving bridge end, at
    /// resonance (`sqrt(kg)/s`). With the end displaced by `u_b`, the string
    /// is `w_s + (x / L) u_b`; the tension and bending cross terms vanish for
    /// pinned partials, and the kinetic one is `u_b' sum_n G_n eta_n'` with
    /// `G_n = sqrt(2 mu L) (-1)^(n+1) / (n pi)`. Partial `n` against a
    /// support mode `Phi` of the same frequency then exchanges as two
    /// resonators split by `omega_n G_n Phi`, and
    /// `omega_n G_n = (-1)^(n+1) sqrt(2 T / L) sqrt(1 + B n^2)` (which gives
    /// the transmission-line decay `sigma_n = (T / L)(1 + B n^2) Re Y` against
    /// a support admittance `Y`). Zero in the slots that are not retained.
    #[must_use]
    pub fn moving_end_weights(&self, t: f64, retained: usize) -> [f64; MODE_COUNT] {
        let scale = (2.0 * t * self.tension_n / self.speaking_length_m).sqrt();
        let b = self.inharmonicity(t);
        std::array::from_fn(|index| {
            if index >= retained {
                return 0.0;
            }
            let n = index as f64 + 1.0;
            let sign = if index % 2 == 0 { 1.0 } else { -1.0 };
            sign * scale * (1.0 + b * n * n).sqrt()
        })
    }

    /// Amplitude decay rate of each partial by the wire's own radiation, a
    /// thin cylinder of radius `a` in air (`ka << 1`, a line dipole). The
    /// exact field `p = A H1(kr) cos(theta)` with `ka << 1` radiates
    /// `pi^2 / 4 rho c k^3 a^4 U^2` per unit length, so
    /// `sigma_n = pi^2 rho omega_n^3 a^4 / (4 c^2 mu)`. With the projection
    /// radiation law, a radiation fraction of `4 sum(sigma) / fs` along this
    /// map's direction removes about these rates (modes well separated).
    #[must_use]
    pub fn dipole_radiation_sigma(
        &self,
        t: f64,
        retained: usize,
        outer_diameter_m: f64,
        air_density_kg_m3: f64,
        speed_of_sound_m_s: f64,
    ) -> [f64; MODE_COUNT] {
        let a4 = (0.5 * outer_diameter_m).powi(4);
        let pi2 = std::f64::consts::PI * std::f64::consts::PI;
        std::array::from_fn(|index| {
            if index >= retained {
                return 0.0;
            }
            let omega = std::f64::consts::TAU * self.partial_hz(index + 1, t);
            pi2 * air_density_kg_m3 * omega.powi(3) * a4
                / (4.0 * speed_of_sound_m_s * speed_of_sound_m_s * self.linear_density_kg_m)
        })
    }

    fn validate(&self, coordinates: &[ConfigurationCoordinate], sample_rate_hz: f64) -> Result<(), PlanError> {
        let dims = coordinates.len();
        let positive = [
            self.speaking_length_m,
            self.core_diameter_m,
            self.linear_density_kg_m,
            self.youngs_modulus_pa,
            self.tension_n,
            self.band_limit_hz,
        ];
        if self.tension_coordinate >= dims
            || coordinates[self.tension_coordinate].lower <= 0.0
            || positive.iter().any(|v| !v.is_finite() || *v <= 0.0)
            || !self.decay_constant_per_s.is_finite()
            || !self.decay_quadratic_s.is_finite()
            || self.decay_constant_per_s < 0.0
            || self.decay_quadratic_s < 0.0
            || self.decay_constant_per_s + self.decay_quadratic_s <= 0.0
            || self.band_limit_hz > 0.48 * sample_rate_hz
        {
            return Err(PlanError::new(
                "invalid-stiff-wire",
                "stiff wire needs positive length, core, density, modulus and tension, positive loss, \
                 a positive tension coordinate and a band limit inside the runtime band",
            ));
        }
        let t_max = coordinates[self.tension_coordinate].upper;
        let retained = self.retained_partials(t_max);
        if retained == 0 {
            return Err(PlanError::new(
                "invalid-stiff-wire-band",
                "the wire's fundamental lies above its band limit",
            ));
        }
        for t in [coordinates[self.tension_coordinate].lower, t_max] {
            let mut point: Vec<f64> = coordinates.iter().map(|c| c.initial).collect();
            point[self.tension_coordinate] = t;
            self.configuration_at(&point, retained).validate(sample_rate_hz).map_err(|error| {
                PlanError::new("invalid-stiff-wire-band", format!("at tension ratio {t}: {error}"))
            })?;
        }
        Ok(())
    }
}

/// A wire has no compliant aperture; the configuration carries an inert one
/// so the shared metric type stays valid.
fn wire_aperture() -> ApertureSpec {
    ApertureSpec {
        natural_frequency_hz: 500.0,
        quartic_stiffness: 0.0,
        asymmetry: 0.0,
        equilibrium_displacement: 0.0,
        opening_bias: 0.0,
        damping_per_second: 50.0,
        source_to_aperture_rate_hz: 0.0,
        source_to_field_rate_hz: 0.0,
        sweep_rate_hz: 0.0,
        active_rate_per_second: 0.0,
        active_target_momentum: 0.25,
    }
}

/// What happens to stored energy that the retained modes cannot hold after a
/// move (only the mass-momentum connection can produce it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisposalPolicy {
    /// Booked as internal loss.
    Dissipate,
    /// Booked as radiation.
    Radiate,
    /// Refuse a move whose disposal exceeds `1e-9` of the stored energy.
    Reject,
}

/// The declared state correspondence for a move `x -> x'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConnectionPolicy {
    /// Hold the normalized modal coordinates `(q_k, m_k)` fixed by mode index;
    /// the metric changes under them. This is the legacy 1-D connection
    /// (`Body::set_configuration_amount`), generalized. Work is the exact
    /// change of stored energy. It is flat: an instantaneous closed loop
    /// returns the state and bills zero net work.
    ModalCoordinates,
    /// Hold the physical air state fixed: the excess air volume of every
    /// compressible section (`C p`) and the air momentum per area of every
    /// inertive column (`L u`), the quantities the network's continuity and
    /// momentum equations integrate (for time-varying `C(t)`, `L(t)`:
    /// `d(Cp)/dt = sum u`, `d(Lu)/dt = -Delta p`). The state is re-projected
    /// onto the new modes in the energy metric; the part the retained modes
    /// cannot hold is disposed by `disposal`. Requires a tube-network map.
    MassMomentum { disposal: DisposalPolicy },
}

/// A body's configuration space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationSpaceSpec {
    pub coordinates: Vec<ConfigurationCoordinate>,
    pub map: ConfigurationMapSpec,
    pub connection: ConnectionPolicy,
}

impl ConfigurationSpaceSpec {
    #[must_use]
    pub fn dimensions(&self) -> usize {
        self.coordinates.len()
    }

    #[must_use]
    pub fn initial_point(&self) -> Vec<f64> {
        self.coordinates.iter().map(|c| c.initial).collect()
    }

    /// Clamp a point into the box (non-finite components become `lower`).
    pub fn clamp_into(&self, point: &[f64], out: &mut [f64]) {
        for (index, coordinate) in self.coordinates.iter().enumerate() {
            let value = point.get(index).copied().unwrap_or(coordinate.initial);
            out[index] = if value.is_finite() {
                value.clamp(coordinate.lower, coordinate.upper)
            } else {
                coordinate.lower
            };
        }
    }

    pub(crate) fn validate(&self, sample_rate_hz: f64) -> Result<(), PlanError> {
        let dims = self.coordinates.len();
        if dims == 0 || dims > MAX_CONFIGURATION_DIMENSIONS {
            return Err(PlanError::new(
                "invalid-configuration-dimensions",
                format!("a configuration space needs 1..={MAX_CONFIGURATION_DIMENSIONS} coordinates"),
            ));
        }
        let mut names = std::collections::BTreeSet::new();
        for coordinate in &self.coordinates {
            if coordinate.name.trim().is_empty()
                || coordinate.name.chars().any(char::is_whitespace)
                || !names.insert(coordinate.name.as_str())
            {
                return Err(PlanError::new(
                    "invalid-configuration-coordinate",
                    format!("coordinate names must be unique and whitespace-free: {:?}", coordinate.name),
                ));
            }
            if !coordinate.lower.is_finite()
                || !coordinate.upper.is_finite()
                || !coordinate.initial.is_finite()
                || coordinate.lower >= coordinate.upper
                || !(coordinate.lower..=coordinate.upper).contains(&coordinate.initial)
            {
                return Err(PlanError::new(
                    "invalid-configuration-bounds",
                    format!("coordinate {:?} needs finite lower < upper and initial inside", coordinate.name),
                ));
            }
        }
        match (&self.map, self.connection) {
            (ConfigurationMapSpec::Multilinear { axes, anchors }, ConnectionPolicy::ModalCoordinates) => {
                validate_multilinear(&self.coordinates, axes, anchors, sample_rate_hz)
            }
            (ConfigurationMapSpec::Multilinear { .. }, ConnectionPolicy::MassMomentum { .. }) => {
                Err(PlanError::new(
                    "connection-needs-physical-frame",
                    "the mass-momentum connection needs a tube-network map (anchors carry no physical frame)",
                ))
            }
            (ConfigurationMapSpec::TubeNetwork { network }, _) => {
                network.validate(dims, sample_rate_hz, self)
            }
            (ConfigurationMapSpec::StiffString { string }, ConnectionPolicy::ModalCoordinates) => {
                string.validate(&self.coordinates, sample_rate_hz)
            }
            (ConfigurationMapSpec::StiffString { .. }, ConnectionPolicy::MassMomentum { .. }) => {
                Err(PlanError::new(
                    "connection-needs-physical-frame",
                    "the mass-momentum connection needs a tube-network map",
                ))
            }
            (ConfigurationMapSpec::StiffWire { wire }, ConnectionPolicy::ModalCoordinates) => {
                wire.validate(&self.coordinates, sample_rate_hz)
            }
            (ConfigurationMapSpec::StiffWire { .. }, ConnectionPolicy::MassMomentum { .. }) => {
                Err(PlanError::new(
                    "connection-needs-physical-frame",
                    "the mass-momentum connection needs a tube-network map",
                ))
            }
        }
    }
}

fn validate_multilinear(
    coordinates: &[ConfigurationCoordinate],
    axes: &[Vec<f64>],
    anchors: &[WorldConfiguration],
    sample_rate_hz: f64,
) -> Result<(), PlanError> {
    if coordinates.len() > MAX_MULTILINEAR_DIMENSIONS {
        return Err(PlanError::new(
            "invalid-configuration-dimensions",
            format!("a multilinear map takes at most {MAX_MULTILINEAR_DIMENSIONS} coordinates"),
        ));
    }
    if axes.len() != coordinates.len() {
        return Err(PlanError::new(
            "invalid-multilinear-axes",
            "one axis per coordinate is required",
        ));
    }
    let mut count = 1usize;
    for (axis, coordinate) in axes.iter().zip(coordinates) {
        if axis.len() < 2
            || axis.iter().any(|v| !v.is_finite())
            || axis.windows(2).any(|w| w[1] <= w[0])
            || axis[0] != coordinate.lower
            || *axis.last().unwrap() != coordinate.upper
        {
            return Err(PlanError::new(
                "invalid-multilinear-axes",
                format!(
                    "axis of {:?} must be strictly increasing from lower to upper with >= 2 values",
                    coordinate.name
                ),
            ));
        }
        count = count.saturating_mul(axis.len());
    }
    if count > MAX_ANCHORS || anchors.len() != count {
        return Err(PlanError::new(
            "invalid-multilinear-anchors",
            format!("expected {count} anchors (<= {MAX_ANCHORS}), found {}", anchors.len()),
        ));
    }
    for (index, anchor) in anchors.iter().enumerate() {
        anchor.validate(sample_rate_hz).map_err(|error| {
            PlanError::new("invalid-multilinear-anchor", format!("anchor {index}: {error}"))
        })?;
    }
    // Frequencies, T60s and aperture coefficients stay inside convex domains
    // under the interpolation. Directions can cancel: check every cell's
    // centre and corners (a sampled check, as the legacy path's 33 samples).
    let mut map = MultilinearMap::compile(axes, anchors);
    let dims = coordinates.len();
    let mut cell = vec![0usize; dims];
    let mut point = vec![0.0; dims];
    loop {
        for corner in 0..=(1usize << dims) {
            for axis in 0..dims {
                let lo = axes[axis][cell[axis]];
                let hi = axes[axis][cell[axis] + 1];
                point[axis] = if corner == 1usize << dims {
                    0.5 * (lo + hi)
                } else if corner >> axis & 1 == 1 {
                    hi
                } else {
                    lo
                };
            }
            map.evaluate(&point).validate(sample_rate_hz).map_err(|error| {
                PlanError::new("invalid-multilinear-interior", error.to_string())
            })?;
        }
        let mut axis = dims;
        loop {
            if axis == 0 {
                return Ok(());
            }
            axis -= 1;
            cell[axis] += 1;
            if cell[axis] + 1 < axes[axis].len() {
                break;
            }
            cell[axis] = 0;
        }
    }
}

/// Number of interpolated scalars per anchor: log-frequency, T60, 11 aperture
/// coefficients, radiation direction.
const MULTILINEAR_WIDTH: usize = MODE_COUNT * 3 + 11;

/// Compiled tensor-grid interpolation.
#[derive(Debug, Clone)]
pub(crate) struct MultilinearMap {
    axes: Vec<Vec<f64>>,
    /// Per anchor: the interpolated scalars, log-frequency pre-taken.
    rows: Vec<[f64; MULTILINEAR_WIDTH]>,
    strides: Vec<usize>,
    scratch: Vec<[f64; MULTILINEAR_WIDTH]>,
}

impl MultilinearMap {
    pub(crate) fn compile(axes: &[Vec<f64>], anchors: &[WorldConfiguration]) -> Self {
        let dims = axes.len();
        let mut strides = vec![1usize; dims];
        for axis in (0..dims.saturating_sub(1)).rev() {
            strides[axis] = strides[axis + 1] * axes[axis + 1].len();
        }
        Self {
            axes: axes.to_vec(),
            rows: anchors.iter().map(row_of).collect(),
            strides,
            scratch: vec![[0.0; MULTILINEAR_WIDTH]; 1usize << dims],
        }
    }

    /// Configuration at `point` (already inside the box).
    ///
    /// The corner values are reduced one axis at a time, last axis first, with
    /// the legacy lerp forms, so a 1-D two-anchor map is bitwise the legacy
    /// `interpolate_configuration`.
    pub(crate) fn evaluate(&mut self, point: &[f64]) -> WorldConfiguration {
        let dims = self.axes.len();
        let mut base = 0usize;
        let mut t = [0.0_f64; MAX_MULTILINEAR_DIMENSIONS];
        for axis in 0..dims {
            let grid = &self.axes[axis];
            let x = point[axis].clamp(grid[0], *grid.last().unwrap());
            let mut cell = 0usize;
            while cell + 2 < grid.len() && x > grid[cell + 1] {
                cell += 1;
            }
            let (lo, hi) = (grid[cell], grid[cell + 1]);
            t[axis] = if x == hi { 1.0 } else { ((x - lo) / (hi - lo)).clamp(0.0, 1.0) };
            // The legacy path takes `amount` in [0,1] directly: keep that exact.
            if lo == 0.0 && hi == 1.0 {
                t[axis] = x;
            }
            base += cell * self.strides[axis];
        }
        let corners = 1usize << dims;
        let values = &mut self.scratch;
        for (corner, slot) in values.iter_mut().enumerate().take(corners) {
            let mut index = base;
            for axis in 0..dims {
                // Corner bit for `axis` sits at position (dims - 1 - axis) so the
                // last axis pairs adjacent slots.
                if corner >> (dims - 1 - axis) & 1 == 1 {
                    index += self.strides[axis];
                }
            }
            *slot = self.rows[index];
        }
        let mut width = corners;
        for axis in (0..dims).rev() {
            let amount = t[axis];
            width /= 2;
            for pair in 0..width {
                let (left, right) = (values[2 * pair], values[2 * pair + 1]);
                values[pair] = lerp_row(&left, &right, amount);
            }
        }
        configuration_of(&values[0])
    }
}

fn row_of(configuration: &WorldConfiguration) -> [f64; MULTILINEAR_WIDTH] {
    let mut row = [0.0; MULTILINEAR_WIDTH];
    for mode in 0..MODE_COUNT {
        row[mode] = configuration.metric.frequency_hz[mode].ln();
        row[MODE_COUNT + mode] = configuration.metric.t60_seconds[mode];
        row[2 * MODE_COUNT + 11 + mode] = configuration.radiation_direction[mode];
    }
    let a = configuration.aperture;
    let aperture = [
        a.natural_frequency_hz,
        a.quartic_stiffness,
        a.asymmetry,
        a.equilibrium_displacement,
        a.opening_bias,
        a.damping_per_second,
        a.source_to_aperture_rate_hz,
        a.source_to_field_rate_hz,
        a.sweep_rate_hz,
        a.active_rate_per_second,
        a.active_target_momentum,
    ];
    row[2 * MODE_COUNT..2 * MODE_COUNT + 11].copy_from_slice(&aperture);
    row
}

/// The legacy forms: `l + a (r - l)` for log-frequency, T60 and aperture
/// (`ir::interpolate_configuration`, `ir::interpolate_aperture`);
/// `(1 - a) l + a r` for the radiation direction.
fn lerp_row(
    left: &[f64; MULTILINEAR_WIDTH],
    right: &[f64; MULTILINEAR_WIDTH],
    amount: f64,
) -> [f64; MULTILINEAR_WIDTH] {
    let mut out = [0.0; MULTILINEAR_WIDTH];
    let direction_start = 2 * MODE_COUNT + 11;
    for i in 0..direction_start {
        out[i] = left[i] + amount * (right[i] - left[i]);
    }
    for i in direction_start..MULTILINEAR_WIDTH {
        out[i] = (1.0 - amount) * left[i] + amount * right[i];
    }
    out
}

fn configuration_of(row: &[f64; MULTILINEAR_WIDTH]) -> WorldConfiguration {
    let a = &row[2 * MODE_COUNT..2 * MODE_COUNT + 11];
    let direction: [f64; MODE_COUNT] = std::array::from_fn(|i| row[2 * MODE_COUNT + 11 + i]);
    let norm = direction.iter().map(|v| v * v).sum::<f64>().sqrt();
    WorldConfiguration {
        metric: MetricSpec {
            frequency_hz: std::array::from_fn(|i| row[i].exp()),
            t60_seconds: std::array::from_fn(|i| row[MODE_COUNT + i]),
        },
        aperture: ApertureSpec {
            natural_frequency_hz: a[0],
            quartic_stiffness: a[1],
            asymmetry: a[2],
            equilibrium_displacement: a[3],
            opening_bias: a[4],
            damping_per_second: a[5],
            source_to_aperture_rate_hz: a[6],
            source_to_field_rate_hz: a[7],
            sweep_rate_hz: a[8],
            active_rate_per_second: a[9],
            active_target_momentum: a[10],
        },
        radiation_direction: direction.map(|v| v / norm),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{interpolate_configuration, WorldSpec};

    #[test]
    fn one_dimensional_two_anchor_map_is_bitwise_the_legacy_path() {
        let spec = WorldSpec::string_brass_v1();
        let path = &spec.configuration_path;
        let mut map = MultilinearMap::compile(&[vec![0.0, 1.0]], &[path.left, path.right]);
        for step in 0..=1_000 {
            let amount = step as f64 / 1_000.0;
            let legacy = interpolate_configuration(path.left, path.right, amount);
            let nd = map.evaluate(&[amount]);
            // Bitwise: serialized JSON with float_roundtrip is identical.
            assert_eq!(
                serde_json::to_string(&legacy).unwrap(),
                serde_json::to_string(&nd).unwrap(),
                "amount {amount}"
            );
        }
    }

    #[test]
    fn multilinear_reproduces_every_anchor_and_is_separable() {
        let spec = WorldSpec::string_brass_v1();
        let (l, r) = (spec.configuration_path.left, spec.configuration_path.right);
        let mid = interpolate_configuration(l, r, 0.5);
        // 2-D grid: axis 0 two values, axis 1 three values.
        let anchors = vec![l, mid, r, r, l, mid];
        let axes = vec![vec![0.0, 1.0], vec![-1.0, 0.0, 2.0]];
        let mut map = MultilinearMap::compile(&axes, &anchors);
        for (i, x0) in [0.0, 1.0].into_iter().enumerate() {
            for (j, x1) in [-1.0, 0.0, 2.0].into_iter().enumerate() {
                let got = map.evaluate(&[x0, x1]);
                let want = anchors[i * 3 + j];
                for m in 0..MODE_COUNT {
                    assert!((got.metric.frequency_hz[m] / want.metric.frequency_hz[m] - 1.0).abs() < 1e-14);
                }
            }
        }
    }
}
