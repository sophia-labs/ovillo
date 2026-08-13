# Ariadne fidelity and performance campaign v1

**Status:** measured reference campaign; implementation primitives landed for review  
**Base:** `agent/ariadne-pure-physics`  
**Campaign branch:** `agent/ariadne-fidelity-performance-v1`  
**Date:** 2026-08-13  
**Constitution:** circuit-free Ariadne only

## 1. Executive result

The current pure-material Ariadne is already a lawful and computationally plausible live body. It is not yet a comprehensive electric-guitar region.

The baseline closes its global work ledger at approximately floating-point scale, keeps low-level pitch identity, and runs far faster than real time in the native reference implementation. Its weakness is behavioral breadth: it has almost no persistent constitutive memory, its active reservoir is barely depleted, its tail decays quickly, and it does not move continuously from clean through edge, grind, sustain, and feedback-rich behavior with one control family.

The campaign tested five deeper physical mechanisms:

1. essentially nonlinear energy-sink cells;
2. sparse bistable inclusions;
3. state-dependent internal-resonance transport;
4. a global saturable Liénard activity law;
5. a thresholded, room-synchronized, reservoir-paid active inclusion.

The best current candidate is:

```text
current positive asymmetric material
+ thresholded synchronized active inclusion
+ state-dependent internal-resonance transport
+ existing finite-aperture radiation and causal room
```

At 48 kHz it reached the following operational region scores:

```text
clean      0.699
edge       0.763
grind      0.622
sustain    0.891
feedback   0.166
memory     0.891
breadth    0.811
```

These are search objectives, not listener scores. The candidate preserves low-level fundamental identity (`0.954` fundamental-energy fraction), develops strong level-dependent harmonic growth, creates measurable loud-after-soft history, retains roughly half of its late-tail amplitude in the declared probe, and closes the global ledger with a maximum residual of `1.96e-14`.

The candidate is not ready to become the product default. Its highest input level crosses too far into harmonic takeover: THD rises to `6.80`, while the fundamental-energy fraction falls to `0.014`. This proves reachability of a heavy driven region; it does not yet prove a well-shaped production control manifold.

## 2. Claim boundary

This campaign establishes:

- exact or numerically tight local and global work identities in the reference model;
- reproducible movement through controlled clean, edge, grind, sustain, memory, and room-return descriptors;
- comparative performance of candidate physical laws;
- a concrete candidate extension that remains inside Ariadne’s mechanical/acoustic/material ontology;
- negative results identifying mechanisms that are too global, too fragile, or too expensive.

It does **not** establish:

- perceptual equivalence to a named guitar, pedal, amplifier, speaker, or recording chain;
- listener-recognized electric-guitar fidelity;
- production Rust performance;
- sample-rate-invariant timbre;
- stable self-sustaining feedback;
- alias-free output under the strongest nonlinear conditions;
- universal coverage of electric-guitar behavior.

External hardware-recorded corpora were identified, but their audio could not be materialized in this execution environment. Current fidelity evidence therefore uses controlled analytic and physically generated guitar-like probes rather than corpus matching.

## 3. Experimental protocol

### 3.1 Reference implementation

The executable oracle is a NumPy/Numba mirror of `mus_dsp::ariadne_terminal`:

- twelve positive asymmetric material cells;
- discrete-gradient material propagation;
- finite-aperture reciprocal boundary exchange;
- finite work reservoir and explicit recharge;
- closed ordered Givens transport;
- passive loss and orthogonal radiation;
- a twelve-course reciprocal room field;
- one global source/loss/radiation/storage ledger.

Candidate laws are inserted into that same world. No target waveform, impulse response, learned residual, circuit model, or emitted-sample waveshaper appears in the renderer.

### 3.2 Probe suite

The production replay uses:

- a 110 Hz sine ladder at input levels `0.015`, `0.04`, `0.10`, `0.24`, and `0.48`;
- a 110/165 Hz two-tone probe;
- a 110 Hz physically generated plucked-string probe;
- a loud-pluck then soft-pluck sequence compared with an isolated soft pluck;
- a short excitation followed by unforced material/room evolution;
- native-rate replays at 48, 96, and 192 kHz;
- 48-versus-96 kHz gain-aligned shape comparison.

### 3.3 Descriptor family

Measured descriptors include:

```text
input/output compression slope
fundamental-energy fraction
harmonic and even/odd growth
intermodulation products
pluck spectral cosine
transient correlation
late/early energy
loud-to-soft memory ratio
room/tail ratio and decay slope
peak and maximum state
reservoir work
local equation residual
global ledger residual
nanoseconds per sample
real-time factor
sample-rate correlation and relative error
```

The region scores are broad, explicit functions of these descriptors. They are useful for search and ablation. They are not substitutes for listening evidence.

## 4. Baseline

The landed pure terminal is an excellent numerical control condition.

| Measure | Baseline at 48 kHz |
|---|---:|
| Clean score | 0.633 |
| Edge score | 0.196 |
| Grind score | 0.038 |
| Sustain score | 0.505 |
| Feedback score | 0.000 |
| Memory score | 0.000 |
| Breadth score | 0.220 |
| Compression slope | 1.056 |
| Low-level THD | 0.0126 |
| High-level THD | 0.357 |
| Low fundamental fraction | 0.955 |
| High fundamental fraction | 0.849 |
| Pluck spectral cosine | 0.979 |
| Memory ratio | 0.99999 |
| Tail ratio | 0.0130 |
| Tail decay | −44.9 dB/s |
| Max ledger residual | `3.69e-16` |

Interpretation:

- clean identity and transient spectral structure are strong;
- some nonlinear color is already latent;
- the input/output slope is nearly expansive rather than compressive;
- loud events do not materially alter subsequent soft response;
- the room can ring, but the chosen baseline trajectory does not approach sustained acoustic takeover;
- the existing reservoir-active law performs too little constitutive work to own sag, bloom, and feedback-rich behavior.

## 5. Candidate mechanisms

### 5.1 Essentially nonlinear energy sinks

Selected material coordinates receive very low linear stiffness, stronger quartic storage, and enhanced passive loss. They have no narrow preferred low-energy resonance and can capture energy over a wider amplitude-dependent range.

Measured result:

```text
breadth        0.520
edge           0.812
grind          0.515
low THD        0.159
high THD       0.735
IMD            0.521
48 kHz RT      8.89x
ledger residual 4.78e-16
```

This is the strongest passive extension in the campaign. It substantially widens the edge/grind frontier with negligible reference CPU cost. It does not create memory or feedback by itself.

**Decision:** promote as a research material primitive.

### 5.2 Bistable inclusions

Sparse cells receive a shifted nonnegative double-well potential. This creates snap-through and true mechanical history without applying a nonlinear map to the emitted sample.

Measured result:

```text
grind          0.622
sustain        0.778
memory ratio   0.965
48/96 corr     0.784
relative error 0.621
```

Bistability creates meaningful breakup and a small state-history effect, but the present discretization is strongly sample-rate sensitive and compromises low-level identity.

**Decision:** retain as an offline/research mechanism; do not place in the live default until a sample-rate-stable transition law and a much better control chart exist.

### 5.3 State-dependent internal-resonance transport

For authored pairs `(i,j)`, define

\[
g_{ij}(q)=\tanh\!\left(1.6q_iq_j+0.25(q_i^2-q_j^2)\right)
\]

and apply the metric-compatible momentum rotation

\[
(p_i,p_j)\leftarrow R\!\left(
\frac{2\pi f_T}{f_s}\,w_{ij}\,(0.35+0.65m)\,g_{ij}(q)
\right)(p_i,p_j).
\]

Every factor is orthogonal; the transport performs zero kinetic work. Because the paired modes subsequently evolve under different material frequencies and nonlinear potentials, the exchange produces amplitude-dependent saturation, beating, harmonic exchange, and intermodulation.

Measured result:

```text
clean          0.636
sustain        0.752
low THD        0.0077
high THD       0.962
low fundamental 0.955
pluck cosine   0.977
48 kHz RT      8.10x
ledger residual 4.26e-16
```

This mechanism preserves the low-level instrument while opening a high-level nonlinear/modal region. It adds about thirteen percent reference cost versus baseline.

**Decision:** promote as a research transport primitive. `ariadne_resonant_transport.rs` is the Rust landing.

### 5.4 Global saturable Liénard activity

The first active candidate applied a global saturable self-oscillating law to one dominant material direction. It produced a near-stationary tail, but it activated too broadly and replaced the instrument’s low-level identity.

Measured result:

```text
clean          0.000
edge           0.000
breadth        0.038
low fundamental 0.002
pluck cosine   0.221
tail ratio     0.975
48/96 corr     0.413
```

This is an important negative result. A lawful active source can still be architecturally wrong when it globally dominates the world.

**Decision:** reject as a product/default mechanism; retain as a control proving that sustain alone is not success.

### 5.5 Thresholded synchronized active inclusion

The successful active candidate localizes active conversion in three orthogonal material directions and gives them one slow constitutive activation state.

Let boundary activity be `b`, material momentum RMS be `r_m`, and room-head RMS be `r_h`. The sensed activity is

\[
s=b+0.08\,u_{sustain}r_m
+12\,u_{feedback}u_{space}r_h.
\]

The threshold is

\[
h=0.025+0.25\,u_{headroom}.
\]

The target activation is

\[
a_*(s)=\frac{[s-h]_+}{h+[s-h]_++\varepsilon}.
\]

The slow state follows

\[
\dot a=\frac{a_*-a}{\tau(a_*,a)},
\]

with a faster attack and a slower sag-dependent release.

For normalized material direction `v_r` and momentum projection

\[
d_r=v_r^Tp,
\]

its requested power is

\[
P_r=\mu_r a\,d_r^2\left(1-\frac{d_r^2}{A_r^2}\right).
\]

- `P_r > 0`: the inclusion requests work, capped by the finite reservoir;
- `P_r < 0`: the inclusion dissipates work;
- `d_r = 0`: exact zero state remains zero;
- `s <= h`: activation target is exactly zero;
- room motion can cross the threshold, but the room itself supplies no hidden work.

This is an acoustic synchronization law translated into Ariadne’s material ontology. Returning room state controls access to stored supply; all positive work remains explicitly paid.

`ariadne_active_inclusion.rs` implements the law and tests zero-state invariance, threshold behavior, orthonormal directions, reservoir closure, saturation loss, room-triggered activation, and release memory.

## 6. Best current combination

The best production replay combines the thresholded active inclusion with internal-resonance transport. The expensive collective quartic solver was disentangled and is **not required** for this result.

### 6.1 Operational scores

| Region | Score |
|---|---:|
| Clean | 0.699 |
| Edge | 0.763 |
| Grind | 0.622 |
| Sustain | 0.891 |
| Feedback | 0.166 |
| Memory | 0.891 |
| Breadth | 0.811 |

### 6.2 Physical and acoustic descriptors

| Measure | Value |
|---|---:|
| Compression slope | 0.847 |
| Low-level THD | 0.0374 |
| High-level THD | 6.80 |
| Low fundamental fraction | 0.954 |
| High fundamental fraction | 0.0140 |
| Intermodulation ratio | 0.223 |
| Pluck late/early ratio | 0.556 |
| Pluck spectral cosine | 0.888 |
| Loud-after-soft memory ratio | 1.308 |
| Tail ratio | 0.490 |
| Tail decay | −7.37 dB/s |
| Peak | 1.108 |
| Maximum state | 2.55 |
| Pump work over probe bundle | 0.0147 normalized work units |
| Maximum global residual | `1.96e-14` |
| Maximum material equation residual | `2.69e-14` |

Interpretation:

- low-level pitch and spectral identity survive;
- the transition from low to high input is decisively nonlinear;
- the high-level endpoint is currently too extreme and should become one corner of the manifold rather than the top of a generic `drive` knob;
- persistent activation produces a real loud-to-soft history effect;
- the room materially lengthens and colors the tail;
- feedback remains subcritical: the tail decays at approximately `−7.37 dB/s` rather than reaching stable self-sustain.

### 6.3 Candidate trajectory

The winning reference controls are ordered as:

```text
drive, headroom, bias, tone, bloom, sustain, sag,
interlace, chirality, memory, curvature, feedback,
space, room_decay
```

and equal:

```text
0.6743571006
0.8296639557
-0.1717981123
0.7529624612
0.9777207145
0.4355779235
0.6811642594
0.7731114103
0.3107578679
0.6616995956
0.4505989442
0.5505833497
0.2270924621
0.9830668154
```

This is a scientific fixture, not a product preset.

## 7. Performance

Reference timing used Numba/LLVM on one Intel Xeon Platinum 8573C core. The figures exclude JIT compilation and use the best of repeated 0.75-second native-rate renders.

| Variant | 48 kHz | 96 kHz | 192 kHz | ns/sample at 48 kHz |
|---|---:|---:|---:|---:|
| Baseline | 9.33× RT | 4.59× | 2.26× | 2233 |
| Essential NES | 8.89× | 4.44× | 2.25× | 2342 |
| Resonant transport | 8.10× | 4.19× | 2.12× | 2571 |
| Synchronized active + resonant | 8.36× | 4.12× | 2.03× | 2491 |
| Collective quartic + resonant | 4.62× | 2.20× | 1.11× | 4513 |

At an eighty-percent one-core budget, the best active/resonant reference corresponds to approximately:

```text
6.69 instances at 48 kHz
3.29 instances at 96 kHz
1.62 instances at 192 kHz
```

These numbers are encouraging, but they are not Rust or plug-in measurements. The candidate is computationally plausible; the production claim remains open until the Rust workspace compiles and the CLAP/VST path is benchmarked under host-block processing.

An initial apparent high-rate long-tail slowdown was not reproduced in isolated fixed/no-flush tests. No denormal or room-retirement product defect is claimed from that transient harness failure.

## 8. Sample-rate behavior

Native-rate rendering is not yet invariant enough.

| Variant | 48/96 correlation | Gain-aligned relative RMS error |
|---|---:|---:|
| Baseline | 0.951 | 0.311 |
| Essential NES | 0.962 | 0.273 |
| Resonant transport | 0.947 | 0.322 |
| Synchronized active + resonant | 0.924 | 0.381 |
| Bistable | 0.784 | 0.621 |
| Global Liénard | 0.413 | 0.911 |

The active/resonant law remains recognizable across rates but moves materially in amplitude and shape. A controlled resampling experiment using one fixed 96 kHz internal lattice produced identical outputs across 48 and 96 kHz host paths after conversion. This strongly supports a fixed internal material clock or an equivalent dimensionless time law as the next production architecture.

The fixed-rate performance of the final candidate must be remeasured in Rust; one exploratory Python timing was contaminated by JIT/harness effects and is not retained as evidence.

## 9. External fidelity plan

Three public corpora are appropriate evaluation sources:

### EGFxSet

Hardware-recorded clean tones from every fret of a Stratocaster under five pickup configurations, with the same material processed through twelve real effects, including BD-2, Tube Screamer, and RAT2 families. DOI: `10.5281/zenodo.7044411`.

Use:

- clean notes for articulation and position/fret holdouts;
- the three nonlinear effect families as behavioral regions;
- hardware labels as nuisance metadata, never as renderer anatomy.

### IDMT-SMT-Audio-Effects

Approximately thirty hours and 55,044 mono files containing monophonic and polyphonic guitar/bass examples with no effect, distortion, overdrive, and other effects. It is available for evaluation under CC BY-NC-ND 4.0.

Use:

- broad held-out note, articulation, and polyphony validation;
- effect/no-effect recognition and descriptor-distribution comparison;
- no redistribution or derivative corpus publication outside the license.

### GUITAR-FX-DIST

Paired unprocessed and processed electric-guitar recordings across overdrive, distortion, and fuzz, with mono/polyphonic and continuous/discrete subsets.

Use:

- paired dry/behavioral-target comparisons;
- level-dependent descriptor curves;
- polyphonic intermodulation and chord-clarity evaluation.

The corpus protocol is:

1. freeze train/validation/blind-test splits before fitting;
2. use one Ariadne anatomy across all corpus families;
3. fit only lawful target-specific trajectories;
4. compare descriptor distributions, not raw sample error alone;
5. never place target audio or a learned residual in playback;
6. run listener tests after objective model selection.

## 10. Promotion decisions

### Promote as research primitives

- **Essentially nonlinear energy-sink cells:** cheap and highly effective for edge/grind behavior.
- **Internal-resonance transport:** mathematically clean, zero-work, preserves low-level identity, and adds high-level exchange.
- **Thresholded synchronized active inclusion:** the first active law to create sustain and memory without destroying the clean region.

### Retain offline

- **Collective quartic Hamiltonian:** expressive and structure-preserving but roughly doubles reference cost; not required for the current champion.
- **Bistable cells:** interesting mechanical memory and snap-through; present sample-rate behavior is unacceptable for live promotion.

### Reject as default

- **Global Liénard activity:** obtains sustain by erasing low-level identity.
- **All-mechanisms combined:** confounds ownership, costs more, and does not outperform the simpler active/resonant candidate.

## 11. Required next gates

### R0 — compile and law replay

- Rustfmt, Clippy, unit tests, and workspace tests.
- Independent float64 comparison of the two new Rust primitives.
- Long-horizon adversarial closure and finite-state tests.

### R1 — experimental integration

Integrate active inclusion and resonant transport behind an explicit experimental anatomy/profile. Do not change the default terminal path yet.

Required receipts:

```text
activation trajectory
positive requested/accepted work
saturating loss
reservoir state
per-pair resonance angles
transport metric residual
room activity entering the activation detector
global closure
```

### R2 — fixed internal material clock

Compare:

- native host-rate evolution;
- fixed 48 kHz material clock;
- fixed 96 kHz material clock;
- multi-rate room with fixed material body;
- structure-preserving host conversion.

Promotion requires waveform/descriptor agreement, CPU, latency, aliasing, and state-serialization evidence.

### R3 — external corpus fidelity

Materialize EGFxSet first. It is smaller and gives clean plus real-hardware nonlinear families with consistent source notes. Freeze note, fret, pickup/configuration, and effect holdouts before search.

### R4 — control-manifold fitting

The present champion’s high endpoint is too extreme. Fit a safe trajectory manifold whose low/mid/high regions preserve:

```text
clean fundamental identity
monotonic harmonic growth
bounded intermodulation
continuous compression
non-destructive memory
stable peak and state envelope
```

### R5 — feedback threshold

Search geometry, room delay, radiation, active threshold, and reservoir coupling for three separable regimes:

```text
subcritical decay
bounded self-sustain
harmonic takeover
```

The current candidate reaches only the first regime with a much slower decay.

### R6 — listener and player evidence

- blinded recognition of clean/edge/grind/sustain/feedback regions;
- mechanism-matched negative controls;
- player tasks for cleanup, chord clarity, sustain, and controllable feedback;
- preference and resemblance reported separately.

## 12. Bottom line

The circuit-free wager survived its first serious campaign.

The baseline already contains a credible clean physical body. Essentially nonlinear attachments and internal-resonance transport reveal substantial driven behavior with almost no architectural compromise. A thresholded active inclusion, synchronized by boundary and returning room motion but paid entirely by a finite reservoir, adds the missing memory and sustain axis without erasing the clean region.

The strongest current result is therefore not “we can imitate an amplifier without circuits.” It is:

> **One persistent Ariadne material can preserve a clean plucked identity at low excitation and cross, through lawful state and geometry, into compression, intermodulation, harmonic takeover, memory, slow sustain, and room-conditioned return.**

The remaining deficits are now specific and tractable: control-manifold shaping, sample-rate invariance, true self-sustaining feedback, external-corpus fidelity, listener evidence, and production Rust performance.