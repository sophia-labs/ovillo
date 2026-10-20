//! A Pink-Trombone-like vocal tract made as an Ariadne World.
//!
//! *After Neil Thapen's Pink Trombone (2017); implemented independently, its
//! source not consulted.*
//!
//! What is physical here, in Ariadne's sense
//! (`ARIADNE-PURE-PHYSICS-CONSTITUTION.md` §3, §5):
//!
//! - **Persistent distributed state**: travelling pressure waves in an oral and
//!   a nasal tube, two radiating air plugs, a fold mass, a lung reservoir.
//! - **Lawful interconnection**: every junction (two-port, the three-port velum)
//!   is admittance-weighted scattering that conserves `sum Y w²` exactly.
//! - **Transport with work receipts**: the area function, fold tension and
//!   adduction are moved under fixed state; the change of stored energy is
//!   booked as control work.
//! - **Explicit active work**: sound is paid for by a finite lung reservoir the
//!   breathing muscles refill; the fold self-oscillates from lung pressure.
//! - **One ledger**: `mus_watsn::WorkLedger`, one `WorkFrame` per block.
//!
//! What is not (see [`world`] and the README in `site/tract/`): the World is
//! not an instance of `mus_ariadne::World` (whose bodies are 24-mode modal
//! fields with a one-dimensional configuration path), losses are frequency
//! independent, the fold is one mass with a lumped mucosal delay, closures stop
//! at GARGANTA's 0.025 cm² floor (no rank change), and the aerodynamic work on
//! the folds is booked to the lungs without resolving its path.

pub mod anatomy;
pub mod measure;
pub mod presets;
pub mod world;

pub use anatomy::{Articulation, Provenance, TractAnatomy};
pub use presets::{Gesture, Preset};
pub use world::{Constriction, Flows, GlottisMode, TractControls, TractReadings, TractWorld, Vibrato};
