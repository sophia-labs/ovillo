//! Domain-neutral, non-loading observations for persistent Ariadne worlds.
//!
//! An observation schema names a finite set of typed channels on one absolute
//! material clock. A physical runtime binds those names to state projections at
//! construction time; traces and live packets carry only typed fixed-stride
//! values. This crate owns no Ariadne physics and no GPU interpretation.

#![forbid(unsafe_code)]

pub mod schema;
pub mod trace;

pub use schema::{
    Aggregation, ChannelSpec, ClaimDomain, Interpolation, ObservationSchema,
    ScalarType, SchemaError, OBSERVATION_SCHEMA_VERSION,
};
pub use trace::{
    decode_trace, encode_trace, ObservationFrame, ObservationTrace,
    ObservationValue, TraceError, TraceMetadata, TRACE_FORMAT_VERSION,
    TRACE_MAGIC,
};
