# Extension experimental program

This is the execution plan for moving from the present operator probes to a credible sequence-model paper. Every stage has a positive target, negative controls, and a stop condition. Later stages should not begin by deleting failed earlier artifacts.

## Global protocol

Every run writes a canonical receipt containing:

- git commit and dirty-tree status;
- config and random seeds;
- dataset or generator digest;
- parameter count;
- optimizer/scheduler state;
- wall-clock and device information;
- train/validation/test metrics;
- energy, work, algebra, and geometry diagnostics;
- artifact hashes;
- explicit claim class: exact theorem replay, numerical identity, finite-task fit, statistical generalization, or perceptual result.

Use at least five seeds for architecture probes and ten seeds when comparing close methods. Predeclare primary metrics and aggregate seed failures rather than silently rerunning them.

## E0 — Reproduce the current foundation

### Commands

```bash
cd mus-rs/experiments/extension_v1
python -m venv .venv
. .venv/bin/activate
pip install -e '.[test]'
pytest

extension-v1 exact --trials 10000 --output artifacts/exact.json
extension-v1 development --output artifacts/development.json
extension-v1 geometry --ambient-dim 4 --kernel-width .38 \
  --trials 4096 --output artifacts/geometry-d4.json
extension-v1 train-closed --steps 600 --fibers 1 --seed 101 \
  --output-dir artifacts/closed-101 \
  --control-output artifacts/closed-control.json
```

### Gates

- all unit tests pass;
- Clifford residuals below `1e-12` in float64;
- random determinant residual below `1e-12`;
- closed-loop inverse and reparameterization residual below `1e-12`;
- one-fiber closed-token accuracy 100%;
- commuting control accuracy exactly 50%;
- loop closure below `1e-12`;
- state-energy drift below `1e-11`.

Failure means the foundation has changed and must be re-understood before scaling.

## E1 — Generalization of learned closed gestures

The current model fits all 24 permutations. E1 asks whether it learns reusable geometry rather than a finite lookup.

### E1.1 Continuous scale interpolation

Train token loops using rectangle/loop scales sampled from a finite set. Test unseen scales and smooth timing warps.

Tasks:

- sign of oriented area for two-token loops;
- sign and magnitude bins of oriented four-volume;
- classify loop orientation under random rotations of latent coordinates;
- regress conjugacy-invariant operator observables.

Primary metrics:

- sign accuracy on unseen scales;
- normalized determinant error;
- calibration near degenerate frames;
- energy drift;
- reparameterization residual.

Gate: at least 99% sign accuracy away from a preregistered degeneracy threshold and no loss under 2–8x segment refinement.

### E1.2 Vocabulary and length transfer

Generate sequences over 6–12 gesture types. Hold out:

- entire permutations;
- token subsets;
- longer concatenations;
- compositions of loops seen only separately.

Do not use a task whose label can be recovered from token counts. Include same-multiset matched pairs.

Gate: meaningful improvement over bag, endpoint, and Markov-order baselines on held-out compositions.

### E1.3 Representation comparison

Compare the Clifford fiber to:

1. generic `SO(16)` path development with matched parameter count;
2. generic skew generators with locality penalty;
3. truncated signatures of matched feature dimension;
4. Neural CDE with matched state dimension;
5. GRU/orthogonal RNN;
6. point-collapsed endpoint model.

This comparison is load-bearing. Parity solvability by itself is not Clifford-specific.

Report:

- accuracy and sample efficiency;
- gradient norms versus sequence length;
- state norm and condition number;
- compute per token;
- robustness to timing and coordinate rotations;
- feature-rank and probe complexity.

## E2 — Exact local compiler into `mus-gesture` / `mus-watsn`

Replace Python dense reference matrices with the existing sparse operator infrastructure.

### Required lowering

```text
Clifford axis j
  -> 8 signed blade-pair Givens rotations
  -> mus_gesture::OperatorProgram
  -> mus_watsn execution
  -> replay receipt
```

### Gates

- Python dense, Rust sparse, and live executor states agree below `1e-11` float64;
- every factor acts on an authored contact;
- determinants and energy receipts close;
- token programs are host-block invariant;
- loop inversion and path refinement replay exactly;
- no allocation in the real-time callback;
- exact basis program requires eight local rotations, not a dense fallback.

### Formal layer

Add Lean definitions for:

- blade bitmasks and grade;
- pairing involution `S -> S xor {j}`;
- signed quarter-turn program;
- disjointness of planes;
- generator square and anticommutation;
- top-grade permutation sign.

The Lean result specifies algebraic structure; it does not certify floating-point DSP.

## E3 — Geometry and morphology as first-class compilation

### E3.1 Better soft lowering

Compare four realizations of an arbitrary vector gesture:

1. linear combination of approximate generators, current baseline;
2. skew projection `A -> (A-A^T)/2` before exponentiation;
3. compiler decomposition into exact local basis events;
4. learned local event schedule constrained to `SO(16)`.

Primary metrics:

- algebra residual;
- arbitrary-vector energy drift;
- determinant/top-grade error;
- factor count;
- sensitivity to contact width and coordinate noise.

Gate: energy drift below `1e-8` and volume-sign accuracy above 99% at a width where the current baseline is materially worse.

### E3.2 Morphology discovery ladder

Run increasingly free inverse design:

A. directions only, fixed topology — current experiment;
B. anchors and directions;
C. contact centers/widths;
D. sparse edge selection from a candidate contact graph;
E. string count and site allocation;
F. bounded topology changes with state transport.

At every level compare against an unconstrained dense operator with matched effective degrees of freedom.

### E3.3 Four-dimensionality test

The relevant hypothesis is not “4D always wins.” It is:

> For a fixed local-contact budget and a target family requiring four independent gesture directions, four ambient dimensions provide the first reliable geometric embedding, while additional dimensions give diminishing returns.

Sweep:

- ambient dimensions `2,3,4,5,6,8,12`;
- site counts `16,32,64,128`;
- contact degree `2,4,8,16`;
- widths relative to nearest-neighbor spacing;
- random and optimized morphologies.

Measure algebra fidelity at matched physical volume, total string length, and contact budget. A raw higher-dimensional model must not receive free extra separation volume.

## E4 — Distill the exact operator into an active string medium

This is the immediate next implementation target.

### Architecture

Each exact blade mode becomes a compartment with state

```text
(q, p, recovery, memory, gate).
```

The passive backbone executes exact Clifford rotations. A shared active residual law acts locally:

```text
z_next = exact_clifford_step(z, gesture)
         + dt * active_residual(z, local_neighbors, slow_state).
```

The active residual is bounded by one of:

- antisymmetric plus contractive decomposition;
- port-Hamiltonian parameterization;
- spectral normalization with explicit work ledger;
- learned positive/negative work ports with budgets.

### Curriculum

1. **Operator imitation:** reproduce exact one-token operators.
2. **Trajectory imitation:** reproduce complete closed-loop state trajectories.
3. **Noisy robustness:** imitate under geometry/contact perturbations.
4. **Task preservation:** maintain algebraic probe performance while active residual unlocks.
5. **Task extension:** add objectives the linear Clifford system cannot solve.

Loss terms:

```text
L = L_task
  + lambda_state L_state_distill
  + lambda_op L_local_jacobian
  + lambda_alg L_clifford_relations
  + lambda_energy L_ledger
  + lambda_geom L_morphology
  + lambda_complexity L_program.
```

### Controls

- active residual disabled;
- exact backbone removed;
- distillation removed;
- geometry frozen/randomized;
- state reset between tokens;
- commuting backbone;
- dense recurrent residual.

### Gate

The active medium must solve a task that the exact linear backbone cannot, while retaining:

- 100% foundational path probes;
- bounded work/energy residual;
- robustness to remeshing;
- no hidden MLP decoder capable of solving the task alone.

## E5 — Algorithmic sequence ladder

Use fixed train/test generators with length extrapolation.

### Tier A — group and path tasks

- word reduction in small non-Abelian groups;
- braid-word orientation;
- commutator depth;
- loop equivalence under reparameterization;
- signed area/volume accumulation;
- path inversion.

### Tier B — finite-state tasks

- parity at lengths 4–256;
- modular counting;
- regular languages requiring long state retention;
- permutation composition;
- group representation prediction.

### Tier C — memory tasks

- delayed copy;
- selective copy;
- associative recall;
- induction heads benchmark;
- multi-query associative retrieval;
- noisy key/value binding.

### Tier D — hierarchical tasks

- Dyck-1 and Dyck-2;
- nested arithmetic;
- compositional transductions;
- graph traversal encoded as gestures.

Primary metric is extrapolation beyond training length, not in-distribution fit.

Baselines:

- Transformer;
- GRU/LSTM;
- orthogonal/unitary RNN;
- Mamba-style state-space model;
- Neural CDE/RDE;
- Path Development Network;
- signature features plus linear/MLP readout;
- generic graph neural field;
- point-collapsed version of the same code.

## E6 — In-context learning

The hypothesis is that persistent extended dynamics can execute an online algorithm without inference-time weight changes.

### First task: linear regression

Provide demonstrations `(x_i,y_i)` as gestures, then query `x_*`. Evaluate prediction of `y_*` across changing latent linear maps.

Compare:

- frozen exact Clifford reservoir plus trained pickup;
- active medium;
- GRU;
- Transformer;
- linear attention;
- DendriCL-inspired compartment model;
- state-space model.

Probe whether medium state contains:

- sufficient statistics `X^T X` and `X^T y`;
- an iterative optimizer trajectory;
- a learned kernel representation;
- task identity in slow morphology/memory variables.

### Gates

- generalize to unseen regression weights;
- improve with number of demonstrations;
- extrapolate beyond training demonstration count;
- no parameter update during evaluation;
- fixed-size recurrent state;
- interpretable ablation showing which field component carries the online algorithm.

## E7 — Small language modelling

Proceed only after E5 or E6 gives a real generalization result.

### Datasets

1. byte-level synthetic grammar corpus;
2. Tiny Shakespeare or text8/enwik8;
3. TinyStories;
4. BabyLM-scale data.

### Model families

- exact Clifford fiber bank with learned closed token loops;
- active string medium with exact backbone;
- hybrid local-attention/contact model;
- point-collapsed and generic path-development controls.

### Token interface

Test three representations:

1. token selects a learned closed loop;
2. token predicts a short spline conditioned on medium state;
3. tokenizer produces gesture primitives shared compositionally across tokens.

### Readout

Compare:

- linear vocabulary pickup;
- gestural read path;
- energy-constrained local decoder;
- conventional MLP head as a declared scaffold.

### Metrics

- validation bits-per-byte/perplexity;
- context-length extrapolation;
- recurrent-state memory footprint;
- token throughput;
- gradient stability;
- path/state interpretability;
- sensitivity to token timing and segmentation;
- recovery after distractors;
- performance after remeshing or geometry perturbation.

A publishable first result need not beat a Transformer. It must demonstrate a capability caused by extension, such as superior length extrapolation at fixed recurrent state or robust in-context adaptation through slow field dynamics.

## E8 — Scale architecture

A language-scale system should use many small Clifford/active fibers, not one enormous algebra.

Candidate hierarchy:

```text
material site
  -> 16-state Clifford micro-fiber
  -> extended neuron/string
  -> local bundle of strings
  -> regional field graph
  -> sparse long-range contact choreography
```

Capacity knobs:

- number of fibers;
- material sites per string;
- active channels per site;
- sparse contact degree;
- regional hierarchy;
- slow contextual geometry;
- source/pickup count;
- precision and checkpoint interval.

Distributed execution should partition by geometry/contact regions and preserve deterministic event ordering. State transport during repartitioning must be explicit.

## Compute envelopes

Approximate, before kernel optimization:

| Stage | Device | Expected scale |
|---|---|---|
| E0–E1 | laptop CPU | seconds to minutes/run |
| E2 Rust replay | CPU | real-time or faster for 16-state fibers |
| E3 sweeps | single GPU | hours for thousands of morphologies |
| E4 active distillation | single modern GPU | hours to days |
| E5 algorithmic suite | 1–8 GPUs | days including baselines/seeds |
| E6 in-context learning | 1–8 GPUs | days |
| E7 TinyStories/BabyLM | 8–64 GPUs depending width | days to weeks |

These are planning envelopes, not quotes. Profile the actual sparse kernels before budgeting the larger stages.

## Publication stop conditions

A workshop/system paper is justified when E2 and E3 demonstrate a real-time sparse compiler, exact receipts, and a robust geometric lowering.

A machine-learning paper is justified when E4/E5 shows out-of-distribution sequence generalization that survives generic path-development and point-collapse baselines.

A language-model paper is justified when E7 demonstrates a meaningful scaling or capability advantage attributable to the extended medium.
