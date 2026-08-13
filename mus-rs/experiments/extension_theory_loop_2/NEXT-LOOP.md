# NEXT LOOP — Predictive Alignment, Quotient Optimization, and Untyped Category Discovery

## Objective

Loop 2 identified four remaining bottlenecks:

1. generic balanced-rank objectives do not target the correct predictive modes;
2. positive observable factorization is powerful but nonmonotonic and conditioning-sensitive;
3. functional attention rank is lawful but head-emergence timing lacks a mechanics;
4. typed transport-category selection works, but the type information is partly supplied by construction.

Loop 3 should attack these bottlenecks directly.

---

## Experiment A — Observable predictive-coordinate supervision

### Question

Can canonical coordinates derived from observed Hankel factors guide a recurrent model better than generic future-token or CCA auxiliaries?

### Construction

1. Estimate positive factors `L, R, M_x` from observed sequences only.
2. Treat normalized factor-state trajectories as an observable predictive teacher.
3. Train E1, GRU, linear, and volumetric models under:
   - next-token only;
   - future-token auxiliary;
   - predictive CCA;
   - state distillation to observable factors;
   - operator/Jacobian distillation.
4. Never use true hidden-state labels.

### Primary observables

- excess BPS;
- future-mode `R²` by mode;
- principal angles to observable predictive subspace;
- weak-mode recovery time;
- model Hankel singular spectrum;
- sensitivity to teacher factor gauge.

### Prediction

Observable-factor supervision should outperform generic CCA on weak predictive modes. Operator distillation should be more gauge-robust than coordinate-state distillation.

### Falsifier

No improvement after gauge alignment, or gains disappear under equivalent positive-factor gauges.

---

## Experiment B — Quotient-aware positive realization learning

### Question

Can gauge balancing and projective preconditioning make positive factor recovery stable and monotonic in sample size?

### Conditions

- ordinary AdamW;
- periodic positive similarity balancing;
- mirror descent in log-positive coordinates;
- natural-gradient or Gauss–Newton approximation;
- rank continuation;
- observed-Hankel initialization plus each optimizer.

### Sweeps

- stream length: `10k` to `2m`;
- hidden rank: `3–10`;
- predictive condition number;
- weak-mode spectral gap;
- factor gauge condition.

### Primary observables

- held-out sequence likelihood;
- factor condition number;
- distance to exact character invariants where available;
- optimizer basin size;
- monotonicity with observed data.

### Prediction

Canonical balancing should reduce variance across NMF seeds and restore approximate monotonicity with sample size.

---

## Experiment C — Untyped transport-category discovery

### Question

Will a sparse model discover projective versus Clifford/Lie transport when every branch sees the same event stream?

### Bank

- positive projective fiber;
- Clifford fiber;
- commuting orthogonal fiber;
- affine contractive linear fiber.

Each branch receives a common token/gesture encoding. Use sparse gates and a description-length penalty.

### Tasks

- pure filtering;
- pure loop orientation;
- mixed filtering + orientation;
- switched task specified in context;
- task with no projective or path-order demand.

### Primary observables

- selected category mass;
- functional contribution rank;
- gate entropy;
- transport-algebra commutators;
- performance after branch ablation;
- transfer of selected modules across tasks.

### Prediction

Filtering selects projective transport; loop orientation selects noncommuting transport; mixed tasks select both; null tasks collapse unused branches.

---

## Experiment D — Mechanics of head emergence

### Question

Which variables determine the time required to leave the head-collision stratum?

### Sweep

- optimizer: full-batch GD, SGD, AdamW;
- learning rate;
- batch size;
- output multiplier;
- collision noise;
- route-signal singular-value imbalance;
- weight decay.

### Track

- functional rank;
- route assignment;
- quotient head distance;
- smallest nonzero secant tangent singular value;
- top Hessian eigenvalue;
- gradient-noise covariance;
- conservation-law residuals.

### Goal

Find a dimensionless emergence number involving signal, curvature, optimization step, and collision distance.

### Falsifier

No low-dimensional collapse of emergence curves across optimizers and scales.

---

## Experiment E — Inverse localization of a learned positive realization

### Question

How much local material complexity is required to realize an arbitrary positive predictive operator family?

### Compiler variables

- graph topology;
- number of physical and latent compartments;
- local edge generators;
- diagonal evidence operations;
- number of microsteps;
- optional topology switching by symbol.

### Objective

\[
\sum_x
\|M_x-\widehat M_x(\text{local compiler})\|^2
+
\lambda_E|E|
+
\lambda_KK
+
\lambda_W\mathcal R_{\mathrm{ledger}}.
\]

### Observables

- predictive excess BPS;
- operator error;
- locality/edge count;
- latent-compartment count;
- positivity;
- mass/work residual;
- remeshing consistency.

### Prediction

Auxiliary latent compartments and repeated local steps should trade spatial complexity for temporal depth.

---

## Experiment F — Residual natural-text Hankel structure

Only after A–E are stable:

1. fit a strong character or byte `n`-gram baseline;
2. define future tests on residual log-probability structure;
3. estimate low-order observable predictive factors;
4. train transport banks on the residual task;
5. carry state across contiguous chunks.

This prevents local token statistics from dominating the predictive-state analysis.

---

## Stop conditions

Loop 3 is complete when it either establishes or cleanly falsifies:

- observable predictive-coordinate supervision beats generic CCA;
- quotient-aware positive learning improves basin stability;
- transport categories can be discovered from untyped input;
- a low-dimensional head-emergence law exists across optimizer regimes;
- nonlocal positive transports can be compiled to sparse material graphs with a measurable complexity/error frontier.
