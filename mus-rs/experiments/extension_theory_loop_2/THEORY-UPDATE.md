# Theory Update: From the Rank Quartet to the Realization Pentad

## Status

This note records how Theory → Experiment Loop 2 changed the conceptual stack. It is not a replacement for the full report; it extracts the new theoretical commitments and the claims that were weakened or falsified.

## 1. Previous picture

The previous theory centered a rank quartet:

1. predictive rank of the data;
2. controllable-observable rank of the model;
3. Lie/transport-algebra rank;
4. functional attention-head rank.

This was useful but too coarse. Loop 2 showed that generic rank can be increased without improving prediction and that a correct minimal model can remain unlearned from random initialization.

## 2. Realization pentad

A useful learned predictive system requires five coupled structures.

### A. Predictive spectrum and condition

The data's future-relevant modes have amplitudes, spectral gaps, and condition numbers. Effective rank without spectral conditioning is insufficient.

### B. Transport category

Transport geometry is mechanism-dependent:

- positive projective transport naturally represents normalized Bayesian filtering;
- Clifford/Lie transport naturally represents stable path order, area, and holonomy;
- affine contraction represents driven forgetting;
- material semigroups represent local propagation and anatomy.

### C. Predictive controllability-observability alignment

State modes must be writable and readable **in the directions that distinguish future distributions**. Generic CCA or balanced rank is not enough.

### D. Factorization basin and optimizer geometry

The model family may contain an exact minimal realization that gradient descent does not find. Positive Hankel factorization, gauge conditioning, and initialization determine access to useful strata.

### E. Anatomical localization

An abstract operator becomes a physical Extension medium only after it is lowered to local interactions with a declared storage metric, conservation/work ledger, and refinement semantics.

## 3. New bridge theorem target

Let `H_data` be an observable predictive Hankel operator and `H_model` the system Hankel operator induced by input-to-state and state-to-output dynamics. The target is not equality of scalar ranks. It is alignment of leading singular subspaces under a task-relevant weighting.

A candidate quantity is

\[
\mathcal A_r
=
\left\|
U_{\mathrm{data},r}^{\top}
\mathcal C
U_{\mathrm{model},r}
\right\|_F^2/r,
\]

where `C` maps model output observables into the data future-test coordinate system.

The experiment program should determine whether loss is predictable from:

- spectral mass captured;
- principal-angle alignment;
- weak-mode condition;
- and optimization distance to a balanced minimal realization.

## 4. Revised learning claim

Old, falsified form:

> Increasing jointly controllable-observable rank improves prediction.

Revised form:

> Increasing **predictively aligned** controllable-observable spectral mass should improve prediction, provided the corresponding realization is sufficiently conditioned and reachable by the optimizer.

## 5. Positive observable factorization as the missing bridge

The NMF+NNLS result shows that finite observed word-probability blocks contain enough information to initialize a near-oracle positive projective model. This route reduced the random-initialization excess-loss gap by more than 95%.

This is structurally analogous to supplying the missing quantitative bridge in a larger proof strategy: the projective realization was known to exist, but observable operator factorization supplied a concrete route from isolated probability observations to the special representation locus.

The remaining problem is not existence but stable, identifiable, statistically consistent factor recovery.

## 6. Functional attention rank survives; emergence timing does not

The invariant law

\[
r_{\mathrm{head}}
\approx
\min(H,r_{\mathrm{route}})
\]

held across the routing-rank grid.

The simple timing conjecture

\[
t_{\mathrm{emerge}}
\propto
-\log \epsilon_{\mathrm{collision}}
\]

did not.

A mechanics of head emergence must include optimizer and curvature variables. Secant/moduli geometry identifies the transition strata but not by itself the training clock.

## 7. Typed transport categories

A mixed projective/Clifford bank selected the predicted category for each task. This supports a typed transport ontology and weakens any claim that Clifford transport should be universal.

The long-term model should be a sparse bank of small transport representations selected by the predictive task. Category selection itself should eventually be learned from untyped input rather than fixed by channel design.

## 8. Local compilation

The cube experiment establishes a clean local positive-transport lowering:

\[
P=e^{\Delta tQ},
\qquad
P_n=(I+\Delta tQ/n)^n,
\]

with local `Q`, positive stochastic microsteps, mass conservation, and an exact quadratic storage ledger.

The next theory problem is inverse localization: characterize which positive representations admit low-complexity local generators, perhaps after adding latent compartments.

## 9. Revised full stack

\[
\text{observations}
\to
\text{predictive Hankel structure}
\to
\text{typed transport realization}
\to
\text{predictive mode alignment}
\to
\text{quotient-aware learning}
\to
\text{local material compiler}
\to
\text{work/convergence receipts}.
\]

## 10. Current central thesis

> **EXTENSION is a theory of selecting a predictive transport category, recovering a well-conditioned realization from observations, aligning its writable/readable modes with predictive demand, and compiling that realization into a local body.**
