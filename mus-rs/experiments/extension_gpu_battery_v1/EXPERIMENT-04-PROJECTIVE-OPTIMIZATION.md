# Experiment 4 — Quotient-Aware Positive-Realization Learning

**Stable ID:** `E4-PROJOPT`  
**Primary question:** Can explicit gauge conditioning and projective-aware optimization make observable positive realization recovery more stable across factorization seeds, more robust to perturbation, and more monotone with observed data volume?

---

# 1. Theory under test

Loop 2 found that observable NMF + NNLS entered a near-oracle positive-projective basin, while random task-only optimization did not. It also found a nonmonotonic relation between observed stream length and downstream predictive quality.

The proposed explanation is that four distinct problems were being conflated:

1. estimating observable word probabilities;
2. factorizing them into a positive realization;
3. choosing a well-conditioned representative of a gauge-equivalent function;
4. optimizing within or near the useful basin.

The hypothesis is:

> Canonical gauge balancing and projective-aware preconditioning reduce factorization-seed variance, enlarge the recoverable basin, and restore approximate statistical monotonicity without sacrificing held-out likelihood.

---

# 2. Functional model and gauge

For row-state convention:

```text
alpha_{t+1} = alpha_t M_x
p(x_next | alpha_t) ∝ alpha_t M_x terminal.
```

For an invertible positive-compatible similarity `G`:

```text
alpha'    = alpha G
M_x'      = G^-1 M_x G
terminal' = G^-1 terminal
initial'  = initial G.
```

Predictions are unchanged.

The positive cone restricts admissible gauges. The core exact gauge interventions are:

- positive diagonal similarities;
- permutations;
- accepted near-identity positive similarities whose transformed factors remain above `1e-8`.

Raw factor distance across these gauges is meaningless.

---

# 3. Observable initialization pipeline

For each process and stream length:

1. generate or load the locked `factor_train` stream;
2. estimate word probabilities through basis depth 2 by default and depth 3 in extended conditions;
3. construct `H` and `H_x`;
4. fit multiple KL-NMF candidates;
5. solve positive `M_x`, initial, and terminal using NNLS;
6. score candidates on a held-out observed validation stream;
7. retain the top four candidate factorizations plus one median and one poor-but-valid candidate;
8. pass all retained candidates to differentiable refinement.

Do not select by exact hidden-state distance or oracle beliefs.

CPU preprocessing artifacts are immutable inputs to GPU refinement.

---

# 4. Canonical gauge objective

Let `g ∈ R^r` with `sum(g)=0` parameterize diagonal similarity `G=diag(exp(g))`.

Define transformed factors as above and minimize the gauge-conditioning objective:

```text
J_gauge(g) =
  w_init * Var(log(initial'))
  + w_term * Var(log(terminal'))
  + w_row * mean_x Var(log(row_sum(M_x')))
  + w_col * mean_x Var(log(col_sum(M_x')))
  + w_entry * mean_x Var(log(M_x'))
```

with all logs using floor `1e-12`.

Default weights:

```text
w_init=1, w_term=1, w_row=1, w_col=1, w_entry=0.1.
```

Optimize `g` with deterministic float64 L-BFGS for at most 100 iterations. Apply the exact similarity transform and verify prediction equivalence on the fixed gauge probe.

This is a chosen canonical section, not a theorem that the optimum is unique. Emit Hessian eigenvalues and residual symmetries.

## 4.1 Global scale

A single global scale across every symbol operator changes sequence-length-dependent probability unless initial/terminal conventions compensate. Normalize only through a function-preserving transformation explicitly verified on word probabilities through length 8.

---

# 5. Optimizer conditions

## `Q0-ADAMW`

Loop 2 baseline: softplus factors, ordinary AdamW, no periodic gauge operation.

## `Q1-BALANCE-RESET`

Every 25 updates:

1. map effective factors to float64;
2. solve the canonical gauge objective;
3. transform factors, initial, and terminal exactly;
4. map back to raw parameterization;
5. reset optimizer moments for transformed factor parameters;
6. preserve schedule step and readout-independent state.

This cleanly tests function-preserving reconditioning without pretending moments are covariant.

## `Q2-BALANCE-COVARIANT`

Apply the same gauge transform but transform first moments approximately with the factor Jacobian and second moments by squared scale factors. Verify on a quadratic toy model before use. If covariance tests fail, this condition is marked experimental and excluded from the primary optimizer comparison.

## `Q3-MIRROR`

Optimize positive effective factors in log coordinates:

```text
M_x = exp(theta_x)
initial = exp(theta_i)
terminal = exp(theta_t).
```

Use mirror/exponentiated-gradient updates with gradient clipping in dual coordinates. Project only the manifest-declared global normalization after each step.

A softplus-versus-exponential parameterization control separates mirror geometry from positivity parameterization.

## `Q4-GAUSS-NEWTON`

Use a damped empirical Gauss–Newton/natural-gradient approximation on sequence negative log likelihood.

- block diagonal over symbol operators plus initial/terminal blocks;
- Fisher/Jacobian estimated on the current minibatch;
- conjugate-gradient solve with at most 20 iterations;
- damping grid selected on validation from `{1e-4,1e-3,1e-2,1e-1}` during Stage 1;
- trust-region clip by predicted KL change.

The implementation must report residual of the linear solve and actual versus predicted loss reduction.

## `Q5-HYBRID`

Canonical balance every 25 steps plus the promoted projective-aware optimizer (`Q3` or `Q4`). This is the primary candidate if its components pass their individual numerical gates.

## `Q6-RANK-CONT`

Rank continuation control:

- begin at rank `r-2`;
- train to validation plateau;
- split the two states with largest predictive residual contribution;
- add small log-positive perturbations;
- continue at target rank.

This is not quotient-aware by itself; it tests whether staged rank is an alternative route into the basin.

---

# 6. Gauge transformations during training

Each optimizer is tested under four initialization gauges:

1. canonical gauge;
2. random diagonal gauge with condition `1e2`;
3. random diagonal gauge with condition `1e4`;
4. random permutation plus condition-`1e2` diagonal gauge.

The function at step zero must agree to `1e-8` float64.

A quotient-aware method should produce similar predictive trajectories after accounting for optimizer stochasticity. Report:

- trajectory BPS disagreement;
- character-invariant disagreement;
- gradient/update norms in canonical coordinates;
- time to return to a conditioned gauge.

---

# 7. Process and data sweep

Core processes:

- `P-EASY4`;
- `P-BASE6`;
- `P-NOISY6`;
- `P-WEAK8`.

Observed stream lengths:

```text
10k, 20k, 50k, 100k, 250k, 500k, 1m, 2m
```

Target ranks:

- process-matched exact rank;
- one under-ranked control;
- one over-ranked control.

NMF seeds:

```text
[7, 11, 19, 23, 31, 37, 41, 43]
```

Training seeds are independently derived from factorization seed and optimizer condition.

---

# 8. Staged schedule

## 8.1 CPU preprocessing

For all process/stream/NMF-seed combinations:

- estimate factors;
- emit reconstruction and shifted-block residuals;
- compute canonical gauge;
- score held-out observable likelihood;
- persist top/median/poor valid candidates.

This phase is parallel CPU work and does not count against GPU hours.

## 8.2 Stage 1 GPU screen

- processes: `P-BASE6`, `P-NOISY6`;
- stream lengths: `20k, 100k, 500k`;
- conditions: `Q0`–`Q6`;
- factorization seeds: `[7, 11, 19]`;
- training updates: 500;
- sequence length: 128;
- batch: 256;
- evaluation every 50 steps.

Promote mandatory `Q0`, `Q1`, and the best projective-aware condition. Promote `Q5` only if both components pass gates.

## 8.3 Stage 2 confirmation

- all four core processes;
- all eight stream lengths;
- selected optimizer conditions;
- five core factorization seeds;
- target rank plus under/over-rank controls on `P-BASE6` and `P-WEAK8`;
- 1,200 updates;
- sequence length 256;
- batch 256.

## 8.4 Basin sweep

For each process and selected optimizer, perturb the best observable initialization in log-positive coordinates with RMS noise:

```text
0.02, 0.05, 0.10, 0.20, 0.50, 1.0, 1.5
```

Run ten perturbation seeds at 600 updates. A recovery is declared when final excess BPS is within `0.01` of the unperturbed selected initialization.

Fit a logistic curve and estimate the 50%-recovery perturbation radius.

---

# 9. Primary metrics

## 9.1 Held-out predictive quality

- initial and final excess BPS;
- best validation BPS;
- final test BPS after configuration lock;
- future-mode `R²` and weak-mode recovery.

## 9.2 Across-seed stability

For each process/stream/optimizer:

- variance and median absolute deviation over NMF seeds;
- worst valid seed;
- probability of entering the useful basin;
- rank correlation between factor-selection score and final likelihood.

## 9.3 Sample-size monotonicity

For adjacent stream lengths `n_i < n_j`, define success when median final excess BPS at `n_j` is no greater than at `n_i` plus tolerance `0.002`.

Report:

```text
monotonicity_score = successful_adjacent_pairs / total_adjacent_pairs.
```

Also fit isotonic regression and report total violation mass.

## 9.4 Conditioning

- canonical gauge objective;
- operator entry dynamic range;
- aggregate operator condition;
- initial/terminal dynamic range;
- Gauss–Newton/Fisher condition estimates;
- optimizer update anisotropy.

## 9.5 Functional invariants

For all words through length 6 on the four-symbol alphabet, sample or enumerate a versioned subset and record normalized word probabilities and trace/character-like invariants. Equivalent gauges must agree.

---

# 10. Mandatory controls

## 10.1 Exact initialization

Known HMM factors initialize the same optimizer conditions. This separates optimizer damage from factor-recovery error. Exact initialization is analysis-only and never used to tune observable candidate selection.

## 10.2 Random initialization

Standard random positive model under each optimizer. This verifies whether quotient-aware methods help only near a good factorization or also enlarge global access.

## 10.3 Function-preserving gauge replay

Take one checkpoint, apply each allowed gauge, and resume training. A quotient-aware method should have less gauge-dependent predictive drift.

## 10.4 Frozen factors/readout-only

Freeze transport factors and train only any external readout/probe. This measures how much refinement changes the realization rather than merely adapting output coordinates.

## 10.5 Factor-selection oracle ceiling

For analysis only, select the best candidate by test likelihood and report the resulting optimistic ceiling. It may not appear in the main comparison or guide training.

---

# 11. Registered prediction and falsifier

Support requires the four-part threshold in `ANALYSIS-AND-DECISION-PLAN.md`:

- lower factorization-seed variance;
- improved sample-size monotonicity;
- larger recoverable basin;
- no material loss penalty.

Expected mechanism:

- canonical balancing reduces factor dynamic range and optimizer anisotropy;
- mirror or Gauss–Newton updates align with projective likelihood geometry;
- trajectories become less sensitive to equivalent initial gauges;
- weak predictive modes survive refinement more consistently.

The claim is falsified if conditioning metrics improve but predictive variance/monotonicity do not, or if gauge operations damage held-out likelihood beyond tolerance.

---

# 12. Numerical and implementation gates

- every gauge application preserves fixed-probe probabilities within `1e-8` float64 and `2e-5` fp32;
- no transformed factor falls below `1e-12` in float64 before raw reparameterization;
- inverse-softplus/exponential mappings remain finite;
- Gauss–Newton CG residual `<=1e-3` relative or the step is rejected;
- predicted KL trust region is respected within 25%;
- optimizer-state transformation unit tests pass on a small exact quadratic before `Q2` is scientifically enabled;
- checkpoint reload and canonicalization are idempotent within tolerance.

---

# 13. Required artifacts

CPU factor candidate:

```text
hankel.npz
shifted-hankel.npz
factorization.npz
factor-diagnostics.json
canonical-gauge.json
validation-score.json
```

GPU run:

```text
gauge-history.jsonl.zst
condition-history.jsonl.zst
character-invariants.json
basin-result.json
optimizer-diagnostics.json
```

Aggregate:

```text
variance-table.json
monotonicity-table.json
basin-curves.json
gauge-trajectory-comparison.json
decision-E4-PROJOPT.json
```