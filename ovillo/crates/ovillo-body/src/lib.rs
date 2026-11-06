//! `ovillo-body`: a Body to speak through.
//!
//! Her recorded voice is the only energy (besides the optional powered
//! material's finite supply): every sample enters one volume of a tube network
//! as a volume velocity, and the World books its exact work as source work,
//! as `ovillo-tract-world`'s `impact` does for one pulse. What she hears is
//! the pressure at a listener placed in front of the Body's openings, a
//! non-loading observation. The Body is one `mus_ariadne::World`: the air (a
//! tree of tubes), up to three passive resonators and the canonical powered
//! material touching it reciprocally, and the shared delayed field.
//!
//! - [`surface::KNOBS`]: the surface, every knob a physical thing.
//! - [`Body`]: the streaming core (`new`, `process`, `receipt`).
//! - [`presets`]: the built-in bodies.
//!
//! `SURFACE.md` beside this crate is the map.

pub mod body;
pub mod build;
pub mod presets;
pub mod surface;

pub use body::{Body, Receipt, Transfer};
pub use surface::{BodySpec, Knob, KnobKind, KNOBS};

/// The whole surface as JSON (`ovillo-body describe`).
pub fn describe() -> serde_json::Value {
    serde_json::json!({
        "schema": "ovillo-body.surface.v1",
        "sample_rate_hz": 48000.0,
        "source": "her recording, as a volume velocity (m3/s) into one air volume; exact work booked as source work",
        "listener": "pressure (Pa) at two ears in the plane of the openings: each air end a monopole p_end A / (4 pi r l) delayed r/c; resonators and the room as spherical waves of their radiated power; the dry path added at the listener",
        "knobs": KNOBS,
        "groups": ["bore", "drive", "branches", "air", "flap", "resonators", "powered", "room", "listener"],
        "presets": presets::names(),
        "receipt": {
            "voice_work_j": "work her voice did on the air (the ledger's source work less the powered supply)",
            "ledger": "the World's books: source + control work = internal loss + radiation + stored, residuals near 1e-15 J",
            "air": "wall loss, radiation per end, jet loss and turbulence, the flap's losses",
            "bandwidth_hz": "the 24th retained mode of the air: the Body is dark above it",
            "modes": "the air's 24 modes: Hz, wall T60, radiation rate per end, shape at the valve and the mouth"
        }
    })
}
