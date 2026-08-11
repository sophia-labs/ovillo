//! Work-accounted time-varying scattering primitives.
//!
//! This crate is intentionally independent of MUS parsing, RDF, hosts, and
//! audio I/O. It supplies the small executable law layer used by the Ariadne
//! work/holonomy experiments: exact first-order allpass storage, energy-neutral
//! periodic delay remapping, sparse Givens programs, and an explicit work
//! ledger.

pub mod allpass;
pub mod complex;
pub mod complex_program;
pub mod delay;
pub mod givens;
pub mod ledger;
pub mod onezero;
pub mod program;
pub mod string;

pub use allpass::{
    allpass_normalized_state, allpass_storage_energy, neutral_allpass_state_transport,
    AllpassSection, AllpassTransportReceipt,
};
pub use complex::{state_energy, Complex64};
pub use complex_program::{
    real_givens_symplectic_residual, ComplexGate, ComplexOperatorProgram, ComplexPhaseGate,
    ComplexProgramError, ComplexProgramReceipt, ComplexTwoModeGate,
};
pub use delay::{
    quadratic_energy, resample_periodic_linear, resample_periodic_linear_neutral,
    PeriodicRemapReceipt,
};
pub use givens::{
    balanced_inverse_branches, commutator_matrix, givens, mat3_mul, order_defect,
    principal_rotation_angle,
};
pub use ledger::{WorkFrame, WorkLedger};
pub use onezero::{onezero_dissipation, onezero_storage_energy, onezero_tick};
pub use program::{OperatorProgram, PlaneRotation, ProgramError};
pub use string::{
    run_string_experiment, RetunePolicy, StringExperimentConfig, StringExperimentResult,
    StringExperimentRun,
};
