#!/usr/bin/env python3
"""Reject electrical/circuit anatomy in Ariadne's active reference renderer.

Historical research notes and external evaluation metadata may discuss legacy
hardware. This check intentionally targets active Ariadne physical modules and
the module-export surface, where those concepts would become implementation
ontology.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DSP_SRC = ROOT / "mus-rs" / "crates" / "mus-dsp" / "src"

FORBIDDEN_FILES = (
    DSP_SRC / "electrodynamic_radiator.rs",
    DSP_SRC / "power_port.rs",
)

FORBIDDEN_TERMS = {
    "voltage",
    "current",
    "charge",
    "flux_linkage",
    "resistor",
    "resistance_ohm",
    "capacitor",
    "capacitance",
    "inductor",
    "inductance",
    "diode",
    "triode",
    "pentode",
    "transistor",
    "transformer",
    "rectifier",
    "netlist",
    "pickup_coil",
    "voice_coil",
    "back_emf",
    "electrodynamic",
    "electromagnetic",
    "cabinet_ir",
    "cabinet_sections",
    "soft_clip",
    "hard_clip",
    "waveshaper",
}

FORBIDDEN_EXPORTS = (
    r"\bpub\s+mod\s+electrodynamic_radiator\b",
    r"\bpub\s+mod\s+power_port\b",
)


def active_sources() -> list[Path]:
    sources = set(DSP_SRC.glob("ariadne_*.rs"))
    directory = DSP_SRC / "ariadne"
    if directory.exists():
        sources.update(directory.rglob("*.rs"))
    return sorted(sources)


def main() -> int:
    failures: list[str] = []

    for path in FORBIDDEN_FILES:
        if path.exists():
            failures.append(
                f"forbidden Ariadne electrical-domain module exists: "
                f"{path.relative_to(ROOT)}"
            )

    lib = DSP_SRC / "lib.rs"
    if lib.exists():
        text = lib.read_text(encoding="utf-8")
        for pattern in FORBIDDEN_EXPORTS:
            if re.search(pattern, text):
                failures.append(
                    f"forbidden module export in {lib.relative_to(ROOT)}: {pattern}"
                )

    sources = active_sources()
    if not sources:
        failures.append("no active Ariadne pure-physics source was found")

    for path in sources:
        text = path.read_text(encoding="utf-8").lower()
        for term in sorted(FORBIDDEN_TERMS):
            if re.search(rf"\b{re.escape(term)}\b", text):
                failures.append(
                    f"forbidden implementation term {term!r} in {path.relative_to(ROOT)}"
                )

        # A serial legacy-effects topology is forbidden even under neutral names.
        serial_markers = (
            "contact_stages",
            "cabinet_sections",
            "stage_1",
            "stage_2",
            "stage_3",
        )
        present = [marker for marker in serial_markers if marker in text]
        if len(present) >= 2:
            failures.append(
                f"serial stage topology markers in {path.relative_to(ROOT)}: "
                f"{', '.join(present)}"
            )

    if failures:
        print("Ariadne pure-physics source check failed:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1

    print(
        f"Ariadne pure-physics source check passed ({len(sources)} active source file(s))."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
