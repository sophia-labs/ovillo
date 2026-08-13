# Experiment 2 — Untyped Transport-Category Discovery

**Stable ID:** `E2-CATEGORY`  
**Primary question:** When all candidate transport branches receive the same raw event stream, does a sparse model select projective transport for filtering, noncommuting transport for closed-path orientation, both for mixed tasks, and neither when special transport is unnecessary?

---

# 1. Theory under test

Loop 2 showed that a typed bank assigned approximately 97% projective weight to filtering and 97% Clifford weight to oriented-loop memory. That experiment established nonredundant utility but partly supplied the category distinction through branch-specific signal channels.

This experiment tests the stronger claim:

> Transport category can be selected as an inductive bias from common observations under a controlled complexity budget.

This is not a claim of universal spontaneous architecture search. It is a test of whether the proposed categories are sufficiently distinct, learnable, and causally interpretable when type information is not handed to the gate.

---

# 2. Common event stream

Every branch receives exactly the same event tensor:

```text
e_t = {
  symbol_one_hot,       # discrete observed symbol or segment symbol
  control_increment,   # four-dimensional delta-g; may be nuisance
  phase_features,      # sin/cos loop phase and boundary flags
  task_context,        # present only in switched-task condition
  validity_mask
}
```

No branch receives hidden-state identity, teacher belief, loop orientation label, future target, or a branch-specific copy of the input.

The common encoder is one affine map plus layer normalization. Its output is copied to each structural branch. Branch-specific maps begin only after this shared encoder.

## 2.1 Event families

### Filtering episodes

- symbols come from a locked observable HMM;
- control increments form random exactly closed nuisance loops independent of the HMM state and future symbols;
- target is next observed symbol at every step.

### Loop episodes

- control increments form randomly rotated, rescaled, and time-reparameterized closed loops;
- observed symbols are an independent nuisance HMM stream with matched marginal entropy;
- target is loop orientation or commutator class at the closing boundary.

### Mixed episodes

The same episode contains a filtering target at every step and a loop target at boundaries. Neither channel is removable from the raw event.

### Switched episodes

A context token at the start declares whether the current episode's scored target is filtering or loop orientation. The raw event structure after the cue is shared.

### Null/local episodes

Target depends only on the current symbol through a manifest-fixed mapping. This condition tests whether the MDL penalty prunes unnecessary recurrent categories.

---

# 3. Transport bank

All branches expose a state vector, a state transition, a branch prediction contribution, and a receipt.

## 3.1 Positive projective branch `B-PROJ`

- rank 6 in the core condition;
- strictly positive event-conditioned operators;
- projective normalization by learned positive terminal;
- operator generated from the common event embedding as a positive combination of a small basis plus a symbol table;
- optional observable NMF initialization in the calibrated condition.

The branch may use `symbol_one_hot` and the complete common embedding, but no branch-specific filtering feature.

## 3.2 Noncommuting Clifford/Lie branch `B-CLIFF`

- two `Cl(0,3)` fibers or one `Cl(0,4)` fiber under matched state budget;
- control increment mapped through exact skew generators;
- explicit source and retention;
- exact loop closure audit;
- noncommuting commutator receipt.

## 3.3 Commuting orthogonal branch `B-COMM`

- same state dimension, source, retention, readout budget, and control-input map as `B-CLIFF`;
- all control directions use one shared skew generator;
- closed loops therefore collapse at the operator level up to numerical tolerance.

This is the causal noncommutation control, not a low-capacity straw model.

## 3.4 Affine contractive branch `B-AFFINE`

- stable symbol/event-conditioned affine recurrence;
- state dimension matched to the largest special branch;
- spectral normalization or contractive parameterization;
- no projective normalization or exact geometric algebra.

## 3.5 Optional unconstrained branch `B-LINEAR`

A capacity control allowed in the extended tier. It is excluded from the primary complexity-selection claim unless its parameter and operation costs are included honestly.

---

# 4. Gate and combined prediction

Let branch features be `z_b(t)`. One gate network sees only the shared encoder state and a pooled causal summary:

```text
g_t = sparsemax_or_entmax(gate_net(common_context_t)).
```

The combined feature is

```text
z_t = concat_b sqrt(g_t[b]) * normalized(z_b(t)).
```

Task heads are affine over the gated branch contributions. Branch feature normalization is frozen and identical across branches so gate mass cannot be reduced merely by increasing feature scale.

## 4.1 Complexity charge

Each branch has a declared cost:

```text
cost_b =
  state_scalar_cost * recurrent_state_scalars
  + parameter_cost * trainable_parameters
  + operation_cost * measured_forward_FLOPs
```

Normalize costs so `B-PROJ` and `B-CLIFF` each cost approximately 1.0 in the core configuration. The loss includes:

```text
L_MDL = lambda_cost * sum_b mean(g_b) * cost_b
      + lambda_entropy * mean entropy(g)
      + lambda_active * expected_active_branches.
```

The exact coefficients are set by a calibration rule: on the null task, the cost of keeping one unnecessary specialized branch active must equal 2% of initial task loss. Freeze coefficients before scientific runs.

## 4.2 Gate schedule

- first 10% of steps: uniform gate, no sparsity penalty, all branches learn a viable local response;
- next 20%: linearly introduce complexity charge;
- final 70%: full charge and learned sparse gate.

A separate `no-warmup` control tests whether this schedule simply protects a favored branch.

---

# 5. Initialization regimes

## 5.1 `CALIBRATED` — primary regime

- `B-PROJ`: observable factorization on the filtering process, readout and common-event residual map random;
- `B-CLIFF`: exact generators, random learned gesture/event map;
- `B-COMM`: exact shared generator, matched random map;
- `B-AFFINE`: stable random initialization;
- gate and all task heads random.

This asks whether the optimizer selects among mechanisms that are viable but not pre-routed.

## 5.2 `COLD` — stronger secondary regime

All trainable maps, including positive projective operators, begin from the standard random initializations. Exact structural constraints remain. This asks whether the useful basin and category can be found jointly.

A calibrated success plus cold failure supports category selection conditional on viable branch initialization, not full spontaneous discovery.

## 5.3 `MISMATCHED`

Initialize the projective branch from a different process with the same alphabet and rank. This tests whether the calibrated result is merely giving the answer to the filtering task.

---

# 6. Task distributions and out-of-distribution tests

## 6.1 Filtering

Core processes:

- `P-BASE6`;
- `P-NOISY6`;
- `P-WEAK8` with projective rank 8 in the process-matched condition.

OOD filtering tests:

- changed persistence with fixed emissions;
- changed emissions with fixed transition;
- longer sequences;
- held-out process seed.

## 6.2 Loop orientation

Training loops:

- rectangles and four-segment commutator loops;
- random coordinate planes;
- scale in `[0.15, 0.55]`;
- 4–8 segment time refinements;
- random orthogonal rotation in control space.

Test loops:

- scale `[0.05, 0.75]`;
- unseen rotations;
- nonrectangular piecewise-linear loops;
- reversed parameterization;
- same endpoint and matched path length controls.

The label cannot be inferred from endpoint, total path length, or symbol multiset.

---

# 7. Training schedule

## 7.1 Stage 1 screen

- tasks: filtering, loop, mixed, null;
- initialization: `CALIBRATED` and `COLD`;
- MDL multipliers: `[0.5, 1.0, 2.0]` around calibrated value;
- seeds: `[7, 11]`;
- batch: 256 episodes;
- episode length: 96;
- updates: 800;
- optimizer: AdamW;
- gate temperature/entmax alpha locked in manifest;
- evaluation every 50 updates.

This is 48 core micro-runs plus mandatory single-branch controls, vectorized by task and initialization regime.

## 7.2 Stage 2 confirm

- tasks: filtering, loop, mixed, switched, null;
- initialization: `CALIBRATED`, plus promoted `COLD` and `MISMATCHED`;
- selected MDL coefficient;
- seeds: `[7, 11, 19, 23, 31]`;
- episode length: 192;
- updates: 2,000;
- OOD suite after training;
- branch ablations and gate permutations on the fixed final checkpoint.

---

# 8. Primary observables

- filtering excess BPS and accuracy;
- loop accuracy and calibration;
- per-task gate mass;
- expected active branch count;
- gate entropy;
- branch contribution singular values and participation rank;
- pairwise branch mutual information conditional on target;
- projective operator condition;
- Clifford and commuting commutator norms;
- retention/source parameters;
- common-encoder gradient received from each branch;
- MDL/task-loss Pareto point;
- performance after branch ablation;
- performance after gate permutation;
- transfer to OOD task/process;
- GPU seconds.

Gate mass alone is never interpreted as mechanism. A selected branch must have a matching causal ablation effect.

---

# 9. Mandatory controls

## 9.1 Single-branch ceilings

Train every branch alone under the same event stream and a matched step/data budget. These establish whether a branch is capable of the task and how much the mixed model could gain.

## 9.2 Commuting control

The commuting branch must remain near chance on OOD loop orientation while matching Clifford state size and readout budget.

## 9.3 Gate-label permutation

After training, permute gates among branch features. If performance is unchanged, gate mass does not identify a functional allocation.

## 9.4 Branch-output rescaling

Multiply one branch feature by scales `[0.5, 2.0]` while applying the corresponding inverse readout transform. Gate choices should remain stable under this function-preserving scale intervention.

## 9.5 Nuisance swap

Swap filtering nuisance loops and loop-task nuisance symbol streams across episodes. Branch selection should track the scored mechanism, not superficial marginal statistics.

## 9.6 Equal-cost control

Set every branch cost to exactly one. This reveals whether the category result depends on the engineering cost model.

---

# 10. Registered prediction and falsifier

The calibrated prediction is:

- filtering selects projective transport and loses filtering ability under projective ablation;
- loop orientation selects noncommuting transport and loses orientation under Clifford ablation;
- commuting transport does not substitute on OOD closed loops;
- mixed and switched tasks use context-dependent combinations;
- null tasks prune unnecessary specialized recurrence;
- selected modules transfer more cleanly to a related task than unselected modules.

The theory is falsified in this regime if:

- gates remain diffuse despite a functioning sparsity control;
- gate choices do not match causal ablations;
- all tasks select the same generic branch;
- category assignment follows nuisance channels;
- or commuting transport solves the orientation controls equally well.

Cold-start failure is not by itself a falsification of calibrated category selection, but it is a failure of full spontaneous discovery and must be reported plainly.

---

# 11. Required artifacts

Per run:

```text
common-event-spec.json
gate-schedule.json
branch-costs.json
branch-diagnostics.json
branch-contributions.npz
ablation-results.json
ood-results.json
```

Aggregate:

```text
gate-allocation.json
causal-ablation-matrix.json
category-pareto.json
cold-vs-calibrated.json
decision-E2-CATEGORY.json
```