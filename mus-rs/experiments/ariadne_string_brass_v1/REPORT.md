# Ariadne string–brass unification campaign

**Status:** first shared-body constructive result; research primitives landed, default renderer unchanged  
**Branch:** `agent/ariadne-string-brass-unification`  
**Parent:** `agent/ariadne-fidelity-performance-v1`  
**Date:** 2026-08-13  
**Constitution:** one circuit-free mechanical/acoustic/material world

## 1. Result

The challenge was deliberately stronger than making two good presets:

> One persistent body should be convincingly playable as an electric guitar or
> a trombone depending on configuration and gesture.

The campaign produced a first constructive answer. One fixed 24-coordinate
Hamiltonian wave body, one compliant boundary, one sparse internal-resonance
graph, and one radiation law can occupy both of these controlled regions:

```text
transient, finite-aperture, weakly dispersive plucked body

and

sustained, compliant-boundary, near-harmonic bore-like body.
```

A third render changes the live field metric, boundary compliance, source
ownership, and aperture geometry continuously while preserving the same field
state. It begins with plucked gestures and ends in a self-maintaining brass-like
oscillation. There is no crossfade between renderers and no state reset.

This is not yet listener-certified instrument identity. The guitar endpoint is
currently strongest as a clean finite-aperture electric-string body. The prior
electric-region campaign's active/resonant material has not yet been integrated
into the shared-body demonstration. The trombone endpoint sustains and develops
a harmonic spectrum, but robust slide/embouchure co-control across several bore
resonances remains open.

## 2. What “same body” means

The following are invariant across both regimes:

- state dimension and state storage;
- modal propagation algorithm;
- Hamiltonian and work ledger;
- compliant-boundary state;
- source/field scattering vocabulary;
- internal-resonance transport;
- radiation operation;
- fixed internal material clock in the reference campaign.

The following are lawful configuration or gesture coordinates:

- effective field length;
- endpoint phase and weak dispersion;
- passive loss distribution;
- finite source and radiation aperture shapes;
- compliant-boundary stiffness, asymmetry, rest opening, and damping;
- transient versus sustained source ownership;
- source/aperture/field coupling rates;
- configuration trajectory over time.

A different state vector, renderer, convolution response, or target-conditioned
sidecar is not admitted.

## 3. Common field theory

A stretched string and a one-dimensional acoustic bore have different physical
coordinates and boundary conditions, but both admit structure-preserving
reductions of the form

\[
M_\lambda \ddot q + D_\lambda \dot q + K_\lambda q
+ \nabla V_\lambda(q) = B_\lambda f.
\]

In normalized modal coordinates the shared reference body uses

\[
H_\lambda(q,p)
=\frac{\Delta t}{2}
\sum_{n=1}^{N}
\left[p_n^2+\omega_n(\lambda)^2q_n^2\right].
\]

The configuration-dependent spectrum is

\[
\omega_n(\lambda)
=2\pi f_{ref}\,
\frac{n+\phi}{\ell}
\sqrt{1+B(n+\phi)^2}.
\]

Here:

- `ell` is effective material length;
- `phi` is a continuous endpoint phase;
- `B` is a weak stiffness/flare coordinate;
- damping and aperture geometry are separate coordinates.

This is not claimed as a complete string PDE or Webster bore. It is the first
fixed-rank common chart in which both can be represented without changing the
state category.

### 3.1 Configuration work

Changing geometry while live displacement is stored is not a coefficient
setter. At fixed `(q,p)` it performs

\[
W_{config}
=H_{\lambda'}(q,p)-H_\lambda(q,p).
\]

The reference implementation books this quantity directly. A production
trajectory may instead compile a metric-compatible state transport plus a
positive deformation, following Ariadne's work–holonomy semantics. Silent
retuning is not allowed.

### 3.2 Exact propagation and radiation

At fixed configuration each modal pair advances by the exact harmonic rotation

\[
\begin{aligned}
q'_n &= \cos(\omega_n\Delta t)q_n
+\frac{\sin(\omega_n\Delta t)}{\omega_n}p_n,\\
p'_n &= \cos(\omega_n\Delta t)p_n
-\omega_n\sin(\omega_n\Delta t)q_n.
\end{aligned}
\]

A finite normalized aperture `r` radiates projection `d=r^Tp` using

\[
y=\sqrt{\eta}\,d,
\qquad
p'=p-\left(1-\sqrt{1-\eta}\right)rd.
\]

Therefore the removed field energy equals

\[
E_{rad}=\frac{\Delta t}{2}y^2
\]

up to floating-point residual. There is no output filter standing in for a
body, pickup, bell, or cabinet.

## 4. One compliant boundary

The key missing primitive was not a trombone renderer. It was a source/contact
boundary capable of two playing regimes.

The boundary owns

\[
x_a,\quad p_a,
\]

with positive asymmetric potential

\[
V_a(x)
=\frac12\omega_a^2(x-x_0)^2
+\frac{\beta}{4}
\left[(x-x_0)(x-x_0+2s)\right]^2.
\]

The open fraction is a bounded positive function of the configured rest gap
and live displacement. It controls source/aperture and source/field scattering.

### 4.1 Through and sweeping components

Measurements of trombone playing distinguish flow through the lip aperture
from flow produced by lip motion sweeping volume into the bore. The shared
boundary represents these as two separate lawful operations:

1. **through/source exchange:** an incoming external wave scatters with aperture
   and field projections; net source work is incoming minus returned boundary
   energy;
2. **sweeping exchange:** aperture momentum and field flow undergo a reciprocal
   Givens rotation, performing zero work in the joint kinetic metric.

For normalized field aperture `v`, field flow `d=v^Tp`, and aperture momentum
`p_a`, the sweep is

\[
\begin{bmatrix}p_a'\\d'\end{bmatrix}
=R(\theta_s(x_a,e_b))
\begin{bmatrix}p_a\\d\end{bmatrix},
\qquad
{p_a'}^2+{d'}^2=p_a^2+d^2.
\]

The field can therefore load the aperture and the moving aperture can return
energy to the field without a one-way source/filter split.

### 4.2 Localized sustained work

A sustained trombone-like regime needs external player work. It is localized at
the compliant boundary rather than applied globally to the body. For target
momentum `A`, opening `h`, source activity `s`, synchronization factor `chi`, and
aperture momentum `p_a`, the declared work request is

\[
W_a
=\Delta t\,\mu s h\chi\,p_a^2
\left(1-\frac{p_a^2}{A^2}\right).
\]

- below `A`, positive work enters through the external source receipt;
- above `A`, the same law is dissipative;
- exact zero momentum remains zero;
- no sustained source means no positive active-boundary work;
- field phase changes `chi`, allowing resonator/aperture synchronization.

This is a localized saturable constitutive law, not an output oscillator or a
sample-domain waveshaper.

### 4.3 Guitar regime

```text
near-harmonic weakly stiff field
stiff, highly damped compliant boundary
short external player pulse
no sustained source
no active boundary work
narrow source and observation apertures
```

The pulse exchanges work through the same source boundary and sweeps momentum
into the field. The body subsequently evolves and radiates without an onset
envelope or sample playback.

### 4.4 Trombone regime

```text
longer near-harmonic field
compliant low-frequency boundary
sustained external source wave
localized saturable boundary work
broad inlet and bell-like radiation apertures
reciprocal field loading and sweeping exchange
```

The field, boundary, and source settle into a periodic regime. The source does
not write a prescribed oscillator waveform into the bore.

## 5. Rust landing

### `ariadne_morphing_field.rs`

Provides:

- one fixed-capacity state;
- guitar and trombone construction configurations;
- exact modal propagation;
- exact metric/configuration work;
- passive loss receipts;
- finite-aperture effort, flow, and radiation;
- state-preserving configuration changes;
- tests proving inverse length/frequency scaling and shared state dimension.

### `ariadne_compliant_aperture.rs`

Provides:

- one persistent aperture state;
- positive nonlinear stored potential;
- state-preserving configuration changes and control work;
- incoming/returned source-wave receipts;
- reciprocal zero-work sweeping exchange;
- localized sustained source work and saturating loss;
- discrete-gradient aperture propagation;
- exact zero, pulse, sustained-work, saturation, and fault tests.

Both modules are exported under `ariadne-live`. Neither is wired into the
default `AriadneTerminal`; promotion first requires an experimental composite
anatomy and real Rust execution evidence.

## 6. Independent law oracle

The dependency-free oracle ran 20,000 randomized trials.

| Law | Maximum residual |
|---|---:|
| Modal propagation, relative | `1.06e-15` |
| Configuration work, relative | `3.20e-16` |
| Finite-aperture radiation | `1.83e-19` |
| Source-boundary scattering | `7.79e-20` |
| Sweeping metric | `5.42e-20` |
| Saturable source work/loss | `2.17e-19` |
| Aperture propagation, relative | `4.80e-12` |
| Inverse length/frequency law | `8.88e-16` |
| Exact zero | exact |

The larger absolute modal/configuration residuals in `RESULTS.json` occur at
large randomly generated energies; their relative errors remain at ordinary
binary64 scale.

## 7. Behavioral campaign

### 7.1 Fixed body

```text
state dimension        24 modal pairs + one aperture pair
internal clock         96 kHz
reference frequency    116.5 Hz
field reset            never during a render
renderer switch        none
```

The same sparse internal-resonance graph is present in all configurations.
Source and radiation rows move continuously between string and bore aperture
geometries.

### 7.2 Guitar probe

The guitar trajectory uses six short player pulses and a fretting/length path
covering 0, 3, 5, and 7 semitones before returning. The sustained source and
active aperture are exactly zero.

Measured:

| Descriptor | Value |
|---|---:|
| Peak | `0.1154` |
| RMS | `0.02383` |
| First attack RMS | `0.04053` |
| First late-tail RMS | `0.01772` |
| First-note decay ratio | `0.437` |
| Dominant first-note partial | `232.5 Hz` |
| First-note spectral centroid | `201.7 Hz` |

The dominant second partial is a consequence of the selected finite observation
aperture, not a fixed EQ. Moving the aperture changes the modal nulls.

### 7.3 Trombone probe

The trombone trajectory applies a smooth sustained source envelope to the same
body under the compliant configuration.

Measured:

| Descriptor | Value |
|---|---:|
| Peak | `0.1444` |
| RMS | `0.04602` |
| Steady/attack RMS | `1.272` |
| Spectral peak | `119.33 Hz` |
| Fundamental-energy fraction | `0.280` |
| Harmonic richness | `0.720` |
| Crest factor | `2.585` |

The steady amplitude exceeds the early attack-region RMS, while the played
frequency lies close to the configured first field resonance. This is a
self-maintaining coupled regime rather than a decaying impulse response.

### 7.4 State-continuous morph

The continuous demonstration performs three plucked gestures, then moves the
live body through:

```text
string metric -> bore metric
stiff contact -> compliant aperture
pulse source -> sustained source
narrow readout -> broad radiation
```

No state is reset. Measured:

| Descriptor | Value |
|---|---:|
| Pre-morph RMS | `0.02779` |
| Post-morph RMS | `0.04908` |
| Late spectral peak | `118.89 Hz` |
| Late fundamental fraction | `0.185` |
| Source work | `0.011109` |
| Configuration work | `-6.20e-8` |
| Declared loss | `0.011019` |

The small signed configuration-work total is state/path contingent; it is not a
claim that arbitrary geometry motion is free.

## 8. Reference performance

Numba/LLVM timing used one CPU core and excludes compilation.

| Internal rate | Real-time factor | ns/sample |
|---|---:|---:|
| 48 kHz | `27.97x` | `745` |
| 96 kHz | `13.69x` | `761` |
| 192 kHz | `6.84x` | `761` |

This reference is considerably cheaper than the earlier full terminal campaign
because it is a compact 24-mode field without the twelve-course room. These are
not Rust plug-in numbers.

The approximately constant cost per sample supports the fixed-clock direction.
A 96 kHz internal material clock leaves substantial reference headroom for host
conversion, active electric material, room coupling, and a richer radiator.

## 9. Negative and incomplete results

### 9.1 Slide control is not yet robust

Changing field length alone moves passive resonances exactly as `1/ell`.
However, the sustained active regime did not reliably follow several slide
positions under one fixed aperture-tension trajectory. It sometimes remained
captured by a neighboring resonance.

This is physically meaningful rather than a reason to add a pitch shifter. A
player must coordinate slide length, lip resonance/tension, source pressure,
and opening phase. The next compiler must learn or solve this joint trajectory.

### 9.2 The guitar endpoint is presently the clean region

The shared-body demonstration proves a plucked finite-aperture string regime.
It does not yet include the first campaign's thresholded active material and
full electric edge/grind manifold in the same composite runtime. That is the
next integration, not a completed result.

### 9.3 Bore physics is reduced

The current spectrum is a fixed-rank modal chart. It does not yet include:

- a resolved positive area field;
- frequency-dependent viscothermal memory;
- bell radiation impedance and directivity;
- finite-amplitude nonlinear wave steepening;
- mouthpiece-cavity state;
- explicit aperture-flow versus sweeping-flow volume units.

These are candidates for deeper shared physics, not invitations to create a
separate trombone renderer.

### 9.4 Perceptual identity is open

The endpoints are objectively different and musically recognizable as
transient-string-like versus sustained-brass-like in the internal review pack.
No blinded listener study has yet established convincing electric-guitar or
trombone identity.

## 10. Next program

### U1 — composite experimental anatomy

Build one `AriadneStringBrassBody` containing:

```text
MorphingWaveField<24>
CompliantApertureBoundary<24>
InternalResonanceTransport<24, P>
SynchronizedActiveInclusion<24>
finite radiator
optional reciprocal room
one ledger
```

The active inclusion is enabled by trajectory, not constructed only for the
guitar regime. The trombone source remains external boundary work.

### U2 — fixed internal clock

Run the body at a fixed 96 kHz material clock with structure-preserving host
conversion. Test 44.1, 48, 88.2, 96, 176.4, and 192 kHz hosts for:

- waveform and descriptor agreement;
- latency and block invariance;
- aliasing under electric breakup and brass fortissimo;
- state serialization across host-rate changes;
- CPU and instance count.

### U3 — joint slide/embouchure compiler

Given desired pitch and gesture, solve a trajectory over:

```text
field length
boundary natural frequency
opening/rest gap
source level
sweep timing
aperture geometry
```

Require stable periodic capture, bounded work, and continuity through glissandi.
Optimization failure is `unknown`, not proof of unreachability.

### U4 — shared resolved field

Investigate a structure-preserving one-dimensional lattice whose constitutive
metric can interpolate between tension-dominated transverse mechanics and
compressibility-dominated longitudinal acoustics. A fixed-rank modal chart may
remain the live reduction, but the high-resolution oracle should make area,
tension, density, compliance, and moving boundaries explicit.

Dynamic-grid work on both trombones and stiff strings suggests that changing
physical length can share one numerical theory. Ariadne should translate that
insight into work-accounted state transport rather than run two solvers.

### U5 — nonlinear propagation and radiator

Add amplitude-dependent wave speed/steepening and a resolved radiating end. The
objective is to reach brassy high-level spectra and guitar-string stiffness from
shared constitutive coordinates, while preserving the clean linear limit.

### U6 — external validation

- electric guitar: EGFxSet plus held-out DI/player gestures;
- trombone: isolated attacks, sustained notes, dynamics, mutes, and glissandi;
- one anatomy across both corpora;
- separate lawful performance trajectories;
- blind instrument-family recognition;
- resemblance and preference reported separately.

## 11. Promotion gates

The shared body may become an experimental plug-in anatomy only after:

1. Rust formatting, compilation, Clippy, and workspace tests;
2. cross-runtime replay against the independent oracle;
3. one closing global ledger for field, aperture, active material, radiation,
   room, source, and configuration work;
4. fixed-clock host-rate evidence;
5. stable multi-note trombone capture and continuous glissando;
6. integration of clean/edge/grind electric regions without destroying brass;
7. listening evidence for both endpoints;
8. no-circuit and anti-smuggling checks remain green.

## 12. Bottom line

The challenge reveals a deeper Ariadne ontology:

> A string and a bore need not be different instruments at the renderer level.
> They can be different metric and boundary regimes of one persistent wave
> material.

The first implementation already crosses from transient plucked behavior to a
sustained, harmonically rich, resonator-locked boundary oscillation without a
renderer switch. The unification is therefore plausible and executable.

The remaining task is not to bolt GARGANTA onto the guitar. It is to make the
shared field, compliant boundary, and trajectory compiler rich enough that a
player can inhabit both regions—and the strange continuous territory between
them—with the same body.
