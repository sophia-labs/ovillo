//! One delayed spatial field shared by several Ariadne bodies.

mod runtime;
mod types;

pub(crate) use runtime::SharedSpace;
pub use types::{SharedSpaceControls, SharedSpaceError, SharedSpaceExchange, SharedSpaceFrame};

#[cfg(test)]
mod tests;
