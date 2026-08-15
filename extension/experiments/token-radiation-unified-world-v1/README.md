# Token Radiation Unified World v1

**Two constitutive bodies, one delayed Ariadne field, one material clock, one ledger, and no privileged token channel.**

This tranche moves Token Radiation from two separately coordinated Extension models into the architecture of Unified Ariadne's canonical shared world.

```text
surface gestures
      |
      v
lexical body -- reciprocal port -- shared delayed field -- reciprocal port -- predictive body
      |                                  |                                  |
      +---------------- one material clock / one global ledger -------------+
                                         |
                                 non-loading analysis
                              byte / operator / event views
```

In the `shared` condition no byte ID, token ID, operator ID, detector logit, or hidden tensor is passed from the lexical body to the predictive body. Operator heads are non-loading probes used only to train and measure the predictive body state.

## Ariadne-core integration

The Rust change adds an additive, content-addressed `ariadne-world-plan-v3` schema. Existing `ariadne-world-plan-v2` artifacts retain their exact meaning and digest. A lossless `ConstitutiveWorldPlan::from_modal_v2` upgrade creates a new v3 identity when migration is desired. Both plan families compile into the same `mus_ariadne::World` executor.

A constitutive body may be:

- the existing authored modal `BodySpec`; or
- a `LocalMediumPlan` behind a fixed-rank, storage-bearing world boundary.

The boundary is the stable ABI used by the existing shared-space and direct-coupling laws. Local-medium input/output transduction is explicitly finite-supply and work-accounted. World-integrated local-medium plans refuse embedded observers; non-loading observation programs remain outside the physical plan and are not reused as transducers.

The canonical step remains:

```text
all bodies process locally
-> direct reciprocal coupling
-> every body exchanges with one shared delayed field
-> field scattering / loss / radiation
-> body radiation
-> one global WorkLedger frame
```

## CPU construction

The differentiable CPU mirror compares four conditions:

| Condition | Communication |
|---|---|
| `oracle_handoff` | Explicit latent-operator source gesture; positive ceiling |
| `isolated` | Two bodies, one clock, all inter-body coupling disabled |
| `direct` | Storage-free instantaneous reciprocal coupling |
| `shared` | No direct edge; communication only through the delayed shared field |

The world uses the same controlled noncommutative operator process as Lexical Interface v1, including the context-dependent surface homograph `ba`.

### Three-seed result

| Condition | Current operator accuracy | Homograph accuracy | Exact 8-event sequence |
|---|---:|---:|---:|
| Oracle handoff | 0.9465 ± 0.0272 | 1.0000 | 0.5720 |
| Isolated | 0.3356 ± 0.0688 | 0.3917 | 0.0000 |
| Direct reciprocal | **0.8364 ± 0.0246** | **0.7673** | 0.1415 |
| Shared delayed field | 0.7145 ± 0.0566 | 0.7100 | 0.0382 |

Shared-field causal controls:

```text
shared intact current accuracy          0.7145
cross-world field permutation        0.3367
lexical port disabled                   0.2854
predictive port disabled                0.2772
chance                                  0.2500
```

The shared field therefore carries real sequence-specific operator information. It is not yet as efficient as direct coupling, and whole-sequence reliability remains poor.

Maximum float32 differentiable-mirror closure residual:

```text
5.72e-05
```

Scientific digest:

```text
fe412bf08a1c1b84c895c00d67ddf746aa3107fe5e38b1b07e68fb3c9f28b502
```

## Run

```bash
cd extension/experiments/token-radiation-unified-world-v1
python -m pip install -e '.[dev]'
pytest -q

token-radiation-unified-world-v1 run \
  configs/construction.json \
  runs/construction

token-radiation-unified-world-v1 verify runs/construction
python scripts/verify_committed_result.py
```

Rust gates:

```bash
cd mus-rs
cargo fmt --all -- --check
cargo clippy -p mus-ariadne --all-targets -- -D warnings
cargo test --release -p mus-ariadne
cargo test --release -p mus-ariadne --test constitutive_shared_world
cargo run --release -p mus-ariadne --example constitutive_shared_world_proof
```

## Decision

Supported at construction scale:

- local-medium bodies can be admitted to the canonical shared-world schema without creating a second executor;
- a delayed shared field can carry context needed to interpret a surface-neutral predictive operator;
- removing either physical port or permuting the field across sequences destroys the signal;
- token/operator observers are not required in the causal communication path;
- direct and delayed coupling are experimentally distinguishable constitutions.

Not supported:

- parity with the privileged operator ceiling;
- superiority to instantaneous reciprocal coupling;
- reliable free whole-sequence generation;
- a learned event clock or articulatory body;
- canonical Rust execution in the construction environment;
- GPU scaling.

## Files

- [`PROGRAM.md`](PROGRAM.md) — architectural integration and ownership.
- [`EXPERIMENT-SPEC.md`](EXPERIMENT-SPEC.md) — conditions, controls, and gates.
- [`FINDINGS.md`](FINDINGS.md) — result interpretation.
- [`NEXT-TRANCHE.md`](NEXT-TRANCHE.md) — CPU work before scaling.
- [`schemas/ariadne-world-plan-v3.schema.json`](schemas/ariadne-world-plan-v3.schema.json) — public plan shape.
