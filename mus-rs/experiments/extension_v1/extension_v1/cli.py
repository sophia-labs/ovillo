"""Command-line entry point for Extension-v1 experiments."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import torch

from .active_cable import ActiveCableConfig, train_active_cable
from .clifford import CliffordConfig, exact_receipt
from .closed_gestures import (
    ClosedGestureConfig,
    commuting_control,
    seed_sweep,
    train_closed_gestures,
)
from .development import DevelopmentConfig, development_receipt
from .geometry import GeometryConfig, dimension_sweep, geometry_receipt
from .morphology import MorphologyConfig, optimize_morphology
from .receipts import write_receipt


def emit(value: dict[str, Any], output: Path | None) -> None:
    if output is not None:
        write_receipt(output, value)
    print(json.dumps(value, indent=2, sort_keys=True))


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(
        prog="extension-v1",
        description="Gestural sequence computation in Clifford string media",
    )
    sub = root.add_subparsers(dest="command", required=True)

    exact = sub.add_parser("exact", help="run exact Clifford algebra checks")
    exact.add_argument("--trials", type=int, default=4096)
    exact.add_argument("--seed", type=int, default=19)
    exact.add_argument("--output", type=Path)

    development = sub.add_parser(
        "development", help="run closed-path development experiments"
    )
    development.add_argument("--output", type=Path)

    geometry = sub.add_parser(
        "geometry", help="realize Clifford generators on five soft strings"
    )
    geometry.add_argument("--ambient-dim", type=int, default=4)
    geometry.add_argument("--kernel-width", type=float, default=0.28)
    geometry.add_argument("--trials", type=int, default=2048)
    geometry.add_argument("--seed", type=int, default=31)
    geometry.add_argument("--output", type=Path)

    geometry_sweep = sub.add_parser(
        "geometry-sweep", help="sweep ambient dimension and contact width"
    )
    geometry_sweep.add_argument("--trials", type=int, default=512)
    geometry_sweep.add_argument("--output", type=Path)

    closed = sub.add_parser(
        "train-closed", help="train exactly closed token gestures"
    )
    closed.add_argument("--steps", type=int, default=600)
    closed.add_argument("--fibers", type=int, default=1)
    closed.add_argument("--segments", type=int, default=6)
    closed.add_argument("--seed", type=int, default=101)
    closed.add_argument("--output-dir", type=Path)
    closed.add_argument("--control-output", type=Path)

    closed_sweep = sub.add_parser(
        "closed-sweep", help="repeat closed-gesture fitting over seeds/fibers"
    )
    closed_sweep.add_argument("--steps", type=int, default=600)
    closed_sweep.add_argument("--output", type=Path)

    morphology = sub.add_parser(
        "morphology", help="inverse-design five grade-string geometries"
    )
    morphology.add_argument("--ambient-dim", type=int, default=4)
    morphology.add_argument("--kernel-width", type=float, default=0.50)
    morphology.add_argument("--steps", type=int, default=200)
    morphology.add_argument("--seed", type=int, default=41)
    morphology.add_argument("--output-dir", type=Path)

    active = sub.add_parser(
        "active-control", help="run the active-cable negative control"
    )
    active.add_argument("--steps", type=int, default=250)
    active.add_argument("--seed", type=int, default=7)
    active.add_argument("--output-dir", type=Path)

    return root


def main() -> None:
    # The reference receipts and tolerances are defined in binary64. Console
    # entry points do not execute ``__main__``, so establish the dtype here.
    torch.set_default_dtype(torch.float64)
    args = parser().parse_args()
    if args.command == "exact":
        emit(
            exact_receipt(
                CliffordConfig(random_trials=args.trials, seed=args.seed)
            ),
            args.output,
        )
    elif args.command == "development":
        emit(development_receipt(DevelopmentConfig()), args.output)
    elif args.command == "geometry":
        emit(
            geometry_receipt(
                GeometryConfig(
                    ambient_dim=args.ambient_dim,
                    kernel_width=args.kernel_width,
                    random_trials=args.trials,
                    seed=args.seed,
                )
            ),
            args.output,
        )
    elif args.command == "geometry-sweep":
        emit(dimension_sweep(random_trials=args.trials), args.output)
    elif args.command == "train-closed":
        model, result = train_closed_gestures(
            ClosedGestureConfig(
                train_steps=args.steps,
                fibers=args.fibers,
                segments=args.segments,
                seed=args.seed,
            ),
            output_dir=args.output_dir,
        )
        print(json.dumps(result, indent=2, sort_keys=True))
        if args.control_output is not None:
            emit(commuting_control(model), args.control_output)
    elif args.command == "closed-sweep":
        emit(seed_sweep(train_steps=args.steps), args.output)
    elif args.command == "morphology":
        _, result = optimize_morphology(
            MorphologyConfig(
                ambient_dim=args.ambient_dim,
                kernel_width=args.kernel_width,
                train_steps=args.steps,
                seed=args.seed,
            ),
            output_dir=args.output_dir,
        )
        print(json.dumps(result, indent=2, sort_keys=True))
    elif args.command == "active-control":
        result = train_active_cable(
            ActiveCableConfig(
                seed=args.seed,
                string_count=4,
                nodes_per_string=8,
                channels=4,
                reaction_rank=8,
                microsteps_per_token=3,
                read_paths=1,
                read_steps=4,
                contact_neighbors=3,
                dt=0.06,
                train_steps=args.steps,
                learning_rate=0.01,
                trace_every=25,
            ),
            args.output_dir,
        )
        print(json.dumps(result, indent=2, sort_keys=True))
    else:  # pragma: no cover - argparse enforces choices
        raise AssertionError(args.command)


if __name__ == "__main__":
    main()
