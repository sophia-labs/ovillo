"""Graph-constrained U(n) compiler for physically lawful complex-mode gates.

Unlike the existing SO(2n) tree-QR lowering, every intermediate factor here is
complex linear.  Its realification is therefore both orthogonal and symplectic,
which is the required law for oscillator quadrature pairs.
"""

from __future__ import annotations

from collections import deque
from dataclasses import asdict, dataclass
import math
from typing import Sequence, TypeAlias

import numpy as np
from numpy.typing import NDArray

from .model import complex_structure, realify, unitary_law_residuals

ComplexMatrix = NDArray[np.complex128]
Edge = tuple[int, int]


def canonical_edge(i: int, j: int) -> Edge:
    if i == j:
        raise ValueError("self edges are invalid")
    return (i, j) if i < j else (j, i)


@dataclass(frozen=True)
class ComplexGraph:
    n: int
    edges: tuple[Edge, ...]

    def __post_init__(self) -> None:
        if self.n < 1:
            raise ValueError("graph dimension must be positive")
        normalized = tuple(sorted({canonical_edge(*edge) for edge in self.edges}))
        object.__setattr__(self, "edges", normalized)
        for i, j in normalized:
            if not (0 <= i < self.n and 0 <= j < self.n):
                raise ValueError("graph edge is out of range")

    @property
    def adjacency(self) -> tuple[frozenset[int], ...]:
        rows = [set() for _ in range(self.n)]
        for i, j in self.edges:
            rows[i].add(j)
            rows[j].add(i)
        return tuple(frozenset(row) for row in rows)

    def is_connected(self) -> bool:
        seen = {0}
        queue = deque([0])
        while queue:
            node = queue.popleft()
            for neighbor in self.adjacency[node]:
                if neighbor not in seen:
                    seen.add(neighbor)
                    queue.append(neighbor)
        return len(seen) == self.n

    def bfs_spanning_tree(self, root: int = 0) -> tuple[Edge, ...]:
        if not self.is_connected():
            raise ValueError("graph is disconnected")
        seen = {root}
        queue = deque([root])
        tree: list[Edge] = []
        while queue:
            node = queue.popleft()
            for neighbor in sorted(self.adjacency[node]):
                if neighbor not in seen:
                    seen.add(neighbor)
                    queue.append(neighbor)
                    tree.append(canonical_edge(node, neighbor))
        return tuple(tree)


@dataclass(frozen=True)
class TwoModeUnitary:
    i: int
    j: int
    block: ComplexMatrix

    def __post_init__(self) -> None:
        block = np.asarray(self.block, dtype=np.complex128)
        if block.shape != (2, 2):
            raise ValueError("two-mode block must be 2x2")
        if np.linalg.norm(block.conj().T @ block - np.eye(2), ord="fro") > 1.0e-9:
            raise ValueError("two-mode block must be unitary")
        object.__setattr__(self, "block", block)

    @property
    def edge(self) -> Edge:
        return canonical_edge(self.i, self.j)

    def inverse(self) -> "TwoModeUnitary":
        return TwoModeUnitary(self.i, self.j, self.block.conj().T)

    def to_json_dict(self) -> dict:
        return {
            "kind": "two_mode_unitary",
            "i": self.i,
            "j": self.j,
            "block": complex_matrix_json(self.block),
        }


@dataclass(frozen=True)
class PhaseShift:
    i: int
    phase: float

    def inverse(self) -> "PhaseShift":
        return PhaseShift(self.i, -self.phase)

    def to_json_dict(self) -> dict:
        return {"kind": "phase_shift", "i": self.i, "phase": self.phase}


ComplexGate: TypeAlias = TwoModeUnitary | PhaseShift


def complex_matrix_json(matrix: ComplexMatrix) -> list[list[list[float]]]:
    matrix = np.asarray(matrix, dtype=np.complex128)
    return [
        [[float(value.real), float(value.imag)] for value in row]
        for row in matrix
    ]


def embedded_gate(n: int, gate: ComplexGate) -> ComplexMatrix:
    out = np.eye(n, dtype=np.complex128)
    if isinstance(gate, PhaseShift):
        if not (0 <= gate.i < n):
            raise ValueError("phase index out of range")
        out[gate.i, gate.i] = np.exp(1.0j * gate.phase)
        return out
    if not (0 <= gate.i < n and 0 <= gate.j < n and gate.i != gate.j):
        raise ValueError("two-mode gate index out of range")
    indices = (gate.i, gate.j)
    out[np.ix_(indices, indices)] = gate.block
    return out


def schedule_matrix(n: int, schedule: Sequence[ComplexGate]) -> ComplexMatrix:
    total = np.eye(n, dtype=np.complex128)
    for gate in schedule:
        total = embedded_gate(n, gate) @ total
    return total


def elimination_block(a: complex, b: complex) -> ComplexMatrix:
    """SU(2) block sending [a,b]^T to [sqrt(|a|^2+|b|^2),0]^T."""
    radius = math.sqrt(abs(a) ** 2 + abs(b) ** 2)
    if radius <= 1.0e-30:
        return np.eye(2, dtype=np.complex128)
    return np.array(
        [[np.conj(a) / radius, np.conj(b) / radius], [-b / radius, a / radius]],
        dtype=np.complex128,
    )


def _tree_adjacency(n: int, tree_edges: Sequence[Edge]) -> list[set[int]]:
    if len(tree_edges) != n - 1:
        raise ValueError("a spanning tree must contain n-1 edges")
    rows = [set() for _ in range(n)]
    for i, j in tree_edges:
        rows[i].add(j)
        rows[j].add(i)
    seen = {0}
    queue = deque([0])
    while queue:
        node = queue.popleft()
        for neighbor in rows[node]:
            if neighbor not in seen:
                seen.add(neighbor)
                queue.append(neighbor)
    if len(seen) != n:
        raise ValueError("tree is disconnected")
    return rows


def _leaf_order(n: int, tree_edges: Sequence[Edge]) -> tuple[int, ...]:
    adjacency = _tree_adjacency(n, tree_edges)
    alive = set(range(n))
    order: list[int] = []
    while len(alive) > 1:
        leaves = sorted(node for node in alive if len(adjacency[node] & alive) == 1)
        if not leaves:
            raise RuntimeError("tree lost leaf invariant")
        leaf = leaves[0]
        order.append(leaf)
        alive.remove(leaf)
    order.extend(alive)
    return tuple(order)


def _elimination_edges_to_root(
    alive: set[int], adjacency: Sequence[set[int]], root: int
) -> list[tuple[int, int]]:
    active = set(alive)
    edges: list[tuple[int, int]] = []
    while len(active) > 1:
        leaves = sorted(
            node
            for node in active
            if node != root and len(adjacency[node] & active) == 1
        )
        if not leaves:
            raise RuntimeError("no removable non-root leaf")
        leaf = leaves[0]
        parent = next(iter(adjacency[leaf] & active))
        edges.append((parent, leaf))
        active.remove(leaf)
    return edges


@dataclass(frozen=True)
class ComplexCompileCertificate:
    n: int
    graph_edges: tuple[Edge, ...]
    tree_edges: tuple[Edge, ...]
    leaf_order: tuple[int, ...]
    two_mode_gate_count: int
    phase_gate_count: int
    reconstruction_error_fro: float
    unitarity_error_fro: float
    real_orthogonality_error_fro: float
    real_symplectic_error_fro: float
    maximum_factor_unitarity_error_fro: float
    maximum_factor_symplectic_error_fro: float
    all_two_mode_gates_on_graph: bool
    schedule: tuple[ComplexGate, ...]

    def to_json_dict(self) -> dict:
        data = asdict(self)
        data["schedule"] = [gate.to_json_dict() for gate in self.schedule]
        return data


def compile_unitary_on_tree(
    target: ComplexMatrix,
    graph: ComplexGraph,
    *,
    tree_edges: Sequence[Edge] | None = None,
    tolerance: float = 1.0e-9,
) -> ComplexCompileCertificate:
    """Exact graph-constrained U(n) factorization.

    The elimination is the complex analogue of the repository's real tree-QR
    compiler.  It emits n(n-1)/2 graph-edge SU(2) blocks followed by at most n
    local phase shifts.  Every realified factor lies in O(2n) intersect Sp(2n,R).
    """
    target = np.asarray(target, dtype=np.complex128)
    if target.shape != (graph.n, graph.n):
        raise ValueError("target shape does not match graph")
    if np.linalg.norm(target.conj().T @ target - np.eye(graph.n), ord="fro") > tolerance:
        raise ValueError("target is not unitary within tolerance")
    if not graph.is_connected():
        raise ValueError("disconnected graph cannot realize arbitrary U(n)")

    tree = tuple(tree_edges or graph.bfs_spanning_tree())
    adjacency = _tree_adjacency(graph.n, tree)
    order = _leaf_order(graph.n, tree)
    alive = set(range(graph.n))
    reduced = target.copy()
    elimination: list[TwoModeUnitary] = []

    for root in order[:-1]:
        column = root
        for parent, leaf in _elimination_edges_to_root(alive, adjacency, root):
            block = elimination_block(reduced[parent, column], reduced[leaf, column])
            gate = TwoModeUnitary(parent, leaf, block)
            reduced = embedded_gate(graph.n, gate) @ reduced
            elimination.append(gate)
        alive.remove(root)

    off_diagonal = reduced - np.diag(np.diag(reduced))
    if np.linalg.norm(off_diagonal, ord="fro") > 100.0 * tolerance:
        raise RuntimeError("complex tree elimination did not reduce target to phases")
    diagonal = np.diag(reduced)
    if np.max(np.abs(np.abs(diagonal) - 1.0)) > 100.0 * tolerance:
        raise RuntimeError("reduced diagonal is not unit modulus")
    phases = tuple(PhaseShift(index, float(np.angle(value))) for index, value in enumerate(diagonal))
    schedule: tuple[ComplexGate, ...] = phases + tuple(
        gate.inverse() for gate in reversed(elimination)
    )
    replay = schedule_matrix(graph.n, schedule)
    laws = unitary_law_residuals(replay)

    maximum_factor_unitarity = 0.0
    maximum_factor_symplectic = 0.0
    for gate in schedule:
        factor_laws = unitary_law_residuals(embedded_gate(graph.n, gate))
        maximum_factor_unitarity = max(
            maximum_factor_unitarity, factor_laws["complex_unitarity_fro"]
        )
        maximum_factor_symplectic = max(
            maximum_factor_symplectic, factor_laws["real_symplectic_fro"]
        )

    edge_set = set(graph.edges)
    expected_two_mode = graph.n * (graph.n - 1) // 2
    if len(elimination) != expected_two_mode:
        raise AssertionError("complex tree-QR gate-count invariant failed")
    return ComplexCompileCertificate(
        n=graph.n,
        graph_edges=graph.edges,
        tree_edges=tuple(sorted(tree)),
        leaf_order=order,
        two_mode_gate_count=len(elimination),
        phase_gate_count=len(phases),
        reconstruction_error_fro=float(np.linalg.norm(replay - target, ord="fro")),
        unitarity_error_fro=laws["complex_unitarity_fro"],
        real_orthogonality_error_fro=laws["real_orthogonality_fro"],
        real_symplectic_error_fro=laws["real_symplectic_fro"],
        maximum_factor_unitarity_error_fro=float(maximum_factor_unitarity),
        maximum_factor_symplectic_error_fro=float(maximum_factor_symplectic),
        all_two_mode_gates_on_graph=all(gate.edge in edge_set for gate in elimination),
        schedule=schedule,
    )


def path_graph(n: int) -> ComplexGraph:
    return ComplexGraph(n=n, edges=tuple((index, index + 1) for index in range(n - 1)))


def complete_graph(n: int) -> ComplexGraph:
    return ComplexGraph(
        n=n,
        edges=tuple((i, j) for i in range(n) for j in range(i + 1, n)),
    )


def random_unitary(n: int, rng: np.random.Generator) -> ComplexMatrix:
    matrix = rng.normal(size=(n, n)) + 1.0j * rng.normal(size=(n, n))
    q, r = np.linalg.qr(matrix)
    phases = np.diag(r)
    phases = np.where(np.abs(phases) > 0.0, phases / np.abs(phases), 1.0)
    return q @ np.diag(np.conj(phases))
