from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import numpy as np


def load(path: Path) -> Any:
    return json.loads(path.read_text())


def build(root: Path, output: Path) -> dict[str, Any]:
    atlas = load(root / "runs/atlas_summary.json")
    projective = load(root / "runs/projective_summary.json")
    balanced = load(root / "runs/balanced_summary.json")
    attention = load(root / "runs/attention_summary.json")
    mixture = load(root / "runs/mixture_summary.json")
    compartmental = load(root / "runs/compartmental/result.json")
    search100 = load(root / "runs/projective_search/n100k_s7.json")
    search500 = load(root / "runs/projective_search/n500k_s7.json")

    random_gap = projective["conditions"]["random"]["mean_excess_bps"]
    nmf_gap = projective["conditions"]["nmf"]["mean_excess_bps"]
    reduction = 1.0 - nmf_gap / random_gap

    stream_rows = []
    for path in sorted((root / "runs/projective_stream").glob("*.json")):
        row = load(path)
        stream_rows.append(
            {
                "stream_length": row["stream_length"],
                "initial_excess_bps": row["initial"]["excess_bps"],
                "factorization_error": row["factorization"]["nmf_reconstruction_error"],
                "mean_shift_residual": float(np.mean(row["factorization"]["shift_relative_residuals"])),
            }
        )
    # Add the separately run 500k initialization if not already represented.
    row500 = load(root / "runs/projective/nmf500k_s7.json")
    if not any(row["stream_length"] == 500_000 for row in stream_rows):
        stream_rows.append(
            {
                "stream_length": 500_000,
                "initial_excess_bps": row500["initial"]["excess_bps"],
                "factorization_error": row500["factorization"]["nmf_reconstruction_error"],
                "mean_shift_residual": float(np.mean(row500["factorization"]["shift_relative_residuals"])),
            }
        )
    stream_rows.sort(key=lambda row: row["stream_length"])

    payload = {
        "predictive_atlas": {
            "process_count": len(atlas["processes"]),
            "model_run_count": len(atlas["models"]),
            "correlations": atlas["correlations"],
            "strongest_within_process_metric": max(
                atlas["correlations"].items(),
                key=lambda item: item[1]["within_process_pairwise_concordance"],
            )[0],
        },
        "projective_initialization": {
            "conditions": projective["conditions"],
            "random_to_nmf_excess_bps_reduction_fraction": reduction,
            "stream_sweep": stream_rows,
            "search_100k": {
                "selected_config": search100["selected_config"],
                "candidate_count": search100["candidate_count"],
                "initial_excess_bps": search100["initial"]["excess_bps"],
                "final_excess_bps": search100["final"]["excess_bps"],
            },
            "search_500k": {
                "selected_config": search500["selected_config"],
                "candidate_count": search500["candidate_count"],
                "initial_excess_bps": search500["initial"]["excess_bps"],
                "final_excess_bps": search500["final"]["excess_bps"],
            },
        },
        "balanced_training": balanced["conditions"],
        "attention": {
            "rank_fit": attention["functional_rank_vs_min_heads_routes"],
            "collision_fit": attention["collision_emergence_fit"],
            "phase_rows": attention["phase_rows"],
        },
        "transport_mixture": mixture["conditions"],
        "compartmental": {
            "spec": compartmental["spec"],
            "oracle_bps": compartmental["oracle_bps"],
            "rows": compartmental["rows"],
        },
    }
    output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    print(json.dumps(payload, indent=2)[:100_000])
    return payload


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    build(args.root, args.output)


if __name__ == "__main__":
    main()
