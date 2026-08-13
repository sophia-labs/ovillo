# Analysis and Decision Plan

## 0. Status

This document is pre-registration for EXTENSION GPU Battery v1. The implementation may correct algebraic or software errors before campaign lock, but may not change primary endpoints, success thresholds, seed partitions, or exclusion rules after the first non-smoke scientific result is inspected.

---

# 1. General analysis principles

## 1.1 Unit of replication

The independent replication unit is normally a **training seed within a process/task condition**. Sequence positions and minibatches are not treated as independent scientific replications.

For process-atlas conclusions, process is an additional random/blocking factor. Analyses report:

- within-process paired effects;
- process-level heterogeneity;
- and pooled hierarchical estimates only as secondary summaries.

## 1.2 Paired design

Conditions being compared share:

- process specification;
- training/evaluation stream hashes where scientifically appropriate;
- seed;
- model size tier;
- step budget;
- and evaluation probes.

Primary effects are paired differences, not comparisons of independently noisy means.

## 1.3 Test-set discipline

The test split is evaluated once after Stage 2 configuration lock. Stage 1 selection uses validation only. Untouched replication seeds use the same locked test protocol but are reported separately.

## 1.4 Uncertainty

Primary uncertainty is a paired bootstrap over seeds, stratified by process/task. Report:

- median paired effect;
- mean paired effect;
- 95% bootstrap interval;
- sign consistency across seeds;
- and process/task-specific effects.

With only five core seeds, the interval is descriptive rather than asymptotically exact. Promotion requires both effect size and sign consistency, not a p-value alone.

## 1.5 Multiple comparisons

Each experiment has one primary decision endpoint. Secondary endpoints use Benjamini–Hochberg FDR at `q=0.10` within the experiment family. Exploratory diagnostics are labeled exploratory and never retroactively promoted to primary.

## 1.6 Missing and censored runs

Runs may be excluded only for:

- invalid numerics under a predeclared invariant gate;
- infrastructure failure before the minimum training fraction;
- corrupt or incomplete artifacts;
- or a manifest mismatch.

Poor performance is not an exclusion reason.

Attention emergence runs that do not emerge by the deadline are right-censored, not discarded. Survival/AFT analyses must include them.

---

# 2. Common metrics

## 2.1 Predictive loss

```text
BPS = negative log2 probability per symbol
excess BPS = model BPS - exact oracle BPS
```

All BPS values are accumulated in float64 over a fixed probe stream.

## 2.2 Future-mode recovery

From the fixed future-test matrix, compute singular modes ordered by data singular value. For each mode, fit a ridge probe on a training probe set and evaluate held-out `R²`.

Report:

- vector of per-mode `R²`;
- count above `0.5`;
- singular-value-weighted `R²`;
- weak-mode mean over the lower half of non-negligible modes;
- acquisition step when each mode first remains above threshold for three checkpoints.

The future-test basis and ridge regularization grid are fixed before training.

## 2.3 Principal-angle alignment

Given teacher predictive coordinates and model states, estimate linear subspaces on a training probe and compute canonical principal angles on held-out probes. Report squared cosine mass and the full angle spectrum.

Raw state-coordinate correlation is never sufficient because teacher and model gauges differ.

## 2.4 Functional contribution rank

For attention or transport-bank branches, stack per-component outputs over the fixed probe family and compute singular values `s_k`. Report:

```text
participation_rank = (sum s_k)^2 / sum s_k^2
effective_rank_0_1 = count(s_k / s_1 > 0.1)
```

The participation rank is primary for continuous phase diagrams; threshold rank is a diagnostic.

## 2.5 Condition and gauge metrics

Positive realizations report:

- aggregate operator condition number;
- initial/terminal dynamic range;
- per-symbol operator dynamic range;
- shifted-Hankel residual;
- normalized trace-word/character invariants;
- prediction disagreement under equivalent gauges.

No single factor condition metric is assumed to be canonical; all named metrics are emitted.

---

# 3. Experiment-specific primary decisions

## 3.1 E1-PREDCOORD

Primary contrast:

```text
operator/future-test distillation
minus
predictive CCA
```

Primary endpoint: paired test excess BPS averaged equally over `P-NOISY6` and `P-WEAK8`.

Support requires all of:

1. median improvement at least `0.010` excess BPS;
2. improvement in at least four of five core seeds on each difficult process, or nine of ten process-seed pairs overall with neither process showing a negative median;
3. 95% paired bootstrap interval below zero;
4. weak-mode mean `R²` improves by at least `0.10` absolute on one difficult process and does not decline by more than `0.03` on the other;
5. equivalent teacher gauges change the estimated treatment effect by less than 20% of its magnitude.

Falsified if the primary effect is nonnegative or if gains disappear under equivalent-gauge tests. Otherwise `inconclusive`.

State-coordinate distillation is a secondary treatment. Its purpose is to test the prediction that operator/future-test distillation is more gauge robust.

## 3.2 E2-CATEGORY

Primary endpoint: category-selection/causal-ablation score over the calibrated common-input condition.

For task `q` and branch `b`, define:

```text
selection(q,b) = mean gate mass assigned to b
causal_drop(q,b) = performance(full) - performance(ablated b)
```

Support requires:

- filtering: projective selection `>=0.70` and its ablation causes at least 80% of the gap between full and chance/oracle-naive performance;
- loop orientation: noncommuting selection `>=0.70`, Clifford ablation reduces accuracy by at least 35 percentage points, commuting ablation changes accuracy by less than 5 points;
- mixed task: both projective and noncommuting branches receive `>=0.20` mass and both ablations produce the predicted selective deficit;
- null/local task: at most one specialized branch remains above `0.20` mass under the MDL penalty;
- all conditions replicate in four of five seeds.

The `cold` from-random bank is reported separately. Failure of cold discovery with successful calibrated selection means `supported_calibrated_only`, encoded as `inconclusive` in the coarse outcome plus a reason field.

Falsified if branch selection is nonselective, if ablations do not match gate claims, or if the commuting branch substitutes for the noncommuting branch on loop orientation.

## 3.3 E3-HEADMECH

The geometric rank law is a required sanity gate, not the new primary claim:

```text
corr(functional_rank, min(H, routes)) >= 0.98
MAE <= 0.10
```

Primary claim: a low-dimensional dimensionless emergence law predicts held-out emergence times.

The analysis has three partitions determined before launch:

- law-fit conditions: 60%;
- law-selection validation: 20%;
- untouched optimizer/scale holdout: 20%.

Candidate laws are sparse linear or generalized-additive models over log dimensionless groups defined in the experiment spec. The selected law must be frozen before holdout evaluation.

Support requires:

- holdout `R² >= 0.80` on uncensored log emergence time;
- median absolute relative error `<=25%`;
- concordance index `>=0.85` including censored runs;
- correct ordering for at least 80% of held-out condition pairs;
- no optimizer-specific signed bias larger than 20% of median emergence time.

If no candidate meets these thresholds, the simple-law hypothesis is falsified for the tested regime. A strong optimizer-specific law but no universal law is reported as a narrower secondary result.

## 3.4 E4-PROJOPT

Primary comparison:

```text
best preregistered quotient-aware optimizer
versus
AdamW baseline
```

Primary endpoint is a composite with separately reported components:

1. across-factorization-seed variance of held-out excess BPS;
2. adjacent sample-size monotonicity score;
3. basin recovery radius;
4. mean held-out excess BPS.

Support requires:

- at least 2× reduction in seed variance;
- monotonic ordering in at least 80% of adjacent stream-size pairs, averaged over core processes;
- at least 1.5× increase in 50%-success perturbation radius;
- mean excess BPS no worse than baseline by more than `0.005`;
- equivalent-gauge prediction residual below numerical tolerance.

No scalar composite may hide a tradeoff. All four conditions must hold. If balancing improves conditioning but worsens likelihood beyond tolerance, the claim is not supported.

---

# 4. Secondary analyses

## 4.1 Sequential mode acquisition

For E1 and E4, compare acquisition time with teacher singular values. Test whether stronger predictive modes emerge earlier and whether canonical supervision specifically accelerates weak modes.

## 4.2 Rich versus lazy change

Track normalized changes in:

- transport operators;
- state representation geometry;
- observer/readout;
- gate/category mass;
- and, where present, anatomy.

Report a richness vector rather than one parameter-distance scalar.

## 4.3 Universality across seeds

Compare trained systems through:

- future-test behavior;
- trace-word characters;
- functional contribution spectra;
- principal angles after canonical alignment;
- and transport-category allocations.

Raw coordinate alignment is exploratory only.

## 4.4 Compute-normalized performance

Report GPU-seconds and train tokens for every condition. Scientific decisions use matched step/data budgets, but compute-normalized curves are important for later engineering promotion.

---

# 5. Early stopping and promotion

Training may stop early for numerical invalidity or explicit convergence:

- validation primary metric changes by less than the experiment-specific epsilon over five checkpoints;
- gradient norm remains below a floor;
- and required mechanism diagnostics have stabilized.

A condition may not stop solely because it is currently worse than another condition unless Stage 1 successive halving permits elimination.

Tier P promotion requires:

- primary support at Tier G;
- successful untouched-seed replication;
- all causal controls passing;
- no unresolved gauge or numerical defect;
- and a written explanation of what additional compute will adjudicate.

---

# 6. Figure and table requirements

The aggregator must produce at least:

1. campaign flow and run accounting;
2. E1 excess-BPS paired effects and per-mode recovery curves;
3. E1 gauge-robustness plot;
4. E2 gate mass plus causal-ablation matrix;
5. E3 functional-rank sanity diagram;
6. E3 emergence-time collapse and holdout residuals;
7. E4 variance/monotonicity/basin panels;
8. GPU utilization and runtime accounting;
9. invariant/numerical gate table;
10. decision summary with supported/falsified/inconclusive outcomes.

Every plot is generated from immutable result artifacts by a versioned analysis command.