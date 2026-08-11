#!/usr/bin/env python
"""Export NPZ-derived test vectors for the Rust unified-instrument replay.

RUST-API-CONTRACT.md §16.3: "Compare dense matrices and random states
against NPZ-derived test vectors." Rust must not parse NPZ, so this tool
converts the committed operator tables + compiled words into one JSON
file the `mus-watsn` replay tests read. Everything emitted is re-derived
and cross-checked here against the mathematicians' own compiler module
before writing, so the vectors file is self-consistent by construction.

Run from the repo root:
  .venv/bin/python mus-rs/tools/unified_export_test_vectors.py
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
PACKET = ROOT / "paper" / "ariadne-protected-nonabelian"
sys.path.insert(0, str(PACKET))

from unified_reference.compiler import (  # noqa: E402
    ComplexGraph,
    PhaseShift,
    TwoModeUnitary,
    compile_unitary_on_tree,
    schedule_matrix,
)

WORD_TO_TABLE = {
    "A": "loop_a_holonomy",
    "B": "loop_b_holonomy",
    "preparation": "observable_preparation_unitary",
    "readout": "observable_readout_unitary",
}


def cjson(matrix: np.ndarray) -> list:
    matrix = np.asarray(matrix, dtype=np.complex128)
    return [[[float(v.real), float(v.imag)] for v in row] for row in matrix]


def gates_from_schedule(entries: list[dict]):
    gates = []
    for entry in entries:
        if entry["kind"] == "phase_shift":
            gates.append(PhaseShift(entry["i"], entry["phase"]))
        else:
            block = np.array(
                [[complex(re, im) for re, im in row] for row in entry["block"]],
                dtype=np.complex128,
            )
            gates.append(TwoModeUnitary(entry["i"], entry["j"], block))
    return gates


def state_pairs(rng, matrix: np.ndarray, count: int) -> list[dict]:
    n = matrix.shape[0]
    pairs = []
    for _ in range(count):
        state = rng.standard_normal(n) + 1j * rng.standard_normal(n)
        state /= np.linalg.norm(state)
        pairs.append({
            "input": cjson(state.reshape(1, -1))[0],
            "output": cjson((matrix @ state).reshape(1, -1))[0],
        })
    return pairs


def main() -> None:
    words = json.loads((PACKET / "unified_artifacts" / "unified_compiled_words.json").read_text())
    tables = np.load(PACKET / "unified_artifacts" / "unified_operator_tables.npz")
    manifest = json.loads((PACKET / "unified_artifacts" / "unified_manifest.json").read_text())

    rng = np.random.default_rng(0xA71AD9E)
    out: dict = {
        "schema": "ariadne.unified-rust-test-vectors.v1",
        "provenance": {
            "words_schema": words["schema"],
            "gate_law": words["gate_law"],
            "manifest_sha256": {
                name: entry
                for name, entry in manifest.items()
                if isinstance(entry, (str, dict))
            },
        },
        "words": {},
    }

    for word, table in WORD_TO_TABLE.items():
        record = words[word]
        expected = np.asarray(tables[table], dtype=np.complex128)
        replay = schedule_matrix(record["n"], gates_from_schedule(record["schedule"]))
        residual = float(np.linalg.norm(replay - expected, ord="fro"))
        assert residual < 1.0e-12, (word, residual)
        out["words"][word] = {
            "n": record["n"],
            "graph_edges": record["graph_edges"],
            "schedule": record["schedule"],
            "expected_dense": cjson(expected),
            "python_replay_residual_fro": residual,
            "state_pairs": state_pairs(rng, expected, 2),
        }

    # occupied restriction, eigenphases, and the non-Abelian witness
    frame = np.asarray(tables["base_frame"], dtype=np.complex128)
    occupied = {}
    for word in ("A", "B"):
        dense = np.asarray(tables[WORD_TO_TABLE[word]], dtype=np.complex128)
        occ = frame.conj().T @ dense @ frame
        stored = np.asarray(
            tables["loop_a_occupied" if word == "A" else "loop_b_occupied"],
            dtype=np.complex128,
        )
        assert np.linalg.norm(occ - stored, ord="fro") < 1.0e-10
        phases = sorted(float(p) for p in np.angle(np.linalg.eigvals(occ)))
        occupied[word] = {"matrix": cjson(occ), "eigenphases": phases}
    wa = np.array([[complex(re, im) for re, im in row] for row in occupied["A"]["matrix"]])
    wb = np.array([[complex(re, im) for re, im in row] for row in occupied["B"]["matrix"]])
    out["occupied"] = {
        "base_frame": cjson(frame),
        "A": occupied["A"],
        "B": occupied["B"],
        "commutator_fro": float(np.linalg.norm(wa @ wb - wb @ wa, ord="fro")),
    }

    # random U(n) tree-compiler fixtures, dimensions 2..32 (exit gate U1)
    out["random_unitaries"] = []
    for n in (2, 3, 8, 32):
        raw = rng.standard_normal((n, n)) + 1j * rng.standard_normal((n, n))
        target, upper = np.linalg.qr(raw)
        target = target @ np.diag(np.diag(upper) / np.abs(np.diag(upper)))
        graph = ComplexGraph(n, tuple((i, i + 1) for i in range(n - 1)))
        cert = compile_unitary_on_tree(target, graph)
        replay = schedule_matrix(n, cert.schedule)
        assert np.linalg.norm(replay - target, ord="fro") < 1.0e-9
        out["random_unitaries"].append({
            "n": n,
            "graph_edges": [list(edge) for edge in graph.edges],
            "schedule": [gate.to_json_dict() for gate in cert.schedule],
            "expected_dense": cjson(target),
            "state_pairs": state_pairs(rng, target, 1),
        })

    destination = PACKET / "unified_artifacts" / "unified_rust_test_vectors.json"
    destination.write_text(json.dumps(out) + "\n")
    size = destination.stat().st_size
    print(f"{destination}  {size/1024:.0f} KiB  words={list(out['words'])} "
          f"random_n={[r['n'] for r in out['random_unitaries']]} "
          f"commutator_fro={out['occupied']['commutator_fro']:.15f}")


if __name__ == "__main__":
    main()
