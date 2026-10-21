//! N-dimensional configuration spaces for Ariadne bodies.
//!
//! The legacy body moves along one scalar `amount` in `[0, 1]` between two
//! anchor configurations (`ir::ConfigurationPath`). A configuration space
//! generalizes that to a point `x` in a box of named coordinates, mapped to a
//! body configuration by one of two declared maps:
//!
//! - [`ConfigurationMapSpec::Multilinear`]: a tensor grid of anchor
//!   configurations, interpolated axis by axis with exactly the legacy 1-D
//!   rules. This is an **interpolation** (a fit between authored anchors), not
//!   a law. A one-coordinate, two-anchor space reproduces the legacy path
//!   bit for bit.
//! - [`ConfigurationMapSpec::TubeNetwork`]: a lumped acoustic tube network
//!   (compressible air volumes joined by inertive air columns) whose section
//!   areas are a declared function of `x`; the body's modes are **computed**
//!   from that network at every point (a constitutive law: the discretized
//!   lossless Webster horn equation; the area family itself is whatever the
//!   anatomy declares).
//!
//! A move from `x` to `x'` is a reconfiguration at fixed physical state under a
//! declared connection ([`ConnectionPolicy`]); its work receipt is the exact
//! change of stored energy, and a path through the space is a sequence of such
//! moves. See `mus-rs/SPECS/ariadne/ND-CONFIGURATION.md`.

pub(crate) mod eigen;
pub mod network;
pub mod space;
pub(crate) mod tree_eigen;

pub use network::{
    AffineSpec, AreaTermSpec, RadiatingEndSpec, TubeBranchSpec, TubeNetworkSpec, TubeSpec,
    TUBE_NETWORK_MAX_NODES,
};
pub use space::{
    ConfigurationCoordinate, ConfigurationMapSpec, ConfigurationSpaceSpec, ConnectionPolicy,
    DisposalPolicy, StiffStringSpec, MAX_CONFIGURATION_DIMENSIONS,
};
