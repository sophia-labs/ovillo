#!/usr/bin/env python3
"""Fail if tokenizer or task semantics leak into Ariadne's generic radiator."""

from __future__ import annotations

from pathlib import Path

FORBIDDEN = (
    "bpe",
    "unigram",
    "token_id",
    "vocabulary",
    "morpheme",
    "language_model",
    "cross_entropy",
    "optimizer",
)


def main() -> None:
    repository = Path(__file__).resolve().parents[4]
    source = (
        repository
        / "mus-rs"
        / "crates"
        / "mus-ariadne"
        / "src"
        / "law"
        / "radiation.rs"
    )
    text = source.read_text().lower()
    found = [term for term in FORBIDDEN if term in text]
    if found:
        raise SystemExit(
            f"Ariadne radiation law contains Extension vocabulary terms: {found}"
        )
    print("Ariadne radiation boundary valid")


if __name__ == "__main__":
    main()
