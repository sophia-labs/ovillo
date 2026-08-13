# EXTENSION Theory → Experiment Loop 2

This package contains the second controlled theory/experiment cycle for the EXTENSION research program in `sophia-labs/instrument`.

## Headline findings

- Across 56 trained model/process pairs, future predictive mode decodability ranked models within each process substantially better than raw state size.
- Observable NMF+NNLS factorization reduced the random-initialization gap of a positive projective predictor by 95.17% without hidden-state supervision.
- Increasing generic predictive CCA/balanced rank did not improve next-symbol prediction.
- Functional multi-head attention rank matched `min(declared heads, independent retrieval routes)` with correlation 0.9991.
- A mixed transport bank selected projective transport for filtering and Clifford transport for oriented path memory.
- A sparse cube-compartment implementation converged to dense positive transport with exact mass and quadratic-storage receipts.

## Documents

- `REPORT.md` — complete scientific narrative and claim boundary.
- `THEORY-UPDATE.md` — revised realization-pentad theory.
- `NEXT-LOOP.md` — experiment specification for the next cycle.
- `IMPLEMENTATION-HANDOFF.md` — repository and scaling handoff.
- `VALIDATION.md` — local validation record.
- `RESULTS.json` — compact machine-readable aggregate.

## Source modules

```text
src/extension_theory_loop2/
  core.py                 exact process/filter and model families
  metrics.py              predictive-state and balanced-mode analysis
  atlas.py                process × architecture atlas
  projective_init.py      observed Hankel NMF+NNLS initialization
  projective_search.py    held-out factorization candidate search
  balanced_training.py    future auxiliary and CCA interventions
  attention_phase.py      routing-rank/head-rank phase diagram
  transport_mixture.py    mixed projective/Clifford bank
  compartmental.py        local positive cube transport and ledger
  make_plots.py           deterministic figure generation
  summarize.py            aggregate result construction
```

## Install and test

```bash
python -m pip install -e '.[test]'
python -m compileall -q src tests
pytest
```

## Rebuild plots

```bash
python -m extension_theory_loop2.make_plots \
  --root . \
  --output plots
```

## Scope

These experiments are controlled scientific probes. They do not establish a natural-language advantage, a universal performance law, or a production-ready physical neural architecture.
