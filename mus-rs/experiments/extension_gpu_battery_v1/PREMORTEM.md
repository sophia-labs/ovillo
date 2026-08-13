# Pre-mortem and Failure Register

## 1. GPU utilization is poor

**Symptom:** utilization below 20%, GPU slower than CPU.

**Likely cause:** one tiny model per process, host-side data generation, Python loops.

**Prevention:** device-cached process tensors, pinned/pre-generated streams, ensemble batching, separate heavy diagnostics, calibration gate.

**Decision:** do not buy a larger GPU; increase compatible ensemble count or move the condition back to CPU.

## 2. Teacher leakage invalidates E1/E4

**Symptom:** exact hidden identities influence factor selection, auxiliary targets, or early stopping.

**Prevention:** separate observable and oracle packages; training API has no hidden-state argument; test-access and oracle-access audit logs; code review of candidate selection.

**Decision:** affected campaign is invalid, not merely caveated.

## 3. Gauge artifact is mistaken for a mechanism

**Symptom:** state-distillation gain changes under equivalent teacher scaling/permutation; positive optimizer trajectories depend strongly on initial gauge.

**Prevention:** eight-gauge audits, future-test coordinates, character invariants, function-preserving replay.

**Decision:** raw-coordinate claim is falsified; retain any gauge-invariant result separately.

## 4. Sparse gate selects feature scale, not transport category

**Symptom:** gate mass changes under function-preserving branch rescaling or gate permutation has no effect.

**Prevention:** branch normalization, inverse readout rescaling control, contribution spectra, causal ablations.

**Decision:** no category-selection claim, even if gate weights look interpretable.

## 5. Projective branch or generic affine branch solves the loop task by shortcut

**Symptom:** loop accuracy persists under commuting control, endpoint-only data, or nuisance swap.

**Prevention:** randomized rotations/scales, same-endpoint matched controls, OOD loop shapes, branch ceilings, commutator receipts.

**Decision:** redesign task under a new campaign; do not reinterpret shortcut as category discovery.

## 6. E3 emergence threshold manufactures a law

**Symptom:** timing law changes radically with reasonable threshold variants.

**Prevention:** one primary emergence definition, report continuous rank/assignment curves, threshold sensitivity only as secondary, censored survival analysis.

**Decision:** if the law is threshold-specific, outcome is inconclusive or falsified.

## 7. Heavy diagnostics exhaust budget

**Symptom:** Hessian/noise measurements dominate GPU time.

**Prevention:** lightweight online nomination, checkpoint replay, randomized SVD/Lanczos, diagnostic subset fixed before holdout.

**Decision:** preserve training matrix and reduce secondary diagnostic density, never primary conditions.

## 8. NMF preprocessing becomes the bottleneck

**Symptom:** GPU idle waiting for factors; depth-3 blocks consume excessive RAM.

**Prevention:** preprocess all factors before GPU launch, shard by process/stream, cache immutable candidates, use high-memory CPU nodes.

**Decision:** campaign waits; workers do not rebuild factors independently.

## 9. More data appears worse because candidate search is underpowered

**Symptom:** E4 nonmonotonicity reflects different NMF seed counts or selection effort.

**Prevention:** identical candidate-count and validation budget at every stream length; report best/median/worst candidates.

**Decision:** monotonicity analysis invalid if search budgets differ.

## 10. Spot interruptions bias conditions

**Symptom:** long/heavy conditions retry more often and receive different effective training.

**Prevention:** exact step/cursor resume, retry accounting, checkpoints, paired on-demand replication for selected conditions.

**Decision:** infrastructure retries are covariates and may trigger confirm rerun; partial trajectories are not silently accepted.

## 11. Test set leaks into successive halving

**Symptom:** test metric appears before `confirm_locked`.

**Prevention:** separate object prefix and IAM/read guard; audit event; planner omits test URI from screen manifests.

**Decision:** campaign invalid.

## 12. Multiple comparisons create a narrative after the fact

**Symptom:** primary endpoint fails, but one exploratory metric is promoted.

**Prevention:** machine-generated decision receipts; FDR for secondaries; exploratory labels.

**Decision:** report the primary falsification first.

## 13. Vectorized members are not independent

**Symptom:** shared RNG or broadcasted parameter bug makes seeds identical.

**Prevention:** named per-member RNG streams, scalar-loop equivalence tests, pairwise initial-parameter and data-hash checks.

**Decision:** affected shards rerun after fix.

## 14. Optimizer-state gauge transform is wrong

**Symptom:** `Q2` changes predictions before an optimizer step or fails quadratic covariance fixtures.

**Prevention:** keep `Q1` reset-moment method as primary; gate `Q2` behind tests.

**Decision:** disable `Q2`; do not block the rest of E4.

## 15. Volumetric confirmation becomes an opaque capacity increase

**Symptom:** E1 volumetric student uses a global decoder or larger effective state without matched controls.

**Prevention:** secondary-only status, declared state/material budget, local observer receipt, within-architecture comparisons.

**Decision:** omit from primary conclusion.

## 16. CI/account infrastructure is unavailable

**Symptom:** GitHub checks do not start or cloud quota is unavailable.

**Prevention:** local verifier, container self-check, object-store manifests, explicit cloud preflight.

**Decision:** no claim of remote validation until a worker actually runs; local results remain separately identified.