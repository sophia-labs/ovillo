# Campaign Matrix and Provisional Compute Envelope

## 0. Interpretation

The counts below are **micro-runs**, not pods. A GPU shard batches compatible micro-runs along an ensemble dimension. Wall-time estimates are provisional until Tier C calibration measures the target GPU.

The planner is authoritative: it uses measured step times and a hard 21,600 GPU-second core budget. If the full promoted matrix exceeds budget, it applies the registered successive-halving rules; it never removes mandatory baselines or controls silently.

---

# 1. Provisional core matrix

| Component | Approximate micro-runs before halving | Typical ensemble/shard | Provisional GPU cap |
|---|---:|---:|---:|
| CUDA calibration | 12–24 | 1 calibration process | 0.4 h |
| E1 Stage 1 | 72 | 16–32 | 0.35 h |
| E1 confirmation + gauge audit | 180–380 | 16–32 | 0.9 h |
| E2 Stage 1 + single-branch controls | 60–90 | 8–24 | 0.35 h |
| E2 confirmation/OOD/ablations | 100–180 | 8–24 | 0.7 h |
| E3 rank sanity and light sweep | 150–300 | 32–64 | 0.45 h |
| E3 anchor/LHS training | 500–750 | 32–64 | 0.8 h |
| E3 diagnostic replay | 80–160 checkpoints | 2–16 | 0.8 h |
| E4 differentiable screen | 100–150 | 16–32 | 0.25 h |
| E4 confirm and basin sweep | 300–600 | 16–32 | 0.65 h |
| retries/reserve | — | — | 0.3 h |
| **Total target** | — | — | **5.95 h** |

These caps are allocations, not promises. A modern GPU may execute the tiny train loops much faster when vectorized; second-order diagnostics may dominate E3.

---

# 2. Packing priorities

## E1

Compatibility key:

```text
student architecture × treatment family × process state size × sequence length
```

Gauge audits can batch eight equivalent teachers against the same student initialization. Teacher construction remains CPU preprocessing.

## E2

Pack by:

```text
task × initialization regime × branch shape × MDL multiplier
```

Ablations are forward-only replay jobs and should not retrain unless explicitly required.

## E3

Use three shard classes:

- 64-member light training where shapes match;
- 8–16-member tangent/Hessian replay;
- 2–4-member gradient-covariance replay.

Do not let heavy diagnostics serialize the entire light sweep.

## E4

NMF/NNLS is CPU preprocessing. GPU shards batch differentiable refinements by target rank and optimizer. Basin perturbations are ideal ensemble members.

---

# 3. Successive-halving floor

The following cells are never removed:

- E1: `T0`, `T2`, `T4` on every primary process/student;
- E2: full bank, each single-branch ceiling, commuting control, and null task;
- E3: static rank sanity grid, all three optimizer families, and censored-condition accounting;
- E4: `Q0`, `Q1`, exact-init control, random-init control, and all core stream lengths for the primary process.

If budget cannot cover these floors after calibration, the campaign does not launch; it is redesigned under a new spec revision.

---

# 4. Extended-tier estimates

A 20–40 GPU-hour extension can add:

- ten-seed confirmation;
- all seven Loop 2 process regimes;
- depth-3 observable Hankel factors;
- broader projective ranks and gauges;
- denser E3 optimizer/curvature phase diagrams;
- volumetric E1 confirmation at multiple grid resolutions;
- and independent external replication manifests.

Natural text and inverse localization remain separate campaigns rather than being squeezed into unused reserve.