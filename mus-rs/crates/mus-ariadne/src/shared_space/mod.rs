//! One delayed spatial field shared by several Ariadne bodies.

mod runtime;
mod types;

pub use types::{
    SharedSpaceControls, SharedSpaceError, SharedSpaceExchange, SharedSpaceFrame,
};
pub(crate) use runtime::SharedSpace;

#[cfg(test)]
mod tests;
