# Extension Generation 3 — learned constitutive worlds

Extension Generation 3 turns the project from a collection of unusual neural
architectures into a compiler and empirical science for **learned constitutive
worlds**.

```text
predictive demand + interventions
              |
              v
Extension learner / developmental program
              |
              v
ariadne-local-medium-v1 plan
              |
              v
mus-ariadne::law::medium
  persistent local state
  ordered metric-compatible transport
  positive evidence transport
  finite reservoirs and active transfer
  passive loss
  damage / restoration
  non-loading observation
  exact receipts
```

The runtime law lives in Ariadne because persistence, storage metrics, work,
loss, coupling, reconfiguration, and exact-zero behavior are Ariadne concerns.
The learner, task, optimizer, causal analysis, architecture comparison, and
compiler search remain Extension concerns.

## What is executable in this tranche

- a Rust `mus-ariadne::law::medium` implementation;
- a differentiable PyTorch mirror;
- compilation from learned parameters to the shared JSON plan;
- a deterministic joint construction task:
  - ordered gesture discrimination with equal token multiset and terminal token;
  - positive-evidence majority classification;
  - temporary local-edge damage without state reset;
- weak affine observers only;
- exact absence controls for wave and evidence sectors;
- a Python/Rust replay fixture with final-state, observation, work, loss, mass,
  and closure comparisons;
- source-boundary, schema, Python, Rust, and replay CI.

The committed construction artifact is under
`artifacts/construction-seed-20260814/`.

## Run the Python proof

```bash
cd extension/experiments/generation-3
python -m pip install -e '.[dev]'
pytest
extension-generation3 train \
  --steps 160 --batch-size 128 \
  --output runs/construction
extension-generation3 replay fixtures/cross-runtime.json
```

## Run the Ariadne law and cross-runtime replay

```bash
cd mus-rs
cargo test -p mus-ariadne law::medium::tests
cargo test -p mus-ariadne --test local_medium_fixture
cargo run -p mus-ariadne --example local_medium_proof
```

## Construction evidence

For the committed seed and 160 training steps:

```text
initial joint BCE             0.708797
final joint BCE               0.019768
path-order accuracy           1.000000
evidence-majority accuracy    1.000000
minimum path accuracy under
  any one registered edge
  damaged at steps 1–2        1.000000
minimum evidence accuracy
  under the same damage       1.000000
zero-wave path control        0.497314
zero-evidence majority ctrl   0.497803
```

This is a construction proof, not a baseline comparison or a natural-sequence
result. It establishes that a lawful four-site constitution can be learned and
compiled while both declared sectors are functionally necessary under the
registered weak observer.

## Documents

- [`PROGRAM.md`](PROGRAM.md): theory and architecture synthesis.
- [`ARIADNE-FOLD-BACK.md`](ARIADNE-FOLD-BACK.md): ownership and integration
  boundary.
- [`LITERATURE-TO-CLAIM-MATRIX.md`](LITERATURE-TO-CLAIM-MATRIX.md): nearest
  predecessors, inherited methods, gaps, and decisive experiments.
- [`NEXT-TRANCHE.md`](NEXT-TRANCHE.md): controlled comparison and scaling
  program.
- [`schemas/local-medium-v1.schema.json`](schemas/local-medium-v1.schema.json):
  transport schema.

## Claim boundary

Established by this tranche, subject to Rust CI execution:

- one content-addressed local constitution;
- persistent signed, positive, and reservoir state;
- ordered local transport;
- non-negative mass-conserving evidence transport;
- finite internal active transfer;
- passive-loss and source-work receipts;
- damage/restoration without state reset;
- fixed-capacity reconfiguration with booked metric work;
- Python/Rust shared replay semantics;
- a learned controlled-task construction.

Not established:

- canonical `WorldPlan` body integration;
- a continuum or mesh-refinement limit;
- developmental growth or topology creation;
- superiority to NCAs, Neural Wave Machines, Neural CDEs, SSMs, or reservoirs;
- natural-language utility;
- physical fabrication;
- biological reconstruction;
- a general material compiler.
