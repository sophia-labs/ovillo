# Extension Generation 3 program

## Thesis

Extension should discover predictive constitutions; Ariadne should execute them
lawfully as persistent worlds.

A constitution is not merely a weight tensor. It names:

- persistent state coordinates;
- a positive storage metric;
- local transport and its order;
- sources and finite supply;
- passive loss;
- apertures and observers;
- damage and reconfiguration semantics;
- exact limits and refusal conditions.

The Generation 2 scaling campaign showed why this distinction matters. Task
fit, target-whitened span, spectral allocation, route assignment, and causal
rank can move independently. A learned body therefore cannot be credited by one
rank or loss scalar. Its constitution and its use must receive separate
receipts.

## The braided architecture

### Neural cellular automata

Inherited:

- one shared local law;
- persistent distributed state;
- damage and regeneration tests;
- global behavior from repeated local exchange.

Changed:

- hidden channels have declared storage and sign constraints;
- every source, internal transfer, and loss has a receipt;
- topology is content-addressed;
- an observer is explicitly non-loading.

### Neural fields and wave machines

Inherited:

- spatial recurrent state;
- propagating modes;
- phase- and order-sensitive computation.

Changed:

- weighted orthogonal transport is exact under the declared metric;
- ordered edge schedules are part of the constitution;
- path-order claims are tested by commuting and zero-sector controls.

### Predictive-state and projective systems

Inherited:

- positive evidence state;
- normalization and filtering as transport rather than generic hidden vectors;
- future-relevant tests as the correctness coordinate system.

Changed:

- positive transport is localized into conservative pair exchanges;
- quadratic contraction is booked as loss rather than hidden in a matrix step;
- predictive rank remains an Extension analysis object, not an Ariadne law.

### Dendritic and compartmental computation

Inherited:

- a unit has a persistent interior;
- compartments have different functional roles;
- local anatomy can carry teaching and predictive state.

Changed:

- the first substrate does not claim biological fidelity;
- compartments are sites in a declared constitutive graph;
- morphology changes must preserve or explicitly transport live state.

### Physical reservoirs and morphological computation

Inherited:

- the body performs temporal computation;
- observers should be weak;
- damage and embodiment are capability tests.

Changed:

- internal parameters are trainable;
- the observer is not allowed to hide an arbitrary nonlinear solver;
- state, work, and loss remain inspectable.

### Port-Hamiltonian and structure-preserving learning

Inherited:

- storage, interconnection, source, dissipation, and observation are distinct;
- internal exchange is not new source work;
- subsystems compose through declared ports.

Changed:

- the v1 medium has a mixed signed/positive computational storage metric rather
  than claiming literal canonical mechanics;
- the general port-Hamiltonian compiler remains future work.

### Neural CDEs, signatures, and Lie development

Inherited:

- input is an ordered path;
- noncommutative development can retain gesture history;
- path functionals are better probes than a final-token label alone.

Changed:

- path development is distributed through local state and edges;
- the compiler must eventually relate signature order to local commutator depth.

### Neural operators and mesh learning

Inherited:

- a law should be distinguishable from one discretization;
- refinement and domain growth are different axes;
- local and global operator error must be measured separately.

Changed:

- v1 makes no refinement claim;
- site creation, deletion, merge, and split refuse until a state-transfer IR is
  available.

## Three state sectors

The first common substrate carries, at site `i`,

```text
wave[i]       signed transport state
evidence[i]   non-negative occupancy/evidence state
reservoir[i]  finite stored supply
```

Its declared storage is

```text
H = 1/2 sum_i g_i wave_i^2
  + 1/2 sum_i evidence_i^2
  + sum_i reservoir_i.
```

The three sectors are not declared universal. They are the smallest useful
intersection of the literatures:

- signed state supports oscillation and noncommuting ordered transport;
- positive state supports localized filtering and accumulation;
- finite supply distinguishes active behavior from unbooked amplification.

## Runtime laws

### Signed transport

For edge `(i,j)`, scale coordinates by `sqrt(g)` and apply an ordered Givens
rotation. This preserves

```text
1/2 g_i wave_i^2 + 1/2 g_j wave_j^2
```

up to numerical residual.

### Positive transport

Apply a symmetric pairwise stochastic exchange

```text
m_i' = (1-a) m_i + a m_j
m_j' = a m_i + (1-a) m_j,
0 <= a <= 1/2.
```

Total mass and non-negativity are preserved. The decrease in the declared
quadratic evidence storage is booked as internal transport loss.

### Active transfer

A local thresholded law may transfer finite reservoir storage into signed wave
storage. The exact increase of wave storage is removed from the reservoir. A
zero body with no evidence or source remains exact zero.

### Reconfiguration

Changing a site's metric preserves the numerical live coordinate and books

```text
W_config = 1/2 (g_after - g_before) wave^2.
```

Shrinking a reservoir below its live state refuses. Identity-changing remeshing
is outside v1.

## Claim ladder

1. **Law validity** — exact limits, non-negativity, mass conservation, and ledger
   closure.
2. **Construction** — a constitution can implement a controlled capability.
3. **Learnability** — an optimizer can discover it from registered data.
4. **Causal anatomy** — sector, edge, aperture, and observer interventions have
   selective effects.
5. **Comparative utility** — it outperforms or enables a capability unavailable
   to matched architecture baselines.
6. **Refinement** — behavior and receipts converge under a declared mesh limit.
7. **World integration** — the constitution participates in canonical Ariadne
   `WorldPlan`, coupling, trajectory, observation, and global ledger semantics.
8. **Natural utility** — the controlled mechanism survives persistent natural
   sequences.

The current tranche reaches rungs 1–3 on one small construction. It supplies
some damage evidence for rung 4 but does not promote a general causal-anatomy
claim.
