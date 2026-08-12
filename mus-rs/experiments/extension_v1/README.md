# Extension-v1

**Neurons as extended bodies; tokens as gestures; sequence state as path-dependent transport.**

This is a runnable research package for the first concrete version of the proposal informally titled:

> **Attention Is All You Need, But It Is Nice to Have Extension**

The package is not a language model. It establishes the smallest coherent computational substrate, records one important failed route, and specifies the next training program.

The repository landing keeps source, tests, and compact text receipts. The accompanying delivery bundle also contains the full sweep rows, generated figures, and the small learned checkpoint.

## What changed during the investigation

The obvious first attempt was an active cable network: learned strings in `R^4`, persistent excitable fields, geometry-derived contacts, closed token gestures, and a gestural pickup. That model retained order-sensitive state in easier probes but stayed at chance on four-token permutation parity.

The useful response was not to hide the failure behind a larger decoder. It was to derive an exact operator oracle.

The resulting core is a **Clifford gesture medium**:

- latent control space: `R^4`;
- persistent state: `Cl(0,4)` on 16 exterior-basis modes;
- physical grouping: five strings with site counts `1,4,6,4,1`;
- basis gesture: eight local signed quarter-turns;
- continuous gesture: path-ordered orthogonal development;
- sequence observable: multivector grade, including oriented area and four-volume;
- morphology: learned string geometry controlling local-contact fidelity.

A stronger trainable experiment then learned **closed token loops**—every token begins and ends at exactly the same control point—while solving all 24 permutations with one 16-state fiber and 130 parameters. A commuting-generator control collapses all closed loops to identity and accuracy to chance.

## Quick start

```bash
cd mus-rs/experiments/extension_v1
python -m venv .venv
. .venv/bin/activate
pip install -e '.[test]'
pytest
```

Run the reference experiments:

```bash
extension-v1 exact --trials 10000 --output artifacts/exact.json
extension-v1 development --output artifacts/development.json
extension-v1 geometry --ambient-dim 4 --kernel-width .38 \
  --trials 4096 --output artifacts/geometry-d4.json
extension-v1 train-closed --steps 600 --fibers 1 --seed 101 \
  --output-dir artifacts/closed-101 \
  --control-output artifacts/closed-control.json
```

Run the retained negative control:

```bash
extension-v1 active-control --steps 250 --output-dir artifacts/active-control
```

Run morphology inverse design:

```bash
extension-v1 morphology --ambient-dim 4 --kernel-width .50 \
  --steps 200 --output-dir artifacts/morphology-d4
```

## Package layout

```text
extension_v1/
  clifford.py          exact exterior/Clifford reference algebra
  development.py      piecewise-linear path development and trainable layer
  geometry.py         five-string soft local realization and dimension sweep
  closed_gestures.py  learned exactly closed token loops and controls
  morphology.py       inverse design of grade-string geometry
  active_cable.py     active extended-neuron negative/control model
  receipts.py         canonical receipts and digests
  cli.py              command-line surface

tests/                12 deterministic Python tests
rust/mus-extension-reference/  candidate sparse Rust lowering (not compiled here)
artifacts/             compact JSON receipts; full raw artifacts are in the delivery bundle
THEORY.md              mathematical construction and novelty boundary
RESULTS.md             exact evidence ledger
EXPERIMENTS.md         staged training and compute program
IMPLEMENTATION-HANDOFF.md  Rust/WATSN/Gesture integration contract
PAPER-DRAFT.md         provisional paper narrative
```

## Headline results

| Experiment | Result |
|---|---:|
| Exact Clifford algebra residual | about `1.2e-16` |
| Basis-permutation parity | 100% |
| Random four-vector determinant sign | 100%, 10k trials |
| Closed-loop endpoint | exactly zero |
| Closed-loop distance from vacuum | `0.5604` |
| Learned closed-token parity | 100%, 5/5 one-fiber seeds |
| Closed-token parameters | 130 |
| Commuting closed-loop control | 50% |
| 4D soft geometry, width .38 | 100% basis parity; 95.2% volume sign |
| Morphology optimization | operator MSE `0.048 -> 0.010` |
| Naive active cable | 50% parity, retained negative result |

See [RESULTS.md](./RESULTS.md) for full numbers and limits.

## Core equation

The exact medium evolves by

```text
dU = U sum_j D_j dg^j,
```

where the four generators satisfy

```text
D_i D_j + D_j D_i = -2 delta_ij I.
```

For four vector gestures, the final top-grade coefficient is their determinant. For a small closed rectangle, the leading residue is its signed area times the generator commutator.

This makes “gesture over latent space” literal rather than rhetorical.

## What this does not establish

- no language-model result;
- no useful biological neuron reconstruction;
- no length extrapolation yet;
- no unique optimality of four dimensions;
- no topological protection;
- no superiority to generic trainable path-development networks;
- no successful active nonlinear medium yet;
- no scalable learned contact topology.

The nearest prior is especially important: trainable finite-dimensional path-development networks already exist. The Extension claim must be the locally realizable Clifford/string medium, learned morphology, work accounting, and any capability that survives generic development baselines—not path development by itself.

A candidate Rust lowering into the existing `mus-watsn::OperatorProgram` API is included, but the delivery environment had no Rust toolchain. It is explicitly uncompiled and must be replayed before workspace integration.

## Immediate next target

Implement an exact passive Clifford backbone inside `mus-gesture`/`mus-watsn`, then distill it into an active work-accounted string medium. The active residual must add a capability the linear backbone cannot provide while preserving the exact closed-loop and algebraic probes.
