#!/usr/bin/env python3
"""Fail when Ariadne grows a second physical-world architecture.

`mus-ariadne` owns construction plans, local bodies, shared fields, physical
couplings, trajectories, observers, execution, and the one global receipt. Host
adapters depend on the canonical crate directly. A musical phenotype may add a
BodySpec or gesture compiler, but not another private World or ledger.
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
    "src/plan.rs",
    "src/body.rs",
    "src/coupling.rs",
    "src/world.rs",
    "src/field.rs",
    "src/space.rs",
    "src/trajectory.rs",
    "src/performance.rs",
    "src/law/active.rs",
    "src/law/aperture.rs",
    "src/law/transport.rs",
)

# Patterns are checked after whitespace removal. Body-local law ordering and
# world-composition ordering are separate constitutional invariants.
REQUIRED_BODY_STAGE_PATTERNS = (
    ("external work", "letwork_input"),
    ("compliant boundary", "self.aperture.process"),
    ("active material", "self.active.process"),
    ("ordered transport", "self.transport.process"),
    ("conservative propagation", "self.field.propagate"),
    ("passive loss", "self.field.damp"),
)

REQUIRED_WORLD_STAGE_PATTERNS = (
    ("body-local evolution", ".process_local("),
    ("direct body coupling", "self.couplings.process"),
    ("open shared field", "self.space.begin_frame"),
    ("body/shared-field exchange", "space.exchange_port"),
    ("shared-field propagation", "self.space.finish_frame"),
    ("body radiation", ".radiate("),
    ("global ledger", "self.ledger.push"),
    ("non-loading observation", "self.observer_outputs[index]"),
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


def check_order(
    failures: list[str],
    text: str,
    function_name: str,
    end_marker: str,
    stages: tuple[tuple[str, str], ...],
    label: str,
) -> None:
    start = text.find(function_name)
    end = text.find(end_marker, start + 1)
    if start < 0 or end < 0:
        failures.append(f"canonical {label} does not expose the frozen execution function")
        return
    body = re.sub(r"\s+", "", text[start:end])
    cursor = -1
    for stage, pattern in stages:
        next_cursor = body.find(pattern, cursor + 1)
        if next_cursor < 0:
            failures.append(f"canonical {label} omits {stage!r} ({pattern!r})")
        elif next_cursor <= cursor:
            failures.append(f"canonical {label} order is not frozen at {stage!r}")
        cursor = max(cursor, next_cursor)


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

    body_path = ARIADNE / "src" / "body.rs"
    if body_path.exists():
        body_text = body_path.read_text(encoding="utf-8")
        check_order(
            failures,
            body_text,
            "pub fn process_local",
            "pub fn radiate",
            REQUIRED_BODY_STAGE_PATTERNS,
            "Body local execution",
        )
        if "pub struct Body" not in body_text:
            failures.append("canonical runtime does not expose persistent Body")
        for forbidden in ("WorkLedger", "ReciprocalSpace"):
            if forbidden in body_text:
                failures.append(
                    f"Body improperly owns world-level component {forbidden!r}"
                )

    plan_path = ARIADNE / "src" / "plan.rs"
    if plan_path.exists():
        plan_text = plan_path.read_text(encoding="utf-8")
        for required in (
            "pub struct WorldPlan",
            "pub struct BodyInstanceSpec",
            "pub struct SharedSpaceSpec",
            "pub struct DirectCouplingSpec",
            "pub struct ObserverSpec",
        ):
            if required not in plan_text:
                failures.append(f"WorldPlan IR omits {required!r}")

    world_path = ARIADNE / "src" / "world.rs"
    if world_path.exists():
        world_text = world_path.read_text(encoding="utf-8")
        check_order(
            failures,
            world_text,
            "pub fn process_shared_sample",
            "pub fn receipt",
            REQUIRED_WORLD_STAGE_PATTERNS,
            "Shared World execution",
        )
        if "pub struct World" not in world_text:
            failures.append("canonical runtime does not expose one World")
        if "WorkLedger" not in world_text:
            failures.append("canonical World does not own one global work ledger")
        if "Vec<Body>" not in world_text:
            failures.append("canonical World does not own several Body instances")
        if "Vec<ReciprocalSpace" in world_text:
            failures.append("canonical World gives bodies private shared fields")

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
        if "compat-terminal" in manifest:
            failures.append("Ariadne plug-in still enables the compatibility terminal")
        if re.search(r"^mus-dsp\s*=", manifest, re.MULTILINE):
            failures.append("Ariadne plug-in still depends on mus-dsp")

    plugin_src = PLUGIN / "src"
    if plugin_src.exists():
        plugin_uses_world = False
        for path in rust_sources(plugin_src):
            lowered = path.read_text(encoding="utf-8").lower()
            if re.search(r"\bworld\s*::\s*try_new\b|\bworld\s*<", lowered):
                plugin_uses_world = True
            if (
                "compat::terminal" in lowered
                or "mus_ariadne::ariadne_terminal" in lowered
            ):
                failures.append(
                    f"plug-in still instantiates the compatibility terminal: "
                    f"{path.relative_to(ROOT)}"
                )
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
        if not plugin_uses_world:
            failures.append("Ariadne plug-in does not instantiate canonical World")

    if failures:
        print("Ariadne architecture check failed:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1

    print(
        "Ariadne architecture check passed: one canonical WorldPlan, many local "
        "Bodies, one shared field, one coupling graph, non-loading observers, "
        "and one global ledger."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
