use serde::{Deserialize, Serialize};

/// The state domain owned or observed by a primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Domain {
    World,
    Observer,
    Sensor,
    Presentation,
}

/// Ariadne-level meaning of the transformation performed by a primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LawClass {
    NeutralTransport,
    WorkInjection,
    ConstitutiveEvolution,
    Dissipation,
    Radiation,
    NonLoadingObservation,
    SensorMemory,
}

/// Spatial support used to choose a suitable execution strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpatialSupport {
    Dense,
    EventRaster,
    ActiveTiles,
    SparseBricks,
    FullScreen,
}

/// Declared scheduling cadence. Presentation is free to interpolate, but the
/// primitive's authoritative cadence remains explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "hz", rename_all = "kebab-case")]
pub enum Cadence {
    EveryFrame,
    FixedHz(u16),
    OnEvent,
    Dirty,
}

/// Human-readable exact-absence or tolerance-bounded law attached to a
/// primitive. The executable test belongs beside the primitive implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct ZeroLaw {
    pub name: &'static str,
    pub statement: &'static str,
}

/// Static constitutional metadata for a GPU primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct PrimitiveDescriptor {
    pub name: &'static str,
    pub domain: Domain,
    pub law: LawClass,
    pub cadence: Cadence,
    pub support: SpatialSupport,
    pub zero_law: ZeroLaw,
}
