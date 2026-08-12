# Extension-v1 validation record

Validation was performed in the supplied CPU environment after a clean-cache pass.

## Environment

```text
Python 3.13.5
PyTorch 2.10.0+cpu
pytest 9.0.2
```

The package declares Python 3.11+ and PyTorch 2.2+; the recorded checks establish only the environment above.

## Commands

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

## Results

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

## Scope

These checks cover algebraic identities, exact path reversal, closed-loop behavior, geometric lowering, morphology smoke optimization, active-cable shape/gradient behavior, closed-gesture learning, commuting controls, and receipt determinism. They do not constitute numerical certification across GPU kernels, PyTorch versions, operating systems, or alternative floating-point precisions.

## Rust candidate

`rust/mus-extension-reference` lowers the exact four basis gestures to the existing `mus-watsn` sparse program API. The source was API-matched against the repository snapshot, but this container had neither `cargo` nor `rustc`. It is not included in the statement “12 passed”; compilation and Rust test replay are an explicit first action in the next environment.

## Repository CI landing

The delivery also adds `.github/workflows/extension-v1.yml`. Its Python job replays this validation surface on Python 3.11. Its separate Rust job installs the stable toolchain and runs the staged sparse-lowering crate against the repository's `mus-watsn` API. At delivery time that workflow had not yet run, so Rust compilation remains pending CI evidence.
