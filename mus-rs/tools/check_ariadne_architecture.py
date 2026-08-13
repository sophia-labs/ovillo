#!/usr/bin/env python3
"""Fail when Ariadne grows a second physical-world architecture.

The canonical extension surface is ``mus-ariadne``. ``mus-dsp`` may retain the
shipped Terminal as an explicitly frozen compatibility boundary, but new bodies,
spaces, active materials, or instrument-specific sample loops must enter the
canonical world instead.
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

DUPLICATE_DSP_FILES = (
    "ariadne_active_inclusion.rs",
    "ariadne_compliant_aperture.rs",
    "ariadne_metric_body.rs",
    "ariadne_metric_field.rs",
    "ariadne_morphing_field.rs",
    "ariadne_resonant_transport.rs",
    "ariadne_room_halo.rs",
    "ariadne_string_brass_body.rs",
    "ariadne_string_brass_profiles.rs",
)

REQUIRED_CANONICAL_FILES = (
    "Cargo.toml",
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

REQUIRED_STAGE_ORDER = (
    "external_work",
    "compliant_boundary",
    "active_material",
    "ordered_transport",
    "conservative_propagation",
    "passive_loss",
    "space_exchange",
    "radiation",
    "ledger",
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

# A new public ``ariadne_*`` module in mus-dsp is almost certainly a second
# runtime. ``ariadne_terminal`` is the sole temporary session-compatibility
# exception and is explicitly named in the unification document.
DSP_MODULE_PATTERN = re.compile(r"\bpub\s+mod\s+(ariadne_[A-Za-z0-9_]+)\s*;")


def rust_sources(directory: Path) -> list[Path]:
    return sorted(path for path in directory.rglob("*.rs") if path.is_file())


def main() -> int:
    failures: list[str] = []

    if not WORKSPACE.exists():
        failures.append("mus-rs/Cargo.toml is missing")
    else:
        workspace_text = WORKSPACE.read_text(encoding="utf-8")
        if '"crates/mus-ariadne"' not in workspace_text:
            failures.append("workspace does not contain crates/mus-ariadne")

    for relative in REQUIRED_CANONICAL_FILES:
        path = ARIADNE / relative
        if not path.exists():
            failures.append(f"canonical Ariadne file is missing: {path.relative_to(ROOT)}")

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
        text = path.read_text(encoding="utf-8")
        lowered = text.lower()
        for forbidden in ("mus_dsp::", "nice_plug::", "nih_plug::"):
            if forbidden in lowered:
                failures.append(
                    f"canonical physical source imports adapter layer {forbidden!r}: "
                    f"{path.relative_to(ROOT)}"
                )

    world_path = ARIADNE / "src" / "world.rs"
    if world_path.exists():
        world_text = world_path.read_text(encoding="utf-8")
        cursor = -1
        for stage in REQUIRED_STAGE_ORDER:
            next_cursor = world_text.find(f'"{stage}"', cursor + 1)
            if next_cursor < 0:
                failures.append(f"canonical stage order omits {stage!r}")
            elif next_cursor <= cursor:
                failures.append(f"canonical stage order is not frozen at {stage!r}")
            cursor = max(cursor, next_cursor)
        if "pub struct World" not in world_text:
            failures.append("canonical runtime does not expose one World")
        if "WorkLedger" not in world_text:
            failures.append("canonical runtime does not own one work ledger")

    dsp_src = DSP / "src"
    for filename in DUPLICATE_DSP_FILES:
        path = dsp_src / filename
        if path.exists():
            failures.append(
                f"duplicate Ariadne runtime remains in mus-dsp: {path.relative_to(ROOT)}"
            )

    dsp_lib = dsp_src / "lib.rs"
    if dsp_lib.exists():
        modules = DSP_MODULE_PATTERN.findall(dsp_lib.read_text(encoding="utf-8"))
        unexpected = sorted(set(modules) - {"ariadne_terminal"})
        if unexpected:
            failures.append(
                "mus-dsp exports noncanonical Ariadne runtimes: " + ", ".join(unexpected)
            )

    plugin_src = PLUGIN / "src"
    if plugin_src.exists():
        for path in rust_sources(plugin_src):
            text = path.read_text(encoding="utf-8").lower()
            physical_markers = (
                "struct materialcell",
                "struct delaycourse",
                "fn propagate_material",
                "fn radiate_material",
                "fn apply_powered_deformation",
            )
            present = [marker for marker in physical_markers if marker in text]
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
        "Ariadne architecture check passed: one canonical world, one stage "
        "order, one ledger, and no duplicate mus-dsp bodies."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
