//! In-world sound sources for the party games. Each is an Ovillo World in the
//! sense of `ARIADNE-PURE-PHYSICS-CONSTITUTION.md`: a persistent physical state
//! that is touched (continuous controls at 60 Hz, discrete events), that does
//! the sounding, and whose every render closes an energy ledger.
//!
//! See PLUGIN.md for how to register them, and each module's header for what
//! it models and what it does not.

pub mod net;
pub mod kart_engine;
pub mod race_props;
pub mod f16;
pub mod maverick;
pub mod explosion;
pub mod catalog;
pub mod abi;

pub use catalog::{catalog, create};

/// The shape every World here has. Mono output in pascals at the World's
/// reference distance, times `OUTPUT_SCALE` (see `output_scale`).
pub trait GameWorld: Send {
    fn kind(&self) -> &'static str;
    /// A continuous control (called at ~60 Hz). Returns false if unknown.
    fn set(&mut self, name: &str, value: f64) -> bool;
    /// A text control (`surface`: "track" | "lakebed"). Returns false if unknown.
    fn set_text(&mut self, _name: &str, _value: &str) -> bool {
        false
    }
    /// A discrete event. `arg` is the event's one number (0 if none).
    fn event(&mut self, name: &str, arg: f64) -> bool;
    /// Render `out.len()` frames of mono audio.
    fn render(&mut self, out: &mut [f32]);
    /// The World's own receipt (ledger, counters, declared sources).
    fn receipt(&self) -> serde_json::Value;
    /// Pa of source pressure at the reference distance that maps to full scale.
    fn output_scale(&self) -> f64;
    fn sample_rate(&self) -> f64;
}
