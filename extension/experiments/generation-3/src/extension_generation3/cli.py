"""Command-line entry point for the Generation 3 construction proof."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from .reference import ReferenceMedium
from .train import train_construction, write_result


def main() -> None:
    parser = argparse.ArgumentParser(prog="extension-generation3")
    subparsers = parser.add_subparsers(dest="command", required=True)

    train = subparsers.add_parser("train", help="train and compile the construction proof")
    train.add_argument("--seed", type=int, default=20260814)
    train.add_argument("--steps", type=int, default=240)
    train.add_argument("--batch-size", type=int, default=256)
    train.add_argument("--learning-rate", type=float, default=0.025)
    train.add_argument("--device", default="cpu")
    train.add_argument("--output", type=Path, default=Path("runs/construction"))

    replay = subparsers.add_parser("replay", help="replay a compiled fixture")
    replay.add_argument("fixture", type=Path)

    args = parser.parse_args()
    if args.command == "train":
        result = train_construction(
            seed=args.seed,
            steps=args.steps,
            batch_size=args.batch_size,
            learning_rate=args.learning_rate,
            device=args.device,
        )
        write_result(result, args.output)
        print(json.dumps(result.receipt, indent=2, sort_keys=True))
        return

    fixture = json.loads(args.fixture.read_text())
    medium = ReferenceMedium(fixture["plan"])
    cumulative_source = 0.0
    cumulative_loss = 0.0
    for step in fixture["steps"]:
        if "edge_enabled" in step:
            medium.set_edge_enabled(
                int(step["edge_enabled"]["edge"]),
                bool(step["edge_enabled"]["enabled"]),
            )
        receipt = medium.step(step.get("drives", []))
        cumulative_source += receipt.source_work
        cumulative_loss += receipt.internal_loss
    print(
        json.dumps(
            {
                "state": medium.state_dict(),
                "stored_energy": medium.stored_energy(),
                "positive_mass": medium.positive_mass(),
                "cumulative_source_work": cumulative_source,
                "cumulative_internal_loss": cumulative_loss,
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
