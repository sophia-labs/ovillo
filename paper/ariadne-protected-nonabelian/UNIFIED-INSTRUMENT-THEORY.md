# Unified instrument mathematics: completed reference construction

## Status and claim boundary

This document specifies and accompanies an executable reference construction for the strongest mathematically coherent version of the unified Ariadne instrument that can presently be defended.

The construction is a **finite-dimensional, coupled, acoustically interpreted modal instrument** with:

- sixteen positive-frequency complex modal amplitudes, or thirty-two real quadrature coordinates;
- four physical regions—played string, body, near field, and late room—with four internal channels per region;
- a globally isolated rank-two complex spectral bundle over a four-torus of cyclic controls;
- second Chern number `C2 = +1` in the declared orientation;
- two same-basepoint closed controls with strongly noncommuting rank-two Kato/Wilson holonomies;
- exact graph-restricted lowering into complex two-mode gates whose realifications are both orthogonal and symplectic;
- a persistent state, fixed string-source preparation, fixed room-port readout, passive radiation, causal room-to-string feedback, and a single state-energy ledger;
- a dense weighted-polar state-connection oracle for variable-rank retuning, plus a preferred fixed-capacity material-coordinate connection;
- explicit refusals for claims that the current evidence does not support.

The reference implementation can support a scoped claim of a **protected-acoustic reference oracle**. It is not yet a production claim. Promotion remains blocked on independent Rust/Weave replay, formal kernel checks, production actuator and floating-point receipts, and listener evidence. It is not fault tolerant.

The central architectural decision is that the protected bundle is not a sidecar. It is a spectral subbundle of the same conservative state that is partitioned into string, body, near-field, and room coordinates. Those regions exchange state energy bidirectionally. Excitation begins at a string port; the order-sensitive state is subsequently radiated through a room port. The renderer does not synthesize an unrelated oscillator bank from a Wilson-loop result.

## 1. Epistemic claim lattice

Every receipt carries both a claim level and an evidence class. These are intentionally separate.

The ordered claim vocabulary is:

1. `order-dependent`: two operations give different results when reversed;
2. `closed-control`: the operations are closed paths with the same physical endpoints;
3. `geometric`: the difference is a holonomy of a spectral projector connection;
4. `gapped-family`: one globally defined operator family has a uniformly isolated spectral cluster;
5. `topological-bundle`: that cluster has a nonzero characteristic class;
6. `adiabatic-realization`: a declared finite-time execution satisfies a rigorous error budget;
7. `protected-acoustic`: the protected state belongs to the acoustically active instrument and reaches a physical radiation observable with a closed ledger;
8. `fault-tolerant`: an error-correcting or fault-tolerant mechanism protects the operation itself.

Evidence classes are:

- theorem or exact algebra;
- validated numerics;
- reference-oracle execution;
- production measurement or cross-runtime replay;
- listener evidence.

The present reference promotes `order-dependent`, `closed-control`, `geometric`, `gapped-family`, `topological-bundle`, and a scoped oracle-level `protected-acoustic` claim. It refuses:

- a one-second theorem-certified adiabatic realization at a five-percent projector budget;
- production promotion;
- listener-discrimination claims;
- fault tolerance.

The executable claim decision lives in `unified_reference/claims.py` and is serialized into `unified_fixture_v2.json`.

## 2. Chosen mathematical category

### 2.1 Primary category

The primary conservative state is

```text
z(t) in C^16,
E(z) = z* z,
z_dot = -i H(lambda(t)) z,
H(lambda) = H(lambda)*.
```

The coordinates are positive-frequency, energy-normalized complex modal amplitudes. This is the most direct category for the existing Wilson–Dirac bundle, Kato transport, unitary compiler, and passive scattering ports.

A scalar carrier is included only when mapping the dimensionless family to physical oscillation frequencies:

```text
H_phys(lambda) = 2 pi [ f_c I + f_s K(lambda) ].
```

The reference values are `f_c = 440 Hz` and `f_s = 20 Hz`. Since the active Dirac radius lies in `[1,7]`, the lowest physical frequency is `440 - 7*20 = 300 Hz`; no positive-frequency mode crosses zero.

### 2.2 Equivalent real port-Hamiltonian form

Write `z = q + i p` and order the real coordinates as

```text
x = [q, p] in R^32.
```

For a complex matrix `A + iB`, define

```text
R(A+iB) = [[A, -B],
           [B,  A]].
```

The complex structure is

```text
J = [[0, -I],
     [I,  0]].
```

The real conservative generator is

```text
G(lambda) = R(-i H(lambda)).
```

For self-adjoint `H`, `G` is real skew-symmetric. Therefore

```text
G^T + G = 0,
(d/dt) (x^T x) = 0.
```

This is also a constant-metric port-Hamiltonian representation. A complex unitary `U` realifies to a matrix satisfying

```text
R(U)^T R(U) = I,
R(U)^T J R(U) = J,
R(U) J = J R(U).
```

Thus the lawful conservative control group is

```text
U(16) ~= O(32) intersect Sp(32,R) intersect Comm(J),
```

not arbitrary `SO(32)`.

### 2.3 Why arbitrary real rotations are refused

An ordinary Givens rotation acting on two isolated real coordinates can preserve Euclidean norm while destroying the oscillator complex structure. Such a factor can mix a `q` coordinate without applying the conjugate transformation to its paired `p` coordinate. It is then not a lawful passive positive-frequency mode transformation.

This invalidates the earlier inference

```text
orthogonal real factor => physically lawful acoustic mode factor.
```

The replacement compiler works directly in `U(n)` and only then realifies. Every intermediate factor is certified orthogonal-symplectic.

## 3. State space and physical regions

The state is partitioned region-major:

```text
z = [z_string,
     z_body,
     z_near,
     z_room],

z_region in C^4.
```

The four internal channels form the minimal Clifford module needed by the five-component Wilson–Dirac family. The region labels are not downstream metadata: the frozen generator has nonzero off-diagonal blocks between regions, the protected projector has nonzero support in every region, and frozen evolution carries interventions in either direction.

At full coupling, the normalized protected participation is exactly

```text
(string, body, near, room) = (1/4, 1/4, 1/4, 1/4).
```

At the decoupled endpoint it is

```text
(1, 0, 0, 0).
```

The coupling interpolation is exact and isospectral, so it supplies the required decoupled-to-coupled homotopy without a sampled-gap argument.

## 4. Clifford representation and active family

Choose Hermitian `4 x 4` matrices satisfying

```text
Gamma_a Gamma_b + Gamma_b Gamma_a = 2 delta_ab I.
```

The implementation uses

```text
Gamma_1 = sigma_x tensor sigma_x
Gamma_2 = sigma_x tensor sigma_y
Gamma_3 = sigma_x tensor sigma_z
Gamma_4 = sigma_y tensor I
Gamma_5 = sigma_z tensor I.
```

For `k = (k1,k2,k3,k4) in T^4`, define

```text
d_i(k) = sin(k_i),               i = 1,...,4
d_5(k) = m + sum_i cos(k_i),
H_D(k) = sum_a d_a(k) Gamma_a.
```

At `m = -3`, the spectrum is

```text
spec H_D(k) = {-r(k), -r(k), +r(k), +r(k)},
r(k) = ||d(k)||_2.
```

The occupied projector is gauge free:

```text
P_D(k) = 1/2 [I - H_D(k)/r(k)].
```

## 5. Stabilization and physical regional coupling

### 5.1 Stabilized supermode family

The sixteen-mode pre-coupling family is

```text
K_0(k) = diag(H_D(k), 9 I_4, 11 I_4, 13 I_4).
```

The three trivial blocks are deliberately above the active upper edge. They supply stable room/body/string modal capacity without altering the occupied rank-two topology.

### 5.2 Regional butterfly

Let `B_s in U(4)` be the product of four real beam-splitter rotations, each with angle `s*pi/4`:

```text
(0,1), (2,3), (0,2), (1,3),
s in [0,1].
```

Lift it to the full state:

```text
V_s = B_s tensor I_4.
```

The coupled family is

```text
K_s(k) = V_s K_0(k) V_s*.
```

This construction has several exact consequences:

1. `K_s` is self-adjoint for every `s,k`.
2. `K_s` is isospectral to `K_0`.
3. The occupied projector is

   ```text
   P_s(k) = V_s [P_D(k) direct-sum 0 direct-sum 0 direct-sum 0] V_s*.
   ```

4. At `s=1`, all four physical regions carry equal protected energy participation.
5. Off-diagonal regional blocks are adjoint pairs, so coupling is bidirectional.
6. The gap and second Chern number survive the entire coupling homotopy exactly.

This is a constructive answer to the requirement that the protected modes belong to the full string–body–space state rather than weakly driving an unrelated renderer.

## 6. Exact global gap

At `m=-3`, let

```text
t_i = 1 - cos(k_i),  0 <= t_i <= 2.
```

Then

```text
sin^2(k_i) = 2 t_i - t_i^2,
-3 + sum_i cos(k_i) = 1 - sum_i t_i.
```

Expanding gives the exact identity

```text
r(k)^2
 = sum_i sin^2(k_i) + (-3 + sum_i cos(k_i))^2
 = 1 + 2 sum_{i<j} t_i t_j.
```

Every term in the sum is nonnegative, so

```text
r(k) >= 1.
```

The maximum occurs at a corner with all `t_i=2`, yielding `r<=7`. Therefore

```text
1 <= r(k) <= 7
```

globally, and the negative doublet is separated from the positive doublet by

```text
g(k) = 2r(k) >= 2.
```

The trivial bands at `9,11,13` never reduce this bound. Unitary regional conjugation does not change eigenvalues. The exact protected-cluster gap is therefore at least `2` dimensionless scale units for every `k in T^4` and every `s in [0,1]`.

In physical frequency units, the minimum negative-to-positive active separation is

```text
2 * f_s = 40 Hz.
```

This does not imply that an arbitrary external waveguide may be attached without closing the gap. Residual modes must obey the extension conditions in Section 17.

## 7. Second Chern class

### 7.1 Analytic invariant

For the lattice Wilson–Dirac family, the second Chern number is obtained from the signed corner masses:

```text
C2(m) = 1/2 sum_{r=0}^4 (-1)^r choose(4,r) sign(m+4-2r).
```

At `m=-3`,

```text
C2 = +1
```

under the declared orientation `(k1,k2,k3,k4)` and occupied negative-energy convention. The phase boundaries are `m=-4,-2,0,2,4`.

### 7.2 Gauge-free degree estimator

Normalize

```text
n(k) = d(k)/||d(k)|| : T^4 -> S^4.
```

Then

```text
C2 = degree(n)
   = 1/Vol(S^4) integral det[n, partial_1 n, ..., partial_4 n] d^4 k,
Vol(S^4)=8 pi^2/3.
```

The implementation differentiates `d` analytically and performs a periodic midpoint integration. The full fixture uses a `16^4` grid and obtains an estimate near one with an error around `2e-4`.

### 7.3 Independent non-Abelian lattice gauge estimator

A second implementation constructs occupied frames only to form polar-unitary link variables. For each lattice plaquette it computes a principal anti-Hermitian logarithm and evaluates

```text
Tr(F_01 F_23 - F_02 F_13 + F_03 F_12)/(4 pi^2).
```

The estimator converges toward one as the grid is refined. The fixture records:

- link singular-value floors;
- maximum plaquette eigenphase;
- principal-branch safety;
- integer rounding error.

A trivial `m=-5` control rounds to zero.

The analytic corner formula, gauge-free degree integral, and independent lattice-link estimator are intentionally redundant. No single numerical pipeline carries the topological claim.

### 7.4 Underlying real bundle

The physical state is real after quadrature expansion, but the family preserves a global complex structure. For the occupied rank-two complex bundle `E`, `c1(E)=0` and `c2(E)=1`. The underlying oriented real rank-four bundle satisfies, in the declared convention,

```text
p1(E_R) = c1(E)^2 - 2 c2(E) = -2.
```

Thus realification does not erase the topological obstruction; it changes the characteristic-class language. The implementation retains `J` explicitly rather than pretending that every real orthogonal representation carries the same complex bundle.

## 8. Symmetry and reciprocity boundary

The active Dirac block admits an effective antiunitary symmetry

```text
T = U_T complex-conjugation,
T^2 = -I,
U_T = I tensor sigma_y
```

lifted identically over the four physical regions. The implementation checks

```text
U_T K_s(k)^* U_T* = K_s(-k)
```

and `U_T U_T^* = -I` to floating-point tolerance.

This is a **pseudo-time-reversal** symmetry of synthetic internal channels. It is not ordinary spinless acoustic time reversal. A passive reciprocal network with only real couplings generally has `T^2=+1`; the present family requires complex I/Q couplings or an equivalent active modulation architecture. The reference therefore refuses to call the design a passive reciprocal mechanical metamaterial.

This boundary is physically important. It identifies the implementation resource that realizes the complex bundle instead of hiding it inside notation.

## 9. Admissible perturbation family

The analytic coefficient box allows:

```text
|delta m| <= 0.05
|delta v_i| <= 0.03            for four sine coefficients
|delta c_i| <= 0.015           for four cosine coefficients
||Delta_extra||_op <= 0.05     for an additional Hermitian perturbation.
```

Clifford anticommutation gives

```text
||sum_a delta d_a Gamma_a||_op = ||delta d||_2.
```

The coefficient uncertainty has the global bound

```text
rho_coeff
 = sqrt(sum_i delta v_i^2 + (delta m + sum_i |delta c_i|)^2)
 <= 0.1252996409.
```

Including the extra Hermitian perturbation gives

```text
rho_total <= 0.1752996409.
```

Weyl's inequality therefore yields

```text
g_perturbed >= 2 - 2 rho_total
             >= 1.6494007183.
```

The spectral cut at zero remains at least `1-rho_total > 0`; rank is preserved. The straight-line homotopy from the nominal family to every point in the box remains gapped, so `C2` is constant throughout the entire declared box.

A deterministic perturbation corpus separately evaluates Wilson-loop noncommutativity, path gaps, transition singular values, and numerical `C2` estimates. This empirical sweep is not used as the proof that `C2` survives; it tests the more fragile, non-quantized observable that the chosen A/B loops remain strongly noncommuting.

## 10. Closed controls and Kato transport

### 10.1 Control loops

Both loops begin and end at

```text
k_base = (-0.4, 0.0, 0.9, -0.7).
```

Loop A moves in the `(k3,k4)` plane with radii `(1.50,1.20)`. Loop B moves in the `(k1,k4)` plane with radii `(1.30,1.00)`.

The phase is a quintic eased cycle:

```text
u(s) = 10s^3 - 15s^4 + 6s^5,
theta(s) = 2 pi u(s),
```

so the velocity and acceleration vanish at both endpoints. The control coordinates are exactly periodic and the physical coupler settings return to the same values.

### 10.2 Direct-rotation transport

For adjacent projectors `P0,P1`, define

```text
X = P1 P0 + (I-P1)(I-P0).
```

When adjacent subspaces are sufficiently close, `X` is invertible. Its unitary polar factor is the direct rotation that intertwines the projectors:

```text
U P0 U* = P1.
```

Products of these direct rotations converge to Kato parallel transport. The fixture records:

- minimum spectral gap along each path;
- minimum transition singular value;
- maximum adjacent projector distance;
- projector-intertwining error;
- endpoint closure;
- full and occupied unitarity residuals;
- coarse/fine convergence;
- reversal and concatenation laws.

### 10.3 Non-Abelian receipt

Restrict the full loop transports to the occupied frame at the basepoint:

```text
W_A, W_B in U(2).
```

The full-resolution fixture obtains occupied eigenphases near

```text
A: +/- 0.922
B: +/- 0.860
```

and

```text
||W_A W_B - W_B W_A||_F ~= 1.688,
||W_A W_B - W_B W_A||_op ~= 1.193.
```

Gauge covariance is checked by an independent random change of occupied frame. Eigenphases, trace, determinant, and commutator norm remain invariant; the loop matrices transform by conjugation.

Because the regional coupling `V_s` is constant over the control torus, changing `s` conjugates the full projector family by a fixed unitary. The occupied Wilson operators are therefore basis-conjugate and their commutator norm is exactly constant over the entire decoupled-to-coupled homotopy.

The Wilson matrices themselves vary continuously under general loop or family deformations. Their noncommutativity is geometric, not an integer invariant. The topological claim concerns the bundle, while the A/B order defect is a robustly measured observable within a declared neighborhood.

## 11. Finite-time adiabatic execution

### 11.1 Numerical dynamics

The active block is evolved by exact midpoint exponentials at a declared sample rate:

```text
i d psi/dt = 2 pi f_s H_D(k(t)) psi.
```

The carrier scalar is omitted because it contributes only a common phase. Each step uses the exact Clifford exponential

```text
exp(-i H_D dt)
 = cos(r dt) I - i sin(r dt) H_D/r.
```

The replay records:

- projective gate error after removing one common phase;
- leakage outside the occupied subspace;
- retained singular-value floor;
- active unitarity residual;
- host-block partition invariance.

At one second and `12 kHz`, the numerical replay errors are around `1e-3` and leakages around `1e-5` for the reference loops.

### 11.2 Rigorous duration refusal

The implementation also evaluates the explicit finite-dimensional bound in Jansen–Ruskai–Seiler, Theorem 3. For a rank-`m` spectral band, the conservative bound has endpoint terms proportional to

```text
m ||H'|| / g^2
```

and integral terms proportional to

```text
m ||H''|| / g^2
+ 7 m sqrt(m) ||H'||^2 / g^3.
```

For the declared quintic paths and a five-percent projector-error budget, the sufficient durations are approximately

```text
A: 22.36 s
B: 15.94 s.
```

The one-second theorem bounds are greater than the requested budget. The fixture therefore marks one-second adiabatic theorem acceptance `false`, even though direct numerical evolution is much better than the bound.

This distinction is mandatory:

```text
small observed error in one numerical solver
!=
accepted rigorous finite-time error budget.
```

A production compiler may offer two modes:

1. **compiled geometric mode**, which executes the certified sparse unitary word exactly up to replay error;
2. **physical adiabatic mode**, which must satisfy a duration/gap/error receipt before promotion.

## 12. Exact graph-restricted complex compiler

### 12.1 Gate set

The compiler emits:

- graph-edge `U(2)` two-mode blocks;
- local `U(1)` phase shifts.

For entries `a,b`, the elimination block is

```text
G(a,b) = [[conj(a)/r, conj(b)/r],
          [-b/r,       a/r]],
r = sqrt(|a|^2+|b|^2).
```

It maps `[a,b]^T` to `[r,0]^T` and lies in `SU(2)`.

### 12.2 Tree elimination theorem

On any connected graph with `n` complex modes, choose a spanning tree. A leaf-elimination QR procedure reduces an arbitrary target `U in U(n)` to a diagonal phase matrix using exactly

```text
n(n-1)/2
```

two-mode factors on tree edges. Inverting the eliminations and prepending the local phases reconstructs the target.

For `n=16`, each full loop, preparation, or readout unitary requires:

```text
120 two-mode gates + 16 local phases.
```

The reference records:

- reconstruction error;
- target and replay unitarity;
- real orthogonality;
- real symplecticity;
- maximum per-factor law residual;
- proof that every two-mode factor belongs to an authored graph edge.

The factors are not asserted to be literal zero-work mechanical couplers. They are lawful energy-preserving transformations in the declared digital/IQ modal category. Physical actuator overhead remains an external production measurement.

## 13. Persistent protected excitation and room readout

### 13.1 Raw observability

Let `B_string` inject four string-region channels and `C_room` observe four late-room channels. Define the protected order-defect operator

```text
D_P = P0 (W_B W_A - W_A W_B) P0.
```

The raw physical transfer is

```text
T_raw = C_room D_P B_string.
```

Its singular spectrum exposes both observable and blind string directions. At full coupling, a string-local vector can contain at most one quarter protected energy; the best raw input achieves that bound. Two raw singular values are substantial and two are numerically zero, so the receipt explicitly characterizes the blind subspace.

### 13.2 Fixed calibrated experiment

To make causal attribution unambiguous, the reference fixture defines one order-independent preparation/readout pair.

1. A source pulse is injected into `string/channel-0`.
2. A fixed unitary `U_pre` maps that source vector into the top right singular vector of the gauge-free protected operator `D_P`.
3. The persistent state executes either A then B or B then A.
4. A fixed unitary `U_post` maps the normalized protected order difference to `late-room/channel-0`.
5. A two-port passive room junction radiates channels 0 and 1.

The physical transfer is

```text
C_room U_post (W_B W_A - W_A W_B) U_pre B_string.
```

`U_pre` and `U_post` are identical in both trials and are independently compiled into the same lawful graph gate set. They therefore cannot encode the order label. Their purpose is analogous to calibrated launch and pickup optics around a non-Abelian interferometer.

The fixture verifies:

- the prepared state lies in the protected subspace;
- the calibrated difference lies in the declared room port;
- the calibrated gain equals the protected commutator operator norm;
- preparation and readout are unitary;
- their sparse compiler replays reconstruct them;
- resetting between A and B destroys the persistent composition result.

### 13.3 Persistence semantics

A gesture boundary is not a state reset. AB means

```text
state <- W_A state
state <- W_B state,
```

so the final state is `W_B W_A state`. BA gives `W_A W_B state`.

The reference compares:

- segmented execution with a persistent instrument object;
- concatenated replay of the same step list;
- direct matrix composition;
- an intentionally invalid reset-between-loops control.

The first three agree to floating-point tolerance. The reset control differs substantially.

## 14. Passive radiation and one energy ledger

### 14.1 Scattering port

Let the rows of `C` be orthonormal radiation directions and let `eta in [0,1]` be the per-step energy coupling. Define

```text
P_C = C* C,
R = I + (sqrt(1-eta)-1) P_C,
S = sqrt(eta) C.
```

Then

```text
R*R + S*S = I.
```

For incoming state `z`,

```text
z_retained = R z,
y_out = S z,
||z||^2 = ||z_retained||^2 + ||y_out||^2.
```

The audible waveform is the real part of the actual outgoing port amplitude. There is no separate oscillator bank.

### 14.2 Ledger equation

Every operation uses the convention

```text
E_final
 = E_initial
 + W_source
 + W_control
 + W_metric
 - D_internal
 - E_radiated
 - E_retired,radiated
 - E_retired,dissipated
 + epsilon_numerical.
```

The implementation records `epsilon_numerical` explicitly and computes a closure error only after including it. Observer taps do not appear as energy sinks. Media encoding gain is not part of the acoustic ledger.

For the current unitary factors,

```text
W_control = 0
```

in the normalized acoustic state-energy ledger. This does not imply a physically free actuator. Control electronics, pump power, or mechanical modulation work must be separately measured and is serialized as `actuator_overhead_external` in production.

### 14.3 Rank change

State coordinates may never disappear silently. If rank contracts, removed energy must be assigned to one of:

- explicit radiation;
- explicit dissipation;
- rejection of the operation.

The default policy is rejection.

## 15. Audible stimuli and causal attribution

The primary AB/BA files use one shared PCM encoding gain. This establishes that the difference is not generated by per-file normalization. The fixture also emits a second pair with constant per-file level-matching gains as a presentation-only control; those gains are excluded from the instrument and ledger.

The receipt reports:

- final persistent-state distance;
- relative waveform RMS distance under shared encoding;
- distance after removing global RMS scale;
- best scalar-fit residual fraction;
- normalized spectral-magnitude distance;
- channel RMS and spectral centroids;
- whole-instrument ledger closure.

The best scalar fit leaves nearly all of the AB/BA difference unexplained, so the signal is not reducible to one master-gain coefficient.

However, the current reference does **not** claim full spectral matching or listener discrimination. A perceptual study must preregister duration, level matching, spectral controls, trial count, null pairs, and decision criteria. The mathematically protected invariant and the audible observable remain distinct claims.

## 16. State connection under retuning

### 16.1 Preferred protected-backbone rule

The protected sixteen-mode backbone has constant dimension, constant metric, and a fixed material-coordinate basis. Retuning changes the generator and propagation law; it does not resize or reinterpret the state fiber. Its connection is therefore the identity in material coordinates.

For delay-like components used elsewhere in the instrument, the preferred architecture is a fixed-capacity ring or modal bank:

```text
capacity fixed,
state coordinates persistent,
active fractional delay changes,
read/propagation interpolation changes,
connection = identity.
```

This avoids fictitious energy creation caused by resampling state merely because a delay parameter changed.

### 16.2 Variable-rank dense oracle

When a discretization genuinely changes dimension, define:

- source metric `M0 > 0`;
- target metric `M1 > 0`;
- raw state correspondence `A`.

Move to energy-normalized coordinates:

```text
B = M1^(1/2) A M0^(-1/2).
```

Let

```text
B = U Sigma V*.
```

The closest partial isometry in Frobenius norm is

```text
Q = U V*.
```

Return to physical coordinates:

```text
T = M1^(-1/2) Q M0^(1/2).
```

If the target dimension is at least the source dimension, `Q*Q=I` and

```text
T* M1 T = M0;
```

all source-state energy is preserved.

If the target dimension is smaller, no injective energy-preserving linear map exists. `Q Q*=I` but `Q*Q` is a projection. The source component in its kernel is retired and must be radiated or dissipated.

The reference includes periodic linear interpolation as the raw correspondence and records the polar correction, singular values, metric conditioning, isometry/coisometry residuals, and rank semantics.

### 16.3 Sparse approximation boundary

A deterministic four-nonzero-per-row truncation is included only as a baseline. The fixture records its operator error, sampled state error, and sampled energy error. It is not promoted as a production connection. A production sparse method must satisfy declared tolerances against the dense oracle and preserve the retirement policy.

## 17. Raw delay spectral-crowding obstruction

A lossless periodic delay of length `L` samples has eigenvalues at all `L`th roots of unity. Its quasienergy spacing is

```text
Delta theta = 2 pi/L,
```

and its modal spacing in hertz is

```text
Delta f = sample_rate/L.
```

At `48 kHz`, a one-second delay has one-hertz spacing. As the delay grows, a fixed rank-two Floquet window cannot retain a uniform gap unless additional structure is imposed.

Therefore the following claim is refused:

```text
"the current raw long-delay Weave graph automatically inherits the protected
rank-two gap from the sixteen-mode reference core."
```

It does not.

There are three mathematically legitimate production routes:

1. **Protected modal backbone.** The sixteen-mode family is authoritative. Strings/body/room are represented by deliberately selected modes; residual delays are passive ports or effects outside the invariant claim.
2. **Engineered band gaps.** All added conservative modes are placed outside a rigorously declared protected spectral window, with coupling small enough for a Weyl/Schur-complement bound.
3. **Reduced conservative dilation.** A finite passive bath is included in the full operator and certified at the chosen resolution; the claim is resolution-specific and must fail safely as the bath densifies.

The current recommendation is route 1, with route 2 for future richer instruments. A raw generic delay network is not an acceptable strong-route proof object.

## 18. Extension by additional acoustic modes

Suppose the certified core `K_core(k)` is extended by a residual conservative block `R` and coupling `G`:

```text
K_full(k) = [[K_core(k), G],
             [G*,          R]].
```

Let the protected cluster have distance `g_core` from the rest of `K_core`, and let every eigenvalue of `R` remain at distance at least `g_residual` from the cluster window. A conservative sufficient condition is

```text
2 ||G|| < min(g_core, g_residual).
```

Then Weyl-type bounds preserve a nonzero isolated window with lower bound at least

```text
min(g_core, g_residual) - 2||G||.
```

Production must compute the actual cut geometry rather than applying this schematic bound blindly. The purpose of the condition is architectural: every added mode needs a declared spectral address and coupling norm. An unlabeled dense room spectrum cannot be appended after the proof.

## 19. Nonlinear stages

The topological bundle belongs to the linear conservative family. Nonlinear waveshaping, saturation, collision, and contact models do not automatically inherit `C2`.

For a bounded nonlinear perturbation

```text
z_dot = G(t) z + N(z,t),
```

assume a Jacobian bound

```text
||D_z N|| <= L
```

and forcing bound

```text
||N(0,t)|| <= b.
```

A linearized cluster with gap `g` is protected by a conservative Weyl condition

```text
2L < g.
```

A Gronwall tube around the linear trajectory is

```text
||delta z(t)|| <= b (exp(Lt)-1)/L
```

with the `L=0` limit `bt`.

The executable nonlinear receipt accepts or refuses a declared envelope. It never labels the nonlinear flow itself as carrying the second Chern class.

## 20. Determinism and replay

`run_unified_experiments.py` generates:

- `unified_fixture_v2.json`;
- `unified_results.json`;
- `unified_operator_tables.npz`;
- `unified_compiled_words.json`;
- primary and level-matched AB/BA WAV files;
- `unified_summary.md`;
- `unified_manifest.json` with SHA-256 hashes.

The fixture includes:

- source commit;
- family digest;
- path/projector digests;
- preparation and readout digests;
- basis, metric, coordinate order, and symmetry boundary;
- topological, gap, and perturbation receipts;
- path and holonomy receipts;
- compiler graph and law;
- persistent-state and causal-feedback receipts;
- finite-time theorem and numerical receipts;
- connection semantics;
- ledger convention;
- explicit acceptance booleans.

The Python fixture is not itself cross-runtime evidence. Production promotion requires Rust to replay the same operator tables and control words independently, compare every named matrix/state/ledger field against tolerance, and emit a new production digest.

## 21. Acceptance matrix

| Requirement | Reference status | Production status |
|---|---:|---:|
| One coupled conservative family | complete | not integrated |
| Rank-two bundle over `T4` | complete | not replayed |
| Exact global gap | complete | not replayed |
| `C2=1` analytic proof | complete | formal kernel check pending |
| Independent lattice-gauge estimator | complete | Rust implementation pending |
| Same-endpoint A/B paths | complete | production schedule pending |
| Gauge-covariant Kato transport | complete | Rust replay pending |
| Strong noncommutativity | complete | production measurement pending |
| Protected support in all regions | complete | production modal mapping pending |
| Bidirectional room feedback | complete in reduced model | production measurement pending |
| Persistent AB/BA state | complete | engine integration pending |
| Lawful graph lowering | complete in Python | Rust execution pending |
| One state-energy ledger | complete in Python | host ledger pending |
| Fixed-capacity connection | specified and tested | implementation pending |
| Variable-rank dense oracle | complete | sparse production method pending |
| One-second rigorous adiabatic budget | refused | refused |
| Audible files from protected state | complete | listening study pending |
| Spectrally matched stimuli | not claimed | pending |
| Raw long-delay Weave protected gap | refused | redesign required |
| Fault tolerance | not claimed | absent |

## 22. What is genuinely new here

The individual ingredients—Wilson–Dirac second-Chern models, Kato/Wilczek–Zee transport, non-Abelian Wilson loops, unitary interferometer decompositions, passive scattering, and weighted polar maps—have substantial prior art.

The candidate Ariadne contribution is the exact composition of these ingredients into one instrument contract:

- the topological bundle belongs to a state partitioned into audible physical regions;
- the decoupled-to-coupled homotopy is exact and isospectral;
- the graph compiler preserves the oscillator complex structure at every intermediate factor;
- excitation, persistent transport, room readout, radiation, and retirement share one ledger;
- state retuning has an explicit connection rather than implicit array resizing;
- raw-delay spectral crowding is treated as an architectural obstruction, not hidden by sampling;
- the claim lattice prevents a numerical, production, or perceptual result from being promoted into a stronger category.

That integration is the strongest result of this packet. The work should be presented as a constructive reference architecture and proof-carrying compiler target, not as an already fabricated acoustic topological material.

## References used by the reference construction

- S. Jansen, M.-B. Ruskai, and R. Seiler, “Bounds for the adiabatic approximation with applications to quantum computation,” *Journal of Mathematical Physics* 48, 102111 (2007), arXiv:quant-ph/0603175.
- M. Mochol-Grzelak et al., “Efficient algorithm to compute the second Chern number in four dimensional systems,” arXiv:1803.07003.
- W. R. Clements et al., “Optimal design for universal multiport interferometers,” *Optica* 3, 1460–1465 (2016), arXiv:1603.08788.
- Y. Yang et al., “Observation of Non-Abelian Thouless Pump,” arXiv:2110.08789 and associated experimental publication.
- T. Kato, foundational work on adiabatic parallel transport and perturbation theory of linear operators.
- F. Wilczek and A. Zee, non-Abelian geometric phase for degenerate eigenspaces.
