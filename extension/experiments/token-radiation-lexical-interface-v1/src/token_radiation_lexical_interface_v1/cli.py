"""Command-line entry point."""

from __future__ import annotations

import argparse
from pathlib import Path

from .experiment import (
    ConstructionConfig,
    run_construction,
    verify_result,
    write_result,
)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="token-radiation-lexical-interface-v1")
    subcommands = parser.add_subparsers(dest="command", required=True)

    run = subcommands.add_parser("run", help="run the CPU construction")
    run.add_argument("config", type=Path)
    run.add_argument("output", type=Path)

    verify = subcommands.add_parser("verify", help="verify a result directory")
    verify.add_argument("output", type=Path)

    smoke = subcommands.add_parser("smoke", help="run a tiny deterministic smoke")
    smoke.add_argument("output", type=Path)
    return parser


def main() -> None:
    arguments = _parser().parse_args()
    if arguments.command == "run":
        config = ConstructionConfig.from_json(arguments.config)
        result = run_construction(config)
        write_result(arguments.output, config, result)
        print(result["scientific_digest"])
        return
    if arguments.command == "verify":
        result = verify_result(arguments.output)
        print(result["scientific_digest"])
        return
    if arguments.command == "smoke":
        config = ConstructionConfig(
            seed=17,
            event_count=4,
            train_examples=16,
            calibration_examples=8,
            test_examples=8,
            lexical_steps=2,
            predictor_steps=2,
            interface_steps=2,
            macro_steps=2,
            batch_size=4,
            interface_augmentations=1,
            sites=8,
            trace_window=2,
        )
        result = run_construction(config)
        write_result(arguments.output, config, result)
        print(result["scientific_digest"])
        return
    raise AssertionError("unreachable")
