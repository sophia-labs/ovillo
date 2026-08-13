#!/usr/bin/env python3
"""Fail when Ariadne grows a second physical-world architecture.

`mus-ariadne` owns anatomy, law, state, space, trajectories, execution, and
receipts. `mus-dsp` may expose a temporary inline import shim, but it may not own
an Ariadne source file. Host adapters depend on the canonical crate directly.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKSPACE = ROOT / "mus-rs" / "Cargo.toml"
ARIADNE = ROOT / "mus-rs" / "crates" / "mus-ariadne"
DSP = ROOT / "mus-rs" / "crates" / "mus-dsp"
PLUGIN = ROOT / "mus-rs" / "plugins" / "ariadne-clap"

REQUIRED_CANONICAL_FILES = (
    "Cargo.toml",
    "README.md",
    "src/lib.rs",
    "src/ir.rs",
    "src/world.rs",
    "src/field.rs",
    "src/space.rs",
    "src/trajectory.rs",
    "src/performance.rs",
    "src/law/active.rs",
    "src/law/aperture.rs",
    "src/law/transport.rs",
)

# Patterns are checked inside `World::process_sample`, in execution order.
REQUIRED_STAGE_PATTERNS = (
    ("external work", "let work_input"),
    ("compliant boundary", "self.aperture.process"),
    ("active material", "self.active.process"),
    ("ordered transport", "self.transport.process"),
    ("conservative propagation", "self.field.propagate"),
    ("passive loss", "self.field.damp"),
    ("space exchange", "self.space.process"),
    ("radiation", ".radiate("),
    ("ledger", "self.ledger.push"),
)

FORBIDDEN_CANONICAL_DEPENDENCIES = (
    "mus-dsp",
    "nice-plug",
    "nih-plug",
    "cpal",
    "hound",
    "rustfft",
    "realfft",
    "libsoxr",
)

DSP_MODULE_PATTERN = re.compile(r"\bpub\s+mod\s+(ariadne_[A-Za-z0-9_]+)\s*;")


def rust_sources(directory: Path) -> list[Path]:
    return sorted(path for path in directory.rglob("*.rs") if path.is_file())


def fail_if_missing(failures: list[str], path: Path, label: str) -> None:
    if not path.exists():
        failures.append(f"{label}: {path.relative_to(ROOT)}")


def main() -> int:
    failures: list[str] = []

    fail_if_missing(failures, WORKSPACE, "workspace manifest is missing")
    if WORKSPACE.exists() and '"crates/mus-ariadne"' not in WORKSPACE.read_text(
        encoding="utf-8"
    ):
        failures.append("workspace does not contain crates/mus-ariadne")

    for relative in REQUIRED_CANONICAL_FILES:
        fail_if_missing(
            failures,
            ARIADNE / relative,
            "canonical Ariadne file is missing",
        )

    canonical_manifest = ARIADNE / "Cargo.toml"
    if canonical_manifest.exists():
        text = canonical_manifest.read_text(encoding="utf-8").lower()
        for dependency in FORBIDDEN_CANONICAL_DEPENDENCIES:
            if dependency in text:
                failures.append(
                    f"canonical Ariadne depends on adapter/offline crate {dependency!r}"
                )

    canonical_sources = [
        path
        for path in rust_sources(ARIADNE / "src")
        if "compat" not in path.relative_to(ARIADNE / "src").parts
    ]
    if not canonical_sources:
        failures.append("canonical Ariadne has no non-compatibility Rust sources")
    for path in canonical_sources:
        lowered = path.read_text(encoding="utf-8").lower()
        for forbidden in ("mus_dsp::", "nice_plug::", "nih_plug::"):
            if forbidden in lowered:
                failures.append(
                    f"canonical physical source imports adapter layer {forbidden!r}: "
                    f"{path.relative_to(ROOT)}"
                )

    world_path = ARIADNE / "src" / "world.rs"
    if world_path.exists():
        world_text = world_path.read_text(encoding="utf-8")
        start = world_text.find("pub fn process_sample")
        end = world_text.find("pub fn receipt", start + 1)
        if start < 0 or end < 0:
            failures.append("canonical World does not expose process_sample and receipt")
        else:
            process_text = world_text[start:end]
            cursor = -1
            for stage, pattern in REQUIRED_STAGE_PATTERNS:
                next_cursor = process_text.find(pattern, cursor + 1)
                if next_cursor < 0:
                    failures.append(
                        f"canonical sample execution omits {stage!r} ({pattern!r})"
                    )
                elif next_cursor <= cursor:
                    failures.append(
                        f"canonical sample execution order is not frozen at {stage!r}"
                    )
                cursor = max(cursor, next_cursor)
        if "pub struct World" not in world_text:
            failures.append("canonical runtime does not expose one World")
        if "WorkLedger" not in world_text:
            failures.append("canonical runtime does not own one work ledger")

    dsp_src = DSP / "src"
    for path in sorted(dsp_src.glob("ariadne_*.rs")):
        failures.append(
            f"duplicate Ariadne source remains in mus-dsp: {path.relative_to(ROOT)}"
        )

    dsp_lib = dsp_src / "lib.rs"
    if dsp_lib.exists():
        modules = DSP_MODULE_PATTERN.findall(dsp_lib.read_text(encoding="utf-8"))
        if modules:
            failures.append(
                "mus-dsp exports file-backed Ariadne runtimes: "
                + ", ".join(sorted(set(modules)))
            )

    plugin_manifest = PLUGIN / "Cargo.toml"
    if plugin_manifest.exists():
        manifest = plugin_manifest.read_text(encoding="utf-8")
        if "mus-ariadne" not in manifest:
            failures.append("Ariadne plug-in does not depend on mus-ariadne")
        if re.search(r"^mus-dsp\s*=", manifest, re.MULTILINE):
            failures.append("Ariadne plug-in still depends on mus-dsp")

    plugin_src = PLUGIN / "src"
    if plugin_src.exists():
        for path in rust_sources(plugin_src):
            lowered = path.read_text(encoding="utf-8").lower()
            if "mus_dsp::ariadne_" in lowered:
                failures.append(
                    f"plug-in imports the retired mus-dsp Ariadne namespace: "
                    f"{path.relative_to(ROOT)}"
                )
            physical_markers = (
                "struct materialcell",
                "struct delaycourse",
                "fn propagate_material",
                "fn radiate_material",
                "fn apply_powered_deformation",
            )
            present = [marker for marker in physical_markers if marker in lowered]
            if present:
                failures.append(
                    f"plug-in owns physical law instead of adapting a world: "
                    f"{path.relative_to(ROOT)} ({', '.join(present)})"
                )

    if failures:
        print("Ariadne architecture check failed:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1

    print(
        "Ariadne architecture check passed: one canonical crate, one World, "
        "one sample order, one ledger, and no duplicate DSP bodies."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
