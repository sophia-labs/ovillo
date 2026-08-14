//! Construction-time binding from authored trajectories into one Shared World.
//!
//! Names and body IDs are resolved once. The material-clock path advances a
//! monotone cursor through typed updates and performs no parsing, allocation,
//! hashing, or string lookup.

mod runtime;
mod types;

pub use types::{
    SharedCoordinates, SharedTrajectoryBindError, SharedTrajectoryBinding,
};
