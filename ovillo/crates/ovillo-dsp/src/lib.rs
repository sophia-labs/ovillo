//! Ovillo's sound. A simulated electric guitar ([`guitar`], played by
//! [`player`] from its repertoire in [`compose`] or a [`score`]) feeds the
//! canonical Ariadne World ([`ariadne`]); [`rig`] joins them on one sample
//! clock, puts a master stage after them, and taps everything the screen draws.

pub mod ariadne;
pub mod compose;
pub mod guitar;
pub mod player;
pub mod rig;
pub mod score;
pub mod trombone;
#[cfg(feature = "weave")]
pub mod weave;
