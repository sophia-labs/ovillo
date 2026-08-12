# Neurons Have Extension
## Gestural sequence computation in Clifford string media

*Provisional paper draft. Subtitle for talks and internal circulation: “Attention Is All You Need, But It Is Nice to Have Extension.”*

## Abstract

Modern sequence models represent tokens and neurons as point-like vectors transformed by repeated algebraic layers. We investigate a complementary design in which computational units possess spatial extension and sequence elements act as gestures through a learned latent medium. We introduce a provisional model whose persistent state is carried by one-dimensional bodies embedded in a low-dimensional geometry. Its exact reference core is a Clifford path development: four latent control directions act on a 16-dimensional multivector state through skew-orthogonal generators, so sequence computation is a path-ordered, energy-preserving evolution rather than a stack of instantaneous layers. The exterior grades encode directed, oriented-area, oriented-volume, and four-volume components of gesture history. We prove that the top-grade coefficient after four vector gestures is their determinant, and show that closed control loops with identical endpoints leave nonzero, invertible residues proportional at leading order to enclosed area. We lower the exact generators into sparse signed plane rotations between sites on five strings, evaluate finite-width geometric crosstalk across ambient dimensions, and inverse-design string morphology toward the target algebra. Finally, we train exactly closed token loops end to end on a controlled permutation-parity task. A one-fiber model with 130 parameters solves all 24 permutations in five of five seeds while preserving state norm and closing every token path to numerical precision; replacing the noncommuting generators by commuting ones collapses every token operator to identity and accuracy to chance. A naive active-cable model fails on the same task, motivating a staged program in which exact gestural operators supervise a work-accounted nonlinear medium. These results establish a concrete architecture and experimental program, not a competitive language model. The central open question is whether extended, locally realizable path memory supplies useful inductive bias for long-context algorithmic and linguistic computation.

## 1. Introduction

The Transformer begins from an extraordinarily effective abstraction: every token is a vector, every computational site is a point, and relations among points are recomputed algebraically. Depth is represented by repeated layers. Time inside the model is primarily the order in which layer maps are applied. Even recurrent and state-space models usually retain the same point ontology: a finite vector is updated by a map.

That ontology is not mandatory. A physical signal-processing system stores information in extended bodies. A string does not possess one activation; it possesses a field. Its internal state propagates with finite speed, interferes, resonates, decays, and meets other bodies at localized boundaries. A gesture applied to the system is not merely an input vector. Its path, timing, and order determine what state remains.

We ask whether this distinction can be made computationally substantive:

> What changes when an artificial neuron is an extended excitable medium rather than a point, and when a symbol is a gesture rather than an embedding?

The long-term architecture we envision contains one-dimensional neural bodies embedded in a learned four-dimensional latent geometry. Fields travel along bodies; proximity and declared contacts determine communication; input gestures excite and reconfigure the medium; readout is itself a path through the field. Morphology and material parameters are trainable but constrained by explicit locality and energy/work laws.

This paper does not claim to realize that full architecture. It identifies and executes its smallest mathematically coherent nucleus.

The key observation is that a sequence of gestures should not be summarized only by its endpoint. The natural object is a path development: a path drives a noncommuting family of state transformations, and the residual state records ordered geometric information. Path signatures and controlled differential equations already provide a mature account of this idea. Trainable finite-dimensional path-development layers already exist. We therefore do not claim novelty for path development itself.

Our specific construction chooses a Clifford representation whose state is the exterior algebra of a four-dimensional latent space. This choice gives four properties needed by the extended-neuron program:

1. **Path order is explicit.** Distinct latent directions anticommute.
2. **The state has geometric grades.** It separates vector, area, volume, and four-volume components.
3. **Dynamics can be energy preserving.** Unit-direction generators are skew-orthogonal.
4. **The dense reference operator has a sparse local decomposition.** Each basis direction lowers to eight disjoint signed plane rotations among 16 blade modes.

This yields a five-string realization with grade sizes `1,4,6,4,1`. The strings are not decorative. Their learned embedding controls whether desired local contacts are selective or contaminated by crosstalk.

We make the following provisional contributions:

- an exact 16-state Clifford gesture medium in which top-grade state equals oriented four-volume;
- a closed-loop controlled development with same-endpoint path memory, exact inversion, and segment-reparameterization invariance;
- a sparse local-event lowering onto five extended strings;
- an ambient-dimension/contact-width study of geometric algebra fidelity;
- inverse design of string morphology toward a known operator algebra;
- a trainable closed-token model solving complete four-token permutation parity with exact loop closure and norm preservation;
- a commuting control that analytically destroys all information carried by the closed loops;
- a retained negative result showing that a naive active-cable implementation does not learn the same transition directly.

The experiments are intentionally small. Their role is to turn a philosophical proposal into an executable scientific object with positive mechanisms, negative controls, and named failure modes.

## 2. From token points to control gestures

Let the ambient latent space be `V = R^d`, initially with `d=4`. An input is a bounded-variation path

```text
g : [0,T] -> V.
```

For discrete data, a token may select a learned piecewise-linear loop or spline. A sequence concatenates these paths. Let the medium's operator state evolve as

```text
dU_t = U_t M(dg_t),     U_0 = I,
```

where `M : V -> g` maps latent directions into a matrix Lie algebra. For a piecewise-linear path with increments `delta_1,...,delta_K`,

```text
U[g] = product_k exp(M(delta_k)).
```

This is a finite-dimensional path development. It obeys

```text
U[g*h] = U[g]U[h]
```

and is invariant to nondecreasing reparameterization of each geometric segment. Noncommutation is the source of order sensitivity.

A token can be constrained to a closed loop:

```text
sum_k delta_{x,k} = 0.
```

If all generators commute, the token operator becomes identity. If they do not, a closed token can leave nonzero internal state. This creates an unusually clean experimental design: endpoint and bag-of-token information can be held exactly constant while path order remains computationally available.

## 3. Clifford path development

### 3.1 State space

Let `Lambda* V` be the exterior algebra. For `d=4`, it has 16 basis blades indexed by bitmasks:

```text
1,
e_1,...,e_4,
e_12,...,e_34,
e_123,...,e_234,
e_1234.
```

We implement right Clifford multiplication with negative-definite signature. For basis generators `D_i`,

```text
D_i D_j + D_j D_i = -2 delta_ij I.
```

Every `D_i` is real skew-symmetric and orthogonal:

```text
D_i^T = -D_i,
D_i^2 = -I,
D_i^T D_i = I.
```

Thus `exp(theta D_i)` is a norm-preserving rotation. A linear combination `D(v)=sum_i v_iD_i` obeys

```text
D(v)^2 = -||v||^2 I.
```

### 3.2 Determinant observable

Start from scalar vacuum `z_0=1`. Apply four vectors through Clifford multiplication:

```text
z_4 = v_1 v_2 v_3 v_4.
```

The grade-four component is

```text
pi_4(z_4) = v_1 wedge v_2 wedge v_3 wedge v_4
          = det([v_1;v_2;v_3;v_4]) e_1234.
```

Therefore the sign of the pseudoscalar is the orientation of the ordered gesture frame. For basis-token permutations, it is permutation parity.

The full Clifford product contains contractions as well as exterior products; it is not the same as a truncated tensor signature. It is a structured quotient that retains selected symmetric and antisymmetric interactions in a fixed 16-dimensional state.

### 3.3 Closed loops and area

For two axes, define a rectangular loop

```text
C(a,b)=exp(aD_1)exp(bD_2)exp(-aD_1)exp(-bD_2).
```

The controls return to zero. Because `D_1,D_2` anticommute,

```text
[D_1,D_2]=2D_1D_2 != 0.
```

For small `a,b`,

```text
C(a,b)=I+ab[D_1,D_2]+higher order.
```

Applied to scalar vacuum, the leading residue lies in the `e_12` bivector coordinate. The medium therefore contains an explicit second-order area memory.

## 4. Local string lowering

### 4.1 Exact sparse schedule

For each axis `j`, every blade `S` not containing `j` pairs with `S union {j}`. Clifford multiplication acts as a signed quarter-turn in each pair. There are eight pairs in dimension four, and the pairs are disjoint.

The state modes are grouped into five strings by grade. A basis gesture activates contacts between neighboring grades:

```text
grade k <-> grade k+1.
```

This is a sparse, graph-local implementation of the exact operator. The dense 16-state matrices are retained only as a reference oracle.

### 4.2 Soft local modes

We assign every blade mode an ambient coordinate. A contact event between sites `a,b` uses Gaussian lobes around the two coordinates. The lobes are orthonormalized, and a signed plane rotation acts in their span. Each local event is therefore exactly norm preserving.

Crosstalk appears because a lobe has support on unintended sites. Multiple events then produce a generator that differs from the exact one and no longer satisfies exact anticommutation.

This distinction is useful:

- numerical instability would change norm during a local event;
- geometric crosstalk preserves event norm but changes the realized algebra.

The latter is the actual research problem.

### 4.3 Trainable morphology

Five strings are parameterized as piecewise-linear curves. Their segment directions are trainable. With the target pairing schedule fixed, we minimize generator error plus curvature and isotropy regularization.

This is an inverse-instrument problem analogous to GARGANTA/Threader: infer or design a body whose local dynamics realize a desired family of effects. Here the target is computational rather than acoustic.

## 5. Learned closed token programs

For each token `x`, we learn `K-1` increments. The `K`th is fixed by closure. Each fiber may learn an orthogonal latent frame. The token operator is the product of segment exponentials. Sequence state starts from scalar vacuum and recurrently applies token operators. A single linear pickup classifies the final multivector.

The model contains no attention, hidden MLP, or unconstrained recurrent matrix. Its trainable computation is concentrated in gesture shape, latent frame, and final pickup.

This model is not yet physically embedded on the five strings. It is the operator-level compiler target. The geometry and morphology experiments test a separate lowering of the same algebra.

## 6. Active-cable extension

The intended nonlinear model replaces each blade site with a field compartment. The first attempt used:

- four piecewise-linear strings in `R^4`;
- eight sites per string;
- four active field channels;
- finite-difference material diffusion;
- geometry-derived sparse contacts;
- a shared low-rank multiplicative reaction law;
- fast activation, recovery, and slower memory state;
- closed Bézier input gestures;
- a closed pickup path and linear decoder.

It was trained with final parity loss and temporary prefix supervision. It remained at chance.

The failure suggests that task-level gradient descent did not discover the required local operator program through the active solver. We therefore propose an exact-backbone curriculum:

1. initialize or distill local dynamics from the exact Clifford program;
2. fit morphology while the target algebra is fixed;
3. unlock bounded active residual dynamics;
4. preserve algebraic probe losses while introducing algorithmic tasks;
5. only then train language objectives.

## 7. Experiments

### 7.1 Exact algebra

The exact reference generators satisfy Clifford, orthogonality, and determinant identities to approximately `1e-16`. All 24 basis permutations are classified exactly. Across 10,000 random ordered four-frames, pseudoscalar amplitude matches determinant with maximum error `4.44e-16`.

### 7.2 Closed continuous paths

Two two-segment paths with equal endpoint and reversed order produce state distance `0.5604` at equal energy. A closed rectangle produces the same distance from vacuum and a nonzero bivector coefficient `0.3860`. Its inverse returns to vacuum within `4.46e-16`; segment refinement changes the operator within `1.25e-15`; small-loop bivector magnitude has fitted area exponent `0.99785`.

### 7.3 Learned closed tokens

A one-fiber model with 130 parameters learns complete permutation parity. Five independent seeds all reach 100%. Every token path closes within `5e-16`; state energy drifts less than `9e-15` across the sweep.

In the commuting control, every token operator is identity within `4.42e-15`, all pairwise sequence-state distances are below `1e-30`, and a linear probe achieves 50%. With Clifford generators, centered feature rank is 16 and the same probe achieves 100%.

### 7.4 Ambient geometry

At contact width `0.38`, random four-dimensional and eight-dimensional embeddings both achieve perfect basis parity and about 95.4% sign accuracy on random continuous four-volume. Three dimensions achieve about 90.9%; two dimensions about 84.3%. The near equality of four and eight supports sufficiency rather than unique optimality.

### 7.5 Morphology optimization

At width `0.50`, optimizing 44 string-direction parameters reduces target operator MSE from `0.04798` to `0.01004`, restores basis parity from 50% to 100%, and raises random continuous-volume sign accuracy from 51.6% to 80.8%.

### 7.6 Active negative control

The 546-parameter active-cable model remains at 50% for 250 updates despite loss reduction. We report the result because it changes the implementation program: exact operator supervision is not optional scaffolding but the next necessary experiment.

## 8. Related work and novelty boundary

### Path signatures and development

Path signatures encode streams through ordered iterated integrals. Neural CDEs use paths as controls for learned hidden dynamics. Path Development Networks already learn finite-dimensional Lie-group representations with recurrence

```text
z_{n+1}=z_n exp(M_theta delta x_n).
```

Our operator-level recurrence is a constrained instance of this family. The proposed novelty lies in the Clifford choice, local sparse lowering, extended-body morphology, and work-accounted active realization—not in finite-dimensional development alone.

### Clifford neural networks

Geometric Clifford Algebra Networks, Clifford group-equivariant networks, multivector neurons, and Clifford-steerable CNNs already demonstrate that multivectors and geometric products are useful neural representations. Most target equivariance and physical-field prediction inside conventional layer-based architectures. Our question is whether Clifford transport can become the recurrent physical memory of a sequence medium.

### Orthogonal and unitary recurrence

Unitary/orthogonal RNNs use norm-preserving maps to stabilize long-term gradients. Extension shares this benefit but imposes a local geometric decomposition and interprets inputs as paths rather than direct recurrent matrices.

### Physical and compartmental neural computation

Physical neural networks train material dynamics; dendritic and cable models use spatially distributed neuronal state. The active-medium proposal belongs to this lineage. The current Clifford operator is an abstract computational scaffold, not a biological neuron reconstruction.

## 9. Limitations

The strongest limitation is scale. `Cl(0,d)` has dimension `2^d`; latent dimension cannot be used as ordinary hidden width. Capacity must grow through many small fibers, local couplings, active state, or lower-dimensional spinor modules.

The parity task is complete but tiny. Since all examples are used for fitting, the result tests representation and optimization, not statistical generalization. A generic noncommuting path-development model may also solve it. Comparative experiments are required.

The soft geometric realization does not preserve exact Clifford structure for arbitrary vector combinations at broad contact width. Event-level energy preservation is not enough; the algebra must be constrained or reprojected.

The morphology experiment knows the target topology. It does not discover a neural body from task data.

The active cable failed. No present result supports competitive text modeling.

Four dimensions are sufficient and interpretable in the toy geometry, but not established as optimal.

The closed-loop residue is geometric. No topological protection is claimed.

## 10. Next experiments

The immediate next model should be a bank of exact Clifford fibers embedded in trainable strings, with active bounded residual laws. It should be trained on a ladder:

1. reproduce operator and closed-loop receipts;
2. solve parity and braid/area probes;
3. generalize to unseen path scales and lengths;
4. learn finite-state and stack-like algorithmic tasks;
5. perform in-context linear regression;
6. run character-level language modeling;
7. compare fixed recurrent-state scaling against Transformer, GRU, state-space, Neural CDE, and generic path-development baselines.

The crucial ablations are point collapse, commuting generators, state reset, arbitrary dense recurrent operators, fixed versus learned morphology, exact versus soft contacts, and two/three/four/eight ambient dimensions.

## 11. Conclusion

The phrase “neurons have extension” can now denote a concrete technical program rather than a metaphor. A gesture path drives a persistent multivector state; order appears as orientation and holonomy; exact operators lower to local contacts among one-dimensional bodies; morphology controls algebraic fidelity; and work accounting governs reconfiguration.

The present model is not a replacement for attention. It is a proof that a different computational ontology is executable:

> A sequence can be computed as a history of gestures through a body, and meaning can reside in the path-dependent state that body retains.
