//! A vocal tract as a true `mus_ariadne::World`.
//!
//! `ovillo-tract` (another crate, another agent) builds a Kelly-Lochbaum tube
//! tract to Ariadne's constitution in its own executor, because
//! `mus_ariadne::World`'s bodies were 24-mode modal fields on a one-dimensional
//! configuration path. With N-dimensional configuration spaces
//! (`mus-rs/SPECS/ariadne/ND-CONFIGURATION.md`) the tract runs in the canonical
//! runtime: one `ModalSpace` body whose four coordinates are tongue index,
//! tongue diameter, lips and velum; whose 24 modes `mus_ariadne` computes from
//! the branched oral+nasal tube network at every point; voiced by
//! `mus_ariadne`'s flow valve; booked by the World's one ledger.
//!
//! What is law, fit and approximation is listed in `CHECKPOINT-2A.md` at the
//! worktree root.

pub mod abi;
pub mod anatomy;
pub mod bench;
pub mod gestures;
pub mod measure;
pub mod modal;
pub mod world;

pub use anatomy::TractNdAnatomy;
pub use world::{Articulation, MoveStats, TractControls, TractWorldNd};

#[cfg(target_arch = "wasm32")]
pub mod wasm;
