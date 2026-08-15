# Unified Ariadne lexical world — program

## Thesis

A token should be an observation of interaction already occurring among persistent bodies in a shared world, not the privileged message passed between neural modules.

The canonical causal path is:

```text
surface source gesture
-> lexical constitutive body
-> reciprocal world port
-> delayed shared field
-> predictive constitutive body
-> continued field/body evolution
```

Vocabulary and operator detectors bind non-loadingly to this trajectory. Deleting them must not stop body-to-body communication.

## Core-schema decision

The immutable modal `WorldPlan` v2 schema remains unchanged. A new additive `ConstitutiveWorldPlan` admits typed body constitutions while preserving one canonical `World` executor.

This is preferable to silently altering the v2 body record because content-addressed plans must not change meaning under an existing schema version. A modal-v2 upgrade helper produces an explicit v3 plan and a new digest rather than mutating historical identity.

## Body constitutions

### Modal

The existing 24-mode body, existing `BodyControls`, and existing audio behavior remain available without reinterpretation.

### Local medium

An arbitrary `LocalMediumPlan` sits behind a stable, storage-bearing `MODE_COUNT` world boundary. The boundary permits all bodies to participate in the same existing:

- `SpacePortSpec`;
- `DirectCouplingSpec`;
- delayed shared field;
- body radiation path;
- global work ledger.

The boundary is not an observer. Its input and output bindings are constitutive transducers:

- incoming boundary coordinates modulate declared medium sources;
- the boundary reservoir pays or receives the exact source work;
- declared wave/evidence/reservoir projections may drive boundary momentum;
- the same finite reservoir pays or receives output-transduction work;
- passive boundary decay is booked as internal loss.

World-integrated `LocalMediumPlan`s must omit embedded `LocalMediumObserverSpec`s. Non-loading observation programs bind separately to the physical world digest and are never reused to drive a port.

## Why a fixed-rank boundary

The local medium may have any number of sites and edges, while the shared-world field has a stable finite-rank ABI. A fixed-rank boundary:

- preserves existing shared-space and direct-coupling laws;
- isolates internal morphology from world topology;
- permits constitutions to change internally without reinterpreting field state;
- gives remeshing a future explicit state-transfer seam;
- keeps the hot shared-world path statically allocated.

It is an Ariadne aperture, not a hidden embedding.

## Runtime order

Both legacy and constitutive plans use the same executor order:

1. process each persistent body locally;
2. apply storage-free reciprocal direct couplings;
3. exchange every body boundary with one delayed shared field;
4. scatter, damp, and radiate the field;
5. radiate body boundaries;
6. close one global ledger frame.

## Extension ownership

Extension continues to own:

- controlled processes and natural corpora;
- learned body constitutions;
- source-gesture search;
- event clocks and detector atlases;
- articulatory models;
- causal interventions;
- differentiable mirrors and optimization;
- compilation to `ConstitutiveWorldPlan`.

Ariadne owns:

- plan validation and identity;
- persistent state;
- local laws;
- finite supplies;
- port transduction;
- reciprocal coupling;
- shared fields;
- radiation;
- reconfiguration refusals;
- global receipts.

## Current claim boundary

The CPU mirror establishes a causal shared-field channel in a controlled task. The Rust branch establishes the intended canonical construction surface in source and tests. Because no Rust toolchain was available, this tranche does not claim executed Rust parity or ledger closure for the new adapter.
