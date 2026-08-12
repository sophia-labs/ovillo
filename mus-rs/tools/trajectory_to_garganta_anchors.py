#!/usr/bin/env python
"""Validate a trajectory JSON and print its current GARGANTA anchor lowering."""

from __future__ import annotations

import argparse
import json

from mus_analysis.garganta_trajectory import trajectory_to_anchor_genome
from mus_analysis.trajectory import TrajectoryProgram


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("program")
    args = parser.parse_args()
    program = TrajectoryProgram.load(args.program)
    genome = trajectory_to_anchor_genome(program)
    print(
        json.dumps(
            {
                "program_sha256": program.digest_hex(),
                "shape": list(genome.shape),
                "anchors": genome.tolist(),
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
