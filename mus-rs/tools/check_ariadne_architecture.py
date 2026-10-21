#!/usr/bin/env python3
"""Fail when Ariadne grows a second physical-world architecture.

`mus-ariadne` owns construction plans, local bodies, shared fields, physical
couplings, trajectories, execution, and the one global receipt. Observation is
a separate non-loading program over the World. Host adapters depend on the
canonical crate directly. A musical phenotype may add a BodySpec or gesture
compiler, but not another private World or ledger.

Every file this check reads must be compiled: it is resolved through the crate's
module tree from `src/lib.rs`, so an uncompiled leftover cannot satisfy (or
fail) an assertion about the running code.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORKSPACE = ROOT / "mus-rs" / "Cargo.toml"
ARIADNE = ROOT / "mus-rs" / "crates" / "mus-ariadne"
ARIADNE_SRC = ARIADNE / "src"
DSP = ROOT / "mus-rs" / "crates" / "mus-dsp"
PLUGIN = ROOT / "mus-rs" / "plugins" / "ariadne-clap"

# The canonical layout since the Shared World landing (2026-08-13):
#   plan/          content-addressed WorldPlan IR (was planned as plan.rs)
#   shared_world/  the one World and its frozen execution order (the one-body
#                  world.rs left the module tree in 420d1d6)
#   shared_space/  the one shared field (was space.rs)
#   shared_trajectory/  typed body/world trajectory binding (was trajectory.rs)
#   observation/   non-loading listener and typed observation programs
REQUIRED_CANONICAL_FILES = (
    "Cargo.toml",
    "README.md",
    "src/lib.rs",
    "src/ir.rs",
    "src/plan/mod.rs",
    "src/plan/types.rs",
    "src/body.rs",
    "src/constitutive_body.rs",
    "src/coupling.rs",
    "src/shared_world/mod.rs",
    "src/shared_world/build.rs",
    "src/shared_world/process.rs",
    "src/field.rs",
    "src/shared_space/mod.rs",
    "src/shared_trajectory/mod.rs",
    "src/observation/mod.rs",
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

# Every World entrypoint runs body-local evolution for all bodies, then hands
# the frame to the one shared tail below.
REQUIRED_WORLD_ENTRY_STAGE_PATTERNS = (
    ("body-local evolution", ".process_local("),
    ("shared world tail", "self.finish_after_local("),
)

REQUIRED_WORLD_STAGE_PATTERNS = (
    ("direct body coupling", "self.couplings.process"),
    ("open shared field", "self.space.begin_frame"),
    ("body/shared-field exchange", "space.exchange_port"),
    ("shared-field propagation", "self.space.finish_frame"),
    ("body radiation", ".radiate("),
    ("global ledger", "self.ledger.push"),
)

# The tail for a world whose bodies all have inert shared-field ports: coupling
# has already run in `finish_after_local`; the field is untouched.
REQUIRED_WORLD_INERT_SPACE_STAGE_PATTERNS = (
    ("body radiation", ".radiate("),
    ("global ledger", "self.ledger.push"),
)

WORLD_ENTRYPOINTS = (
    ("pub fn process_sample", "pub fn process_constitutive_sample"),
    ("pub fn process_constitutive_sample", "fn finish_after_local"),
)

# Every body constitution a World may hold (`WorldBody`), none of which may own
# a world-level component.
BODY_CONSTITUTION_FILES = (
    "src/body.rs",
    "src/space_body.rs",
    "src/constitutive_body.rs",
    "src/constitutive_body/local_medium.rs",
    "src/constitutive_body/world_body.rs",
)
WORLD_LEVEL_COMPONENTS = ("WorkLedger", "SharedSpace", "ReciprocalSpace")

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
MOD_DECLARATION = re.compile(
    r"^[ \t]*(?:#\[path\s*=\s*\"(?P<path>[^\"]+)\"\]\s*)?"
    r"(?:pub(?:\([^)]*\))?\s+)?mod\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*;",
    re.MULTILINE,
)
INCLUDE = re.compile(r"\binclude!\(\s*\"([^\"]+)\"\s*\)")


def rust_sources(directory: Path) -> list[Path]:
    return sorted(path for path in directory.rglob("*.rs") if path.is_file())


def without_comments(text: str) -> str:
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.DOTALL)
    return re.sub(r"//[^\n]*", "", text)


def compiled_sources(root: Path) -> set[Path]:
    """Files reachable from the crate root through `mod` and `include!`."""
    compiled: set[Path] = set()
    pending = [root.resolve()]
    while pending:
        path = pending.pop()
        if path in compiled or not path.is_file():
            continue
        compiled.add(path)
        text = without_comments(path.read_text(encoding="utf-8"))
        if path.name in ("lib.rs", "main.rs", "mod.rs"):
            child_dir = path.parent
        else:
            child_dir = path.parent / path.stem
        for match in MOD_DECLARATION.finditer(text):
            if match.group("path"):
                pending.append((path.parent / match.group("path")).resolve())
                continue
            name = match.group("name")
            for candidate in (child_dir / f"{name}.rs", child_dir / name / "mod.rs"):
                if candidate.is_file():
                    pending.append(candidate.resolve())
                    break
        for included in INCLUDE.findall(text):
            pending.append((path.parent / included).resolve())
    return compiled


def fail_if_missing(failures: list[str], path: Path, label: str) -> None:
    if not path.exists():
        failures.append(f"{label}: {path.relative_to(ROOT)}")


def item_block(text: str, header: str) -> str | None:
    """The brace-delimited body following the first occurrence of `header`."""
    start = text.find(header)
    if start < 0:
        return None
    open_brace = text.find("{", start)
    if open_brace < 0:
        return None
    depth = 0
    for index in range(open_brace, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return text[open_brace + 1 : index]
    return None


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
        failures.append(
            f"canonical {label} does not expose the frozen execution function "
            f"{function_name!r}"
        )
        return
    body = re.sub(r"\s+", "", text[start:end])
    # Each stage's FIRST occurrence must follow the previous stage's first
    # occurrence; a stage that also runs early (say, a radiation before the
    # shared field is finished) breaks the order even if it recurs later.
    cursor = -1
    for stage, pattern in stages:
        next_cursor = body.find(pattern)
        if next_cursor < 0:
            failures.append(f"canonical {label} omits {stage!r} ({pattern!r})")
            continue
        if next_cursor <= cursor:
            failures.append(f"canonical {label} order is not frozen at {stage!r}")
        cursor = max(cursor, next_cursor)


def main() -> int:
    failures: list[str] = []

    fail_if_missing(failures, WORKSPACE, "workspace manifest is missing")
    if WORKSPACE.exists() and '"crates/mus-ariadne"' not in WORKSPACE.read_text(
        encoding="utf-8"
    ):
        failures.append("workspace does not contain crates/mus-ariadne")

    compiled = compiled_sources(ARIADNE_SRC / "lib.rs")

    def canonical(relative: str) -> Path | None:
        """A required source, or None after recording why it cannot be read."""
        path = ARIADNE / relative
        if not path.exists():
            return None
        if path.suffix == ".rs" and path.resolve() not in compiled:
            failures.append(
                "canonical Ariadne file is not in the crate's module tree: "
                f"{path.relative_to(ROOT)}"
            )
            return None
        return path

    for relative in REQUIRED_CANONICAL_FILES:
        fail_if_missing(
            failures,
            ARIADNE / relative,
            "canonical Ariadne file is missing",
        )
        canonical(relative)

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
        for path in rust_sources(ARIADNE_SRC)
        if "compat" not in path.relative_to(ARIADNE_SRC).parts
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

    body_path = canonical("src/body.rs")
    if body_path is not None:
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

    for relative in BODY_CONSTITUTION_FILES:
        path = ARIADNE / relative
        if not path.exists() or path.resolve() not in compiled:
            continue
        body_text = without_comments(path.read_text(encoding="utf-8"))
        for forbidden in WORLD_LEVEL_COMPONENTS:
            if re.search(rf"\b{forbidden}\b", body_text):
                failures.append(
                    f"Body improperly owns world-level component {forbidden!r}: "
                    f"{path.relative_to(ROOT)}"
                )

    plan_sources = [
        path
        for path in rust_sources(ARIADNE_SRC / "plan")
        if path.resolve() in compiled
    ]
    if plan_sources:
        plan_text = "\n".join(path.read_text(encoding="utf-8") for path in plan_sources)
        for required in (
            "pub struct WorldPlan",
            "pub struct BodyInstanceSpec",
            "pub struct SharedSpaceSpec",
            "pub struct DirectCouplingSpec",
        ):
            if required not in plan_text:
                failures.append(f"WorldPlan IR omits {required!r}")
        # The physical plan carries no observers (2026-08-13, "Make WorldPlan
        # purely physical"): observation binds to the plan by digest and cannot
        # change the identity of the physical world.
        if re.search(r"\bpub\s+struct\s+Observer", without_comments(plan_text)):
            failures.append("WorldPlan IR owns an observer; observation must bind by digest")

    observation_sources = [
        path
        for path in rust_sources(ARIADNE_SRC / "observation")
        if path.resolve() in compiled
    ]
    observation_text = "\n".join(
        without_comments(path.read_text(encoding="utf-8")) for path in observation_sources
    )
    for required in ("pub struct StereoListenerSpec", "pub struct WorldObservationProgram"):
        if required not in observation_text:
            failures.append(f"observation layer omits {required!r}")
    if not re.search(r"\bworld\s*:\s*&\s*World\b", observation_text):
        failures.append("observation layer does not read the World by shared reference")
    if re.search(r"&\s*mut\s+(?:crate::)?World\b", observation_text):
        failures.append("observation layer takes &mut World; observers must be non-loading")

    world_path = canonical("src/shared_world/mod.rs")
    if world_path is not None:
        world_text = without_comments(world_path.read_text(encoding="utf-8"))
        world_fields = item_block(world_text, "pub struct World")
        if world_fields is None:
            failures.append("canonical runtime does not expose one World")
        else:
            if not re.search(r":\s*WorkLedger\s*,", world_fields):
                failures.append("canonical World does not own one global work ledger")
            if not re.search(r":\s*Vec<\s*WorldBody\s*>", world_fields):
                failures.append("canonical World does not own several Body instances")
            if len(re.findall(r":\s*SharedSpace\s*,", world_fields)) != 1:
                failures.append("canonical World does not own exactly one shared field")
            if re.search(r"Vec<\s*(?:SharedSpace|ReciprocalSpace)", world_fields):
                failures.append("canonical World gives bodies private shared fields")
            if re.search(r"observer|listener", world_fields, re.IGNORECASE):
                failures.append("canonical World embeds an observer")

    world_body_path = canonical("src/constitutive_body.rs")
    if world_body_path is not None:
        variants = item_block(
            without_comments(world_body_path.read_text(encoding="utf-8")),
            "enum WorldBody",
        )
        if variants is None or not re.search(r"\bModal\s*\(\s*Body\s*\)", variants):
            failures.append("canonical WorldBody does not hold the persistent modal Body")

    process_path = canonical("src/shared_world/process.rs")
    if process_path is not None:
        process_text = without_comments(process_path.read_text(encoding="utf-8"))
        for function_name, end_marker in WORLD_ENTRYPOINTS:
            check_order(
                failures,
                process_text,
                function_name,
                end_marker,
                REQUIRED_WORLD_ENTRY_STAGE_PATTERNS,
                "Shared World entry",
            )
        check_order(
            failures,
            process_text,
            "fn finish_after_local",
            "fn finish_without_space",
            REQUIRED_WORLD_STAGE_PATTERNS,
            "Shared World execution",
        )
        tail_start = process_text.find("fn finish_after_local")
        compact_tail = re.sub(r"\s+", "", process_text[tail_start:])
        coupling = compact_tail.find("self.couplings.process")
        inert_branch = compact_tail.find("self.finish_without_space(")
        if inert_branch >= 0 and not 0 <= coupling < inert_branch:
            failures.append(
                "canonical Shared World skips direct body coupling on the inert-field path"
            )
        check_order(
            failures,
            process_text,
            "fn finish_without_space",
            "pub fn receipt",
            REQUIRED_WORLD_INERT_SPACE_STAGE_PATTERNS,
            "Shared World inert-field execution",
        )
        inert_start = process_text.find("fn finish_without_space")
        inert_end = process_text.find("pub fn receipt", inert_start + 1)
        if inert_start >= 0 and "self.space." in process_text[inert_start:inert_end]:
            failures.append("canonical Shared World inert-field path touches the shared field")
        if re.search(r"observer|listener", process_text, re.IGNORECASE):
            failures.append("canonical Shared World execution embeds an observer")

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
            lowered = without_comments(path.read_text(encoding="utf-8")).lower()
            # The canonical World is constructed from a plan (shared_world/build.rs).
            # `World::try_new(WorldSpec)` was the one-body executor of world.rs.
            imports_world = re.search(
                r"\buse\s+mus_ariadne::(?:\{[^}]*\bworld\b[^}]*\}|world\s*;)", lowered
            )
            constructs_world = re.search(
                r"\bworld\s*::\s*from_(?:constitutive_)?plan\s*\(", lowered
            )
            if imports_world and constructs_world:
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

    uncompiled = [
        path.relative_to(ROOT)
        for path in rust_sources(ARIADNE_SRC)
        if path.resolve() not in compiled
    ]

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
    if uncompiled:
        print(
            "note: these mus-ariadne sources are outside the module tree and were "
            "not checked: " + ", ".join(str(path) for path in uncompiled)
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
