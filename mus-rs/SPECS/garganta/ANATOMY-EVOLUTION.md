# GARGANTA anatomy evolution — implementation contract

**Status:** additive G7 contract.  
**Audience:** current Python anatomy fitter, G6 surrogate/reachability work, `mus-watsn`, `mus-dsp`, MUS/Garden, and evidence-pipeline implementers.  
**Integration rule:** preserve the landed sample-conditioned fitter; add population design and correct evidence semantics without creating a competing renderer.

## 1. Runtime boundary

An evolved throat is one persistent instrument body. The render path receives:

```text
ValidatedAnatomy
PersistentThroatState
SourceProgram
ArticulationProgram
EnvironmentConfiguration
```

It must not receive target audio, optimizer state, a neural imitation model, or a per-note replacement anatomy.

Construction-time flow:

```text
campaign + anatomy prior + target/repertoire
    -> schema and physical validation
    -> G7.1 reachability screen
    -> G6 fast-map control solve
    -> multiobjective anatomy search
    -> production replay and ledger
    -> isophonemy clustering
    -> archive / representative + receipts
```

## 2. Existing chart versus target anatomy IR

The current Python `AnatomyGenome` is a useful ten-coordinate construction chart for one supraglottal model. It remains the sample fitter’s oracle input.

The committed `garganta-anatomy.schema.json` is the target persistent IR. It carries explicit fixed-capacity geometry, passive losses, region bridges, branch topology, actuator limits, provenance, and morphology domain. The Python chart must lower deterministically into the target IR; it must not be treated as the complete mature organ by renaming fields.

Conceptual Rust types:

```rust
pub enum AnatomyDomain {
    HumanPlausible,
    Pseudobiological,
    PostHuman,
}

pub enum EvolutionMode {
    PopulationDesign,
    SampleConditioned,
}

pub enum DisposalPolicy {
    Radiate,
    Dissipate,
    Retire,
    Reject,
}

pub struct BranchAnatomy {
    pub id: BranchId,
    pub enabled: bool,
    pub attach_after_section: SectionId,
    pub section_lengths_m: Box<[f64]>,
    pub neutral_areas_m2: Box<[f64]>,
    pub coupling: f64,
    pub wall_loss: f64,
    pub topology_policy: DisposalPolicy,
}

pub struct ThroatAnatomy {
    pub schema_version: SchemaVersion,
    pub id: AnatomyId,
    pub domain: AnatomyDomain,
    pub section_lengths_m: Box<[f64]>,
    pub neutral_areas_m2: Box<[f64]>,
    pub epilarynx_area_m2: f64,
    pub lip_area_m2: f64,
    pub wall_loss: f64,
    pub viscous_loss: f64,
    pub radiation_loss: f64,
    pub body_coupling: f64,
    pub room_return_coupling: f64,
    pub max_log_area_velocity_s: f64,
    pub max_log_area_acceleration_s2: f64,
    pub branches: Box<[BranchAnatomy]>,
    pub provenance: CanonicalJson,
}
```

Production state is never serialized inside `ThroatAnatomy`. Its digest identifies material configuration, not current pressure/flow history.

## 3. Validation and lowering

```rust
pub fn validate_anatomy(
    anatomy: &ThroatAnatomy,
    bounds: &AnatomyBounds,
) -> AnatomyValidation;

pub fn lower_anatomy(
    anatomy: &ValidatedAnatomy,
    capacity: &TractCapacity,
    sample_rate: SampleRate,
) -> Result<AnatomyLowering, AnatomyLoweringRefusal>;
```

Validation must check:

- finite values and exact units;
- matching geometry arrays and fixed-capacity limits;
- positive length/area and declared morphology envelope;
- total tract/branch length and bounded adjacent log-area steps;
- strict reflection/passivity margin;
- passive loss and normalized coupling;
- valid branch addresses, unique IDs, and explicit disposal policy;
- finite positive articulation velocity/acceleration bounds;
- deterministic canonical digest.

`AnatomyLowering` records:

```text
anatomy and schema digests
sample rate and state layout
SectionId / BranchId address map
pressure/flow normalization and metric provenance
propagation and fractional-delay stages
junction coefficients and passivity residual
loss and radiation ports
body and room bridges
actuator basis and rate bounds
model-order approximation error
zero-coupling witnesses
```

Different authored section counts lower into one declared maximum capacity. No allocation or topology discovery occurs in the callback.

A zero static area refuses. Exact closure is a dynamic G2 operation with a moving-state connection and disposal receipt.

## 4. Source and articulation are separate programs

```rust
pub struct ArticulationProgram {
    pub anatomy_digest: Digest,
    pub sample_rate: SampleRate,
    pub frames: Box<[ArticulationFrame]>,
    pub connection_policy: ConnectionPolicy,
    pub predicted_work: WorkPrediction,
}

pub struct SourceProgram {
    pub anatomy_digest: Digest,
    pub glottal_frames: Box<[GlottalFrame]>,
    pub stochastic_supply: Option<StochasticSupplySpec>,
}
```

A sample-conditioned campaign may author different source and articulation programs for each recording. It may not author a different anatomy for each sample under one subject claim.

F0, glottal supply, aspiration, prosody, microphone, and room nuisance are never hidden anatomy coordinates.

## 5. Campaign mode A: population design

The campaign schema must name:

```text
objective profile and probe repertoire
anatomy domain / topology envelope
source and environment distributions
resource limits
surrogate, engine, and error contracts
seeds, stopping rule, archive size
objective names and policy weights
isophonemy metric and threshold
```

The campaign output contains:

```text
candidate/refusal statistics
complete objective vectors
Pareto archive before and after production replay
functional isophonemy clusters
archive stability across seeds
optional policy champion
withheld claims
```

Constraints are evaluated before objectives. Optimization never repairs an invalid passivity law, topology edit, state connection, or resource bound.

A scalar score may order a UI or select a product default. The archive remains the scientific result, and the winner’s exact statement is `policy-optimal` under the named campaign—not universally optimal.

## 6. Campaign mode B: sample-conditioned fitting

The existing one-recording fitter is the starting oracle. The target-evidence record adds sample digests, sample rate, duration, voiced/SNR coverage, source/tract method and confidence, phone/gesture coverage, selection/validation/test partitions, consent/provenance, and optional direct articulatory evidence.

The bilevel problem is:

```text
outer: anatomy theta shared across all recordings
inner: source_i(t), articulation_i(t), environment nuisance_i
```

Required loss channels remain separately receipted:

```text
acoustic residual by feature family
source-control residual
articulatory work and regularity
anatomy prior
cross-recording consistency
validation and untouched-test residual
production ledger residual
```

The final result is a weighted set of anatomy candidates with source/control programs, functional clusters, posterior/policy weights, and an identifiability diameter.

### 6.1 Default one-recording claim

A single reference recording defaults to:

```json
{
  "class": "sample_conditioned_equivalent_throat",
  "sample_specific": true,
  "speaker_identity_model": false,
  "cross_utterance_generalization": false,
  "medical_anatomy_reconstruction": false
}
```

`SpeakerClusterConsistent` requires disjoint utterances, independent-run convergence, and novel-content/condition replay. `BiologicallyReconstructed` requires direct measurement and a validated observation model. Audio-only runs always carry the latter refusal.

### 6.2 Holdout semantics

Every receipt records:

```text
selection digests
validation digests
untouched test digests
holdout_weight_in_search
blind_holdout = (holdout_weight_in_search == 0)
```

When validation states influence anatomy selection, they are not blind holdout. A genuinely untouched partition is required for a generalization claim.

## 7. Population-to-sample transfer

A population archive may initialize sample-conditioned search, but it cannot define its support. The receipt records:

```text
prior archive digest
prior seed count
fresh global exploration count
posterior movement from prior
```

This supports the desired workflow—first evolve excellent pseudobiological species, then specialize one family toward supplied evidence—without reducing fitting to preset selection.

## 8. Reachability

```rust
pub enum ReachabilityResult {
    Reachable(ReachabilityWitness),
    Unreachable(UnreachabilityCertificate),
    Unknown(ReachabilityDiagnostic),
}
```

Only a theorem or validated bound may return `Unreachable`. Numerical search failure returns `Unknown`.

Static and dynamic targets are distinct:

- static: acoustic feature interval plus a control witness;
- dynamic: static witnesses plus rate, acceleration, topology, work, and discretization receipts.

## 9. Isophonemy

```rust
pub struct IsophonemicSignature {
    pub repertoire_digest: Digest,
    pub inventory_features: ArrayRef,
    pub dynamic_features: ArrayRef,
    pub minimum_work_features: ArrayRef,
    pub source_loading_features: ArrayRef,
}
```

The metric is versioned and observable-relative. Production preserves raw feature references. It may not deduplicate by anatomy-array distance, one vowel, or one static formant vector.

## 10. Mutation, topology, and lineage

Continuous mutation operates in positive log geometry with spatial correlation. Every proposal records:

```text
parent digests
operator/settings digest
seed
raw proposal digest when retained
repair list
child digest
validation/refusal
```

Silent repair is forbidden. Crossover is allowed only for compatible topology in v1. Branch birth/death and junction-count changes use separate operators because they require model-order and state-disposal semantics.

## 11. Typed refusals and claims

The implementation preserves these categories without collapsing them:

```text
InvalidAnatomy
CertifiedUnreachable
ReachabilityUnknown
InsufficientSampleEvidence
SourceTractConfounded
SelectionValidationLeakage
UntouchedTestMissing
HoldoutFailure
IdentifiabilityWide
BudgetExhausted
ProductionReplayFailed
ClaimUnsupported
BiologicalReconstructionUnsupported
```

`IdentifiabilityWide` may coexist with a set-valued `SampleConditionedPlausible` result. It refuses a unique-anatomy interpretation, not the plural result.

Anatomy claims are explicit and separate from speech/intelligibility claims:

```text
ValidPseudobiological
RepertoireCapable
PolicyOptimal
InventoryCertified
SampleResemblant
SampleConditionedPlausible
SpeakerClusterConsistent
BiologicallyReconstructed
```

## 12. Real-time constraints

Search and inference are construction-time. Runtime requirements:

- no callback allocation;
- fixed state capacity;
- deterministic sample-indexed control;
- host-block and score-segmentation invariance;
- state persists across notes and events;
- moving-area state connection has measured control work;
- branch/closure rank edits require disposal;
- glottis, tract, body, and room couple bidirectionally;
- physical radiation removes the energy it emits;
- target-analysis models are absent from render dependencies.

## 13. Conformance tests

Minimum cross-language fixtures:

1. target anatomy JSON validates in Python tooling and Rust;
2. canonical digest agrees exactly;
3. Python chart-to-target-IR lowering is deterministic;
4. reflection coefficients and passivity margin agree;
5. zero area refuses with the same category;
6. deterministic mutation replays by seed or committed child fixture;
7. incompatible topology crossover refuses;
8. campaign mode, target, and partition invariants agree;
9. Pareto/isophonemy fixtures agree;
10. sample-fit claim ceiling and identifiability receipt agree;
11. production lowering/replay adds measured evidence without mutating construction-time evidence.

## 14. Implementation order

```text
A0  schemas, Python adapter, Rust mirror, digest parity
A1  fixed-capacity anatomy lowering
A2  persistent state and regional bridges
A3  articulation/source program separation
A4  G6 surrogate and reachability adapter
A5  deterministic population campaign
A6  sample campaign partition/claim correction
A7  population-seeded sample search with fresh exploration
A8  isophonemy and ensemble archive
A9  production replay and evidence promotion
```

Population search may begin against the existing Python fast map while A0–A4 land. No population or sample-fit claim is promoted from the construction oracle alone.

## 15. Acceptance

The implementation is complete when:

- a generally optimized pseudobiological campaign yields a production-replayed, functionally distinct Pareto archive and policy-qualified champion;
- supplied voice evidence yields one persistent anatomy set plus separate source/control programs, passes untouched test replay, and exports a playable representative without erasing uncertainty;
- one-clip evidence visibly caps its claim;
- anatomy/source/control/environment confounds are separately reported;
- full work/loss/radiation/state-retirement ledgers close;
- no target analysis is required at playback;
- Python, Rust, schemas, fixtures, and receipts name the same content-addressed objects.
