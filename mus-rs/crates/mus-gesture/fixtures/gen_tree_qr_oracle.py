"""Emit the Rust replay fixture for the exact tree-QR rotation compiler.

`paper/ariadne-gesture-compiler/reference/results.json` is the reference
package's own experiment receipt: it records aggregate invariants
(gate-count mismatches, worst reconstruction error per dimension) but no
per-target schedules, so it cannot by itself drive a cross-implementation
replay of the compiler.  This script closes that gap.  It calls the same
independent float64 oracle
(`paper/ariadne-gesture-compiler/reference/ariadne_gesture/core.py`) and
dumps, per case, everything `mus-gesture` needs to recompute the identical
word in Rust:

    authored graph -> spanning tree -> leaf order -> SO(n) target
        -> gate sequence with angles -> thermodynamic timing

Both implementations use the *deterministic* branch of the oracle
(`rng=None`: minimum-index leaf at every peel), so the elimination order is
fixed by the tree alone and the two languages must agree gate-for-gate, with
angles differing only by float64 rounding.

Usage (from anywhere; paths are resolved relative to this file):

    <repo>/.venv/bin/python mus-rs/crates/mus-gesture/fixtures/gen_tree_qr_oracle.py

Writes `tree_qr_oracle.json` next to this script.
"""

from __future__ import annotations

from pathlib import Path
import json
import math
import sys

import numpy as np

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
REFERENCE = REPO / "paper" / "ariadne-gesture-compiler" / "reference"
sys.path.insert(0, str(REFERENCE))

from ariadne_gesture import (  # noqa: E402
    Graph,
    canonical_edge,
    compile_so_on_tree,
    random_so,
    thermodynamic_schedule,
)

SEED = 0x6E5751E
SCHEMA = "mus-gesture/tree-qr-oracle/1"

# Full family sweep up to the dimension the reference experiments themselves
# reach; a single random-connected case per dimension from 17 to 32, which is
# where the Rust gates go beyond the Python oracle's own range.
FULL_FAMILY_MAX = 16
MAX_DIMENSION = 32


def path_graph(n: int) -> Graph:
    return Graph(n=n, edges=tuple(canonical_edge(i, i + 1) for i in range(n - 1)))


def star_graph(n: int) -> Graph:
    return Graph(n=n, edges=tuple(canonical_edge(0, i) for i in range(1, n)))


def balanced_tree_graph(n: int) -> Graph:
    return Graph(
        n=n, edges=tuple(canonical_edge((i - 1) // 2, i) for i in range(1, n))
    )


def complete_graph(n: int) -> Graph:
    return Graph(
        n=n,
        edges=tuple(
            canonical_edge(i, j) for i in range(n) for j in range(i + 1, n)
        ),
    )


def random_connected_graph(n: int, rng: np.random.Generator) -> Graph:
    """Random recursive tree plus sparse extra edges (as run_experiments.py)."""
    edges: set[tuple[int, int]] = set()
    for node in range(1, n):
        parent = int(rng.integers(0, node))
        edges.add(canonical_edge(parent, node))
    for i in range(n):
        for j in range(i + 1, n):
            if rng.random() < 0.18:
                edges.add(canonical_edge(i, j))
    return Graph(n=n, edges=tuple(sorted(edges)))


FAMILIES = {
    "path": path_graph,
    "star": star_graph,
    "balanced_tree": balanced_tree_graph,
    "complete": complete_graph,
}


def case(name: str, family: str, graph: Graph, target, rng) -> dict:
    certificate = compile_so_on_tree(target, graph)
    friction = {
        edge: float(math.exp(rng.uniform(-1.2, 1.2))) for edge in graph.edges
    }
    duration = 2.0
    thermo = thermodynamic_schedule(
        certificate.schedule, edge_friction=friction, duration=duration
    )
    assert certificate.all_gates_on_graph
    assert certificate.gate_count == graph.n * (graph.n - 1) // 2
    return {
        "name": name,
        "family": family,
        "n": graph.n,
        "graph_edges": [list(edge) for edge in graph.edges],
        "edge_friction": [
            [edge[0], edge[1], friction[edge]] for edge in graph.edges
        ],
        "tree_edges": [list(edge) for edge in certificate.tree_edges],
        "leaf_order": list(certificate.leaf_order),
        "target": [[float(value) for value in row] for row in target],
        "gate_count": certificate.gate_count,
        "schedule": [
            {"i": gate.i, "j": gate.j, "theta": gate.theta}
            for gate in certificate.schedule
        ],
        "reconstruction_error_fro": certificate.reconstruction_error_fro,
        "orthogonality_error_fro": certificate.orthogonality_error_fro,
        "determinant_error": certificate.determinant_error,
        "duration": duration,
        "thermodynamic_length": thermo.thermodynamic_length,
        "minimum_quadratic_cost": thermo.minimum_quadratic_cost,
        "segment_durations": list(thermo.segment_durations),
        "constant_speed_residual": thermo.constant_speed_residual,
    }


def main() -> None:
    rng = np.random.default_rng(SEED)
    cases: list[dict] = []

    for n in range(2, MAX_DIMENSION + 1):
        target = random_so(n, rng)
        if n <= FULL_FAMILY_MAX:
            for family, build in FAMILIES.items():
                cases.append(
                    case(f"n{n:02d}-{family}", family, build(n), target, rng)
                )
        cases.append(
            case(
                f"n{n:02d}-random_connected",
                "random_connected",
                random_connected_graph(n, rng),
                target,
                rng,
            )
        )

    # One deliberately degenerate case: the identity target must compile to an
    # all-zero word with zero thermodynamic length.
    identity_graph = path_graph(6)
    cases.append(
        case("n06-path-identity", "path", identity_graph, np.eye(6), rng)
    )

    payload = {
        "schema": SCHEMA,
        "seed": SEED,
        "oracle": "paper/ariadne-gesture-compiler/reference/ariadne_gesture",
        "oracle_branch": "deterministic (rng=None, minimum-index leaf peel)",
        "numpy": np.__version__,
        "case_count": len(cases),
        "cases": cases,
    }
    out = HERE / "tree_qr_oracle.json"
    out.write_text(json.dumps(payload, indent=1, sort_keys=True) + "\n")
    print(f"wrote {out} ({out.stat().st_size} bytes, {len(cases)} cases)")


if __name__ == "__main__":
    main()
