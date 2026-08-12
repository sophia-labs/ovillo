# Extension-v1 results ledger

All results below are architecture probes on small synthetic tasks. They establish executable mechanisms and falsify one initial training route. They do **not** establish language-model quality, biological fidelity, long-context scaling, or unique superiority of four latent dimensions.

## R0 — Active-cable end-to-end control: negative

The first model directly optimized a shared active cable law on four learned strings in `R^4`. Each string had eight material sites and four field channels. Token inputs were closed cubic Bézier gestures; contacts were geometry-derived; field state persisted across tokens; a learned closed pickup path produced a linear readout.

| Quantity | Value |
|---|---:|
| Trainable parameters | 546 |
| Updates | 250 |
| Complete examples | 24 permutations |
| Best accuracy | 50% |
| Restored final accuracy | 50% |
| Runtime, recorded CPU run | 14.14 s |

The loss fell substantially while classification did not improve. The medium found a weak, nearly order-insensitive solution. This is not evidence against active extended neurons in general. It is evidence that the original end-to-end objective did not provide a usable credit-assignment route to the required noncommuting state transition.

Artifact: `artifacts/active_cable_negative.json`.

## R1 — Exact Clifford gesture medium

The state space is `Cl(0,4)` represented on the 16-dimensional exterior algebra. Four basis generators act as right Clifford multiplication. Each generator is exactly the product of eight disjoint signed quarter-turns.

| Check | Value |
|---|---:|
| State dimension | 16 |
| Grade-string site counts | 1, 4, 6, 4, 1 |
| Local rotations per basis gesture | 8 |
| Max square-law residual | 1.225e-16 |
| Max anticommutator residual | 1.225e-16 |
| Max orthogonality residual | 0 |
| Determinant of every generator | 1 |

For all 24 permutations of four basis gestures, the pseudoscalar amplitude equals the permutation sign exactly to floating-point tolerance.

For 10,000 random ordered sets of four unit vectors in `R^4`, the final pseudoscalar coefficient equals their determinant/oriented four-volume:

| Check | Value |
|---|---:|
| Sign accuracy | 100% |
| Max determinant residual | 4.441e-16 |
| RMS determinant residual | 6.410e-17 |
| Max energy drift | 1.221e-15 |

Artifact: `artifacts/exact_clifford.json`.

## R2 — Continuous closed-loop development

The exact generators drive the controlled system

```text
dU = U sum_j D_j dg^j.
```

Piecewise-linear segments are integrated with exact matrix exponentials.

Two paths with the same endpoint but opposite segment order had:

| Observable | Value |
|---|---:|
| State L2 contrast | 0.560425 |
| Operator Frobenius contrast | 2.241700 |
| Energy of each state | 1.0 |

A rectangle with exactly closed controls had:

| Observable | Value |
|---|---:|
| Endpoint norm | 0 |
| Distance from vacuum | 0.560425 |
| Operator distance from identity | 2.241700 |
| `e01` bivector coefficient | 0.385965 |
| Energy | 1.0000000000000004 |

The inverse loop returned the state with residual `4.46e-16`. Splitting four segments into 26 collinear subsegments changed the operator by only `1.25e-15`. Across small rectangles, bivector magnitude scaled with enclosed area with fitted log-log exponent `0.99785`.

This is geometric, noncommuting path memory. It is **not** claimed to be topologically protected.

Artifact: `artifacts/closed_development.json`.

## R3 — Learned closed token gestures

The stronger trainable probe does not use open basis moves. Each of four tokens owns a learned six-segment path in `R^4`; the final segment is the negative sum of the others, so every token loop closes exactly. The path develops through a single 16-state Clifford fiber and a final linear readout.

| Quantity | Value |
|---|---:|
| Fibers | 1 |
| Trainable parameters | 130 |
| Complete examples | 24 permutations |
| Accuracy | 100% |
| Recorded CPU runtime | 2.19 s |
| Max loop-closure residual | 4.44e-16 |
| Max energy drift | 7.33e-15 |
| Mean pairwise operator-commutator norm | 7.92 |

A seed sweep found 100% accuracy in 5/5 one-fiber runs, 3/3 two-fiber runs, and 3/3 four-fiber runs. One fiber was enough.

The decisive control replaces all four Clifford generators by one shared commuting generator. Since each token path is closed, every token operator then collapses to identity. All 24 sequence states become equal to numerical precision; a linear probe falls to 50%. The noncommuting realization retains 100% linear-probe accuracy and centered feature rank 16.

This establishes that the probe is solved by path order, not endpoint, bag-of-token content, or norm growth. It does **not** establish that Clifford development is the only noncommuting solution, or that the learned loops extrapolate to longer algorithms.

Artifacts:

- `artifacts/closed_gesture_run/result.json`
- `artifacts/closed_gesture_sweep.json`
- `artifacts/closed_gesture_controls.json`

## R4 — Five-string geometric realization

The 16 blade modes were placed on five one-dimensional strings grouped by exterior grade. Every scheduled local event rotates two Gaussian field modes after Gram--Schmidt orthogonalization; therefore each event is exactly norm preserving. Finite Gaussian width creates unintended support and breaks the global Clifford relations.

For one random four-dimensional morphology at contact width `0.38`:

| Quantity | Value |
|---|---:|
| Basis-permutation sign accuracy | 100% |
| Minimum parity amplitude | 0.9674 |
| Random continuous-volume sign accuracy | 95.17% |
| Max local-generator orthogonality residual | 8.88e-16 |
| Max anticommutator residual | 0.1864 |
| Mean generator Frobenius error | 0.3806 |

Arbitrary vector combinations are not exactly orthogonal after the Clifford relations are perturbed, so the random continuous-vector test exhibited energy drift as high as `0.311`. That is a real limitation of this naive soft lowering, not a numerical bookkeeping error.

Artifact: `artifacts/geometric_d4.json`.

## R5 — Ambient-dimension/contact-width sweep

At contact width `0.38`, averaged over five random morphologies:

| Ambient dimension | Parity accuracy | Continuous-volume sign accuracy | Anticommutator residual | Worst parity amplitude |
|---:|---:|---:|---:|---:|
| 2 | 95.83% | 84.34% | 0.5078 | 0.0070 |
| 3 | 99.17% | 90.86% | 0.3769 | 0.0079 |
| 4 | 100% | 95.39% | 0.1864 | 0.9674 |
| 8 | 100% | 95.35% | 0.1864 | 0.9674 |

The result supports a **dimension-threshold** interpretation: four dimensions are sufficient to separate the required local contacts much more cleanly than two or three in this construction, while eight offers essentially no additional benefit. It does not prove that four is globally optimal for learned sequence models.

Artifact: `artifacts/geometry_dimension_sweep.json`.

## R6 — Inverse-designed morphology

Only 44 continuous direction parameters defining the five strings were optimized. The target sparse Clifford pairing schedule was fixed.

At ambient dimension four and contact width `0.50`:

| Quantity | Initial | Final |
|---|---:|---:|
| Operator MSE | 0.04798 | 0.01004 |
| Basis-permutation sign accuracy | 50% | 100% |
| Random continuous-volume sign accuracy | 51.61% | 80.81% |

Morphology can therefore recover a useful local realization of a known operator program. The experiment does not discover the operator topology, and at broad contact width its out-of-sample continuous algebra remains approximate.

Artifacts:

- `artifacts/morphology_d4.json`
- `artifacts/morphology_dimension_sweep.json`

## Current evidence boundary

What is now established:

1. A 16-state, energy-preserving Clifford medium exactly records oriented order and four-volume.
2. Continuous closed control loops leave a nonzero, invertible, reparameterization-stable residue.
3. Closed token loops can be learned end to end on a complete controlled sequence task.
4. Replacing noncommuting generators by commuting ones destroys all task information.
5. The exact sparse operators admit a five-string local-event decomposition.
6. Morphology optimization improves a finite-width geometric realization.
7. A naive active-cable model does not learn the same task from the original loss.

What remains unestablished:

1. language modelling;
2. length or algorithmic extrapolation;
3. a successful active nonlinear medium trained through the physical solver;
4. superiority to generic trainable path-development networks;
5. exact passivity of arbitrary soft-geometric vector gestures;
6. scalable contact search and topology learning;
7. biological resemblance beyond the use of extended excitable media;
8. unique optimality of four dimensions;
9. topological protection.
