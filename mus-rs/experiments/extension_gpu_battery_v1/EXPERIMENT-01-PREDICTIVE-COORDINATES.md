# Experiment 1 — Canonical Predictive-Coordinate Supervision

**Stable ID:** `E1-PREDCOORD`  
**Primary question:** Can a recurrent model acquire weak predictive modes more reliably when trained against observable, gauge-audited predictive coordinates than when trained with generic future-token or predictive-CCA objectives?

---

# 1. Theory under test

Loop 2 falsified the naive claim that increasing generic controllable-observable or CCA rank necessarily improves next-symbol prediction. It also showed that observable Hankel NMF + NNLS can recover a near-oracle positive projective predictor without hidden-state labels.

The revised hypothesis is:

> A useful state direction must align with a coordinate of the minimal predictive realization. Observable predictive factors supply a better training target than an arbitrary collection of future-correlated directions, and transition/operator supervision should be more robust to factor gauge than direct coordinate matching.

This experiment distinguishes four possibilities:

1. the observable factors contain genuinely useful predictive coordinates;
2. they help only because they provide extra supervision, not because they are canonical;
3. direct state coordinates help but are gauge-fragile;
4. transition/future-test supervision survives equivalent factor gauges and is therefore closer to the intrinsic mechanism.

---

# 2. Teacher construction

## 2.1 Observable-only inputs

For each process, estimate from `factor_train` symbols:

- a finite prefix/suffix Hankel block `H`;
- shifted blocks `H_x`;
- positive factors `L`, `R`;
- positive symbol operators `M_x`;
- positive initial row `a`;
- positive terminal column `t`.

Use the Loop 2 NMF + NNLS route with held-out observable-sequence likelihood for factor candidate selection. Hidden-state labels and exact beliefs are forbidden from candidate selection.

The factorization and candidate-search manifest is frozen before student training.

## 2.2 Teacher state

For observed prefix `x_1…x_t`, the unnormalized row state is

```text
alpha_t = a M_x1 ... M_xt
```

and the teacher's terminal-normalized simplex coordinate is

```text
q_t[i] = alpha_t[i] * t[i] / (alpha_t @ t).
```

This coordinate is invariant under positive diagonal similarity gauge and transforms only by permutation under a monomial gauge. Permutations are aligned by future-test signatures, never by hidden-state identity.

## 2.3 Gauge-invariant future-test coordinates

Let `V` be a fixed suffix basis containing the empty word and every word through depth 3, pruned only by a manifest-fixed singular-value threshold. For suffix `v`, define

```text
phi_t[v] = alpha_t M_v t / (alpha_t t).
```

`phi_t` is the teacher probability of each future test and is invariant under every exact similarity transform of the realization.

Compress `phi` with an SVD fit on teacher training trajectories. Preserve enough components to explain `99.9%` of teacher future-test variance, with a minimum of `rank(process)` components and a maximum of 32. The SVD basis is frozen before student training.

## 2.4 Equivalent-gauge family

Generate eight functionally equivalent teacher gauges:

1. identity;
2. three random permutations;
3. three positive diagonal similarities with log-scales sampled uniformly from `[-4, 4]` and centered to unit geometric mean;
4. one accepted near-identity positive similarity whose transformed operators remain positive above `1e-8`.

For row-state convention:

```text
alpha' = alpha G
M_x'   = G^-1 M_x G
t'     = G^-1 t.
```

Verify on 10,000 held-out prefixes that every gauge changes future-test probabilities by at most `1e-8` in float64. Invalid gauges are regenerated deterministically.

---

# 3. Student model family

## 3.1 Core students

Primary students at approximately 16 recurrent scalars:

- `clifford16`: `StableCliffordBank(latent_dim=3, fibers=2)`;
- `gru16`: `SmallGRU(hidden=16)`;
- `linear16`: `LinearStateModel(hidden=16)`.

Secondary students:

- `clifford8`;
- `gru8`;
- one selected small volumetric/proprioceptive model after the core code path is validated.

Every scientific comparison is within architecture: treatment versus the same architecture's next-token baseline. Cross-architecture rankings are descriptive.

## 3.2 Student predictive probe

Attach a training-only predictive probe

```text
psi_theta: student_state -> compressed future-test coordinates.
```

Default probe is affine. A two-layer probe is permitted only as a labeled capacity control and may not define the primary result.

The vocabulary readout remains the model's native readout. At inference, the teacher and training-only probe are removed unless a condition explicitly studies probe dependence.

---

# 4. Training treatments

All treatments receive identical symbol streams, train steps, model initialization seed, optimizer schedule, and native next-token loss.

## `T0-NEXT` — next-token only

```text
L = L_next + native_regularization
```

## `T1-FUTURE` — generic future-token auxiliary

Predict symbols at horizons 1–5 from student state with separate affine heads.

```text
L = L_next + lambda_aux L_future
```

## `T2-CCA` — predictive CCA

Use the Loop 2 differentiable CCA objective between student state and the same generic future-symbol feature set. This is the direct negative-control treatment.

## `T3-STATE` — observable state-coordinate distillation

Learn an affine alignment from student state to terminal-normalized teacher simplex `q_t`.

```text
L_state = mean KL(q_t || softmax(A s_t + c))
```

For teacher permutations, align components using frozen future-test signatures. Positive diagonal gauges require no alignment because `q_t` is invariant.

This treatment tests whether a human-readable predictive coordinate helps. It is not expected to be fully gauge invariant under arbitrary non-monomial positive similarities.

## `T4-FUTURETEST` — gauge-invariant future-test distillation

Match the compressed future-test coordinate:

```text
L_phi = mean ||psi(s_t) - phi_t||_whitened^2.
```

Whitening uses teacher singular values with floor `1e-4 * sigma_1` so weak but nonzero modes receive controlled weight without exploding.

This is the primary experimental treatment.

## `T5-TRANSITION` — future-test transition/JVP distillation

Add a gauge-invariant local transition constraint. For symbol `x`, the teacher induces a nonlinear map in future-test coordinates. On a deterministic 1-in-8 subsample of training positions:

1. compute teacher JVPs in two random normalized directions in compressed `phi` space;
2. compute student predictive-probe JVPs through one recurrent step using `torch.func.jvp`;
3. match direction and log magnitude.

```text
L_jvp = mean ||normalize(J_s v) - normalize(J_t v)||^2
      + 0.1 * mean (log||J_s v|| - log||J_t v||)^2
```

`T5` uses `L_next + lambda_phi L_phi + lambda_jvp L_jvp`. It is more expensive and enters Stage 2 only if its Stage 1 validation result is competitive with `T4`.

---

# 5. Auxiliary-weight normalization

Do not tune auxiliary weights independently for every architecture after observing validation outcomes.

For each architecture/process pair, use one deterministic calibration batch at initialization:

1. compute gradient norm of `L_next` over recurrent parameters;
2. compute gradient norm of each auxiliary loss;
3. choose `lambda` so the auxiliary gradient norm is `0.30` times the next-token gradient norm;
4. clip `lambda` to `[0.03, 3.0]`;
5. freeze it for all seeds and gauges of that architecture/process.

Emit calibration norms and selected weights in the run manifest. A secondary fixed-weight control uses `lambda=0.25`.

---

# 6. Data and training schedule

## 6.1 Stage 1 screening

- processes: `P-BASE6`, `P-NOISY6`;
- students: `clifford16`, `gru16`, `linear16`;
- treatments: `T0`–`T5`;
- seeds: `[7, 11]`;
- sequence length: 128;
- batch: 256 sequences;
- updates: 600;
- optimizer: AdamW, architecture-specific baseline learning rate from calibration;
- cosine decay with 5% warmup;
- gradient clipping: 5.0;
- evaluation every 50 updates.

This is 72 micro-runs, packed into vectorized shards by architecture/treatment compatibility.

Mandatory continuations: `T0`, `T2`, `T4`. Continue `T3` and `T5` under the common successive-halving rule.

## 6.2 Stage 2 confirmation

- processes: `P-EASY4`, `P-NOISY6`, `P-WEAK8`;
- primary students: `clifford16`, `gru16`, `linear16`;
- selected treatments: mandatory controls plus promoted treatments;
- seeds: `[7, 11, 19, 23, 31]`;
- sequence length: 256;
- batch: 256;
- updates: 1,500;
- evaluation every 100 updates;
- teacher gauges: identity during main training; all eight gauges in paired gauge-audit runs on `P-NOISY6` and `P-WEAK8`.

## 6.3 Volumetric confirmation

Only after the core path is green:

- processes: `P-BASE6`, `P-WEAK8`;
- treatments: `T0`, `T2`, `T4` and promoted `T5`;
- seeds: `[7, 11, 19]`;
- fixed state/material budget;
- 800 updates;
- same teacher and analysis contracts.

This is secondary evidence. A volumetric failure cannot invalidate a result about canonical supervision in abstract recurrent systems; it instead diagnoses the abstract-to-material bridge.

---

# 7. Primary observables

Per checkpoint and final test:

- BPS and excess BPS;
- per-mode future-test `R²`;
- weak-mode mean `R²`;
- principal-angle spectrum between student and teacher predictive subspaces;
- acquisition step for each future mode;
- native state participation rank;
- controllability-observability spectrum;
- predictive probe condition number;
- recurrent-gradient norm and auxiliary-gradient fraction;
- state/transport diagnostics native to each architecture;
- GPU seconds and tokens processed.

For `T3`–`T5`, also emit:

- training-only teacher loss;
- test teacher loss;
- gauge sensitivity;
- probe-free native predictive metrics after removing the teacher probe.

The primary decision uses native next-token test loss and weak-mode recovery, not teacher-loss minimization.

---

# 8. Controls

## 8.1 Shuffled teacher

Permute teacher trajectories across batch while preserving marginal coordinate distributions. It should not improve predictive loss. If it does, the auxiliary is acting as generic regularization rather than predictive-coordinate supervision.

## 8.2 Rank-matched random teacher

Replace teacher future-test coordinates with a fixed random orthogonal projection of generic future features at the same dimension and variance.

## 8.3 Exact hidden-belief ceiling

As an analysis-only upper bound, train one separately labeled condition with exact belief-state supervision. This condition is not included in the observable-method claim and must be stored under `oracle_control=true`.

## 8.4 Probe capacity

For one core process, compare affine and two-layer student probes without changing recurrent state. If a nonlinear probe creates the entire gain while native logits do not improve, the result does not support better recurrent realization.

---

# 9. Registered prediction and falsifier

Primary support/falsification follows `ANALYSIS-AND-DECISION-PLAN.md`.

Qualitatively, the prediction is:

- `T4` or `T5` improves weak predictive modes and native next-token loss over `T2`;
- `T5` is no more gauge-sensitive than `T4` and less sensitive than raw `T3`;
- shuffled/random teachers do not reproduce the gain;
- improvement appears earlier for strong modes and specifically accelerates weak modes relative to baseline.

The theory is falsified in this regime if:

- generic CCA is as good or better;
- gains vanish after removing the teacher probe;
- equivalent factor gauges change the conclusion;
- or random/shuffled teacher controls produce the same effect.

---

# 10. Required artifacts

Each run additionally stores:

```text
teacher.lock.json
teacher-factor.npz             # observable factors only
teacher-gauge.json
future-test-basis.npz
mode-acquisition.json
principal-angles.json
aux-gradient-calibration.json
```

The aggregate writes:

```text
paired-effects.json
mode-curves.json
gauge-audit.json
decision-E1-PREDCOORD.json
```

No exact hidden trajectory is included in ordinary run artifacts.