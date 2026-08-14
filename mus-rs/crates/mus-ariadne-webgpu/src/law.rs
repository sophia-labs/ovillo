use serde::{Deserialize, Serialize};

/// The state domain owned or observed by a primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Domain {
    /// Authoritative state included in the Ariadne World and global ledger.
    World,
    /// Non-loading persistent interpretation of physical observations.
    Observer,
    /// Camera, retina, phosphor, exposure, or recording-device memory.
    Sensor,
    /// Stateless or freely adaptive viewport composition.
    Presentation,
}

/// Ariadne-level meaning of the transformation performed by a primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LawClass {
    NeutralTransport,
    /// Physical work accepted by an authoritative World-domain primitive.
    PhysicalWorkInjection,
    /// Nonphysical excitation of an observer-owned field from typed readouts.
    ObserverExcitation,
    ConstitutiveEvolution,
    Dissipation,
    Radiation,
    NonLoadingObservation,
    SensorMemory,
    Presentation,
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
/// primitive's constitutive cadence remains explicit.
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
