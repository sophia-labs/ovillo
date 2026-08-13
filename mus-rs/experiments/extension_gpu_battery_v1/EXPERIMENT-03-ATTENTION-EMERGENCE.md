# Experiment 3 — Mechanics of Attention-Head Emergence

**Stable ID:** `E3-HEADMECH`  
**Primary question:** Can the time at which nearly colliding attention heads become functionally independent be predicted by a low-dimensional combination of task signal, optimizer scale, curvature, noise, and initial distance from the collision stratum?

---

# 1. Theory under test

Loop 2 established a strong static law:

```text
functional head rank ≈ min(declared heads, independent retrieval routes)
```

It also showed that functional rank, route assignment, gauge-invariant head distance, and loss improve together as training leaves a nearly rank-one head-collision stratum.

The naive timing hypothesis

```text
t_emerge ∝ -log(collision_noise)
```

failed. The new hypothesis is:

> Secant/moduli geometry identifies the transition, while the clock is set by an instability growth rate modified by optimizer preconditioning, curvature, gradient noise, output scale, and task-spectrum imbalance.

The experiment seeks a compact predictive law. A clean failure to find one is also a valid result: it would separate a lawful static geometry from optimizer-specific microscopic timing.

---

# 2. Task family

Use the existing independent routed-retrieval construction with `keys=5` and routes `m=1…4`.

Each route has:

- an independent key permutation;
- an independent query;
- an independent value channel;
- a route-specific signal scale `s_r`.

The target is the vector of retrieved route values.

## 2.1 Route spectrum

Parameterize route scales by a condition number `kappa_route`:

```text
s_r ∝ exp(-r * log(kappa_route) / max(m-1,1))
```

and normalize `mean(s_r^2)=1`.

Core values:

```text
kappa_route ∈ {1, 2, 4, 8, 16}
```

This creates a known task singular-value hierarchy. The strongest routes should emerge first if head allocation follows greedy spectral learning.

## 2.2 Fixed probes

For every route count, create:

- one 8,192-example validation probe;
- one disjoint 16,384-example test probe;
- one 2,048-example Hessian/diagnostic probe.

All conditions with the same route count use the same probes for paired analysis.

---

# 3. Model and collision stratum

Use `RoutedMHA` with:

- declared heads `H`;
- query dimension `max(8, 2m)`;
- value dimension `max(4, m)`;
- one query token and five data tokens;
- no MLP, normalization stack, or residual path;
- output equal to the sum of head contributions times `alpha_out`.

Core head configurations:

```text
H ∈ {m, m+2}
```

Sanity grid:

```text
m=1…4, H=1…8
```

## 3.1 Controlled collision initialization

For each factor family, initialize a shared base and add head-specific antisymmetric noise of norm `epsilon`:

```text
WQ_h = WQ_0 + epsilon * dWQ_h
WK_h = WK_0 + epsilon * dWK_h
WV_h = WV_0 + epsilon * dWV_h
WO_h = WO_0 + epsilon * dWO_h.
```

Noise tensors are centered across heads so the mean head remains fixed. Normalize each perturbation family to unit Frobenius norm before multiplying by `epsilon`.

Core values:

```text
epsilon ∈ {1e-6, 3e-6, 1e-5, 3e-5, 1e-4,
           3e-4, 1e-3, 3e-3, 1e-2, 3e-2, 1e-1}
```

Record the actual initial invariant distance after factor construction. `epsilon` is an intervention variable, not the final geometric measurement.

## 3.2 Gauge-invariant head coordinates

For head `h`, compute:

```text
B_h = WQ_h WK_h^T
C_h = WV_h WO_h.
```

Use normalized pairwise distance

```text
d_hk^2 = ||B_h-B_k||_F^2 / mean_j ||B_j||_F^2
       + ||C_h-C_k||_F^2 / mean_j ||C_j||_F^2.
```

These are invariant to internal query/key and value/output basis changes.

---

# 4. Optimizer regimes

## `O-GD`

Full-batch gradient descent on a fixed 65,536-example training set.

- no momentum;
- no weight decay unless explicitly swept;
- exact deterministic gradients;
- primary regime for comparing with linear instability theory.

## `O-SGD`

Minibatch SGD without momentum.

- batch values `{64, 256, 1024}`;
- gradient-noise temperature is measured, not inferred solely from `eta/B`.

## `O-ADAMW`

AdamW with standard `beta=(0.9,0.999)` unless otherwise named.

- batch values `{64, 256, 1024}`;
- decoupled weight decay values `{0, 1e-5, 1e-3}`.

## Optional `O-SGD-MOM`

Momentum SGD (`0.9`) is an extended-tier condition, not required for the core decision.

---

# 5. Hyperparameter design

A full Cartesian product is wasteful. Use a registered hybrid design.

## 5.1 Anchor grid

For `m=3`, `H=3`:

- all three optimizers;
- `epsilon` full 11-value grid;
- `kappa_route ∈ {1, 4, 16}`;
- `alpha_out ∈ {0.5, 1.0, 2.0}`;
- two learning rates around the calibrated stable rate;
- seeds `[7, 11, 19]` for a balanced subset.

Anchors make one-dimensional effects and interactions legible.

## 5.2 Space-filling design

Generate a maximin Latin hypercube over log-scaled variables:

- learning rate;
- collision epsilon;
- output multiplier;
- route condition number;
- weight decay;
- effective batch/noise scale.

Use 96 conditions for each optimizer, with two screening seeds. Hold out 20% of conditions by a deterministic hash for final law testing.

## 5.3 Sanity rank grid

Run the existing `m=1…4`, `H=1…8` phase diagram at the reference optimizer configuration. The campaign is invalid if the static rank law no longer passes its sanity gate.

---

# 6. Learning-rate calibration

For each optimizer and route count:

1. initialize at `epsilon=1e-3`, `alpha_out=1`, `kappa_route=1`;
2. estimate top Hessian eigenvalue `lambda_0` on the diagnostic probe;
3. define candidate rates through the edge ratio
   ```text
   chi_0 = eta * lambda_0 / 2
   ```
4. core values `chi_0 ∈ {0.05, 0.15, 0.35, 0.65, 0.9}`;
5. reject rates that diverge in a 50-step calibration run.

For AdamW, also report the preconditioned effective step norm; `chi_0` is descriptive rather than an exact stability parameter.

---

# 7. Training and checkpoint schedule

- maximum updates: 2,000;
- early dense checkpoints: steps `0,1,2,5,10,15,20,30,40,60,80,100`;
- then every 25 steps to 500;
- then every 100 steps;
- lightweight metrics at every checkpoint;
- Hessian/tangent/noise diagnostics at `{0, 20, 50, 100, 200, 400, 800, 1200, 2000}` and at the first detected emergence neighborhood.

A run may stop after emergence and loss convergence, but must continue for at least 200 steps after emergence so post-transition stability is observed.

Runs not emerged by step 2,000 are right-censored.

---

# 8. Definition of emergence

Let `r = min(H,m)`.

A checkpoint is in the emerged state when all hold:

1. `r`th contribution singular value satisfies `s_r/s_1 >= 0.50`;
2. Hungarian route assignment mean attention is `>=0.75`;
3. validation MSE is below `0.20` times the best one-head irreducible baseline for that route task;
4. mean invariant head distance exceeds the 99th percentile of its initialization-noise null.

`emergence_step` is the first of three consecutive checkpoints satisfying the criteria. Linear interpolation between checkpoints is permitted and recorded.

Also emit component-specific emergence times for each route singular mode.

---

# 9. Mechanistic observables

## 9.1 Functional contribution spectrum

Stack per-head output contributions over the fixed probe and compute singular values, participation rank, and effective rank.

## 9.2 Route assignment

Hungarian assignment between heads and route-specific correct-token attention.

## 9.3 Secant tangent conditioning

Define a function vector by concatenating model outputs on the diagnostic probe. Compute a Jacobian with respect to independent head-difference coordinates, not all raw parameters.

Procedure:

1. construct an orthonormal basis of perturbations whose sum across heads is zero;
2. use JVP/VJP or randomized SVD to estimate tangent singular values;
3. report smallest nonzero singular value, condition number, and tangent participation rank.

The basis construction and threshold are versioned. This is the local conditioning of leaving the collision stratum.

## 9.4 Hessian sharpness

Use 20-step Lanczos HVP on the diagnostic loss to estimate:

- top eigenvalue;
- top five eigenvalues where affordable;
- overlap of the top eigendirection with the head-difference subspace.

Record the edge ratio `eta * lambda_max / 2`.

## 9.5 Gradient-noise covariance

At diagnostic checkpoints:

1. compute gradients for 32 independent minibatches;
2. project gradients into the head-difference tangent basis;
3. estimate trace, top eigenvalue, and anisotropy of covariance;
4. report signal-to-noise ratio of the mean gradient in that subspace.

## 9.6 Symmetry/conservation residuals

For every head, track the Q/K balance quantity

```text
N_h = WQ_h^T WQ_h - WK_h^T WK_h
```

and the analogous V/O balance in compatible dimensions. For full-batch GD without weight decay, report drift from initialization. For SGD and AdamW, report systematic and stochastic breaking.

## 9.7 Richness vector

Track normalized changes in:

- invariant `B_h`;
- invariant `C_h`;
- attention maps;
- functional output;
- raw factors.

This distinguishes functional feature learning from gauge motion.

---

# 10. Candidate dimensionless groups

The campaign does not preregister one true formula. It preregisters a small basis from which a sparse law may be selected on the fit partition.

At each run, define:

```text
Delta0 = initial invariant collision distance
S      = route-signal scale in head-difference tangent space
chi    = eta_eff * lambda_max / 2
Tnoise = eta_eff * trace(gradient_covariance) / batch_normalizer
Aout   = output multiplier
Kroute = route-spectrum condition number
W      = weight-decay timescale / update timescale
```

Candidate groups include:

```text
Gamma = eta_eff * S
Xi    = Gamma / max(1-chi, epsilon)
Theta = Tnoise / max(S^2, epsilon)
Tau0  = log(1/Delta0) / max(Gamma, epsilon)
```

Optimizer-specific `eta_eff` is:

- GD/SGD: declared learning rate;
- AdamW: median parameterwise step divided by median gradient magnitude in the head-difference subspace.

All definitions are computed from measured quantities and frozen in analysis code before holdout inspection.

---

# 11. Law fitting

## 11.1 Outcome

Use `log(emergence_step)` for uncensored runs and an accelerated-failure-time/survival formulation for the complete data.

## 11.2 Candidate models

- sparse linear regression over log groups;
- generalized additive model with at most three one-dimensional smooth terms;
- Weibull AFT model with the same covariates;
- optimizer-stratified variants.

No neural predictor is allowed as the primary law.

## 11.3 Partition

- 60% law fit;
- 20% model selection;
- 20% untouched optimizer/scale holdout.

Partition is by condition hash, not individual seed, so all seeds of a condition remain together.

## 11.4 Selection

Select the simplest model within one standard error of the best validation concordance. Freeze it, then evaluate once on holdout.

---

# 12. Registered prediction and falsifier

The primary hypothesis is supported only under the thresholds in `ANALYSIS-AND-DECISION-PLAN.md`.

Expected qualitative effects:

- smaller collision distance delays emergence only when deterministic signal dominates noise;
- larger route singular values emerge earlier;
- rising sharpness and secant tangent conditioning predict approach to emergence;
- optimizer and batch effects collapse through measured effective step and noise temperature better than through raw hyperparameters;
- overcomplete heads add redundant post-emergence factors but do not change required functional rank.

The simple-law hypothesis is falsified if no low-dimensional measured-variable law generalizes to held-out optimizer/scale conditions. The static rank law may remain valid even when the clock is irreducibly optimizer-specific.

---

# 13. Resource plan

This experiment receives the largest core share because many small runs and second-order diagnostics benefit from vectorized GPU execution.

Recommended shard classes:

- `train-light`: 32–64 conditions per GPU, lightweight checkpoints only;
- `diag-medium`: 8–16 conditions per GPU, tangent/Hessian diagnostics;
- `diag-heavy`: 2–4 conditions per GPU near emergence with gradient covariance and Lanczos.

Do not compute expensive diagnostics for every condition at every step. Online lightweight metrics nominate emergence neighborhoods; diagnostic replay loads the nearest checkpoints in a separate GPU shard.

---

# 14. Required artifacts

Per run:

```text
probe-lock.json
initial-collision.json
history-light.jsonl.zst
emergence.json
tangent-spectrum.npz
hessian-spectrum.npz
gradient-noise.npz
conservation-residuals.json
final-head-components.npz
```

Aggregate:

```text
rank-sanity.json
emergence-dataset.parquet
law-fit.json
law-holdout.json
optimizer-residuals.json
decision-E3-HEADMECH.json
```