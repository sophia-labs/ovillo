# Unified protected instrument — negative-results ledger

Negative results are part of the specification. They prevent later work from
quietly reintroducing attractive constructions that fail the strong claim.

## N1. Endpoint-only real `SO(32)` lowering

**Proposal:** realify the final `U(16)` holonomy, compile the resulting
`SO(32)` endpoint with arbitrary real Givens factors, and execute that word.

**Failure:** the endpoint is lawful, but individual arbitrary real factors need
not commute with the canonical complex structure. A factor may preserve
Euclidean energy while mixing one quadrature without the paired action required
by a positive-frequency oscillator interpretation.

**Counterexample:** `ordinary_real_givens_symplectic_residual()` returns a
nonzero complex-structure/symplectic residual for a generic isolated real-plane
rotation.

**Policy:** protected programs use `U(2)` edge blocks and `U(1)` phases. The old
real compiler remains valid only for state spaces explicitly interpreted as
real scattering amplitudes without the protected complex-bundle claim.

## N2. Protected sidecar driving an unrelated renderer

**Proposal:** evolve the four-component protected state independently and use
its overlap, phase, or amplitude as gain/control data for a string synth or
oscillator bank.

**Failure:** the spectral bundle belongs to the sidecar, not to the acoustic
state. Energy does not flow from the acoustic body back into the protected
state, and the emitted signal is not radiation from the protected dynamics.

**Policy:** excitation, transport, regional coupling, and radiation all act on
one persistent state. A sidecar remains a visualization or weak prototype and
must be labeled as such.

## N3. One-way protected core

**Proposal:** embed a protected core that drives string/body/room modes but is
not itself driven by them.

**Failure:** this is a controller-plus-plant cascade. It does not make the
protected modes modes of the complete instrument and cannot pass causal
room-to-string intervention.

**Policy:** every claimed regional bridge is bidirectional and has an exact
zero-coupling limit.

## N4. Raw long-delay Weave inherits the gap

**Proposal:** attach or relabel long delay lines and assert that the core's
rank-two gap remains the gap of the complete graph.

**Failure:** a lossless periodic `L`-sample delay has all `L`th roots of unity as
eigenvalues. Adjacent quasienergy spacing is `2π/L`, or `sample_rate/L` hertz.
The spectrum becomes arbitrarily dense with delay length. A fixed isolated
rank-two window therefore cannot be uniform without additional band structure
or reduction.

**Policy:** use a protected modal backbone; engineered band gaps; or a finite,
resolution-specific conservative dilation. Treat remaining delay modes as
passive attached dynamics unless separately certified.

## N5. Scalar-energy-neutral delay remap

**Proposal:** rescale a remapped delay state so that total `sum(x^2)` is
preserved.

**Failure:** one scalar can close while state direction, phase, waveform,
pitch, and repeated-cycle behavior are badly wrong.

**Policy:** compare against a dense weighted-polar/partial-isometry oracle using
operator error, state error, spectral artifacts, and ledger closure separately.

## N6. Sparse four-nonzero transport as production truth

**Proposal:** truncate each row of the dense weighted-polar transport to four
entries and renormalize or accept it as local.

**Failure:** the committed full packet measures approximately:

```text
operator Frobenius error                 0.1321
worst sampled relative energy error      1.48%
worst sampled state error                0.1656
```

These are useful baseline values, not a production certificate.

**Policy:** retain the truncation as a negative control. A sparse method must
meet explicit tolerances against the dense oracle and preserve disposal laws.

## N7. Silent rank contraction

**Proposal:** shrink a delay/state vector and drop the unrepresented
coordinates.

**Failure:** no injective linear isometry exists from a higher-dimensional
positive-definite source space into a lower-dimensional target. The discarded
subspace has nonnegative stored energy that must go somewhere.

**Policy:** contraction requires `Radiate`, `Dissipate`, or `Reject`. Silent
deletion is ill-typed.

## N8. Exact unitary factor means free physical actuator

**Proposal:** because a digital factor preserves the acoustic state norm,
report zero total implementation work.

**Failure:** zero fixed-metric state work does not imply zero energy consumed by
a modulator, motor, converter, CPU, or active nonreciprocal element.

**Policy:** separate acoustic-state ledger from actuator overhead. Production
must measure or model the latter without injecting it into the acoustic-state
balance.

## N9. One-second numerical replay promotes adiabatic realization

**Proposal:** numerical projective errors around `10^-3` at one second are small,
so claim theorem-certified adiabaticity.

**Failure:** the conservative Jansen--Ruskai--Seiler bound for a 5% projector
budget gives sufficient durations around `22.36 s` for A and `15.94 s` for B.
The one-second theorem receipts fail even though the numerical replay is good.

**Policy:** report numerical and theorem evidence separately. Use
`CompiledGeometric` when exact finite-time path execution is desired without an
emergent adiabatic claim.

## N10. `C2=1` lower-bounds the A/B commutator

**Proposal:** infer that a nonzero second Chern number forces the selected
Wilson loops to remain strongly noncommuting.

**Failure:** the characteristic class constrains the global bundle, not the
value of any particular path-dependent Wilson matrix. Loops can be deformed
and their commutator can approach zero without changing `C2`.

**Policy:** certify bundle topology and A/B noncommutativity separately. The
coefficient-box gap proof is analytic; the wider-box commutator floor remains
an empirical certificate unless a path-specific bound is proved.

## N11. Waveform difference proves audibility

**Proposal:** nonzero waveform norm or visibly different spectrograms prove a
listener can discriminate AB from BA.

**Failure:** deterministic distinguishability is not listener evidence. Level,
duration, spectrum, expectation, and trial structure can confound the result.

**Policy:** use the generated WAVs as deterministic fixtures. Stronger claims
require preregistered blinded listening experiments and matched controls.

## N12. Level matching inside the causal proof

**Proposal:** normalize AB and BA separately or use a limiter inside the
instrument path, then count the result as part of the protected effect.

**Failure:** independent gain can create or erase differences and does not
belong to the physical state-energy ledger.

**Policy:** the primary pair uses one shared media-encoding gain outside the
ledger. A separate constant-gain level-matched pair is clearly labeled as a
presentation control. Spectral matching is not claimed.

## N13. Downstream convolution makes room part of the instrument

**Proposal:** render the protected/string signal and convolve it with a room
impulse response.

**Failure:** no late-field state feeds back into the body or string; room
participation is downstream observation only.

**Policy:** the room region is state inside the conservative/open graph. A room
intervention must causally alter later instrument state.

## N14. Fresh body per note

**Proposal:** preserve the current voice model where each note constructs a new
body and protected state.

**Failure:** score and host segmentation can reset or counterfeit path memory;
AB and BA can cease to be operations on one material object.

**Policy:** one persistent body receives source/release events. Reset and
retirement are explicit operations with receipts.

## N15. Ordinary passive reciprocity realizes the pseudo-TR family automatically

**Proposal:** interpret the complex Wilson--Dirac matrix as an ordinary passive
real reciprocal acoustic network without identifying the physical resource
that implements complex couplings.

**Failure:** the selected synthetic antiunitary structure has `T^2=-1`, whereas
ordinary spinless bosonic reciprocity has the different real structure. The
complex I/Q couplings are not free notation.

**Policy:** require digital ordered scattering, I/Q modal couplers, active time
modulation, or another explicit implementation resource. Account for its
control/actuator boundary.

## N16. Finite nonlinear skeleton automatically inherits `C2`

**Proposal:** freeze authored parameters and apply ordinary linear bundle theory
to a state-dependent scattering/contact map.

**Failure:** the frozen update remains nonlinear in live state. A spectral
projector of one linear matrix is not an invariant of the nonlinear flow.

**Policy:** topology belongs to the unforced linear conservative skeleton or a
declared linearization. Nonlinear operation receives a bounded inheritance
receipt over an amplitude/time domain, not a new topological label.

## N17. Python fixture is production evidence

**Proposal:** load the Python-generated claim level and expose it directly in
the plugin.

**Failure:** shared constants are not independent replay, and a copied boolean
is not production measurement.

**Policy:** Rust recomputes or replays the named objects, emits its own residuals
and digest, and runs the claim evaluator from production evidence.

## N18. A formal toy theorem proves floating DSP

**Proposal:** use finite `Z3`, natural-number passivity, or graph reachability
lemmas as proof of real floating spectral and energy behavior.

**Failure:** the abstraction/refinement link is absent.

**Policy:** retain toy theorems as exact controls. Add real/complex inner-product
formalization, certificate interfaces, and Rust refinement obligations before
claim promotion.
