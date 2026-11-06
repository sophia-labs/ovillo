//! The catalog: every World kind, its anatomies, options, events, continuous
//! controls, telemetry, score format, listening gain and what it does not
//! model. It is built from the kind modules (the same tables the ABI indexes
//! by), so `ovillo-catalog.json` is generated, never written by hand
//! (`catalog_json`, and the drift test in `tests/portable.rs`).

use serde::Serialize;
use serde_json::{json, Value};

use crate::{CreateCtx, Source};

/// The World's own books (`mus_ariadne::World::receipt`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ledger {
    pub source_work: f64,
    pub control_work: f64,
    pub internal_loss: f64,
    pub radiation: f64,
    pub initial_energy: f64,
    pub current_energy: f64,
    pub cumulative_closure_residual: f64,
    pub maximum_frame_residual: f64,
    pub global_closure_residual: f64,
    pub sample_frames: u64,
}

impl Ledger {
    pub fn of(world: &mus_ariadne::World) -> Self {
        let r = world.receipt();
        Self {
            source_work: r.source_work,
            control_work: r.control_work,
            internal_loss: r.internal_loss,
            radiation: r.radiation,
            initial_energy: r.initial_energy,
            current_energy: r.current_energy,
            cumulative_closure_residual: r.cumulative_closure_residual,
            maximum_frame_residual: r.maximum_frame_residual,
            global_closure_residual: r.global_closure_residual,
            sample_frames: r.sample_frames as u64,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnatomyDef {
    pub id: String,
    pub title: String,
    pub about: String,
    pub default: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionDef {
    pub name: String,
    /// JSON type: "number", "string", "array", "object", "boolean".
    #[serde(rename = "type")]
    pub ty: String,
    pub about: String,
    pub default: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArgDef {
    pub name: String,
    pub units: String,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    /// Named values: the argument is an index into this list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    pub about: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventDef {
    pub name: String,
    pub about: String,
    pub args: Vec<ArgDef>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ControlDef {
    pub name: String,
    pub units: String,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    /// A sensible update rate from a game loop (Hz). The World moves no
    /// faster than its own mechanics (`smoothing`) whatever the rate.
    pub rate_hz: f64,
    /// What the World does between updates.
    pub smoothing: String,
    /// Discrete (an index or a switch): never ramped.
    pub steps: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    pub about: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotDef {
    pub name: String,
    pub units: String,
    pub about: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreDef {
    pub format: String,
    pub about: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputDef {
    /// What one unit of output is, before the gain.
    pub observes: String,
    /// The one fixed listening gain (output = observation x gain).
    pub gain: f64,
    pub channels: String,
    pub about: String,
}

pub type Build = fn(&KindDef, &CreateCtx) -> Result<Box<dyn Source>, String>;

/// One World kind.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KindDef {
    pub id: String,
    pub title: String,
    /// The code family that builds it: tab, piano, trombone, tract, fart, bench.
    pub family: String,
    pub summary: String,
    /// Where it is specified.
    pub spec: String,
    /// The only rate it runs at (its plan's), or null for any.
    pub sample_rate: Option<f64>,
    /// 1: the World is heard at one point (left = right); 2: two listening points.
    pub channels: u32,
    pub anatomies: Vec<AnatomyDef>,
    /// Anatomy names beyond the list are accepted (a kind that composes its own).
    #[serde(skip)]
    pub open_anatomies: bool,
    pub options: Vec<OptionDef>,
    pub events: Vec<EventDef>,
    pub controls: Vec<ControlDef>,
    pub telemetry: Vec<SlotDef>,
    pub score: Option<ScoreDef>,
    pub output: OutputDef,
    /// Seconds before its time an event reaches the World (the kind schedules inside it).
    pub lookahead_seconds: f64,
    /// Measured or stated cost, for a game dev choosing sources.
    pub cost: String,
    pub does_not_model: Vec<String>,
    pub does_not_model_ref: String,
    #[serde(skip)]
    pub build: Build,
}

impl std::fmt::Debug for KindDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KindDef").field("id", &self.id).finish()
    }
}

// ---- small builders the kind modules share

pub fn arg(name: &str, units: &str, min: f64, max: f64, default: f64, about: &str) -> ArgDef {
    ArgDef { name: name.into(), units: units.into(), min, max, default, values: None, about: about.into() }
}

pub fn arg_enum(name: &str, values: &[&str], default: usize, about: &str) -> ArgDef {
    ArgDef {
        name: name.into(),
        units: "enum".into(),
        min: 0.0,
        max: (values.len().max(1) - 1) as f64,
        default: default as f64,
        values: Some(values.iter().map(|v| (*v).to_owned()).collect()),
        about: about.into(),
    }
}

pub fn event(name: &str, about: &str, args: Vec<ArgDef>) -> EventDef {
    EventDef { name: name.into(), about: about.into(), args }
}

#[allow(clippy::too_many_arguments)]
pub fn control(name: &str, units: &str, min: f64, max: f64, default: f64, rate_hz: f64, smoothing: &str, about: &str) -> ControlDef {
    ControlDef {
        name: name.into(),
        units: units.into(),
        min,
        max,
        default,
        rate_hz,
        smoothing: smoothing.into(),
        steps: false,
        values: None,
        about: about.into(),
    }
}

pub fn switch(name: &str, values: Option<&[&str]>, max: f64, default: f64, about: &str) -> ControlDef {
    ControlDef {
        name: name.into(),
        units: if values.is_some() { "enum".into() } else { "switch".into() },
        min: 0.0,
        max,
        default,
        rate_hz: 10.0,
        smoothing: "none: applied on its frame".into(),
        steps: true,
        values: values.map(|v| v.iter().map(|s| (*s).to_owned()).collect()),
        about: about.into(),
    }
}

pub fn slot(name: &str, units: &str, about: &str) -> SlotDef {
    SlotDef { name: name.into(), units: units.into(), about: about.into() }
}

pub fn option(name: &str, ty: &str, default: Value, about: &str) -> OptionDef {
    OptionDef { name: name.into(), ty: ty.into(), about: about.into(), default }
}

pub fn anatomy(id: &str, title: &str, about: &str, default: bool) -> AnatomyDef {
    AnatomyDef { id: id.into(), title: title.into(), about: about.into(), default }
}

pub fn lines(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

/// The whole catalog as JSON (what `ovillo-catalog.json` holds).
pub fn catalog_json() -> Value {
    let kinds = crate::kinds::all();
    json!({
        "schema": "ovillo.portable.catalog.v1",
        "producer": format!("ovillo-portable {}", env!("CARGO_PKG_VERSION")),
        "about": "Every World kind the portable Ovillo module builds. Each is an Ariadne World: events and controls touch it, the World does the sounding, and its ledger is its receipt. Indices into `events` and `controls` are what the C ABI takes; the JS package maps names to them.",
        "commonTelemetry": crate::slot::NAMES,
        "kinds": kinds,
    })
}
