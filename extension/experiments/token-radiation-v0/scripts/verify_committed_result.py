#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
from pathlib import Path


def canonical_sha256(value: object) -> str:
    payload = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode()
    return hashlib.sha256(payload).hexdigest()


def main() -> None:
    root = Path(__file__).resolve().parents[1]
    result = root / "results" / "token-radiation-v0-20260814"
    manifest = json.loads((result / "manifest.sha256.json").read_text())

    summary_parts = sorted(result.glob("summary.json.part-*"))
    if not summary_parts:
        raise SystemExit("committed result has no summary shards")
    summary_bytes = b"".join(path.read_bytes() for path in summary_parts)
    materialized = {
        "corpus.json": (result / "corpus.json").read_bytes(),
        "vocabularies.json": (result / "vocabularies.json").read_bytes(),
        "summary.json": summary_bytes,
    }
    for name, expected in manifest.items():
        actual = hashlib.sha256(materialized[name]).hexdigest()
        if actual != expected:
            raise SystemExit(
                f"{name} digest mismatch: expected {expected}, found {actual}"
            )

    summary = json.loads(summary_bytes)
    expected_scientific = summary.pop("sha256")
    actual_scientific = canonical_sha256(summary)
    if actual_scientific != expected_scientific:
        raise SystemExit(
            "scientific summary digest mismatch: "
            f"expected {expected_scientific}, found {actual_scientific}"
        )
    print(
        json.dumps(
            {
                "valid": True,
                "scientific_sha256": expected_scientific,
                "summary_shards": len(summary_parts),
                "detector_results": len(summary["detector_results"]),
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
