# Extension-v1 validation record

Validation was performed locally in the supplied CPU environment and then replayed by the dedicated GitHub Actions workflow on the published branch.

## Local environment

```text
Python 3.13.5
PyTorch 2.10.0+cpu
pytest 9.0.2
```

The package declares Python 3.11+ and PyTorch 2.2+; the local checks establish only the environment above. GitHub Actions separately replayed the Python surface on Python 3.11.

## Local commands

```bash
cd mus-rs/experiments/extension_v1

PYTHONDONTWRITEBYTECODE=1 \
PYTHONPATH=. \
pytest -p no:cacheprovider

python -m pip install --no-build-isolation --no-deps -e .
extension-v1 --help
extension-v1 exact --trials 64 --seed 19 --output /tmp/extension-exact-smoke.json
extension-v1 development --output /tmp/extension-development-smoke.json
```

## Local results

```text
12 passed in 5.79s
editable installation succeeded
console-script discovery succeeded
exact Clifford smoke receipt written
closed-development smoke receipt written
```

Smoke receipt digests:

```text
exact, 64 trials: 3e562ab015bf3223d200eec2fa354fbad629ff30cfc00cfef2603ceb656141fb
development:      a97d0b153c762e94b91eeca39b9ce3926df0e779d99f6ff0c1702816a8795460
```

The exact smoke digest intentionally differs from the committed 10,000-trial artifact because the trial count is part of its configuration. The development digest matches the committed development artifact.

## GitHub Actions replay

Workflow run `31560816000` on commit `0f5629f3e103421de71d11144622f06f4ee65d7c` completed successfully on August 12, 2026.

### `python-reference`

Passed:

- checkout and Python 3.11 setup;
- editable installation with test extras;
- `compileall` over package and tests;
- all 12 pytest checks;
- exact Clifford smoke replay;
- closed-development smoke replay.

### `rust-sparse-lowering`

Passed:

```bash
cargo test --manifest-path \
  mus-rs/experiments/extension_v1/rust/mus-extension-reference/Cargo.toml
```

This compiles and replays the candidate lowering of the four exact basis gestures into the repository's existing `mus_watsn::OperatorProgram` and `PlaneRotation` API. The Rust candidate is therefore no longer merely API-matched source: its staged test suite passed in CI. It remains outside the production workspace intentionally until the experiment API is accepted.

## Other repository checks

On the same commit:

- `Scientific audio CI`: passed;
- `Lean formal core`: passed;
- `Ariadne gesture compiler reference`: passed.

`mus-workspace-ci` continues to fail only its `cargo clippy --workspace --all-targets -- -D warnings` step. The exact base commit `af2a2d377d60d31a63040589b601533809ea3f0a` already fails the same job; its format, check, and test steps pass. This inherited workspace warning debt is distinct from the green Extension-specific Python and Rust gates.

## Scope

These checks cover algebraic identities, exact path reversal, closed-loop behavior, geometric lowering, morphology smoke optimization, active-cable shape/gradient behavior, closed-gesture learning, commuting controls, receipt determinism, and the candidate Rust sparse lowering. They do not constitute numerical certification across GPU kernels, all PyTorch/Rust versions, operating systems, alternative floating-point precisions, or production real-time integration.
