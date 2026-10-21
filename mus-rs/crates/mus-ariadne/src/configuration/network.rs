//! A lumped acoustic tube network whose modes are computed from its geometry.
//!
//! Each tube is cut into `n` equal sections of length `dx`. Section `i` is one
//! compressible air volume with acoustic compliance `C_i = A_i dx / (rho c^2)`
//! (pressure `p_i` at its centre); neighbouring volumes are joined by an
//! inertive air column `L = rho (dx/2) / A_i + rho (dx/2) / A_(i+1)` carrying
//! volume velocity `u`. A branch tube's first volume joins its trunk's volume
//! through its own half-section column. A radiating end joins the last volume
//! to the outside (`p = 0`) through its half-section plus the unflanged end
//! correction `0.61 r` (GARGANTA `_radiation_impedance`, low `ka` reactance),
//! and radiates through a conductance in parallel. The first volume of tube 0
//! is closed (the source enters there as a volume velocity).
//!
//! Hamiltonian `H = 1/2 sum C p^2 + 1/2 sum L u^2`; equations
//! `C dp/dt = -D^T u`, `L du/dt = D p`, so `C p'' = -K p` with
//! `K = D^T L^-1 D`. The body's modes are the lowest `MODE_COUNT` solutions of
//! `K phi = omega^2 C phi`, `phi^T C phi = 1`, solved exactly (to roundoff) at
//! every configuration point. This is the lossless Webster horn equation
//! discretized on a staggered grid: a law, with the grid's dispersion as its
//! declared approximation (`omega = (2c/dx) sin(k dx/2)` for a uniform tube).
//!
//! Map to Ariadne's normalized modal field (energy `1/2 dt (m^2 + w^2 q^2)`):
//! pressure `p = sqrt(dt) Phi m`, volume velocity `u = sqrt(dt) L^-1 F q` with
//! `F = D Phi`. So Ariadne momentum is modal pressure, and a volume-velocity
//! source `U` at volume `n` adds `sqrt(dt) phi(n) U` to the momentum.
//!
//! Losses, both diagonal in the modes (the modal approximation of
//! non-proportional damping; exact when the loss is uniform):
//! - wall/viscothermal: plane-wave attenuation `alpha` Np/m per tube, as a
//!   shunt `alpha c C` and series `alpha c L` on every element; the per-mode
//!   rate is the energy-weighted Rayleigh quotient
//!   `sigma_k = c/2 (sum alpha C phi^2 + sum alpha F^2 / (L omega_k^2))`;
//! - radiation: conductance `G = A r^2 / (4 rho c l^2)` at the radiating
//!   volume, `l = dx/2 + 0.61 r`, chosen so the parallel (end column, G) load
//!   has exactly the oracle's low-`ka` resistance `(rho c / A) (ka)^2 / 4`
//!   (the same matching `ovillo-tract` uses); per-mode rate
//!   `sigma_k = G phi_k(n)^2 / 2`.

use serde::{Deserialize, Serialize};

use crate::field::MetricFieldState;
use crate::ir::{MetricSpec, MODE_COUNT};
use crate::plan::PlanError;

use super::eigen::{EigenError, SymmetricEigen};
use super::tree_eigen::TreeEigen;
use super::space::ConfigurationSpaceSpec;

/// Largest network (volumes) a body may declare.
pub const TUBE_NETWORK_MAX_NODES: usize = 128;
const LN_1000: f64 = 6.907_755_278_982_137;

/// `constant + sum coefficient * x[coordinate]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AffineSpec {
    pub constant: f64,
    #[serde(default)]
    pub terms: Vec<(usize, f64)>,
}

impl AffineSpec {
    #[must_use]
    pub fn constant(value: f64) -> Self {
        Self {
            constant: value,
            terms: Vec::new(),
        }
    }

    #[must_use]
    pub fn eval(&self, point: &[f64]) -> f64 {
        self.terms
            .iter()
            .fold(self.constant, |acc, &(index, coefficient)| acc + coefficient * point[index])
    }

    fn validate(&self, dims: usize) -> bool {
        self.constant.is_finite()
            && self
                .terms
                .iter()
                .all(|&(index, coefficient)| index < dims && coefficient.is_finite())
    }
}

/// One tube of the network.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TubeSpec {
    pub id: String,
    pub length_m: f64,
    /// Natural log of each section's area (m²) before the area terms; its
    /// length is the section count.
    pub base_log_area_m2: Vec<f64>,
    /// Plane-wave attenuation (Np/m), frequency independent.
    pub attenuation_np_per_m: f64,
}

/// Tube `branch`'s first volume joins volume `trunk_section` of tube `trunk`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TubeBranchSpec {
    pub trunk: usize,
    pub trunk_section: usize,
    pub branch: usize,
}

/// A term of the area function. Section `i` of an `n`-section tube sits at
/// `s = (i + 1/2) / n`; `g(s; c, w) = exp(-((s - c)/w)^2 / 2)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AreaTermSpec {
    /// `ln A += amplitude(x) g(s; center(x), width)`.
    Gaussian {
        tube: usize,
        amplitude: AffineSpec,
        center: AffineSpec,
        width: f64,
    },
    /// `ln A += ln(max(scale(x), floor)) g(s; center, width)`.
    LogScaleGaussian {
        tube: usize,
        scale: AffineSpec,
        floor: f64,
        center: f64,
        width: f64,
    },
    /// A constriction pressed into the wall at a coordinate-driven place:
    /// `A *= max(1 - degree(x) bump(s - position(x)), 0)` with
    /// `bump = 1` within `half_width` and a Gaussian shoulder of
    /// `shoulder_width` outside, so degree 1 closes exactly. Applied to the
    /// family's value before the cone's clamp (the flow aperture sees zero;
    /// the acoustic area stays at the floor).
    Constriction {
        tube: usize,
        position: AffineSpec,
        degree: AffineSpec,
        half_width: f64,
        shoulder_width: f64,
    },
    /// `A[section] = area(x)`, applied after the log terms.
    SectionArea {
        tube: usize,
        section: usize,
        area_m2: AffineSpec,
    },
}

/// The last volume of `tube` radiates to the outside.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadiatingEndSpec {
    pub tube: usize,
    /// Unflanged end correction as a multiple of the opening radius.
    pub end_correction_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TubeNetworkSpec {
    pub speed_of_sound_m_s: f64,
    pub air_density_kg_m3: f64,
    /// Positive area cone; every section is clamped into it.
    pub area_floor_m2: f64,
    pub area_ceiling_m2: f64,
    pub tubes: Vec<TubeSpec>,
    #[serde(default)]
    pub branches: Vec<TubeBranchSpec>,
    #[serde(default)]
    pub area_terms: Vec<AreaTermSpec>,
    pub radiating_ends: Vec<RadiatingEndSpec>,
    /// Tissue contact (POC-3): the narrowest column's flow aperture sets its
    /// inertance, down to a numerical seal. Absent, the plan serializes
    /// exactly as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact: Option<ContactSpec>,
}

/// Contact and closure as configuration. Among the columns between
/// sections `i` and `i + 1` of `tube` (`first_junction <= i <=
/// last_junction`), the one with the smallest flow aperture
/// `A_f = max(min(A_i, A_(i+1)) - contact_area, 0)` (unclamped areas) is the
/// jet column; its inertance is `rho dx / max(A_f, sealed_area)` instead of
/// the cone-clamped one, so a closure is the configuration where `A_f = 0`:
/// the modes are recomputed with the column sealed (the closed cavity gets a
/// mode near 0 Hz that holds its pressure) and the move is transported and
/// billed like any other. With `seal_branch_port`, the first branch's port
/// column does the same with its own aperture. `sealed_area` is a numerical
/// floor (the modes need `omega > 0`), not anatomy.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactSpec {
    pub tube: usize,
    pub first_junction: usize,
    pub last_junction: usize,
    pub contact_area_m2: f64,
    pub sealed_area_m2: f64,
    pub seal_branch_port: bool,
    /// The public rule: every column whose unclamped area falls below the
    /// cone's floor follows it (each half-section at `max(A, sealed_area)`),
    /// continuous with the cone above the floor; open acoustics unchanged.
    /// `false` (POC-3): only the narrowest column, at its flow aperture.
    #[serde(default, skip_serializing_if = "is_false")]
    pub follow_below_floor: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl TubeNetworkSpec {
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.tubes.iter().map(|t| t.base_log_area_m2.len()).sum()
    }

    /// Global volume index of `(tube, section)`.
    #[must_use]
    pub fn node(&self, tube: usize, section: usize) -> usize {
        self.tubes[..tube].iter().map(|t| t.base_log_area_m2.len()).sum::<usize>() + section
    }

    pub(crate) fn validate(
        &self,
        dims: usize,
        sample_rate_hz: f64,
        space: &ConfigurationSpaceSpec,
    ) -> Result<(), PlanError> {
        let bad = |code: &'static str, message: String| Err(PlanError::new(code, message));
        for (name, value) in [
            ("speed_of_sound_m_s", self.speed_of_sound_m_s),
            ("air_density_kg_m3", self.air_density_kg_m3),
            ("area_floor_m2", self.area_floor_m2),
            ("area_ceiling_m2", self.area_ceiling_m2),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return bad("invalid-tube-network-constant", format!("{name} must be finite and positive"));
            }
        }
        if self.area_floor_m2 >= self.area_ceiling_m2 {
            return bad("invalid-tube-network-cone", "area floor must be below the ceiling".into());
        }
        if self.tubes.is_empty() {
            return bad("empty-tube-network", "a tube network needs at least one tube".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for tube in &self.tubes {
            if tube.id.trim().is_empty() || !ids.insert(tube.id.as_str()) {
                return bad("invalid-tube-id", format!("tube ids must be unique and nonempty: {:?}", tube.id));
            }
            if !tube.length_m.is_finite()
                || tube.length_m <= 0.0
                || tube.base_log_area_m2.len() < 2
                || tube.base_log_area_m2.iter().any(|v| !v.is_finite())
                || !tube.attenuation_np_per_m.is_finite()
                || tube.attenuation_np_per_m < 0.0
            {
                return bad("invalid-tube", format!("tube {:?} needs length > 0, >= 2 finite sections, attenuation >= 0", tube.id));
            }
        }
        let nodes = self.node_count();
        if !(MODE_COUNT..=TUBE_NETWORK_MAX_NODES).contains(&nodes) {
            return bad(
                "invalid-tube-network-size",
                format!("a tube network needs {MODE_COUNT}..={TUBE_NETWORK_MAX_NODES} volumes, found {nodes}"),
            );
        }
        // Tree: tube 0 is the root; every other tube is a branch exactly once,
        // of an earlier tube.
        let mut attached = vec![false; self.tubes.len()];
        attached[0] = true;
        for branch in &self.branches {
            if branch.branch == 0
                || branch.branch >= self.tubes.len()
                || branch.trunk >= branch.branch
                || attached[branch.branch]
                || branch.trunk_section >= self.tubes[branch.trunk].base_log_area_m2.len()
            {
                return bad("invalid-tube-branch", format!("branch {branch:?} must attach a later tube once to an earlier tube's section"));
            }
            attached[branch.branch] = true;
        }
        if attached.iter().any(|a| !a) {
            return bad("disconnected-tube-network", "every tube must be attached to the tree".into());
        }
        for term in &self.area_terms {
            let ok = match term {
                AreaTermSpec::Gaussian { tube, amplitude, center, width } => {
                    *tube < self.tubes.len() && amplitude.validate(dims) && center.validate(dims) && width.is_finite() && *width > 0.0
                }
                AreaTermSpec::LogScaleGaussian { tube, scale, floor, center, width } => {
                    *tube < self.tubes.len() && scale.validate(dims) && floor.is_finite() && *floor > 0.0 && center.is_finite() && width.is_finite() && *width > 0.0
                }
                AreaTermSpec::SectionArea { tube, section, area_m2 } => {
                    *tube < self.tubes.len() && *section < self.tubes[*tube].base_log_area_m2.len() && area_m2.validate(dims)
                }
                AreaTermSpec::Constriction { tube, position, degree, half_width, shoulder_width } => {
                    *tube < self.tubes.len() && position.validate(dims) && degree.validate(dims)
                        && half_width.is_finite() && *half_width >= 0.0 && shoulder_width.is_finite() && *shoulder_width > 0.0
                }
            };
            if !ok {
                return bad("invalid-area-term", format!("area term {term:?} is outside the network or the space"));
            }
        }
        if self.radiating_ends.is_empty() {
            return bad("closed-tube-network", "at least one radiating end is required (the network must open to the outside)".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for end in &self.radiating_ends {
            if end.tube >= self.tubes.len() || !seen.insert(end.tube) || !end.end_correction_ratio.is_finite() || end.end_correction_ratio < 0.0 {
                return bad("invalid-radiating-end", format!("radiating end {end:?} is invalid"));
            }
        }
        if let Some(contact) = &self.contact {
            let sections = self.tubes.get(contact.tube).map_or(0, |t| t.base_log_area_m2.len());
            if contact.tube >= self.tubes.len()
                || contact.first_junction > contact.last_junction
                || contact.last_junction + 2 > sections
                || !contact.contact_area_m2.is_finite()
                || contact.contact_area_m2 < 0.0
                || !contact.sealed_area_m2.is_finite()
                || contact.sealed_area_m2 <= 0.0
                || (contact.seal_branch_port && self.branches.is_empty())
            {
                return bad("invalid-contact", format!("contact {contact:?} is outside the network"));
            }
        }
        // Sampled band check: every corner of the box and its centre (up to
        // 8 coordinates); beyond that the centre, the two extreme corners and
        // every axis's two ends with the others at the centre.
        let mut network = TubeNetwork::compile(self, dims);
        let mut frame = network.empty_frame();
        let mut point = vec![0.0; dims];
        let samples = if dims <= 8 { (1usize << dims) + 1 } else { 3 + 2 * dims };
        for sample in 0..samples {
            for (axis, coordinate) in space.coordinates.iter().enumerate() {
                let centre = 0.5 * (coordinate.lower + coordinate.upper);
                point[axis] = if dims <= 8 {
                    if sample == 1usize << dims {
                        centre
                    } else if sample >> axis & 1 == 1 {
                        coordinate.upper
                    } else {
                        coordinate.lower
                    }
                } else {
                    match sample {
                        0 => centre,
                        1 => coordinate.lower,
                        2 => coordinate.upper,
                        k => {
                            let (a, upper) = ((k - 3) / 2, (k - 3) % 2 == 1);
                            if a == axis { if upper { coordinate.upper } else { coordinate.lower } } else { centre }
                        }
                    }
                };
            }
            let edge = std::f64::consts::TAU * 0.48 * sample_rate_hz;
            match network.check_band(&point, &mut frame, edge * edge) {
                Some(true) => {}
                Some(false) => {
                    return Err(PlanError::new("tube-network-band", format!("at {point:?}: the lowest {MODE_COUNT} modes are not all inside (0, 0.48 fs)")));
                }
                None => {
                    network.compute(&point, None, &mut frame).map_err(|error| {
                        PlanError::new("tube-network-modes", format!("at {point:?}: {error:?}"))
                    })?;
                    frame.metric().runtime_check(sample_rate_hz).map_err(|message| {
                        PlanError::new("tube-network-band", format!("at {point:?}: {message}"))
                    })?;
                }
            }
        }
        Ok(())
    }
}

impl MetricSpec {
    fn runtime_check(&self, sample_rate_hz: f64) -> Result<(), String> {
        for mode in 0..MODE_COUNT {
            let f = self.frequency_hz[mode];
            if !f.is_finite() || f <= 0.0 || f >= 0.48 * sample_rate_hz {
                return Err(format!("mode {mode} at {f} Hz is outside (0, 0.48 fs)"));
            }
            if !self.t60_seconds[mode].is_finite() || self.t60_seconds[mode] <= 0.0 {
                return Err(format!("mode {mode} has an invalid T60"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct Element {
    a: usize,
    /// `None`: to the outside (a radiating end).
    b: Option<usize>,
    alpha: f64,
    /// Half-section lengths on the `a` and `b` sides (m).
    half_a: f64,
    half_b: f64,
    /// Radiating end index, for end elements.
    end: Option<usize>,
}

/// Everything the body needs at one configuration point.
#[derive(Debug, Clone)]
pub(crate) struct NetworkFrame {
    pub nodes: usize,
    pub elements: usize,
    pub areas_m2: Vec<f64>,
    /// The area family's value before the positive cone's clamp (floored at
    /// zero): what a flow aperture sees.
    pub unclamped_areas_m2: Vec<f64>,
    pub compliance: Vec<f64>,
    pub inertance: Vec<f64>,
    pub omega2: [f64; MODE_COUNT],
    pub frequency_hz: [f64; MODE_COUNT],
    pub t60_seconds: [f64; MODE_COUNT],
    /// Mode shapes `phi[k * nodes + n]` (Pa per unit modal pressure), C-orthonormal.
    pub phi: Vec<f64>,
    /// The same modes as unit eigenvectors of `C^-1/2 K C^-1/2` (`sqrt(C) phi`).
    pub psi: Vec<f64>,
    /// `F = D Phi`: `flux[k * elements + j]`.
    pub flux: Vec<f64>,
    /// Per radiating end: per-mode amplitude decay rate (1/s).
    pub radiation_sigma: Vec<[f64; MODE_COUNT]>,
    /// With contact: the jet column's junction, element and flow aperture
    /// (m², 0 when closed), and the sealed port's element and aperture.
    pub jet: Option<(usize, usize, f64)>,
    pub port: Option<(usize, f64)>,
    /// Columns sealed at this configuration (flow aperture 0).
    pub sealed: Vec<usize>,
    pub radiation_conductance: Vec<f64>,
}

impl NetworkFrame {
    pub(crate) fn metric(&self) -> MetricSpec {
        MetricSpec {
            frequency_hz: self.frequency_hz,
            t60_seconds: self.t60_seconds,
        }
    }

    /// `phi_k(node)` for every retained mode.
    pub(crate) fn shape_at(&self, node: usize) -> [f64; MODE_COUNT] {
        std::array::from_fn(|k| self.phi[k * self.nodes + node])
    }
}

/// Errors computing a configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkError {
    Eigen(&'static str),
    NonPositiveMode,
    NonFiniteGeometry,
}

/// Compiled network with its eigen workspace (no allocation per point).
#[derive(Debug, Clone)]
pub(crate) struct TubeNetwork {
    spec: TubeNetworkSpec,
    nodes: usize,
    node_tube: Vec<usize>,
    node_position: Vec<f64>,
    node_alpha: Vec<f64>,
    dx: Vec<f64>,
    offsets: Vec<usize>,
    elements: Vec<Element>,
    end_nodes: Vec<usize>,
    eigen: SymmetricEigen,
    tree: Option<TreeEigen>,
    matrix: Vec<f64>,
    values: Vec<f64>,
    psi: Vec<f64>,
    log_area: Vec<f64>,
    /// Columns whose inertance follows a flow aperture (contact).
    column_override: Vec<(usize, f64)>,
    band_probe: Option<f64>,
    band_result: Option<bool>,
    /// Solves certified by the warm tree path / done densely.
    pub(crate) warm_solves: u64,
    pub(crate) dense_solves: u64,
}

impl TubeNetwork {
    pub(crate) fn compile(spec: &TubeNetworkSpec, _dims: usize) -> Self {
        let mut offsets = Vec::with_capacity(spec.tubes.len());
        let mut node_tube = Vec::new();
        let mut node_position = Vec::new();
        let mut node_alpha = Vec::new();
        let mut dx = Vec::with_capacity(spec.tubes.len());
        let mut elements = Vec::new();
        for (index, tube) in spec.tubes.iter().enumerate() {
            let sections = tube.base_log_area_m2.len();
            offsets.push(node_tube.len());
            let step = tube.length_m / sections as f64;
            dx.push(step);
            for section in 0..sections {
                node_tube.push(index);
                node_position.push((section as f64 + 0.5) / sections as f64);
                node_alpha.push(tube.attenuation_np_per_m);
            }
            for section in 0..sections - 1 {
                let a = offsets[index] + section;
                elements.push(Element {
                    a,
                    b: Some(a + 1),
                    alpha: tube.attenuation_np_per_m,
                    half_a: 0.5 * step,
                    half_b: 0.5 * step,
                    end: None,
                });
            }
        }
        for branch in &spec.branches {
            let trunk_node = offsets[branch.trunk] + branch.trunk_section;
            let branch_node = offsets[branch.branch];
            elements.push(Element {
                a: trunk_node,
                b: Some(branch_node),
                alpha: 0.5
                    * (spec.tubes[branch.trunk].attenuation_np_per_m
                        + spec.tubes[branch.branch].attenuation_np_per_m),
                half_a: 0.0,
                half_b: 0.5 * dx[branch.branch],
                end: None,
            });
        }
        let mut end_nodes = Vec::new();
        for (end_index, end) in spec.radiating_ends.iter().enumerate() {
            let node = offsets[end.tube] + spec.tubes[end.tube].base_log_area_m2.len() - 1;
            end_nodes.push(node);
            elements.push(Element {
                a: node,
                b: None,
                alpha: spec.tubes[end.tube].attenuation_np_per_m,
                half_a: 0.5 * dx[end.tube],
                half_b: 0.0,
                end: Some(end_index),
            });
        }
        let nodes = node_tube.len();
        let edges: Vec<(usize, usize)> = elements.iter().filter_map(|e| e.b.map(|b| (e.a, b))).collect();
        let tree = TreeEigen::new(nodes, MODE_COUNT, &edges);
        Self {
            spec: spec.clone(),
            tree,
            column_override: Vec::with_capacity(2),
            band_probe: None,
            band_result: None,
            warm_solves: 0,
            dense_solves: 0,
            nodes,
            node_tube,
            node_position,
            node_alpha,
            dx,
            offsets,
            elements,
            end_nodes,
            eigen: SymmetricEigen::new(nodes, MODE_COUNT),
            matrix: vec![0.0; nodes * nodes],
            values: vec![0.0; MODE_COUNT],
            psi: vec![0.0; MODE_COUNT * nodes],
            log_area: vec![0.0; nodes],
        }
    }

    pub(crate) fn node(&self, tube: usize, section: usize) -> usize {
        self.offsets[tube] + section
    }

    /// Band check by inertia (`None` when the network is not a tree).
    pub(crate) fn check_band(&mut self, point: &[f64], frame: &mut NetworkFrame, max_omega2: f64) -> Option<bool> {
        self.tree.as_ref()?;
        self.band_probe = Some(max_omega2);
        self.band_result = None;
        let computed = self.compute(point, None, frame);
        self.band_probe = None;
        computed.ok()?;
        self.band_result.take()
    }

    pub(crate) fn contact_tube(&self) -> Option<usize> {
        self.spec.contact.map(|c| c.tube)
    }

    pub(crate) fn tube_sections(&self, tube: usize) -> usize {
        self.spec.tubes[tube].base_log_area_m2.len()
    }

    /// Element index of the column between sections `junction` and
    /// `junction + 1` of `tube` (internal columns come first, tube by tube).
    pub(crate) fn junction_element(&self, tube: usize, junction: usize) -> usize {
        self.spec.tubes[..tube].iter().map(|t| t.base_log_area_m2.len() - 1).sum::<usize>() + junction
    }

    /// Element index and branch-side volume of branch `index`.
    pub(crate) fn branch_element(&self, index: usize) -> Option<(usize, usize)> {
        let branch = self.spec.branches.get(index)?;
        let internal: usize = self.spec.tubes.iter().map(|t| t.base_log_area_m2.len() - 1).sum();
        Some((internal + index, self.offsets[branch.branch]))
    }

    pub(crate) fn density(&self) -> f64 {
        self.spec.air_density_kg_m3
    }

    pub(crate) fn speed_of_sound(&self) -> f64 {
        self.spec.speed_of_sound_m_s
    }

    pub(crate) fn end_nodes(&self) -> &[usize] {
        &self.end_nodes
    }

    pub(crate) fn empty_frame(&self) -> NetworkFrame {
        let elements = self.elements.len();
        NetworkFrame {
            nodes: self.nodes,
            elements,
            areas_m2: vec![0.0; self.nodes],
            unclamped_areas_m2: vec![0.0; self.nodes],
            compliance: vec![0.0; self.nodes],
            inertance: vec![0.0; elements],
            omega2: [0.0; MODE_COUNT],
            frequency_hz: [0.0; MODE_COUNT],
            t60_seconds: [0.0; MODE_COUNT],
            phi: vec![0.0; MODE_COUNT * self.nodes],
            psi: vec![0.0; MODE_COUNT * self.nodes],
            flux: vec![0.0; MODE_COUNT * elements],
            radiation_sigma: vec![[0.0; MODE_COUNT]; self.end_nodes.len()],
            jet: None,
            port: None,
            sealed: Vec::with_capacity(8),
            radiation_conductance: vec![0.0; self.end_nodes.len()],
        }
    }

    /// Areas (m²) at `point`, into `out` (clamped into the cone) and
    /// `unclamped` (the family's value, floored at zero).
    pub(crate) fn areas_into(&mut self, point: &[f64], out: &mut [f64], unclamped: &mut [f64]) {
        let spec = &self.spec;
        for (n, value) in self.log_area.iter_mut().enumerate() {
            let tube = self.node_tube[n];
            let section = n - self.offsets[tube];
            *value = spec.tubes[tube].base_log_area_m2[section];
        }
        for term in &spec.area_terms {
            match term {
                AreaTermSpec::Gaussian { tube, amplitude, center, width } => {
                    let amplitude = amplitude.eval(point);
                    let center = center.eval(point);
                    let start = self.offsets[*tube];
                    let end = start + spec.tubes[*tube].base_log_area_m2.len();
                    for n in start..end {
                        self.log_area[n] += amplitude * gaussian(self.node_position[n], center, *width);
                    }
                }
                AreaTermSpec::LogScaleGaussian { tube, scale, floor, center, width } => {
                    let amplitude = scale.eval(point).max(*floor).ln();
                    let start = self.offsets[*tube];
                    let end = start + spec.tubes[*tube].base_log_area_m2.len();
                    for n in start..end {
                        self.log_area[n] += amplitude * gaussian(self.node_position[n], *center, *width);
                    }
                }
                AreaTermSpec::SectionArea { .. } | AreaTermSpec::Constriction { .. } => {}
            }
        }
        for (n, value) in out.iter_mut().enumerate().take(self.nodes) {
            let mut raw = self.log_area[n].exp();
            for term in &spec.area_terms {
                if let AreaTermSpec::Constriction { tube, position, degree, half_width, shoulder_width } = term {
                    if self.node_tube[n] == *tube {
                        let d = degree.eval(point).clamp(0.0, 1.0);
                        if d > 0.0 {
                            let dist = (self.node_position[n] - position.eval(point)).abs();
                            let bump = if dist <= *half_width { 1.0 } else { (-0.5 * ((dist - half_width) / shoulder_width).powi(2)).exp() };
                            raw *= (1.0 - d * bump).max(0.0);
                        }
                    }
                }
            }
            unclamped[n] = raw;
            *value = raw.clamp(spec.area_floor_m2, spec.area_ceiling_m2);
        }
        for term in &spec.area_terms {
            if let AreaTermSpec::SectionArea { tube, section, area_m2 } = term {
                let raw = area_m2.eval(point);
                unclamped[self.offsets[*tube] + section] = raw.max(0.0);
                out[self.offsets[*tube] + section] = raw.clamp(spec.area_floor_m2, spec.area_ceiling_m2);
            }
        }
    }

    /// Compute the frame at `point`. With `previous`, mode signs are aligned
    /// to it (`phi'_k^T C phi_k >= 0`) so index-held coordinates stay continuous
    /// away from eigenvalue crossings.
    pub(crate) fn compute(
        &mut self,
        point: &[f64],
        previous: Option<&NetworkFrame>,
        out: &mut NetworkFrame,
    ) -> Result<(), NetworkError> {
        let n = self.nodes;
        let rho = self.spec.air_density_kg_m3;
        let c = self.spec.speed_of_sound_m_s;
        let mut areas = std::mem::take(&mut out.areas_m2);
        let mut unclamped = std::mem::take(&mut out.unclamped_areas_m2);
        self.areas_into(point, &mut areas, &mut unclamped);
        out.areas_m2 = areas;
        out.unclamped_areas_m2 = unclamped;
        if out.areas_m2.iter().any(|a| !a.is_finite() || *a <= 0.0) {
            return Err(NetworkError::NonFiniteGeometry);
        }
        for node in 0..n {
            out.compliance[node] = out.areas_m2[node] * self.dx[self.node_tube[node]] / (rho * c * c);
        }
        self.column_override.clear();
        out.jet = None;
        out.port = None;
        out.sealed.clear();
        if let Some(contact) = self.spec.contact.filter(|c| c.follow_below_floor) {
            let floor = self.spec.area_floor_m2;
            let unclamped = &out.unclamped_areas_m2;
            let aperture = |node: usize| (unclamped[node] - contact.contact_area_m2).max(0.0);
            let half = |node: usize| unclamped[node].max(contact.sealed_area_m2);
            let mut best = (f64::INFINITY, contact.first_junction);
            for junction in contact.first_junction..=contact.last_junction {
                let a = self.offsets[contact.tube] + junction;
                let open = aperture(a).min(aperture(a + 1));
                let element = self.junction_element(contact.tube, junction);
                if open <= 0.0 {
                    out.sealed.push(element);
                }
                if unclamped[a].min(unclamped[a + 1]) < floor || open <= 0.0 {
                    let e = &self.elements[element];
                    let inertance = rho * (e.half_a / half(a) + e.half_b / half(a + 1));
                    self.column_override.push((element, -inertance));
                }
                if open < best.0 || (open <= 0.0 && best.0 <= 0.0) {
                    best = (open, junction);
                }
            }
            let element = self.junction_element(contact.tube, best.1);
            out.jet = Some((best.1, element, best.0));
            if contact.seal_branch_port {
                if let Some((element, node)) = self.branch_element(0) {
                    let open = aperture(node);
                    if unclamped[node] < floor || open <= 0.0 {
                        let e = &self.elements[element];
                        self.column_override.push((element, -(rho * e.half_b / half(node))));
                    }
                    out.port = Some((element, open));
                    if open <= 0.0 {
                        out.sealed.push(element);
                    }
                }
            }
        } else if let Some(contact) = self.spec.contact {
            let aperture = |node: usize| (out.unclamped_areas_m2[node] - contact.contact_area_m2).max(0.0);
            // Every column at contact is sealed; the jet is the narrowest
            // column (the most downstream one among sealed ties, so the
            // seal nearest the opening is the one that releases).
            let mut best = (f64::INFINITY, contact.first_junction);
            for junction in contact.first_junction..=contact.last_junction {
                let a = self.offsets[contact.tube] + junction;
                let open = aperture(a).min(aperture(a + 1));
                if open <= 0.0 {
                    let element = self.junction_element(contact.tube, junction);
                    self.column_override.push((element, contact.sealed_area_m2));
                    out.sealed.push(element);
                }
                if open < best.0 || (open <= 0.0 && best.0 <= 0.0) {
                    best = (open, junction);
                }
            }
            let element = self.junction_element(contact.tube, best.1);
            if best.0 > 0.0 {
                self.column_override.push((element, best.0));
            }
            out.jet = Some((best.1, element, best.0));
            if contact.seal_branch_port {
                if let Some((element, node)) = self.branch_element(0) {
                    let open = aperture(node);
                    self.column_override.push((element, open.max(contact.sealed_area_m2)));
                    out.port = Some((element, open));
                    if open <= 0.0 {
                        out.sealed.push(element);
                    }
                }
            }
        }
        // Assemble S = C^-1/2 K C^-1/2 sparsely: diagonal + one entry per column.
        let mut diagonal = std::mem::take(&mut self.log_area);
        diagonal.iter_mut().for_each(|v| *v = 0.0);
        if let Some(tree) = &mut self.tree {
            tree.off.iter_mut().for_each(|v| *v = 0.0);
        }
        let mut dense_needed = self.tree.is_none() || previous.is_none();
        for (j, element) in self.elements.iter().enumerate() {
            let a_area = out.areas_m2[element.a];
            let mut inertance = rho * element.half_a / a_area;
            if let Some(b) = element.b {
                inertance += rho * element.half_b / out.areas_m2[b];
            }
            if let Some(end) = element.end {
                let radius = (a_area / std::f64::consts::PI).sqrt();
                inertance += rho * self.spec.radiating_ends[end].end_correction_ratio * radius / a_area;
            }
            if let Some(&(_, value)) = self.column_override.iter().find(|(e, _)| *e == j) {
                inertance = if value < 0.0 {
                    // Public rule: an inertance computed below.
                    -value
                } else {
                    // POC-3: the column at its flow aperture (a branch port has no trunk half).
                    rho * (element.half_a + element.half_b) / value
                };
            }
            out.inertance[j] = inertance;
            let ca = out.compliance[element.a];
            diagonal[element.a] += 1.0 / (inertance * ca);
            if let Some(b) = element.b {
                let cb = out.compliance[b];
                diagonal[b] += 1.0 / (inertance * cb);
                let off = -1.0 / (inertance * (ca * cb).sqrt());
                if let Some(tree) = &mut self.tree {
                    if tree.parent(element.a) == Some(b) {
                        tree.off[element.a] += off;
                    } else {
                        tree.off[b] += off;
                    }
                }
            }
        }
        if let (Some(limit), Some(tree)) = (self.band_probe, self.tree.as_mut()) {
            // Planner band check: Sylvester inertia counts on the tree's
            // LDL^T, no eigenvectors: no eigenvalue at or below 0, and at
            // least MODE_COUNT below the band edge.
            tree.diagonal.copy_from_slice(&diagonal);
            let positive = tree.count_below(0.0) == 0;
            let inside = tree.count_below(limit) >= MODE_COUNT;
            self.band_result = Some(positive && inside);
            self.log_area = diagonal;
            return Ok(());
        }
        if !dense_needed {
            let tree = self.tree.as_mut().expect("tree");
            tree.diagonal.copy_from_slice(&diagonal);
            let warm = &previous.expect("previous").psi;
            if tree.refine(warm, &mut self.values, &mut self.psi) {
                self.warm_solves += 1;
            } else {
                dense_needed = true;
            }
        }
        if dense_needed {
            self.matrix.iter_mut().for_each(|v| *v = 0.0);
            for node in 0..n {
                self.matrix[node * n + node] = diagonal[node];
            }
            for (j, element) in self.elements.iter().enumerate() {
                if let Some(b) = element.b {
                    let off = -1.0 / (out.inertance[j] * (out.compliance[element.a] * out.compliance[b]).sqrt());
                    self.matrix[element.a * n + b] += off;
                    self.matrix[b * n + element.a] += off;
                }
            }
            self.eigen
                .lowest(&self.matrix, &mut self.values, &mut self.psi)
                .map_err(|error| match error {
                    EigenError::NonFinite => NetworkError::Eigen("non-finite"),
                    EigenError::NoConvergence => NetworkError::Eigen("no convergence"),
                })?;
            self.dense_solves += 1;
        }
        self.log_area = diagonal;
        for k in 0..MODE_COUNT {
            let omega2 = self.values[k];
            if !omega2.is_finite() || omega2 <= 0.0 {
                return Err(NetworkError::NonPositiveMode);
            }
            out.omega2[k] = omega2;
            out.frequency_hz[k] = omega2.sqrt() / std::f64::consts::TAU;
            for node in 0..n {
                out.psi[k * n + node] = self.psi[k * n + node];
                out.phi[k * n + node] = self.psi[k * n + node] / out.compliance[node].sqrt();
            }
            if let Some(previous) = previous {
                let overlap: f64 = (0..n)
                    .map(|node| out.compliance[node] * out.phi[k * n + node] * previous.phi[k * n + node])
                    .sum();
                if overlap < 0.0 {
                    for node in 0..n {
                        out.phi[k * n + node] = -out.phi[k * n + node];
                        out.psi[k * n + node] = -out.psi[k * n + node];
                    }
                }
            } else {
                // Deterministic sign: positive at the first volume with weight.
                let lead = (0..n).map(|node| out.phi[k * n + node]).find(|v| v.abs() > 1e-9 * out.phi[k * n..(k + 1) * n].iter().fold(0.0_f64, |m, v| m.max(v.abs()))).unwrap_or(1.0);
                if lead < 0.0 {
                    for node in 0..n {
                        out.phi[k * n + node] = -out.phi[k * n + node];
                        out.psi[k * n + node] = -out.psi[k * n + node];
                    }
                }
            }
            let elements = self.elements.len();
            for (j, element) in self.elements.iter().enumerate() {
                let a = out.phi[k * n + element.a];
                let b = element.b.map_or(0.0, |b| out.phi[k * n + b]);
                out.flux[k * elements + j] = a - b;
            }
            // Wall loss: energy-weighted Rayleigh quotient of the attenuation.
            let mut potential = 0.0;
            for node in 0..n {
                let p = out.phi[k * n + node];
                potential += self.node_alpha[node] * out.compliance[node] * p * p;
            }
            let mut kinetic = 0.0;
            for (j, element) in self.elements.iter().enumerate() {
                let f = out.flux[k * elements + j];
                kinetic += element.alpha * f * f / out.inertance[j];
            }
            let sigma = 0.5 * c * (potential + kinetic / omega2);
            out.t60_seconds[k] = if sigma > 0.0 { LN_1000 / sigma } else { f64::MAX };
        }
        for (e, &node) in self.end_nodes.iter().enumerate() {
            let area = out.areas_m2[node];
            let radius = (area / std::f64::consts::PI).sqrt();
            let tube = self.node_tube[node];
            let column = 0.5 * self.dx[tube] + self.spec.radiating_ends[e].end_correction_ratio * radius;
            let conductance = area * radius * radius / (4.0 * rho * c * column * column);
            out.radiation_conductance[e] = conductance;
            for k in 0..MODE_COUNT {
                let p = out.phi[k * n + node];
                out.radiation_sigma[e][k] = 0.5 * conductance * p * p;
            }
        }
        Ok(())
    }
}

/// Energies of a mass-momentum transport.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct MassMomentumTransport {
    /// Stored energy of the moved state in the new geometry, before truncation.
    pub physical_energy_after: f64,
}

/// Hold excess air volume `C p` and air momentum `L u` while the geometry
/// moves from `old` to `new`; project onto `new`'s retained modes in the
/// energy metric. Writes the new modal state into `state`.
pub(crate) fn mass_momentum_transport(
    old: &NetworkFrame,
    new: &NetworkFrame,
    dt: f64,
    state: &mut MetricFieldState<MODE_COUNT>,
    excess: &mut [f64],
    momentum: &mut [f64],
) -> MassMomentumTransport {
    let n = old.nodes;
    let j_count = old.elements;
    // Excess volume / sqrt(dt):  C Phi m.   Air momentum / sqrt(dt):  F q.
    for node in 0..n {
        let mut acc = 0.0;
        for k in 0..MODE_COUNT {
            acc += old.phi[k * n + node] * state.momentum[k];
        }
        excess[node] = old.compliance[node] * acc;
    }
    for j in 0..j_count {
        let mut acc = 0.0;
        for k in 0..MODE_COUNT {
            acc += old.flux[k * j_count + j] * state.displacement[k];
        }
        momentum[j] = acc;
    }
    // A sealed contact carries no air: what its numerical seal accumulated
    // (L u with L huge) is the wall's reaction, not momentum. Drop it; its
    // energy in the old geometry (tiny, L is huge) goes to the contact.
    for &j in &old.sealed {
        momentum[j] = 0.0;
    }
    let mut physical = 0.0;
    for node in 0..n {
        physical += excess[node] * excess[node] / new.compliance[node];
    }
    for j in 0..j_count {
        physical += momentum[j] * momentum[j] / new.inertance[j];
    }
    for k in 0..MODE_COUNT {
        let mut m = 0.0;
        for node in 0..n {
            m += new.phi[k * n + node] * excess[node];
        }
        let mut q = 0.0;
        for j in 0..j_count {
            q += new.flux[k * j_count + j] * momentum[j] / new.inertance[j];
        }
        state.momentum[k] = m;
        state.displacement[k] = q / new.omega2[k];
    }
    MassMomentumTransport {
        physical_energy_after: 0.5 * dt * physical,
    }
}

#[inline]
fn gaussian(x: f64, center: f64, width: f64) -> f64 {
    let z = (x - center) / width;
    (-0.5 * z * z).exp()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configuration::space::{ConfigurationCoordinate, ConfigurationMapSpec, ConnectionPolicy};

    pub(crate) fn uniform_tube(sections: usize, length_m: f64, area_m2: f64, end_correction: f64) -> TubeNetworkSpec {
        TubeNetworkSpec {
            speed_of_sound_m_s: 343.0,
            air_density_kg_m3: 1.204,
            area_floor_m2: 2.5e-6,
            area_ceiling_m2: 1.8e-3,
            tubes: vec![TubeSpec {
                id: "tube".into(),
                length_m,
                base_log_area_m2: vec![area_m2.ln(); sections],
                attenuation_np_per_m: 0.0,
            }],
            branches: vec![],
            area_terms: vec![],
            radiating_ends: vec![RadiatingEndSpec { tube: 0, end_correction_ratio: end_correction }],
            contact: None,
        }
    }

    /// GARGANTA G1.2 (`SPEAKER-THROAT-EVOLUTION.md`): a uniform closed-open
    /// tube approaches the quarter-wave series. Without end correction the
    /// staggered grid's modes are exactly `p_i = cos(k (i + 1/2) dx)`,
    /// `k = (2m + 1) pi / (2 L)`, `omega = (2c/dx) sin(k dx/2)`.
    #[test]
    fn uniform_closed_open_tube_is_the_quarter_wave_series() {
        let length = 0.17;
        let sections = 96;
        let spec = uniform_tube(sections, length, 3.0e-4, 0.0);
        let mut network = TubeNetwork::compile(&spec, 1);
        let mut frame = network.empty_frame();
        network.compute(&[0.0], None, &mut frame).unwrap();
        let dx = length / sections as f64;
        for k in 0..6 {
            let continuum = (2.0 * k as f64 + 1.0) * 343.0 / (4.0 * length);
            let wavenumber = std::f64::consts::TAU * continuum / 343.0;
            let dispersion = (wavenumber * dx / 2.0).sin() / (wavenumber * dx / 2.0);
            let measured = frame.frequency_hz[k];
            // The staggered grid's closed form: omega = (2c/dx) sin(k dx / 2).
            assert!(
                (measured / (continuum * dispersion) - 1.0).abs() < 1e-10,
                "mode {k}: {measured} vs {continuum} x {dispersion}"
            );
        }
        // Equal areas: the modes are C-orthonormal.
        let n = frame.nodes;
        for k in 0..MODE_COUNT {
            for l in 0..MODE_COUNT {
                let dot: f64 = (0..n).map(|i| frame.compliance[i] * frame.phi[k * n + i] * frame.phi[l * n + i]).sum();
                assert!((dot - if k == l { 1.0 } else { 0.0 }).abs() < 1e-10, "{k},{l}: {dot}");
            }
        }
    }

    #[test]
    fn mass_momentum_transport_is_identity_at_a_fixed_point_and_bills_a_move() {
        let mut spec = uniform_tube(40, 0.16, 3.0e-4, 0.61);
        spec.area_terms.push(AreaTermSpec::Gaussian {
            tube: 0,
            amplitude: AffineSpec { constant: 0.0, terms: vec![(0, -2.0)] },
            center: AffineSpec::constant(0.6),
            width: 0.1,
        });
        let space = ConfigurationSpaceSpec {
            coordinates: vec![ConfigurationCoordinate { name: "c".into(), lower: 0.0, upper: 1.0, initial: 0.0 }],
            map: ConfigurationMapSpec::TubeNetwork { network: spec.clone() },
            connection: ConnectionPolicy::MassMomentum { disposal: super::super::space::DisposalPolicy::Dissipate },
        };
        space.validate(48_000.0).unwrap();
        let mut network = TubeNetwork::compile(&spec, 1);
        let mut a = network.empty_frame();
        let mut b = network.empty_frame();
        network.compute(&[0.0], None, &mut a).unwrap();
        network.compute(&[0.0], Some(&a.clone()), &mut b).unwrap();
        let dt = 1.0 / 48_000.0;
        let mut state = MetricFieldState::<MODE_COUNT>::default();
        for k in 0..MODE_COUNT {
            state.momentum[k] = (k as f64 * 0.7).sin();
            state.displacement[k] = (k as f64 * 1.3).cos() / (a.omega2[k].sqrt());
        }
        let before = state;
        let mut excess = vec![0.0; a.nodes];
        let mut momentum = vec![0.0; a.elements];
        mass_momentum_transport(&a, &b, dt, &mut state, &mut excess, &mut momentum);
        for k in 0..MODE_COUNT {
            assert!((state.momentum[k] - before.momentum[k]).abs() < 1e-9);
            assert!((state.displacement[k] - before.displacement[k]).abs() < 1e-9 / a.omega2[k].sqrt());
        }
        // A real move: physical energy after >= retained energy after (Bessel).
        let mut c = network.empty_frame();
        network.compute(&[0.8], Some(&b), &mut c).unwrap();
        let moved = mass_momentum_transport(&b, &c, dt, &mut state, &mut excess, &mut momentum);
        let retained: f64 = 0.5 * dt * (0..MODE_COUNT).map(|k| state.momentum[k].powi(2) + c.omega2[k] * state.displacement[k].powi(2)).sum::<f64>();
        assert!(moved.physical_energy_after >= retained * (1.0 - 1e-12), "{} < {}", moved.physical_energy_after, retained);
    }
}

#[cfg(test)]
mod warm_tests {
    use super::*;

    fn branched() -> TubeNetworkSpec {
        let mut spec = tests_support::uniform(30, 0.16, 3.0e-4);
        spec.tubes.push(TubeSpec { id: "side".into(), length_m: 0.1, base_log_area_m2: vec![(1.0e-4f64).ln(); 16], attenuation_np_per_m: 1.0 });
        spec.branches.push(TubeBranchSpec { trunk: 0, trunk_section: 12, branch: 1 });
        spec.radiating_ends.push(RadiatingEndSpec { tube: 1, end_correction_ratio: 0.61 });
        spec.area_terms.push(AreaTermSpec::Gaussian { tube: 0, amplitude: AffineSpec { constant: 0.0, terms: vec![(0, -3.0)] }, center: AffineSpec::constant(0.7), width: 0.1 });
        spec.area_terms.push(AreaTermSpec::SectionArea { tube: 1, section: 0, area_m2: AffineSpec { constant: 2.5e-6, terms: vec![(1, 1.0e-4)] } });
        spec
    }

    pub(super) mod tests_support {
        use super::super::*;
        pub fn uniform(sections: usize, length_m: f64, area_m2: f64) -> TubeNetworkSpec {
            TubeNetworkSpec {
                speed_of_sound_m_s: 343.0,
                air_density_kg_m3: 1.204,
                area_floor_m2: 2.5e-6,
                area_ceiling_m2: 1.8e-3,
                tubes: vec![TubeSpec { id: "t".into(), length_m, base_log_area_m2: vec![area_m2.ln(); sections], attenuation_np_per_m: 0.45 }],
                branches: vec![],
                area_terms: vec![],
                radiating_ends: vec![RadiatingEndSpec { tube: 0, end_correction_ratio: 0.61 }],
                contact: None,
            }
        }
    }

    /// The warm tree path and the dense path agree to roundoff along a walk
    /// that opens the side branch through avoided crossings.
    #[test]
    fn warm_tree_solves_match_dense_solves_along_a_walk() {
        let spec = branched();
        let mut warm_net = TubeNetwork::compile(&spec, 2);
        let mut dense_net = TubeNetwork::compile(&spec, 2);
        let mut previous = warm_net.empty_frame();
        warm_net.compute(&[0.0, 0.0], None, &mut previous).unwrap();
        let mut worst_f = 0.0_f64;
        let mut worst_shape = 0.0_f64;
        let mut worst_orth = 0.0_f64;
        for step in 1..=400 {
            let t = step as f64 / 400.0;
            let point = [t, (2.0 * t).min(1.0)];
            let mut warm = warm_net.empty_frame();
            warm_net.compute(&point, Some(&previous), &mut warm).unwrap();
            let mut dense = dense_net.empty_frame();
            dense_net.compute(&point, None, &mut dense).unwrap();
            let n = warm.nodes;
            for k in 0..MODE_COUNT {
                worst_f = worst_f.max((warm.frequency_hz[k] / dense.frequency_hz[k] - 1.0).abs());
                let dot: f64 = (0..n).map(|i| warm.psi[k * n + i] * dense.psi[k * n + i]).sum();
                worst_shape = worst_shape.max(1.0 - dot.abs());
            }
            let mut orth = 0.0_f64;
            for k in 0..MODE_COUNT {
                for l in 0..k {
                    let dot: f64 = (0..n).map(|i| warm.psi[k * n + i] * warm.psi[l * n + i]).sum();
                    orth = orth.max(dot.abs());
                }
            }
            worst_orth = worst_orth.max(orth);
            previous = warm;
        }
        println!("warm {} dense-fallback {} | worst relative frequency {worst_f:e}, worst 1-|<psi_w,psi_d>| {worst_shape:e}, worst warm orthogonality {worst_orth:e}", warm_net.warm_solves, warm_net.dense_solves);
        assert!(worst_orth < 1e-12, "{worst_orth}");
        assert!(warm_net.warm_solves > 300, "warm path should certify most steps");
        assert!(worst_f < 1e-11, "{worst_f}");
        assert!(worst_shape < 1e-10, "{worst_shape}");
    }
}
