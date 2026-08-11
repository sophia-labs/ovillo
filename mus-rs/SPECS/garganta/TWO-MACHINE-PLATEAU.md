# GARGANTA two-machine plateau — initial-data response and causal ablation plan

**Status:** binding G7/G8 experiment amendment.  
**Observed base:** `agent/weave-string-network` at `048321b7501ab30dca6d92eb0ceb3b9233ec2e35`.  
**Input:** `speak_mimic.py`, `speak_mimic_unified.py`, the fitted speaker-throat receipt, and the measurement recorded in `GARGANTA.md`.

## 1. The observation

The implementation team ran two word-level optimizations against the same `apple_target.wav` and the same 64-band log-mel MSE:

```text
system                                      best loss   recorded evaluations   search dimension
rung-1 generic segmented instrument           1.9204              7,040               25
fitted MAP throat + smooth gesture/source ES  2.0162              8,401               64
fitted-throat warm interpolation               3.6384                  1               64
```

The unified run uses eight anchors, each carrying five tract controls plus gain, aspiration, and F0 offset. The fitted MAP anatomy is held fixed throughout that run. The generic run has a word-specific five-segment scaffold with an explicit silence closure and a separately authored burst/puff region.

The result is useful. It is also more specific than “two independent machines prove the same physical bottleneck.”

## 2. What the data establish

The strongest supported statements are:

1. The fitted frequency-domain throat can participate in a word-level control search and reach the same **order of aggregate spectral residual** as the established rung-1 instrument.
2. On the fitted anatomy, optimizing the time-varying control/source program reduces log-mel MSE from `3.6384` to `2.0162`: an absolute decrease of `1.6222`, about `44.6%` relative to the warm interpolation.
3. The receipt-state interpolation is not an adequate word renderer by itself. A separate dynamic source/articulation solve is necessary.
4. A large residual remains after thousands of evaluations. Static anatomy fitting does not, by itself, solve speech rendering.
5. G1 fast articulation, G2 closure/burst physics, and G3 source physics remain high-value hypotheses and sensible engineering priorities.

The current evidence class is **exploratory construction-oracle convergence**. It does not promote intelligibility, speaker fidelity, production-tract validity, or causal attribution to any individual G-package.

## 3. One important attribution correction

The `3.6384 -> 2.0162` improvement is **not an anatomy effect**. The same fitted MAP anatomy is used before and after optimization. The variables that change are:

```text
five articulation controls at eight anchors
voicing/gain at eight anchors
aspiration at eight anchors
F0 offset at eight anchors
```

Therefore the measured statement is:

> Given this fitted anatomy, optimizing source and articulation substantially outperforms a heuristic interpolation of the receipt states.

The anatomy effect requires a counterfactual:

```text
same renderer
same gesture/source parameterization
same warm-start policy
same seeds and budgets
fitted anatomy versus neutral, shuffled, random, posterior, and population anatomies
```

Until that experiment exists, “the resonator matters enormously” is physically plausible but not isolated by this measurement.

## 4. Why `1.9204` and `2.0162` are not yet a controlled architecture comparison

### 4.1 Different structural priors

The generic search has a target-specific five-part word template:

```text
vowel -> exact silence closure -> burst/puff -> vowel -> lateral
```

The fitted-throat search has eight uniformly spaced, linearly interpolated anchors and no exact topology-changing closure. For a roughly one-second word, its knot spacing is on the order of 100–150 ms, while consequential stop transitions and releases occur on roughly 10–50 ms scales.

The generic system therefore begins with stronger knowledge of this particular word’s segmentation and more temporal discontinuity than the supposedly more physical system.

### 4.2 Different search dimension and effective budget

The generic run searches 25 dimensions with 7,040 renders. The unified run searches 64 dimensions with 8,401 renders. That is approximately:

```text
generic: 282 evaluations per dimension
unified: 131 evaluations per dimension
```

A single seeded diagonal ES run cannot distinguish an architecture floor from an optimizer/budget floor, especially in the higher-dimensional system.

### 4.3 The warm start is intentionally incomplete

The warm trajectory is assembled only from representative stable voiced frames. It does not contain explicit unvoiced intervals or closures. The current adapter also:

- discards each state’s measured F0 and starts from the global median;
- maps frame RMS in dB linearly to gain rather than converting amplitude logarithmically;
- converts `1 - periodicity` directly into aspiration;
- interpolates independently fitted acoustic states as though they were a coherent articulatory path.

Its high loss is therefore not evidence against the anatomy fit. It is mainly a baseline for that adapter.

### 4.4 The diagnostic renderer needs an identity audit

The unified renderer applies a frame-varying minimum-phase transfer function by overlap-add, but it does not currently record a flat-transfer reconstruction test or normalize by the accumulated analysis window. With a 1,024-sample Hann window, 480-sample hop, and 2,048-sample filtered frame, renderer modulation or overlap error may contribute to the residual.

Each tract response is also normalized to its own peak before filtering. That removes absolute transfer gain state by state. A severe constriction can therefore be made as loud as an open vowel before the separate gain control is applied. This specifically erases one physical acoustic consequence that G2 is meant to produce.

Before using aggregate loss to rank physical work packages, the construction renderer must prove:

```text
flat transfer reproduces its source within tolerance
static transfer agrees with direct convolution
windowed overlap-add has bounded reconstruction error
absolute attenuation is retained or its nuisance normalization is fixed and declared
cache quantization does not create the apparent plateau
```

### 4.5 The two runs share more than a target

They share the same aggregate loss, seeded evolutionary-search family, single target, and broad source/filter limitations. Similar terminal losses could reflect a physical common wall, but they could also reflect the objective’s scale, target alignment, optimizer behavior, candidate peak normalization, or renderer ceilings.

A numerical coincidence within `0.0958` MSE is suggestive, not a confidence interval.

## 5. Revised interpretation

The appropriate statement is:

> Two materially different construction systems, under one target and one aggregate loss, reached residuals of the same order. The result strengthens the case that static timbre/anatomy alone is insufficient and motivates causal ablations of gesture bandwidth, closure/burst physics, and source physics. It does not yet identify their relative contributions or establish a universal floor.

“Round-5 priority order confirmed empirically” should be read as a productive engineering hypothesis, not as a completed causal result.

## 6. Plateau-disambiguation campaign

The next campaign is factorial in concept but staged to keep it affordable.

### P0 — renderer validity

Before another long search:

1. Add a flat-response identity fixture.
2. Add overlap-weight normalization or a declared perfect-reconstruction window pair.
3. Compare frame-wise filtering against static direct convolution.
4. Preserve physical transfer gain, or normalize every state against one fixed neutral reference rather than its own maximum.
5. Record target and candidate peak, LUFS/RMS, trim policy, and alignment.
6. Record loss for target against small time shifts, level shifts, resampling, and benign filtering. This gives the number `~2.0` a calibrated scale.

A failed P0 test classifies the current plateau as `renderer-limited` until repaired.

### P1 — anatomy causality

Run the identical control/source optimizer over:

```text
fitted MAP anatomy
all retained posterior anatomies
neutral center anatomy
length-matched neutral anatomy
parameter-shuffled anatomy
random valid anatomies
population-design archive members, once available
```

Use paired seeds and two initialization policies:

```text
common neutral control start
anatomy-specific fitted/warm start where available
```

Report paired loss curves, best/median loss, budget-to-threshold, and functional-anatomy distance. Anatomy earns causal credit only when fitted/posterior members outperform the controls across independent seeds and targets under the same renderer and search.

### P2 — gesture bandwidth and optimizer floor

Sweep at least:

```text
8, 16, 32, and 64 anchors
uniform versus boundary-aware/free knot locations
linear versus smooth monotone interpolation where applicable
```

For each setting, run both equal-total-budget and approximately equal-evaluations-per-dimension conditions. Use multiple seeds and at least one optimizer that exploits the near-differentiable fast map rather than only the diagonal ES.

Keep two score channels:

```text
unregularized acoustic upper bound
physically admissible score with rate, acceleration, and W_metric penalties
```

A reduction concentrated at consonant transitions is evidence for G1. A global reduction without transition localization is only evidence for additional control capacity.

### P3 — closure and burst upper bound

Before G2 is fully implemented, add an evidence-only oracle ablation:

- target-aligned exact source gating for closure;
- a separately parameterized aperiodic release event;
- a location-conditioned downstream-cavity filter;
- no claim that this oracle is the instrument.

Measure the loss change only in closure, release, and VOT windows as well as globally. This estimates the maximum residual available to G2 without smuggling the oracle into production.

### P4 — source upper bound

Hold anatomy and articulation fixed, then compare:

```text
diagnostic Rosenberg source
richer LF-family diagnostic source
inverse-filtered target-source oracle, evidence path only
future physical G3 fold source
```

Decompose loss over stable voiced regions, onset/offset, aspiration, F0, spectral tilt, HNR, and temporal modulation. This identifies the residual available to G3.

### P5 — objective decomposition

Global log-mel MSE remains a useful common scalar, but every run also reports:

```text
steady-voiced versus transition versus unvoiced loss
per-frame and per-segment loss curves
formant-track and bandwidth residuals
F0 and voicing residuals
aperiodicity/HNR residuals
onset, closure, burst, and VOT timing
source-envelope and tract-envelope residuals where identifiable
```

The same waveform can improve aggregate log-mel while becoming less intelligible. Machine-listener and human-listener evidence remain separate promotion classes.

### P6 — replication and generalization

Minimum replication before calling two values the same plateau:

```text
at least 10 paired seeds
full loss-versus-evaluation curves
bootstrap interval over seed-level best losses
more than one word and more than one phonetic structure
```

For sample-conditioned identity claims, anatomy is fitted on one set of recordings and frozen before novel utterances, F0 trajectories, registers, and channel conditions are rendered. Same-target reconstruction cannot promote beyond the sample-conditioned claim classes.

## 7. Required benchmark receipt

Each future row in the comparison matrix records:

```text
benchmark and target digests
anatomy and posterior-member digest
anatomy-training evidence digests
renderer, source, transfer-law, and loss versions
control parameterization and dimension
knot schedule and interpolation law
optimizer, seed, population, evaluations, and stopping rule
warm-start type and digest
normalization, trim, and alignment policy
renderer-validity fixture results
aggregate and decomposed losses
claim level, evidence class, and withheld claims
```

This makes a “floor” a replayable measured object rather than a number copied into prose.

## 8. Consequences for the anatomy-evolution program

The initial result does not weaken the two-regime design. It sharpens it:

- Population design is needed to provide neutral, random, policy-optimal, and functionally distinct anatomy counterfactuals.
- Posterior retention is necessary because the full word-level optimum may not be the LPC-envelope MAP member.
- Sample-conditioned fitting remains a bilevel problem: anatomy, source, and articulation require separate causal accounting.
- The one-recording claim ceiling remains necessary; this run uses the same target for anatomy evidence and control optimization.
- Production replay remains mandatory because the current frequency-domain renderer is an analysis/construction oracle.

Immediate order after this measurement:

```text
P0 renderer validity
P1 anatomy counterfactuals + paired seeds
P2 control-bandwidth sweep
P3/P4 oracle upper bounds for closure/burst and source
cross-target and cross-utterance replication
then evidence-based reprioritization of G1/G2/G3
```

The experiment is a good first datum. Its most valuable effect is that the next questions can now be causal rather than rhetorical.