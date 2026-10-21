//! Local persistent constitutive media for learned and authored Ariadne bodies.
//!
//! The law is intentionally smaller than a complete [`crate::World`]. It owns
//! one fixed-capacity local graph, persistent signed wave state, non-negative
//! evidence state, finite reservoirs, ordered local transport, passive loss,
//! and non-loading observations. It owns no listener, shared room, optimizer,
//! task labels, or private work ledger.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const LOCAL_MEDIUM_SCHEMA_VERSION: &str = "ariadne-local-medium-v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMediumSiteSpec {
    pub id: String,
    pub wave_metric: f64,
    pub wave_decay_per_second: f64,
    pub evidence_decay_per_second: f64,
    pub reservoir_capacity: f64,
    pub reservoir_leak_per_second: f64,
    pub active_rate_per_second: f64,
    pub active_threshold: f64,
    pub active_target_wave_energy: f64,
    #[serde(default)]
    pub initial_wave: f64,
    #[serde(default)]
    pub initial_evidence: f64,
    #[serde(default)]
    pub initial_reservoir: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMediumEdgeSpec {
    pub id: String,
    pub left: usize,
    pub right: usize,
    pub wave_rate_radians_per_second: f64,
    pub evidence_rate_per_second: f64,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMediumSourceSpec {
    pub id: String,
    pub site: usize,
    pub wave_gain: f64,
    pub evidence_gain: f64,
    pub work_gain: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMediumObserverSpec {
    pub id: String,
    pub wave_weights: Vec<f64>,
    pub evidence_weights: Vec<f64>,
    pub reservoir_weights: Vec<f64>,
    #[serde(default)]
    pub bias: f64,
}

/// Content-addressed fixed-capacity local constitution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMediumPlan {
    pub schema_version: String,
    pub name: String,
    pub sample_rate_hz: f64,
    pub sites: Vec<LocalMediumSiteSpec>,
    #[serde(default)]
    pub edges: Vec<LocalMediumEdgeSpec>,
    #[serde(default)]
    pub sources: Vec<LocalMediumSourceSpec>,
    #[serde(default)]
    pub observers: Vec<LocalMediumObserverSpec>,
}

impl LocalMediumPlan {
    pub fn validate(&self) -> Result<(), LocalMediumError> {
        if self.schema_version != LOCAL_MEDIUM_SCHEMA_VERSION {
            return Err(LocalMediumError::InvalidPlan(format!(
                "expected schema {LOCAL_MEDIUM_SCHEMA_VERSION}, found {}",
                self.schema_version
            )));
        }
        if self.name.trim().is_empty() {
            return Err(LocalMediumError::InvalidPlan(
                "local-medium name must be nonempty".to_owned(),
            ));
        }
        if !self.sample_rate_hz.is_finite() || !(1.0..=1_000_000.0).contains(&self.sample_rate_hz) {
            return Err(LocalMediumError::InvalidPlan(
                "sample_rate_hz must be finite in [1, 1_000_000]".to_owned(),
            ));
        }
        if self.sites.is_empty() {
            return Err(LocalMediumError::InvalidPlan(
                "at least one local-medium site is required".to_owned(),
            ));
        }

        let mut site_ids = HashSet::new();
        for (index, site) in self.sites.iter().enumerate() {
            validate_site(site, index)?;
            if !site_ids.insert(site.id.as_str()) {
                return Err(LocalMediumError::InvalidPlan(format!(
                    "duplicate site id {:?}",
                    site.id
                )));
            }
        }

        let mut edge_ids = HashSet::new();
        for (index, edge) in self.edges.iter().enumerate() {
            if edge.id.trim().is_empty() || !edge_ids.insert(edge.id.as_str()) {
                return Err(LocalMediumError::InvalidPlan(format!(
                    "edge {index} has an empty or duplicate id"
                )));
            }
            if edge.left == edge.right
                || edge.left >= self.sites.len()
                || edge.right >= self.sites.len()
            {
                return Err(LocalMediumError::InvalidPlan(format!(
                    "edge {:?} has invalid endpoints",
                    edge.id
                )));
            }
            if !edge.wave_rate_radians_per_second.is_finite()
                || !edge.evidence_rate_per_second.is_finite()
                || edge.evidence_rate_per_second < 0.0
            {
                return Err(LocalMediumError::InvalidPlan(format!(
                    "edge {:?} has invalid transport coefficients",
                    edge.id
                )));
            }
        }

        let mut source_ids = HashSet::new();
        for (index, source) in self.sources.iter().enumerate() {
            if source.id.trim().is_empty() || !source_ids.insert(source.id.as_str()) {
                return Err(LocalMediumError::InvalidPlan(format!(
                    "source {index} has an empty or duplicate id"
                )));
            }
            if source.site >= self.sites.len() {
                return Err(LocalMediumError::InvalidPlan(format!(
                    "source {:?} references unknown site {}",
                    source.id, source.site
                )));
            }
            if [source.wave_gain, source.evidence_gain, source.work_gain]
                .iter()
                .any(|value| !value.is_finite())
            {
                return Err(LocalMediumError::InvalidPlan(format!(
                    "source {:?} has non-finite gain",
                    source.id
                )));
            }
        }

        let mut observer_ids = HashSet::new();
        for (index, observer) in self.observers.iter().enumerate() {
            if observer.id.trim().is_empty() || !observer_ids.insert(observer.id.as_str()) {
                return Err(LocalMediumError::InvalidPlan(format!(
                    "observer {index} has an empty or duplicate id"
                )));
            }
            for (name, weights) in [
                ("wave", &observer.wave_weights),
                ("evidence", &observer.evidence_weights),
                ("reservoir", &observer.reservoir_weights),
            ] {
                if weights.len() != self.sites.len()
                    || weights.iter().any(|value| !value.is_finite())
                {
                    return Err(LocalMediumError::InvalidPlan(format!(
                        "observer {:?} has invalid {name} weights",
                        observer.id
                    )));
                }
            }
            if !observer.bias.is_finite() {
                return Err(LocalMediumError::InvalidPlan(format!(
                    "observer {:?} has a non-finite bias",
                    observer.id
                )));
            }
        }
        Ok(())
    }

    /// SHA-256 of the plan's canonical JSON: the bytes the Python reference
    /// hashes (`json.dumps(plan, sort_keys=True, separators=(",", ":"),
    /// ensure_ascii=False)`), so a plan has one content address in both
    /// runtimes. serde_json alone spells some floats differently from Python
    /// (`-0.00004658216673761258` vs `-4.658216673761258e-05`).
    pub fn digest(&self) -> Result<String, LocalMediumError> {
        self.validate()?;
        let value = serde_json::to_value(self)
            .map_err(|error| LocalMediumError::Serialization(error.to_string()))?;
        let mut encoded = Vec::new();
        write_python_canonical_json(&value, &mut encoded)?;
        Ok(format!("{:x}", Sha256::digest(encoded)))
    }
}

/// Python's `json.dumps(..., sort_keys=True, separators=(",", ":"),
/// ensure_ascii=False)` for the values a plan serializes to.
fn write_python_canonical_json(
    value: &serde_json::Value,
    out: &mut Vec<u8>,
) -> Result<(), LocalMediumError> {
    use serde_json::Value;
    match value {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(flag) => out.extend_from_slice(if *flag { b"true" } else { b"false" }),
        Value::Number(number) => {
            if number.is_f64() {
                let float = number.as_f64().ok_or_else(|| {
                    LocalMediumError::Serialization("non-finite number".to_owned())
                })?;
                out.extend_from_slice(python_float_repr(float)?.as_bytes());
            } else {
                out.extend_from_slice(number.to_string().as_bytes());
            }
        }
        Value::String(text) => {
            let quoted = serde_json::to_string(text)
                .map_err(|error| LocalMediumError::Serialization(error.to_string()))?;
            out.extend_from_slice(quoted.as_bytes());
        }
        Value::Array(items) => {
            out.push(b'[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                write_python_canonical_json(item, out)?;
            }
            out.push(b']');
        }
        Value::Object(map) => {
            // serde_json's default map is ordered by key, as `sort_keys` is.
            out.push(b'{');
            for (index, (key, item)) in map.iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                let quoted = serde_json::to_string(key)
                    .map_err(|error| LocalMediumError::Serialization(error.to_string()))?;
                out.extend_from_slice(quoted.as_bytes());
                out.push(b':');
                write_python_canonical_json(item, out)?;
            }
            out.push(b'}');
        }
    }
    Ok(())
}

/// CPython's `repr(float)`: the shortest round-trip digits, in positional
/// form when the decimal point falls at -3..=16 digits and in `d.ddde±XX`
/// form otherwise.
fn python_float_repr(value: f64) -> Result<String, LocalMediumError> {
    if !value.is_finite() {
        return Err(LocalMediumError::Serialization(
            "non-finite number in a plan".to_owned(),
        ));
    }
    if value == 0.0 {
        return Ok(if value.is_sign_negative() {
            "-0.0"
        } else {
            "0.0"
        }
        .to_owned());
    }
    // Rust's `{:e}` writes the same shortest round-trip digits.
    let scientific = format!("{:e}", value.abs());
    let (mantissa, exponent) = scientific
        .split_once('e')
        .ok_or_else(|| LocalMediumError::Serialization(scientific.clone()))?;
    let exponent: i32 = exponent
        .parse()
        .map_err(|_| LocalMediumError::Serialization(scientific.clone()))?;
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let point = exponent + 1;
    let mut text = String::new();
    if value.is_sign_negative() {
        text.push('-');
    }
    if point <= -4 || point > 16 {
        text.push_str(&digits[..1]);
        if digits.len() > 1 {
            text.push('.');
            text.push_str(&digits[1..]);
        }
        let shown = point - 1;
        text.push('e');
        text.push(if shown < 0 { '-' } else { '+' });
        text.push_str(&format!("{:02}", shown.unsigned_abs()));
    } else if point <= 0 {
        text.push_str("0.");
        text.push_str(&"0".repeat(point.unsigned_abs() as usize));
        text.push_str(&digits);
    } else if (point as usize) < digits.len() {
        text.push_str(&digits[..point as usize]);
        text.push('.');
        text.push_str(&digits[point as usize..]);
    } else {
        text.push_str(&digits);
        text.push_str(&"0".repeat(point as usize - digits.len()));
        text.push_str(".0");
    }
    Ok(text)
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMediumSourceDrive {
    pub source: usize,
    #[serde(default)]
    pub wave: f64,
    #[serde(default)]
    pub evidence: f64,
    #[serde(default)]
    pub work_rate: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMediumInputFrame {
    #[serde(default)]
    pub drives: Vec<LocalMediumSourceDrive>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMediumState {
    pub wave: Vec<f64>,
    pub evidence: Vec<f64>,
    pub reservoir: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMediumStepReceipt {
    pub energy_before: f64,
    pub energy_after: f64,
    pub source_work: f64,
    pub active_transfer: f64,
    pub transport_residual: f64,
    pub positive_mass_residual: f64,
    pub transport_loss: f64,
    pub passive_loss: f64,
    pub internal_loss: f64,
    pub closure_residual: f64,
    pub observation: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMediumDamageReceipt {
    pub edge_index: usize,
    pub enabled_before: bool,
    pub enabled_after: bool,
    pub state_energy_before: f64,
    pub state_energy_after: f64,
    pub state_energy_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalMediumReconfigurationReceipt {
    pub site_index: usize,
    pub site_id: String,
    pub configuration_work: f64,
    pub energy_before: f64,
    pub energy_after: f64,
    pub closure_residual: f64,
}

#[derive(Debug, Error)]
pub enum LocalMediumError {
    #[error("invalid local-medium plan: {0}")]
    InvalidPlan(String),
    #[error("unknown local-medium source index {0}")]
    UnknownSource(usize),
    #[error("unknown local-medium edge index {0}")]
    UnknownEdge(usize),
    #[error("unknown local-medium site index {0}")]
    UnknownSite(usize),
    #[error("local medium entered a non-finite or negative state")]
    NonFiniteState,
    #[error("local-medium serialization failed: {0}")]
    Serialization(String),
}

/// Fixed-capacity persistent local material/computational medium.
pub struct LocalConstitutiveMedium {
    plan: LocalMediumPlan,
    dt: f64,
    state: LocalMediumState,
    edge_enabled: Vec<bool>,
}

impl LocalConstitutiveMedium {
    pub fn from_plan(plan: LocalMediumPlan) -> Result<Self, LocalMediumError> {
        plan.validate()?;
        let state = LocalMediumState {
            wave: plan.sites.iter().map(|site| site.initial_wave).collect(),
            evidence: plan
                .sites
                .iter()
                .map(|site| site.initial_evidence)
                .collect(),
            reservoir: plan
                .sites
                .iter()
                .map(|site| site.initial_reservoir.min(site.reservoir_capacity))
                .collect(),
        };
        let edge_enabled = plan.edges.iter().map(|edge| edge.enabled).collect();
        let dt = plan.sample_rate_hz.recip();
        let medium = Self {
            plan,
            dt,
            state,
            edge_enabled,
        };
        medium.ensure_finite_state()?;
        Ok(medium)
    }

    #[must_use]
    pub fn plan(&self) -> &LocalMediumPlan {
        &self.plan
    }

    #[must_use]
    pub fn state(&self) -> &LocalMediumState {
        &self.state
    }

    #[must_use]
    pub fn stored_energy(&self) -> f64 {
        self.plan
            .sites
            .iter()
            .enumerate()
            .map(|(index, site)| {
                0.5 * site.wave_metric * self.state.wave[index] * self.state.wave[index]
                    + 0.5 * self.state.evidence[index] * self.state.evidence[index]
                    + self.state.reservoir[index]
            })
            .sum()
    }

    #[must_use]
    pub fn positive_mass(&self) -> f64 {
        self.state.evidence.iter().sum()
    }

    pub fn source_index(&self, id: &str) -> Option<usize> {
        self.plan.sources.iter().position(|source| source.id == id)
    }

    pub fn observer_index(&self, id: &str) -> Option<usize> {
        self.plan
            .observers
            .iter()
            .position(|observer| observer.id == id)
    }

    pub fn observe(&self) -> Vec<f64> {
        self.plan
            .observers
            .iter()
            .map(|observer| {
                let mut value = observer.bias;
                for index in 0..self.plan.sites.len() {
                    value += observer.wave_weights[index] * self.state.wave[index];
                    value += observer.evidence_weights[index] * self.state.evidence[index];
                    value += observer.reservoir_weights[index] * self.state.reservoir[index];
                }
                value
            })
            .collect()
    }

    pub fn set_edge_enabled(
        &mut self,
        edge_index: usize,
        enabled: bool,
    ) -> Result<LocalMediumDamageReceipt, LocalMediumError> {
        let previous = *self
            .edge_enabled
            .get(edge_index)
            .ok_or(LocalMediumError::UnknownEdge(edge_index))?;
        let energy_before = self.stored_energy();
        self.edge_enabled[edge_index] = enabled;
        let energy_after = self.stored_energy();
        Ok(LocalMediumDamageReceipt {
            edge_index,
            enabled_before: previous,
            enabled_after: enabled,
            state_energy_before: energy_before,
            state_energy_after: energy_after,
            state_energy_delta: energy_after - energy_before,
        })
    }

    pub fn reconfigure_site(
        &mut self,
        site_index: usize,
        replacement: LocalMediumSiteSpec,
    ) -> Result<LocalMediumReconfigurationReceipt, LocalMediumError> {
        let current = self
            .plan
            .sites
            .get(site_index)
            .ok_or(LocalMediumError::UnknownSite(site_index))?;
        validate_site(&replacement, site_index)?;
        if replacement.id != current.id {
            return Err(LocalMediumError::InvalidPlan(
                "site reconfiguration must preserve the site id".to_owned(),
            ));
        }
        if self.state.reservoir[site_index] > replacement.reservoir_capacity + 1.0e-15 {
            return Err(LocalMediumError::InvalidPlan(
                "site reconfiguration would silently discard reservoir state".to_owned(),
            ));
        }
        let energy_before = self.stored_energy();
        let configuration_work = 0.5
            * (replacement.wave_metric - current.wave_metric)
            * self.state.wave[site_index]
            * self.state.wave[site_index];
        let site_id = replacement.id.clone();
        self.plan.sites[site_index] = replacement;
        let energy_after = self.stored_energy();
        Ok(LocalMediumReconfigurationReceipt {
            site_index,
            site_id,
            configuration_work,
            energy_before,
            energy_after,
            closure_residual: energy_after - energy_before - configuration_work,
        })
    }

    pub fn step(
        &mut self,
        input: &LocalMediumInputFrame,
    ) -> Result<LocalMediumStepReceipt, LocalMediumError> {
        let energy_before = self.stored_energy();
        let mut source_work = 0.0;

        for drive in &input.drives {
            let source = self
                .plan
                .sources
                .get(drive.source)
                .ok_or(LocalMediumError::UnknownSource(drive.source))?;
            let site_index = source.site;
            let wave_metric = self.plan.sites[site_index].wave_metric;
            let old_wave_energy =
                0.5 * wave_metric * self.state.wave[site_index] * self.state.wave[site_index];
            self.state.wave[site_index] += source.wave_gain * finite(drive.wave);
            let new_wave_energy =
                0.5 * wave_metric * self.state.wave[site_index] * self.state.wave[site_index];
            source_work += new_wave_energy - old_wave_energy;

            let evidence_delta = (source.evidence_gain * finite(drive.evidence)).max(0.0);
            let old_evidence_energy =
                0.5 * self.state.evidence[site_index] * self.state.evidence[site_index];
            self.state.evidence[site_index] += evidence_delta;
            let new_evidence_energy =
                0.5 * self.state.evidence[site_index] * self.state.evidence[site_index];
            source_work += new_evidence_energy - old_evidence_energy;

            let requested = (source.work_gain * finite(drive.work_rate)).max(0.0) * self.dt;
            let capacity = self.plan.sites[site_index].reservoir_capacity;
            let room = (capacity - self.state.reservoir[site_index]).max(0.0);
            let reservoir_add = requested.min(room);
            self.state.reservoir[site_index] += reservoir_add;
            source_work += reservoir_add;
        }

        let mut active_transfer = 0.0;
        for index in 0..self.plan.sites.len() {
            let site = &self.plan.sites[index];
            if site.active_rate_per_second <= 0.0 || self.state.reservoir[index] <= 0.0 {
                continue;
            }
            let activation = (self.state.evidence[index] + self.state.wave[index].abs()
                - site.active_threshold)
                .max(0.0);
            let current_wave_energy =
                0.5 * site.wave_metric * self.state.wave[index] * self.state.wave[index];
            let gap = (site.active_target_wave_energy - current_wave_energy).max(0.0);
            let requested = site.active_rate_per_second * self.dt * activation;
            let transfer = self.state.reservoir[index].min(gap).min(requested);
            if transfer <= 0.0 {
                continue;
            }
            let sign = if self.state.wave[index] < 0.0 {
                -1.0
            } else {
                1.0
            };
            self.state.wave[index] =
                sign * (2.0 * (current_wave_energy + transfer) / site.wave_metric).sqrt();
            self.state.reservoir[index] -= transfer;
            active_transfer += transfer;
        }

        let mut transport_residual = 0.0_f64;
        let mut positive_mass_residual = 0.0_f64;
        let mut transport_loss = 0.0;
        for edge_index in 0..self.plan.edges.len() {
            if !self.edge_enabled[edge_index] {
                continue;
            }
            let edge = &self.plan.edges[edge_index];
            let left = edge.left;
            let right = edge.right;
            let left_metric = self.plan.sites[left].wave_metric;
            let right_metric = self.plan.sites[right].wave_metric;
            let before_wave_energy = 0.5
                * (left_metric * self.state.wave[left] * self.state.wave[left]
                    + right_metric * self.state.wave[right] * self.state.wave[right]);
            let theta = edge.wave_rate_radians_per_second * self.dt;
            let (sine, cosine) = theta.sin_cos();
            let scaled_left = left_metric.sqrt() * self.state.wave[left];
            let scaled_right = right_metric.sqrt() * self.state.wave[right];
            let rotated_left = cosine * scaled_left - sine * scaled_right;
            let rotated_right = sine * scaled_left + cosine * scaled_right;
            self.state.wave[left] = rotated_left / left_metric.sqrt();
            self.state.wave[right] = rotated_right / right_metric.sqrt();
            let after_wave_energy = 0.5
                * (left_metric * self.state.wave[left] * self.state.wave[left]
                    + right_metric * self.state.wave[right] * self.state.wave[right]);
            transport_residual =
                transport_residual.max((after_wave_energy - before_wave_energy).abs());

            let left_before = self.state.evidence[left];
            let right_before = self.state.evidence[right];
            let mass_before = left_before + right_before;
            let evidence_energy_before =
                0.5 * (left_before * left_before + right_before * right_before);
            let mix = (edge.evidence_rate_per_second * self.dt).clamp(0.0, 0.5);
            self.state.evidence[left] = (1.0 - mix) * left_before + mix * right_before;
            self.state.evidence[right] = mix * left_before + (1.0 - mix) * right_before;
            let mass_after = self.state.evidence[left] + self.state.evidence[right];
            let evidence_energy_after = 0.5
                * (self.state.evidence[left] * self.state.evidence[left]
                    + self.state.evidence[right] * self.state.evidence[right]);
            positive_mass_residual = positive_mass_residual.max((mass_after - mass_before).abs());
            transport_loss += (evidence_energy_before - evidence_energy_after).max(0.0);
        }

        let mut passive_loss = 0.0;
        for index in 0..self.plan.sites.len() {
            let site = &self.plan.sites[index];
            let wave_before =
                0.5 * site.wave_metric * self.state.wave[index] * self.state.wave[index];
            let evidence_before = 0.5 * self.state.evidence[index] * self.state.evidence[index];
            let reservoir_before = self.state.reservoir[index];

            self.state.wave[index] *= (-0.5 * site.wave_decay_per_second * self.dt).exp();
            self.state.evidence[index] *= (-0.5 * site.evidence_decay_per_second * self.dt).exp();
            self.state.reservoir[index] *= (-site.reservoir_leak_per_second * self.dt).exp();

            let wave_after =
                0.5 * site.wave_metric * self.state.wave[index] * self.state.wave[index];
            let evidence_after = 0.5 * self.state.evidence[index] * self.state.evidence[index];
            let reservoir_after = self.state.reservoir[index];
            passive_loss += (wave_before + evidence_before + reservoir_before
                - wave_after
                - evidence_after
                - reservoir_after)
                .max(0.0);
        }

        self.ensure_finite_state()?;
        let internal_loss = transport_loss + passive_loss;
        let energy_after = self.stored_energy();
        let closure_residual = energy_after - energy_before - source_work + internal_loss;
        Ok(LocalMediumStepReceipt {
            energy_before,
            energy_after,
            source_work,
            active_transfer,
            transport_residual,
            positive_mass_residual,
            transport_loss,
            passive_loss,
            internal_loss,
            closure_residual,
            observation: self.observe(),
        })
    }

    fn ensure_finite_state(&self) -> Result<(), LocalMediumError> {
        if self.state.wave.iter().any(|value| !value.is_finite())
            || self
                .state
                .evidence
                .iter()
                .any(|value| !value.is_finite() || *value < -1.0e-15)
            || self
                .state
                .reservoir
                .iter()
                .any(|value| !value.is_finite() || *value < -1.0e-15)
        {
            return Err(LocalMediumError::NonFiniteState);
        }
        Ok(())
    }
}

fn validate_site(site: &LocalMediumSiteSpec, index: usize) -> Result<(), LocalMediumError> {
    if site.id.trim().is_empty() {
        return Err(LocalMediumError::InvalidPlan(format!(
            "site {index} id must be nonempty"
        )));
    }
    if !site.wave_metric.is_finite()
        || site.wave_metric <= 0.0
        || !site.reservoir_capacity.is_finite()
        || site.reservoir_capacity <= 0.0
    {
        return Err(LocalMediumError::InvalidPlan(format!(
            "site {:?} requires positive wave metric and reservoir capacity",
            site.id
        )));
    }
    for (name, value) in [
        ("wave_decay_per_second", site.wave_decay_per_second),
        ("evidence_decay_per_second", site.evidence_decay_per_second),
        ("reservoir_leak_per_second", site.reservoir_leak_per_second),
        ("active_rate_per_second", site.active_rate_per_second),
        ("active_threshold", site.active_threshold),
        ("active_target_wave_energy", site.active_target_wave_energy),
        ("initial_evidence", site.initial_evidence),
        ("initial_reservoir", site.initial_reservoir),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(LocalMediumError::InvalidPlan(format!(
                "site {:?} requires non-negative finite {name}",
                site.id
            )));
        }
    }
    if !site.initial_wave.is_finite() {
        return Err(LocalMediumError::InvalidPlan(format!(
            "site {:?} has non-finite initial wave state",
            site.id
        )));
    }
    if site.initial_reservoir > site.reservoir_capacity {
        return Err(LocalMediumError::InvalidPlan(format!(
            "site {:?} initial reservoir exceeds capacity",
            site.id
        )));
    }
    Ok(())
}

#[inline]
fn finite(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

const fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_spelling_is_cpython_repr() {
        // Expected strings are CPython 3 `repr()` of the same doubles.
        let cases: [(f64, &str); 19] = [
            (0.0, "0.0"),
            (-0.0, "-0.0"),
            (1.0, "1.0"),
            (48_000.0, "48000.0"),
            (0.0001, "0.0001"),
            (0.00001, "1e-05"),
            (4.658216673761258e-05, "4.658216673761258e-05"),
            (-2.0218389277721283e-08, "-2.0218389277721283e-08"),
            (1e16, "1e+16"),
            (1e15, "1000000000000000.0"),
            (123456789012345680.0, "1.2345678901234568e+17"),
            (0.1, "0.1"),
            (-0.25, "-0.25"),
            (1.5e-300, "1.5e-300"),
            (5e-324, "5e-324"),
            (f64::MAX, "1.7976931348623157e+308"),
            (12345.678, "12345.678"),
            (0.001234, "0.001234"),
            (9.999999999999999e-05, "9.999999999999999e-05"),
        ];
        for (value, expected) in cases {
            assert_eq!(python_float_repr(value).unwrap(), expected, "{value:e}");
        }
    }

    fn site(id: &str, metric: f64) -> LocalMediumSiteSpec {
        LocalMediumSiteSpec {
            id: id.to_owned(),
            wave_metric: metric,
            wave_decay_per_second: 0.01,
            evidence_decay_per_second: 0.02,
            reservoir_capacity: 1.0,
            reservoir_leak_per_second: 0.01,
            active_rate_per_second: 0.2,
            active_threshold: 0.1,
            active_target_wave_energy: 0.5,
            initial_wave: 0.0,
            initial_evidence: 0.0,
            initial_reservoir: 0.0,
        }
    }

    fn plan_with_edges(edges: Vec<LocalMediumEdgeSpec>) -> LocalMediumPlan {
        LocalMediumPlan {
            schema_version: LOCAL_MEDIUM_SCHEMA_VERSION.to_owned(),
            name: "local-medium-test".to_owned(),
            sample_rate_hz: 32.0,
            sites: vec![site("a", 1.0), site("b", 1.25), site("c", 1.5)],
            edges,
            sources: vec![LocalMediumSourceSpec {
                id: "source".to_owned(),
                site: 0,
                wave_gain: 1.0,
                evidence_gain: 1.0,
                work_gain: 1.0,
            }],
            observers: vec![LocalMediumObserverSpec {
                id: "observer".to_owned(),
                wave_weights: vec![1.0, -0.5, 0.25],
                evidence_weights: vec![0.0, 1.0, -1.0],
                reservoir_weights: vec![0.0, 0.0, 1.0],
                bias: 0.0,
            }],
        }
    }

    fn edges() -> Vec<LocalMediumEdgeSpec> {
        vec![
            LocalMediumEdgeSpec {
                id: "ab".to_owned(),
                left: 0,
                right: 1,
                wave_rate_radians_per_second: 4.0,
                evidence_rate_per_second: 3.0,
                enabled: true,
            },
            LocalMediumEdgeSpec {
                id: "bc".to_owned(),
                left: 1,
                right: 2,
                wave_rate_radians_per_second: -2.5,
                evidence_rate_per_second: 1.5,
                enabled: true,
            },
        ]
    }

    #[test]
    fn zero_is_an_exact_fixed_point() {
        let mut medium = LocalConstitutiveMedium::from_plan(plan_with_edges(edges())).unwrap();
        for _ in 0..64 {
            let receipt = medium.step(&LocalMediumInputFrame::default()).unwrap();
            assert_eq!(receipt.source_work, 0.0);
            assert_eq!(receipt.internal_loss, 0.0);
            assert_eq!(receipt.closure_residual, 0.0);
        }
        assert_eq!(medium.stored_energy(), 0.0);
        assert_eq!(medium.positive_mass(), 0.0);
    }

    #[test]
    fn ledger_and_transport_close() {
        let mut medium = LocalConstitutiveMedium::from_plan(plan_with_edges(edges())).unwrap();
        let mut residual = 0.0_f64;
        for step in 0..256 {
            let input = LocalMediumInputFrame {
                drives: vec![LocalMediumSourceDrive {
                    source: 0,
                    wave: (0.1 * f64::from(step)).sin() * 0.02,
                    evidence: if step % 7 == 0 { 0.01 } else { 0.0 },
                    work_rate: 0.1,
                }],
            };
            let receipt = medium.step(&input).unwrap();
            residual += receipt.closure_residual.abs();
            assert!(receipt.transport_residual < 1.0e-12);
            assert!(receipt.positive_mass_residual < 1.0e-12);
            assert!(medium.state().evidence.iter().all(|value| *value >= 0.0));
        }
        assert!(residual < 1.0e-9, "residual={residual:e}");
    }

    #[test]
    fn edge_order_is_functionally_meaningful() {
        let forward = edges();
        let mut reverse = forward.clone();
        reverse.reverse();
        let mut first = LocalConstitutiveMedium::from_plan(plan_with_edges(forward)).unwrap();
        let mut second = LocalConstitutiveMedium::from_plan(plan_with_edges(reverse)).unwrap();
        let input = LocalMediumInputFrame {
            drives: vec![LocalMediumSourceDrive {
                source: 0,
                wave: 1.0,
                evidence: 0.5,
                work_rate: 0.0,
            }],
        };
        first.step(&input).unwrap();
        second.step(&input).unwrap();
        let difference: f64 = first
            .state()
            .wave
            .iter()
            .zip(&second.state().wave)
            .map(|(left, right)| (left - right).abs())
            .sum();
        assert!(difference > 1.0e-4);
    }

    #[test]
    fn damage_preserves_state_and_reconfiguration_books_work() {
        let mut medium = LocalConstitutiveMedium::from_plan(plan_with_edges(edges())).unwrap();
        medium
            .step(&LocalMediumInputFrame {
                drives: vec![LocalMediumSourceDrive {
                    source: 0,
                    wave: 0.4,
                    evidence: 0.2,
                    work_rate: 0.5,
                }],
            })
            .unwrap();
        let before = medium.state().clone();
        let damage = medium.set_edge_enabled(0, false).unwrap();
        assert_eq!(medium.state(), &before);
        assert_eq!(damage.state_energy_delta, 0.0);

        let mut replacement = medium.plan().sites[0].clone();
        replacement.wave_metric *= 1.5;
        let receipt = medium.reconfigure_site(0, replacement).unwrap();
        assert!(receipt.configuration_work.abs() > 0.0);
        assert!(receipt.closure_residual.abs() < 1.0e-14);
    }
}
