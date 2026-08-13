//! Typed parameter specifications plus the physical-body lifetime selector.
//!
//! The established vocabulary table remains source-identical in [`original`].
//! This thin extension keeps the lifetime gate explicit without rewriting or
//! forking the existing parameter registry.

mod original;

pub use original::{flag_specs, FlagSpec, ParamKind, ParamSpec};

use crate::ParamLayer;

pub fn param_specs() -> Vec<ParamSpec> {
    let mut specs = original::param_specs();
    specs.push(ParamSpec {
        name: "body_lifetime",
        layer: ParamLayer::Extension,
        kind: ParamKind::Enum {
            values: &["event", "track"],
            default: Some("event"),
        },
        doc: "instrument-owned physical-body lifetime: event constructs one body per note; track keeps one fixed-topology string body alive from the first attack through the score",
    });
    specs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_lifetime_is_a_typed_instrument_extension() {
        let spec = param_specs()
            .into_iter()
            .find(|spec| spec.name == "body_lifetime")
            .expect("body_lifetime spec");
        assert!(matches!(spec.layer, ParamLayer::Extension));
        match spec.kind {
            ParamKind::Enum { values, default } => {
                assert_eq!(values, &["event", "track"]);
                assert_eq!(default, Some("event"));
            }
            other => panic!("expected enum, got {other:?}"),
        }
    }
}
