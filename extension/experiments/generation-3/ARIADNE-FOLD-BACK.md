# Ariadne fold-back boundary

## Decision

The reusable local constitutive law belongs in `mus-ariadne::law::medium`.
Training and architecture search do not.

Ariadne's canonical runtime already has the right constitution:

```text
WorldPlan
  -> one persistent World
  -> one material clock
  -> reciprocal couplings and shared fields
  -> one global WorkLedger
  -> non-loading observations
```

Generation 3 adds a new local law vocabulary; it does not create another
`World`, private room, listener, or ledger.

## Ariadne owns

- plan validation and content identity;
- fixed-capacity state allocation;
- signed and positive local transport;
- finite reservoirs and internal active transfer;
- passive loss;
- source-work and closure receipts;
- exact zero and disabled-edge laws;
- damage/restoration without reset;
- fixed-capacity metric reconfiguration;
- non-loading linear observation;
- future integration into body/world coupling and trajectory surfaces.

## Extension owns

- task and process generation;
- future-test and predictive-state definitions;
- differentiable mirrors;
- parameter learning;
- developmental programs;
- topology/morphology search;
- intervention planning;
- functional spectra and causal rank;
- baselines and statistical analysis;
- compilation policy;
- promotion decisions.

## Why the law is not immediately a `BodySpec`

The current canonical body is one fixed 24-mode anatomy. Its plan uses
compile-time constants for modal count, active rank, transport pairs, and space
courses. A learned local graph has dynamic construction-time site and edge
counts.

Forcing it directly into the existing `BodySpec` would either:

- reduce Generation 3 to a disguised 24-mode preset; or
- create a second world/body executor.

The staged integration is therefore:

1. land and validate the domain-neutral local law;
2. make the law replayable from Python and Rust;
3. compare it against non-Ariadne architectures;
4. define a canonical heterogeneous body-kernel enum or trait whose complete
   state, storage, controls, ports, and receipts are statically bound at world
   construction;
5. embed local media as ordinary body instances in one `WorldPlan`;
6. add reciprocal world ports and trajectory coordinates;
7. add remeshing only after a state-transfer constitution exists.

## Required future world interface

A generalized body kernel must expose, without dynamic allocation on the
material thread:

```text
stored_energy()
process_local(controls, returning_world_state)
port_effort_flow(port_index)
apply_reciprocal_exchange(port_index, ...)
radiate()
observe_into(...)
reset()
runtime_layout()
cost_estimate()
```

It must also expose stable construction-time schemas for:

- state dimensions and metrics;
- source and coupling apertures;
- typed trajectory coordinates;
- non-loading observer channels;
- configuration-work and remesh receipts.

## No-smuggling rules

A Generation 3 contribution is rejected if it:

- inserts an opaque neural residual into the material step;
- lets a powerful observer reconstruct the answer from raw input history;
- calls output gain or normalization “work closure”;
- changes topology while silently reinterpreting live state;
- owns a private ledger beside the world ledger;
- uses a separate runtime for training claims and Ariadne claims without a
  replay fixture;
- calls grid resolution “model scale” without separating domain, capacity, and
  refinement.
