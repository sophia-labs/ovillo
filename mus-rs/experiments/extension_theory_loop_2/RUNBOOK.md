# EXTENSION Theory Loop 2 — Runbook

## Installation

```bash
python -m pip install -e '.[test]'
pytest
```

For an offline environment with dependencies already installed:

```bash
python -m pip install --no-build-isolation --no-deps -e .
```

## 1. Predictive atlas

Run one model/process pair:

```bash
python -m extension_theory_loop2.atlas run \
  --process p6_baseline \
  --model clifford8 \
  --steps 200 \
  --seed 7 \
  --output runs/atlas/p6_baseline__clifford8.json
```

Aggregate all JSON results in the atlas directory:

```bash
python -m extension_theory_loop2.atlas aggregate \
  --directory runs/atlas \
  --output runs/atlas_summary.json
```

Process names and model names are declared in `atlas.py`.

## 2. Positive projective initialization

Random baseline:

```bash
python -m extension_theory_loop2.projective_init run \
  --condition random \
  --process p6_baseline \
  --steps 300 \
  --stream-length 100000 \
  --seed 7 \
  --output runs/projective/random_s7.json
```

Observed Hankel NMF+NNLS initializer:

```bash
python -m extension_theory_loop2.projective_init run \
  --condition nmf \
  --process p6_baseline \
  --steps 300 \
  --stream-length 100000 \
  --seed 7 \
  --output runs/projective/nmf_s7.json
```

Candidate factorization search:

```bash
python -m extension_theory_loop2.projective_search \
  --stream-length 100000 \
  --steps 300 \
  --seed 7 \
  --output runs/projective_search/n100k_s7.json
```

## 3. Balanced-state interventions

```bash
python -m extension_theory_loop2.balanced_training run \
  --condition baseline \
  --steps 300 \
  --seed 7 \
  --output runs/balanced/baseline_s7.json

python -m extension_theory_loop2.balanced_training run \
  --condition cca \
  --steps 300 \
  --seed 7 \
  --output runs/balanced/cca_s7.json
```

Conditions: `baseline`, `future_aux`, `cca`, and `aux_cca`.

## 4. Attention routing-rank phase diagram

```bash
python -m extension_theory_loop2.attention_phase run \
  --routes 4 \
  --heads 4 \
  --steps 600 \
  --collision-noise 0.001 \
  --seed 7 \
  --output runs/attention/m4_h4_s7.json
```

Aggregate:

```bash
python -m extension_theory_loop2.attention_phase aggregate \
  --directory runs/attention \
  --output runs/attention_summary.json
```

## 5. Mixed transport bank

```bash
python -m extension_theory_loop2.transport_mixture run \
  --condition full \
  --steps 300 \
  --seed 7 \
  --output runs/mixture/full_s7.json
```

Conditions: `full`, `commuting`, `projective_only`, `clifford_only`.

## 6. Local compartmental lowering

```bash
python -m extension_theory_loop2.compartmental \
  --output runs/compartmental/result.json
```

## 7. Rebuild aggregate and figures

```bash
python -m extension_theory_loop2.summarize \
  --root . \
  --output RESULTS.json

python -m extension_theory_loop2.make_plots \
  --root . \
  --output plots
```

## Reproduction notes

- All training scripts set PyTorch CPU thread count to one for predictable small-run behavior.
- Exact numerical values can vary across PyTorch, BLAS, SciPy, and scikit-learn versions.
- The compact GitHub landing omits `.pt` checkpoints and exhaustive logs. The full portable artifact retains them.
