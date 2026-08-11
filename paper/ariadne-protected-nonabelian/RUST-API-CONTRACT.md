# Rust API contract for the unified protected instrument

This document specifies the production-facing types and refusal semantics
needed to replay `unified_fixture_v2.json`. Names are recommendations; fields
and invariants are normative.

## 1. Crate ownership

### `mus-watsn`

Owns normalized state, complex local factors, passive ports, source/loss/
radiation/retirement laws, and measured work ledgers.

### `mus-gesture`

Owns protected family/path IR, fixture validation, path compilation, timing,
gauge-invariant compiler receipts, and typed refusals. It emits
`mus-watsn` programs.

### `mus-dsp`

Owns the physical mapping to string/body/near/room state, persistent lifecycle,
regional bridges, frozen-family or compiled-path execution, and live telemetry.

### `mus-engine`

Owns sample-indexed scheduling, event segmentation equivalence, score identity,
and content-addressed render receipts. It must not own the mathematical
projector or duplicate factor execution.

## 2. Stable scalar and identifier types

```rust
#[repr(transparent)]
pub struct ModeId(pub u16);

#[repr(transparent)]
pub struct RegionId(pub u8);

#[repr(transparent)]
pub struct ControlEdgeId(pub u16);

#[repr(transparent)]
pub struct SampleIndex(pub u64);

#[repr(transparent)]
pub struct ContentDigest(pub [u8; 32]);

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Complex64 {
    pub re: f64,
    pub im: f64,
}
```

Production may use an existing complex type internally, but serialized fixture
fields must use an unambiguous `[re, im]` convention and little-endian binary64
for generated tables.

## 3. Complex coordinate contract

```rust
pub struct ComplexCoordinateMap {
    pub complex_modes: u16,
    pub real_coordinates: u16,
    pub real_order: RealQuadratureOrder,
    pub region_of_mode: Box<[RegionId]>,
    pub metric: MetricId,
    pub complex_structure_digest: ContentDigest,
}

pub enum RealQuadratureOrder {
    AllRealThenAllImaginary,
}
```

For this fixture:

```text
complex_modes = 16
real_coordinates = 32
real_order = [Re z_0..Re z_15, Im z_0..Im z_15]
metric = identity
```

Any executor using an interleaved representation must convert explicitly and
include the conversion digest in its receipt.

## 4. Lawful factor IR

```rust
#[derive(Clone, Copy, Debug)]
pub struct ComplexTwoModeGate {
    pub edge: ControlEdgeId,
    pub left: ModeId,
    pub right: ModeId,
    /// Row-major U(2) block.
    pub matrix: [Complex64; 4],
    pub sample: SampleIndex,
}

#[derive(Clone, Copy, Debug)]
pub struct ComplexPhaseGate {
    pub mode: ModeId,
    pub phase_radians: f64,
    pub sample: SampleIndex,
}

pub enum ComplexGate {
    TwoMode(ComplexTwoModeGate),
    Phase(ComplexPhaseGate),
}

pub struct ComplexOperatorProgram {
    pub schema_version: u16,
    pub coordinate_map: ContentDigest,
    pub graph: ContentDigest,
    pub composition: CompositionConvention,
    pub gates: Box<[ComplexGate]>,
    pub digest: ContentDigest,
}

pub enum CompositionConvention {
    StateLeftMultiplyInListedExecutionOrder,
}
```

### Validation

A program is executable only if:

1. every coefficient is finite;
2. every two-mode matrix is unitary within `local_unitarity_tolerance`;
3. every edge is authored and matches its endpoint mode IDs;
4. every sample index is monotone nondecreasing;
5. the coordinate map and graph digests match the live state;
6. the complete digest matches canonical serialization;
7. realification preserves the declared complex structure within tolerance.

The executor must not accept a general `PlaneRotation` as a protected factor
merely because it preserves Euclidean norm.

## 5. Program receipt

```rust
pub struct ComplexProgramReceipt {
    pub program: ContentDigest,
    pub initial_state: ContentDigest,
    pub final_state: ContentDigest,
    pub gate_count: u32,
    pub two_mode_gate_count: u32,
    pub phase_gate_count: u32,
    pub maximum_local_unitarity_error_fro: f64,
    pub maximum_local_complex_structure_error_fro: f64,
    pub final_norm_error: f64,
    pub measured_acoustic_state_work: f64,
    pub first_failure: Option<GateFailure>,
}
```

`measured_acoustic_state_work` is zero to floating residual for fixed-metric
unitary factors. It does not include CPU, motor, modulator, or external
actuator overhead.

## 6. Family IR

```rust
pub struct ProtectedFamilyV2 {
    pub schema: ContentDigest,
    pub family_digest: ContentDigest,
    pub coordinate_map: ComplexCoordinateMap,
    pub torus_dimension: u8,
    pub torus_period: [f64; 4],
    pub mass: f64,
    pub carrier_hz: f64,
    pub dirac_scale_hz: f64,
    pub trivial_offsets: [f64; 3],
    pub coupling_fraction: f64,
    pub protected_rank: u8,
    pub spectral_cut: f64,
    pub exact_gap_lower_bound: f64,
    pub invariant: TopologyReceiptV2,
    pub perturbation_domain: PerturbationDomainV2,
    pub symmetry: SymmetryBoundaryV2,
}
```

The initial production implementation should evaluate the closed-form
Wilson--Dirac family directly. A table-only family is allowed for replay but
cannot accept arbitrary control values outside the table without an
interpolation theorem and error receipt.

## 7. Path IR

```rust
pub struct ProtectedPathV2 {
    pub name: Box<str>,
    pub family: ContentDigest,
    pub basepoint: [f64; 4],
    pub closure: ClosureRule,
    pub analytic_path: Option<SmoothTorusLoopV2>,
    pub points: Box<[[f64; 4]]>,
    pub projector_table: ContentDigest,
    pub local_transport_table: Option<ContentDigest>,
    pub schedule: PathScheduleV2,
    pub certificate: PathCertificateV2,
}

pub struct SmoothTorusLoopV2 {
    pub first_axis: u8,
    pub second_axis: u8,
    pub first_radius: f64,
    pub second_radius: f64,
    pub phase_profile: PhaseProfile,
}

pub enum PhaseProfile {
    QuinticSmoothstep,
}
```

A path is closed only when the external torus point, family parameters,
internal actuator state, interpolation phase, and schedule endpoint all return
to the declared base configuration.

## 8. Compilation request and modes

```rust
pub struct ProtectedCompileRequest {
    pub family: ProtectedFamilyV2,
    pub path: ProtectedPathV2,
    pub mode: ProtectedExecutionMode,
    pub sample_rate_hz: f64,
    pub duration_seconds: f64,
    pub budgets: ProtectedBudgets,
    pub requested_claim: ClaimLevel,
}

pub enum ProtectedExecutionMode {
    CompiledGeometric,
    PhysicalAdiabatic,
}

pub struct ProtectedBudgets {
    pub minimum_gap: f64,
    pub maximum_projector_error: f64,
    pub maximum_leakage: f64,
    pub maximum_lowering_error_fro: f64,
    pub maximum_local_unitarity_error_fro: f64,
    pub maximum_control_bandwidth_hz: f64,
    pub maximum_acoustic_control_work: f64,
}
```

### `CompiledGeometric`

Requires local projector/direct-rotation data and lawful factorization. It
returns geometric transport without claiming emergence from slow frozen-family
evolution.

### `PhysicalAdiabatic`

Requires a structure-preserving frozen-family propagator and an accepted
finite-time bound. A numerically small replay error does not override a failed
requested theorem/validated budget.

## 9. Typed refusal

```rust
pub enum ProtectedRefusal {
    UnknownSchema,
    DigestMismatch,
    CoordinateMapMismatch,
    NonFiniteInput,
    NonPeriodicFamily,
    PathNotClosed,
    ActuatorStateNotClosed,
    RankMismatch,
    GapUnverified,
    GapBudgetViolated,
    ProjectorTableMismatch,
    AdjacentOverlapTooSmall,
    GaugeConventionMissing,
    IllegalRealFactor,
    EdgeNotAuthored,
    ComplexStructureViolation,
    LoweringResidualTooLarge,
    DurationUnsupported,
    AdiabaticBoundFailed,
    LeakageBudgetFailed,
    SampleRateUnsupported,
    BlockScheduleAmbiguous,
    MissingRadiationPort,
    MissingDisposalPolicy,
    PersistentStateUnavailable,
    ProductionReplayMissing,
    ClaimTooStrong,
}
```

Each refusal carries a machine-readable path to the failing field, observed
value, required bound, evidence class, and strongest claim still available.

## 10. Persistent instrument interface

```rust
pub trait ProtectedInstrument {
    fn prepare(&mut self, family: &ProtectedFamilyV2)
        -> Result<PrepareReceipt, ProtectedRefusal>;

    fn excite(&mut self, event: ExcitationEvent)
        -> Result<SourceReceipt, ProtectedRefusal>;

    fn set_control(&mut self, sample: ControlSample)
        -> Result<(), ProtectedRefusal>;

    fn release(&mut self, event: ReleaseEvent)
        -> Result<ReleaseReceipt, ProtectedRefusal>;

    fn transfer_thread(&mut self, request: ThreadTransfer)
        -> Result<TransferReceipt, ProtectedRefusal>;

    fn retire(&mut self, policy: DisposalPolicy)
        -> Result<RetirementReceipt, ProtectedRefusal>;

    fn render_block(
        &mut self,
        absolute_start: SampleIndex,
        outputs: &mut PhysicalOutputs,
        receipt: &mut InstrumentReceiptAccumulator,
    ) -> Result<(), ProtectedRefusal>;
}
```

No call except explicit `prepare`, `reset`, or `retire` may reconstruct or clear
the body. Host block size is not part of physical semantics.

## 11. Regional bridge IR

```rust
pub struct PassiveRegionalBridge {
    pub left_region: RegionId,
    pub right_region: RegionId,
    pub left_ports: Box<[ModeId]>,
    pub right_ports: Box<[ModeId]>,
    pub scattering_program: ComplexOperatorProgram,
    pub zero_coupling_digest: ContentDigest,
    pub metric: MetricId,
}
```

Every bridge must have:

- an exact zero-coupling limit;
- adjoint/bidirectional action;
- local energy-law receipt;
- a declared mapping to production variables;
- a causal-intervention test in both directions.

## 12. Passive radiation and observer distinction

```rust
pub struct PassiveRadiationPort {
    pub state_modes: Box<[ModeId]>,
    pub emitted_channels: u16,
    pub retained: Box<[Complex64]>,
    pub emitted: Box<[Complex64]>,
    pub colligation_digest: ContentDigest,
}

pub struct ObserverTap {
    pub rows_digest: ContentDigest,
}
```

A radiation port must satisfy the discrete colligation law

```text
S* S + R* R = I
```

within tolerance. `ObserverTap` does not mutate state and cannot appear as
`radiation` in the ledger.

## 13. Work ledger

```rust
pub struct UnifiedWorkFrame {
    pub sample: SampleIndex,
    pub energy_before: f64,
    pub energy_after: f64,
    pub source_work: f64,
    pub control_work: f64,
    pub metric_work: f64,
    pub internal_loss: f64,
    pub radiated_energy: f64,
    pub retired_energy: f64,
    pub numerical_residual: f64,
}

impl UnifiedWorkFrame {
    pub fn closure_error(&self) -> f64;
}
```

The aggregate receipt must report total and worst-local residual. A master
limiter, WAV encoding gain, or presentation normalization is outside this
ledger and must be labeled separately.

## 14. State connection

```rust
pub enum StateConnection {
    FixedCapacityMaterial {
        capacity: u32,
        old_active_length: f64,
        new_active_length: f64,
    },
    WeightedPartialIsometry {
        source_metric: ContentDigest,
        target_metric: ContentDigest,
        transport: ContentDigest,
        source_rank: u32,
        target_rank: u32,
        disposal: Option<DisposalPolicy>,
    },
}

pub enum DisposalPolicy {
    Radiate { port: ContentDigest },
    Dissipate { sink: ContentDigest },
    Reject,
}
```

Contraction without a disposal policy is unrepresentable. Expansion must name
initialization for the target orthogonal complement.

## 15. Claim evaluator

```rust
pub enum ClaimLevel {
    OrderDependent,
    ClosedControl,
    Geometric,
    GappedFamily,
    TopologicalBundle,
    AdiabaticRealization,
    ProtectedAcoustic,
    FaultTolerant,
}

pub enum EvidenceClass {
    Theorem,
    ValidatedNumerics,
    ReferenceOracle,
    ProductionReplay,
    ListenerEvidence,
}
```

The evaluator returns all promoted claims and all withheld stronger claims with
reasons. It must never reduce the evidence system to one boolean.

For fixture v2:

```text
maximum reference claim  = ProtectedAcoustic
maximum production claim = None
```

## 16. Required Rust acceptance tests

1. Parse and digest fixture v2.
2. Replay A/B/preparation/readout words.
3. Compare dense matrices and random states against NPZ-derived test vectors.
4. Verify every local realification is orthogonal-symplectic.
5. Verify AB/BA commutator and occupied eigenphases.
6. Verify identity, inverse, reversal, concatenation, and gauge controls.
7. Verify persistent segmented/unsegmented render equivalence.
8. Verify passive radiation and retirement closure.
9. Reject one-second `PhysicalAdiabatic` at the requested theorem budget.
10. Reject raw long-delay protected-family promotion.
11. Reject arbitrary real Givens factors that violate the complex structure.
12. Emit a production receipt with a new digest rather than copying the Python
    claim bit.
