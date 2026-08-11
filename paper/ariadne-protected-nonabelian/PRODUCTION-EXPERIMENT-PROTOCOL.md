# Production experiment protocol for the unified protected instrument

## 1. Purpose

This protocol determines whether the production instrument may be promoted
from a reference-oracle result to a protected acoustic instrument claim. It is
not a demonstration script. Every run must produce immutable machine-readable
receipts and preserve failed conditions.

## 2. Preregistration packet

Before rendering primary stimuli, freeze:

- Git commit, Rust toolchain, target triple, compiler profile, CPU features;
- fixture schema and all source/family/path/program digests;
- sample rate, block partitions, floating tolerances, random seeds;
- exact state basis, metric, region and port maps;
- path definitions, duration, execution mode, and composition convention;
- source, preparation, readout, radiation, and loss maps;
- perturbation domain and refusal thresholds;
- primary operator, state, ledger, audio, and listener endpoints;
- all exclusions and stopping rules.

Changing any item requires a new protocol identifier.

## 3. Cross-runtime replay phase

### 3.1 Static object replay

Rust reads the fixture and independently reconstructs or decodes:

- family parameters and regional mixer;
- A/B path points and projectors;
- occupied base frame;
- local direct rotations or compiled programs;
- preparation and readout programs;
- expected topology and conditioning fields.

Report, never silently clamp:

```text
family digest match
path/projector digest match
maximum projector Hermiticity/idempotence residual
minimum adjacent transition singular value
program endpoint residual
maximum local unitarity residual
maximum local complex-structure residual
```

### 3.2 Dynamic state replay

For deterministic initial states, compare Python and Rust after:

```text
A
B
AB
BA
A A^-1
B B^-1
identity
commuting control
preparation
readout
passive radiation
rank expansion
rank contraction with each disposal policy
```

Use both basis vectors and seeded random normalized states. Report absolute and
relative state errors and energy errors.

### 3.3 Promotion gate

No production claim is permitted until Rust emits a receipt whose digest and
numeric fields are independently compared. Copying Python matrices and claim
flags without replay does not pass.

## 4. Persistent lifecycle phase

Run one instrument instance across all variants.

### 4.1 Equivalent partitions

Construct multiple event schedules that induce the same sample-indexed source
and control path:

- one note/event object;
- note split into tied score events;
- host blocks of `1, 7, 64, 257, 1024` samples;
- randomized legal block partitions;
- automation delivered as dense samples and equivalent piecewise segments.

Required equality:

```text
final complete state
checkpoint digests
radiated samples
global ledger
claim receipt
```

Use byte equality where operation order is identical; otherwise use a declared
profile-stable tolerance.

### 4.2 Explicit reset and retirement controls

Repeat AB/BA with:

- no reset;
- reset between loops;
- note-off without retirement;
- note-off with radiative retirement;
- note-off with dissipative retirement.

The persistent primary result must differ from the reset counterfeit in the
predicted direction. Retirement must account for all removed energy.

## 5. Geometry and topology phase

### 5.1 Nominal family

At every committed path point and a denser independent control grid, compute:

- selected eigenvalue cluster;
- rank;
- distance to spectral cut;
- gap to all remaining modes;
- projector residuals;
- regional participation.

The production graph must name which modes are inside the certified family and
which are passive attached dynamics.

### 5.2 Independent invariant

Use a production-independent tool to recompute one of:

- lattice-gauge second Chern number;
- projector-curvature second Chern number;
- validated degree computation.

Do not share the Python degree implementation as the only second method.

### 5.3 Controls

Required:

- `m=-5` trivial family;
- `m=-4` critical closure;
- gauge/frame changes;
- reversed orientation;
- regional coupling zero and full;
- declared coefficient-box extremes;
- residual-mode coupling near its certified threshold.

Critical and out-of-domain requests must produce typed refusals.

## 6. Compiled-geometric execution phase

### 6.1 Path replay

Execute every local factor in sample order. Log:

- factor ID, edge, timestamp;
- local law residual;
- state energy before/after;
- cumulative endpoint residual;
- projector intertwining error at checkpoints;
- control work and actuator-overhead boundary.

### 6.2 Algebraic controls

Required pass conditions:

```text
A^-1 reverses A
B^-1 reverses B
A followed by A^-1 returns protected state
concatenation matches sequential execution
identity is inert
commuting control has null commutator
frame changes conjugate Wilson matrices
```

Loss and radiation may prevent complete live-state return. Their effects must
be predicted by the ledger and separated from protected-subspace operator
return.

## 7. Physical-adiabatic phase

### 7.1 Sweep

For each path, use durations spanning at least:

```text
0.25 s, 0.5 s, 1 s, 2 s,
reference theorem threshold / 2,
reference theorem threshold,
reference theorem threshold * 2.
```

At each duration and supported sample rate, report:

- theorem/validated upper bound;
- measured leakage;
- projective geometric endpoint error;
- dynamical phase;
- discretization comparison;
- maximum control derivative and bandwidth;
- whole-instrument loss/radiation effect.

### 7.2 Pauses, reversals, and nonuniform speed

Include:

- pause at high and low gap locations;
- forward/reverse path;
- quintic, uniform, and deliberately abrupt schedules;
- host-rate changes where supported.

No path may be called adiabatic solely because it was rendered slowly.

## 8. Regional coupling and causal unity phase

### 8.1 Participation

Measure protected projector participation and live state energy in:

```text
string
body
near field
late room
```

The primary family must have nonzero protected participation in each claimed
region.

### 8.2 Causal interventions

Perform state interventions while holding all source/control schedules fixed:

1. perturb room only;
2. perturb string only;
3. zero room bridge;
4. enforce one-way bridge as a counterfeit;
5. restore two-way bridge.

The two-way model must show room-to-body/string influence after causal delay.
The one-way counterfeit must fail that intervention even if its mixed output
sounds similar.

## 9. Open-system and ledger phase

### 9.1 Source types

For every enabled source—pluck, external drive, bow, breath, moving boundary—
measure exact source work or provide a bounded approximation with declared
stored contact state, heat, and radiation split.

### 9.2 Radiation

For every physical output:

```text
retained state energy + emitted energy = incoming state energy
```

within tolerance. Observer-only taps must not mutate state.

### 9.3 Decay

After source removal, run long enough to establish the declared silence/decay
bound. Detect any hidden lossless recirculating component. A conservative core
may persist only if the claim and output coupling explicitly allow it.

## 10. Retuning and topology-edit phase

### 10.1 Fixed-capacity backbone

Sweep active delay/propagation parameters while keeping state coordinates
fixed. Verify identity connection and separate propagation effects from
configuration work.

### 10.2 Variable-rank comparison

For expansion and contraction:

- compare production method to dense weighted-polar oracle;
- report operator/state/energy/waveform errors;
- sweep rate and reversal;
- repeat cycles to expose drift;
- report CPU and memory;
- test `Radiate`, `Dissipate`, and `Reject`.

No candidate passes on scalar energy closure alone.

## 11. Robust observability phase

### 11.1 Port sweep

Compute the physical transfer operator for admissible source and radiation
ports. Report singular values, null spaces, and protected preparation leakage.

### 11.2 Worst-case domain

Optimize or bound the minimum AB/BA transfer distinction over:

- coefficient perturbations;
- regional coupling;
- source vectors;
- pickup/radiation maps;
- loss parameters;
- duration and discretization;
- allowed initial-state uncertainty.

Keep invariant commutator norm and physical transfer distance separate.

### 11.3 Confound controls

Required deterministic controls:

- identity;
- commuting path;
- inverse path;
- trivial `C2=0` family;
- near-gap refusal;
- matched non-topological unitary word;
- zero/one-way/two-way regional bridge;
- same endpoint with no projector path;
- no scalar gain modulation;
- no downstream convolution.

## 12. Stimulus generation

Generate at least two sets.

### Scientific set

- one shared encoding gain;
- no limiter;
- no independent normalization;
- complete state/ledger receipts;
- exact AB/BA causal chain.

### Listener set

Using one fixed post-generation procedure, match:

- duration;
- integrated loudness;
- peak constraints;
- broad spectral descriptors chosen before inspection.

Preserve both original and matched stimuli. Matching operations are outside the
instrument ledger and content-addressed separately.

## 13. Listener study

A defensible first study is a preregistered forced-choice discrimination task.

### Primary endpoint

Participant-level AB/BA discrimination above chance for the protected pair,
compared with the matched non-topological control.

### Minimum design

- randomized double-blind trial order;
- headphone screening and calibration;
- training trials separate from analysis;
- fixed stopping rule and exclusion criteria;
- confidence intervals and hierarchical participant model;
- correction for all declared primary comparisons;
- report null and negative results.

A musical-control follow-up may test whether listeners can intentionally use
path order as a timbral control. It is a distinct claim from discrimination.

## 14. Required output packet

Each production run returns:

```text
production_fixture_v2.json
production_results.json
production_state_checkpoints.bin
production_audio/
production_manifest.json
claim_decision.json
failure_ledger.json
```

The production fixture must cite, but not copy as evidence, the reference
fixture digest. It includes its own source, environment, state, and receipt
digests.

## 15. Promotion rule

The phrase **protected acoustic instrument** is allowed only when the same
production packet passes:

- frozen family/rank/gap correspondence;
- topology correspondence;
- complete path execution;
- lawful local factors;
- persistent state;
- bidirectional acoustic participation;
- open-system ledger;
- robust observable transfer;
- independent cross-runtime replay.

Audibility and fault tolerance remain separate claims even after that
promotion.
