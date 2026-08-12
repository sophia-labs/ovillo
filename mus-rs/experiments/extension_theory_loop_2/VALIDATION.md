# EXTENSION Theory Loop 2 — Validation Record

## Environment

```text
Python       3.13.5
PyTorch      2.10.0+cpu
NumPy        2.3.5
SciPy        1.17.0
scikit-learn 1.8.0
Matplotlib   3.10.8
pytest       9.0.2
Execution    CPU
```

The package declares Python 3.11+ and broader dependency lower bounds. The local record certifies only the environment above; GitHub Actions separately replays the package on Python 3.11.

## Commands

```bash
cd extension-theory-loop-2

python -m pip install --no-build-isolation --no-deps -e .
python -m compileall -q src tests
pytest

python -m extension_theory_loop2.make_plots \
  --root . \
  --output /tmp/extension-loop2-plots-smoke

python -m extension_theory_loop2.compartmental \
  --output /tmp/extension-loop2-compartment-smoke.json
```

## Results

```text
10 tests passed in 1.28 s
9 figures regenerated
compartmental smoke gates passed
```

The compartmental smoke requires:

```text
microstep-64 excess BPS                 < 1e-4
maximum mass residual                   < 1e-10
maximum quadratic-storage ledger error  < 1e-10
```

## Test coverage

The deterministic tests cover:

1. Clifford skew-symmetry, square law, and pairwise anticommutation;
2. exact positive observable realization versus the Bayesian oracle;
3. global projective operator-scale gauge invariance;
4. finite contracting predictive-filter Lyapunov spectrum;
5. observed Hankel NMF+NNLS positivity, shape, and executable model loading;
6. cube-compartment positivity, stochasticity, and edge locality;
7. exact mass and quadratic-storage ledger closure;
8. attention tensor and invariant-component shapes;
9. collapse of closed loops under commuting Clifford generators;
10. finite forward behavior of the typed transport bank.

## Result integrity

`RESULTS.json` is generated from compact aggregate summaries under `runs/`. Full checkpoints and raw logs are retained in the portable local bundle but intentionally excluded from the ordinary Git landing.

## Boundaries

These checks do not certify:

- GPU or accelerator kernels;
- numerical behavior outside the declared dependency range;
- statistical consistency of the NMF estimator;
- exact recovery for arbitrary observable processes;
- continuum remeshing;
- nonlinear EXTENSION-2 work closure;
- or production real-time execution.
