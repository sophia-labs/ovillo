#!/usr/bin/env python3
"""Fail-loud architecture guard for the Generation 3 fold-back."""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
MEDIUM = ROOT / "mus-rs/crates/mus-ariadne/src/law/medium.rs"
CARGO = ROOT / "mus-rs/crates/mus-ariadne/Cargo.toml"
LIB = ROOT / "mus-rs/crates/mus-ariadne/src/lib.rs"


def main() -> None:
    medium = MEDIUM.read_text()
    cargo = CARGO.read_text().lower()
    lib = LIB.read_text()

    forbidden_medium = {
        "use torch": "Ariadne law may not depend on a training runtime",
        "torch::": "Ariadne law may not depend on a training runtime",
        "task_label": "Ariadne law may not own task labels",
        "cross_entropy": "Ariadne law may not own a task loss",
    }
    failures = [message for token, message in forbidden_medium.items() if token in medium.lower()]
    for token in ("torch", "numpy", "extension-generation", "extension_generation"):
        if token in cargo:
            failures.append(
                f"mus-ariadne Cargo.toml contains forbidden learning dependency {token!r}"
            )
    if "pub mod medium;" not in (ROOT / "mus-rs/crates/mus-ariadne/src/law/mod.rs").read_text():
        failures.append("local medium is not registered under Ariadne's law module")
    if "LocalConstitutiveMedium" not in lib:
        failures.append("local medium is not exported from the canonical Ariadne crate")

    if failures:
        raise SystemExit("\n".join(f"- {failure}" for failure in failures))
    print("Generation 3 Ariadne/Extension boundary: valid")


if __name__ == "__main__":
    main()
