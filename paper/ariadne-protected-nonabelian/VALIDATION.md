# Unified protected instrument — validation record

**Pinned repository commit:** `c0c149bab7a9e2fb1f7ba8b3328694fcd948adf6`
**Oracle source-tree SHA-256:** `1bbc9a963c8e2f3663b8df29f4e623144f7c9a24e4f25e1aa0acd1403289bcc5`
**Fixture schema:** `ariadne.unified-instrument.fixture.v2`
**Generation mode:** `full`
**Maximum reference claim:** `protected-acoustic`
**Maximum production claim:** `null`

## Commands executed

From `paper/ariadne-protected-nonabelian`:

```bash
python -m compileall -q unified_reference run_unified_experiments.py
python run_unified_experiments.py --output unified_artifacts
python -m unittest discover -s unified_reference/tests -v
```

Schema validation was performed with JSON Schema draft 2020-12 against
`schemas/unified_fixture_v2.schema.json`.

## Test result

```text
23 tests passed
0 failures
0 errors
```

The suite covers:

- Clifford algebra, exact gap identity, complex structure, and symmetry;
- analytic coefficient-box protection and deterministic perturbation corpus;
- gauge-free degree and independent non-Abelian lattice-gauge `C2` estimators;
- Kato transport, gauge covariance, path reversal/concatenation, and AB/BA;
- graph-restricted complex compilation and per-factor symplectic legality;
- finite-time numerical replay and rigorous-duration refusal;
- persistent state and causal room feedback;
- passive source/radiation/retirement accounting;
- weighted-polar expansion/contraction and sparse negative baseline;
- nonlinear claim boundary and claim-lattice separation;
- fixture source identity, schema identity, and every manifest hash.

## Headline full-resolution results

### Conservative family and topology

```text
complex state dimension               16
real quadrature dimension             32
exact protected gap                   2
regional participation                [0.2500000000000001, 0.25, 0.25, 0.24999999999999992]
analytic second Chern                  1
degree estimate N=16                  0.999784733418126
degree integer error                  2.152666e-04
lattice-gauge estimate N=16          0.928889876497531
lattice maximum plaquette phase       7.885444e-02
lattice minimum link singular value   0.980785280403230
analytic perturbation gap floor       1.649400718277167
perturbation commutator floor          1.591052030578573
```

The lattice-gauge estimator is intentionally reported as a convergent finite-grid
estimate, not rounded and presented as exact numerical equality.

### Non-Abelian path receipt

```text
occupied commutator Frobenius norm    1.687700928033460
occupied commutator operator norm     1.193384770827292
group-commutator distance             1.687700928033459
A occupied eigenphases                [-0.9219696358205642, 0.9219696358205653]
B occupied eigenphases                [-0.8602818492391705, 0.8602818492391705]
```

### Compiler

```text
A reconstruction error                4.641455e-14
B reconstruction error                4.347763e-14
A max factor symplectic residual      8.885518e-16
B max factor symplectic residual      8.919811e-16
A two-mode / phase gates              120 / 16
B two-mode / phase gates              120 / 16
```

### Finite-time execution

```text
JRS sufficient duration A, 5%         22.362855480 s
JRS sufficient duration B, 5%         15.941498263 s
1 s projective error A                1.476037e-03
1 s projective error B                1.066173e-03
1 s leakage A                         1.549464e-05
1 s leakage B                         1.306238e-05
1 s theorem promoted                  false
```

The direct replay is numerically strong. The requested one-second rigorous
adiabatic claim remains refused because the conservative theorem bound does not
pass the declared 5% budget.

### Acoustic observable and persistence

```text
raw string-to-room singular values    [0.29834619270682317, 0.2983461927068217, 5.0424916900232287e-17, 1.466073983690994e-17]
calibrated transfer gain              1.193384770827293
prepared leakage outside protected    2.063104e-16
segmented/concatenated state error     0.000000e+00
reset counterfeit difference          0.889660680171709
maximum room-to-string transfer       1.560112236211245
maximum string-to-room transfer       1.560112236211245
decoupled room-to-string transfer     0.000000e+00
```

### Audio and energy ledger

```text
AB/BA final-state distance            0.775700101037740
relative RMS waveform distance        1.378846163607843
gain-removed waveform distance        1.403916262996598
optimal scalar residual fraction      0.999894730746385
normalized spectral magnitude dist.   0.135133298039087
AB ledger closure                     -2.858824e-15
BA ledger closure                     1.637579e-15
spectral matching claimed             false
listener discrimination claimed       false
```

### State connection

```text
fixed-capacity protected connection   identity
expansion source isometry residual    8.833963e-15
contraction target coisometry resid.   3.157579e-15
contraction requires disposal          true
4/nonzero sparse operator error       0.132069430097711
4/nonzero sparse worst energy error   1.480728%
4/nonzero sparse worst state error    0.165554724714592
```

## Artifact manifest

The full packet contains 9 hashed files. The unit test recomputes
every byte count and SHA-256 digest. The manifest itself is
`unified_artifacts/unified_manifest.json`.

## Deliberate failures/open gates

```text
one-second theorem adiabatic budget   false
production Rust replay                false
Lean kernel check                     false
raw Weave protected integration       false
listener study                        false
fault tolerance                       refused
```

These false fields are acceptance behavior, not missing cosmetic metadata.
They prevent the reference mathematics from being presented as already proven
through the current production callback.
