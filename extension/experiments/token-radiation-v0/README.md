# Token Radiation v0

**Vocabulary as a family of detector views over one byte-driven predictive
radiation field.**

Token Radiation v0 is the first executable slice of Ariadne Lexical Radiation.
It separates five objects that ordinary language models usually collapse into
one token ID:

```text
lossless surface bytes
    -> source gestures
    -> persistent local body
    -> loading predictive radiation
    -> vocabulary detector lattice
```

The physical law is vocabulary-neutral. `mus-ariadne::law::radiation` removes a
finite-rank projection from a positive storage metric and books exactly the
storage carried by the emitted amplitude. Extension owns bytes, vocabularies,
segmentation lattices, detectors, and predictive experiments.

## What is executable

### Ariadne

- arbitrary-rank metric radiation plans;
- row-orthonormal finite apertures;
- exact passive loading;
- a non-loading projection control using the same aperture;
- emitted amplitude, projected intensity, booked radiation, and closure receipt;
- plan validation and content digest;
- exact-zero, full-removal, non-loading, and invalid-frame tests;
- an independent Python/Rust replay fixture.

### Extension

- one local byte-driven signed/evidence body;
- radiation after every byte boundary, including BOS;
- passive and non-loading body conditions;
- byte, BPE, Unigram, predictive-substring, and oracle-motif vocabularies;
- linear, metric-distance, and positive tight-frame detector implementations;
- exact dynamic-programming marginalization over every compatible segmentation;
- exact expected event rate and token utilization;
- byte-normalized likelihood, detector geometry, radiation, and state receipts;
- a deterministic synthetic macro-process and committed construction result.

Input remains byte-driven in v0. Larger units are **detector views only**. This
is deliberate: it lets us ask whether a vocabulary is useful without changing
the underlying body trajectory.

## Run

```bash
cd extension/experiments/token-radiation-v0
python -m pip install -e '.[dev]'
pytest -q

# Reproduce the committed construction scale.
token-radiation-v0 run \
  --output runs/construction \
  --sequence-bytes 32 \
  --train-sequences 48 \
  --validation-sequences 16 \
  --body-steps 60 \
  --detector-steps 24 \
  --vocabulary-size 18 \
  --radiation-modes passive,nonloading \
  --detectors linear,positive_frame

token-radiation-v0 verify runs/construction
python scripts/verify_fixture.py
python scripts/verify_committed_result.py
python scripts/check_boundaries.py
```

Rust gates:

```bash
cd mus-rs
cargo fmt --all -- --check
cargo clippy -p mus-ariadne --all-targets -- -D warnings
cargo test --release -p mus-ariadne law::radiation::tests
cargo test --release -p mus-ariadne --test metric_radiation_fixture
cargo run --release -p mus-ariadne --example metric_radiation_proof
```

## Construction evidence

One deterministic CPU run, seed `20260814`:

| Body condition | next-byte BPB | direct-state linear-probe BPB | booked radiation / byte | maximum closure residual |
|---|---:|---:|---:|---:|
| passive | 2.3555 | 2.2770 | 2.5528 | 1.62e-13 |
| non-loading | 2.3566 | 2.3030 | 0 | 0 |

Best detector-view points:

| Body | Vocabulary | Detector | exact BPB | expected events / byte |
|---|---|---|---:|---:|
| passive | oracle motifs | linear | **1.4050** | 0.4301 |
| non-loading | oracle motifs | linear | 1.4378 | 0.4309 |
| passive | BPE-18 | linear | 1.5754 | 0.4640 |
| non-loading | BPE-18 | linear | 1.6027 | 0.4632 |
| passive | Unigram-18 | linear | 1.7512 | 0.4810 |
| passive | predictive-18 | linear | 2.0690 | 0.5946 |
| passive | bytes | linear | 2.3378 | 1.0000 |

The positive-frame detector closes its contextual tight-frame identity to
floating-point precision, but it is worse than the linear detector in this
short construction. The first predictive-substring heuristic is also worse than
BPE and Unigram. Those are retained negative results, not implementation errors.

The oracle-motif result establishes only that the shared radiation contains
multi-byte predictive information that a suitable detector vocabulary can
extract at a lower event rate. It does not establish that the oracle vocabulary
can be learned, that passive loading is beneficial, or that this transfers to
natural language.

Committed artifact:

```text
results/token-radiation-v0-20260814/
scientific digest:
37481aaa0c437e3c14a33fec94692bde886ae79baaf94599c7d9683a0397cfb0
```

## Files

- [`PROGRAM.md`](PROGRAM.md): theory and vocabulary ontology.
- [`EXPERIMENT-SPEC.md`](EXPERIMENT-SPEC.md): laws, conditions, metrics, and
  falsifiers.
- [`INITIAL-FINDINGS.md`](INITIAL-FINDINGS.md): interpretation of the committed
  construction.
- [`NEXT-TRANCHE.md`](NEXT-TRANCHE.md): macros, event clocks, natural bytes, and
  tokenizer-equivalence gates.
- [`schemas/metric-radiation-v1.schema.json`](schemas/metric-radiation-v1.schema.json):
  cross-runtime aperture transport schema.

## Claim boundary

Established in this construction, subject to Rust execution:

- exact finite-rank metric radiation law;
- a loading versus non-loading comparison surface;
- one byte-driven body shared by several output vocabularies;
- exact segmentation marginalization over the shared trajectory;
- a measurable lexical rate–distortion plane;
- a positive-frame detector with an exact contextual intensity identity;
- reproducible construction artifacts and cross-runtime reference values.

Not established:

- natural-language utility;
- tokenizer-independent body state;
- superiority to BPE, Unigram, BLT, byte models, SSMs, or Transformers;
- learned macro-token input;
- adaptive lexical event timing;
- reservoir-powered articulation;
- free autoregressive feedback;
- multilingual lexical factorization;
- canonical `WorldPlan` integration;
- a general predictive-radiation compiler.
