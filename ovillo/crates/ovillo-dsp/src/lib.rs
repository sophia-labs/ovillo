//! Ovillo's sound. A simulated electric guitar ([`guitar`], played by
//! [`player`]) feeds the canonical Ariadne World ([`ariadne`]); [`rig`] joins
//! them on one sample clock and taps everything the screen draws.

pub mod ariadne;
pub mod guitar;
pub mod player;
pub mod rig;
