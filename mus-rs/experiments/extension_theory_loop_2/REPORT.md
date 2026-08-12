# EXTENSION Theory → Experiment Loop 2

**Status:** completed controlled theory/experiment cycle  
**Scope:** predictive transport, observable realization, functional attention rank, typed transport selection, and local positive anatomy  
**Claim boundary:** synthetic solvable systems; no new natural-language claim

## Executive result

This loop tested six quantitative predictions derived from the predictive-transport theory.

| Question | Result |
|---|---|
| What model statistic best tracks performance? | Future predictive modes decoded with `R² > .5` ranked models within a process with concordance **0.9533**, versus **0.7083** for raw state size. |
| Can the exact positive filter basin be reached from observations? | Hankel NMF + NNLS reduced the random-init excess-loss gap by **95.17%**, reaching **0.0241 excess BPS** without hidden states. |
| Does increasing generic balanced rank improve prediction? | **No.** A CCA regularizer raised measured balanced rank but slightly worsened predictive loss. |
| Does functional attention rank follow declared head count? | It followed `min(heads, independent retrieval routes)` with correlation **0.9991** and MAE **0.0308**. |
| Will a transport bank select different geometries? | Filtering received **97.0% projective** weight; oriented-loop memory received **97.0% Clifford** weight. A commuting control destroyed loop accuracy. |
| Can positive transport be locally embodied? | An eight-compartment cube converged from sparse edge-local steps to its dense exact predictor, with mass and quadratic-storage residuals at floating-point precision. |

The theoretical update is:

> **Predictive alignment—not state dimension or generic rank—is the load-bearing quantity.**

A useful system requires a compatible transport category, a route into a good factorization basin, writable/readable modes aligned with actual predictive distinctions, and a local convergent realization when physical extension is claimed. We call these requirements the **realization pentad**.

## 1. Predictive atlas

Eight stationary observable HMMs varied hidden-state count, persistence, emission clarity, transition structure, and predictive conditioning. Seven small architectures were trained on each, producing 56 runs.

| Statistic | Within-process pairwise concordance with lower excess loss |
|---|---:|
| Future modes with `R² > .5` | **0.9533** |
| Weighted future-mode `R²` | **0.9405** |
| Belief-state probe `R²` | **0.9345** |
| Balanced/future spectral overlap | 0.7976 |
| Balanced participation rank | 0.7083 |
| Raw state size | 0.7083 |
| State participation rank | 0.6429 |

The conclusion is not that one scalar predicts performance globally. Across different processes, spectral decay, oracle entropy, weak-mode conditioning, sample complexity, and optimization difficulty also matter. The result is narrower and stronger:

> Within one predictive problem, models are ordered much better by the future distinctions actually represented in their state than by the amount of state they possess.

## 2. Observable positive realization

For observed symbol streams, short-word probabilities define an empirical Hankel block `H` and shifted blocks `H_x`. We factor

\[
H\approx LR,
\qquad
H_x\approx LM_xR
\]

using nonnegative matrix factorization and NNLS, then load the recovered positive operators into a differentiable projective model.

| Route | Mean excess BPS |
|---|---:|
| Exact hidden realization | ~`6e-9` |
| Observable NMF + NNLS | **0.02413** |
| NMF + future auxiliary | **0.02406** |
| Rank continuation `2→6` | 0.47728 |
| Future auxiliary from random | 0.49847 |
| Random projective rank 6 | 0.49923 |

A held-out search over 18 factorization configurations improved a 100k-observation initializer from `0.02381` to **`0.01792` excess BPS** after fine-tuning.

More data was not monotonically better: 100k observations outperformed 250k and 500k in the default factorization. This means the problem is not only probability estimation. Positive factor gauge, weak-mode conditioning, nonconvex factor selection, and optimization basin all matter.

## 3. Balanced-rank intervention: negative result

A Clifford bank was trained with a future auxiliary, a predictive CCA regularizer, or both.

| Condition | Excess BPS | Balanced participation rank |
|---|---:|---:|
| Baseline | **0.10313** | 2.7086 |
| Future auxiliary | **0.10228** | 2.7104 |
| CCA | 0.10557 | 2.8543 |
| Future auxiliary + CCA | 0.10449 | **2.8622** |

The CCA objective successfully raised its own rank statistic while making prediction slightly worse. Therefore generic controllable-observable rank is a diagnostic, not a sufficient objective.

Revised claim:

> Useful modes must be aligned with the minimal predictive realization, not merely correlated with a chosen batch of future observables.

The next target should be canonical observable predictive coordinates derived from Hankel factors, not generic CCA.

## 4. Functional attention rank

Tasks were constructed with `m=1..4` independent retrieval routes and models with `H=1..6` declared heads. Heads began near a collision stratum.

The learned contribution rank obeyed

\[
r_{\mathrm{functional}}\approx\min(H,m).
\]

Correlation was `0.9991`; MAE was `0.0308`. With four routes, one, two, and three heads retained large error, while four heads solved the task. Five or six heads still realized approximately rank four.

This supports a secant/moduli interpretation:

> Declared heads are coordinates in an overcomplete factorization; functional head rank is primarily set by task routing rank.

Head emergence coincided qualitatively with the appearance of new contribution singular values, route specialization, gauge-invariant head separation, and sharp loss improvement. But a simple law `t_emerge ∝ -log ε_collision` failed: the correlation between emergence time and log collision noise was only `0.130`.

The stratum geometry identifies *what* transition occurs. Its timing also depends on optimizer, noise, curvature, and signal strength.

## 5. Typed transport bank

A mixed model combined:

- an observation-initialized positive projective filter;
- a Clifford path state driven by oriented closed loops.

| Condition | Observation accuracy | Area/order accuracy |
|---|---:|---:|
| Full bank | 58.85% | **100%** |
| Projective only | **58.89%** | 50.08% |
| Clifford only | 32.99% | **100%** |
| Commuting Clifford control | 58.86% | 49.86% |

In the full bank, the filtering gate was `[0.9700 projective, 0.0300 Clifford]`; the area gate was `[0.0302 projective, 0.9698 Clifford]`.

This supports a typed transport ontology and weakens any universal-Clifford claim. The caveat is that the branches received typed signal channels. The next loop must test untyped category discovery from a shared raw stream.

## 6. Local positive anatomy

An eight-compartment cube used a sparse symmetric generator `Q` with only twelve undirected cube edges. Its exact dense transition is

\[
P=e^{\Delta tQ}.
\]

The local realization uses positivity-preserving steps

\[
P_n=(I+\Delta tQ/n)^n.
\]

| Microsteps | Transition error | Excess BPS |
|---:|---:|---:|
| 1 | 1.07005 | 0.066984 |
| 2 | 0.28200 | 0.005489 |
| 4 | 0.12392 | 0.001038 |
| 8 | 0.05847 | 0.000222 |
| 16 | 0.02845 | 0.0000480 |
| 32 | 0.01404 | 0.00000940 |
| 64 | 0.00697 | **0.00000118** |

At every refinement the state remained positive; maximum mass residual was about `4.4e-15`; maximum quadratic-storage ledger residual was about `2.2e-16`. Repeated local propagation becomes globally dense without abandoning local instantaneous dynamics.

This is a favorable construction—the target was generated from local diffusion. Arbitrary positive realizations have not yet been localized.

## 7. Realization pentad

A useful learned predictive system requires:

1. **Predictive spectrum and condition** — what future distinctions exist, at what strength?
2. **Transport category** — projective, Clifford/Lie, affine contractive, material, or another representation?
3. **Predictive controllability-observability alignment** — are the right modes writable and readable?
4. **Factorization basin and optimizer geometry** — can training reach the useful realization?
5. **Anatomical localization** — can it be compiled into local, receipted, convergent dynamics?

The loop falsified several simpler views:

- state size is not usable capacity;
- generic balanced rank is not enough;
- correct category and rank do not guarantee learnability;
- more observations do not guarantee a better nonconvex factorization;
- initial head separation alone does not set emergence time.

## 8. Claim boundary

Established here:

- future-mode decodability is a strong within-process model-order statistic;
- observed positive factorization reaches a near-oracle projective basin;
- generic CCA can improve its metric without improving prediction;
- functional attention rank follows task routing rank;
- projective and Clifford transports solve different mechanisms;
- a local positive compartmental realization can converge with exact receipts.

Not established:

- a universal loss law;
- statistical consistency of the present NMF route;
- spontaneous untyped category discovery;
- an emergence-time law;
- localization of arbitrary learned operators;
- continuum remeshing;
- a natural-language advantage;
- or production readiness.

## 9. Next loop

The next cycle will test:

1. observable predictive-coordinate and operator distillation versus generic CCA;
2. quotient-aware positive optimization and gauge balancing;
3. untyped transport-category discovery with sparse/MDL gates;
4. attention-emergence mechanics across optimizer, batch noise, learning rate, output scale, and task spectrum;
5. inverse localization of arbitrary positive transport with a locality/error/latent-compartment frontier;
6. only then, residual long-range structure in natural text after subtracting strong local `n`-gram statistics.

The complete specification is in `NEXT-LOOP.md`.
