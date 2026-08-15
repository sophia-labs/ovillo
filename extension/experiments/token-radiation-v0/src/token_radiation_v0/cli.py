"""Command line interface for Token Radiation v0."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from .experiment import CampaignConfig, run_campaign, verify_output


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="token-radiation-v0")
    subparsers = parser.add_subparsers(dest="command", required=True)

    run = subparsers.add_parser(
        "run", help="run the controlled token-radiation campaign"
    )
    run.add_argument("--output", type=Path, required=True)
    run.add_argument("--seed", type=int, default=20260814)
    run.add_argument("--sequence-bytes", type=int, default=32)
    run.add_argument("--train-sequences", type=int, default=48)
    run.add_argument("--validation-sequences", type=int, default=16)
    run.add_argument("--body-steps", type=int, default=60)
    run.add_argument("--detector-steps", type=int, default=24)
    run.add_argument("--vocabulary-size", type=int, default=18)
    run.add_argument("--cpu-threads", type=int, default=1)
    run.add_argument(
        "--radiation-modes",
        default="passive,nonloading",
        help="comma-separated passive/nonloading conditions",
    )
    run.add_argument(
        "--detectors",
        default="linear,positive_frame",
        help="comma-separated linear/metric/positive_frame detector kinds",
    )

    verify = subparsers.add_parser(
        "verify", help="verify a completed artifact directory"
    )
    verify.add_argument("output", type=Path)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    if args.command == "run":
        config = CampaignConfig(
            seed=args.seed,
            sequence_bytes=args.sequence_bytes,
            train_sequences=args.train_sequences,
            validation_sequences=args.validation_sequences,
            body_steps=args.body_steps,
            detector_steps=args.detector_steps,
            vocabulary_size=args.vocabulary_size,
            cpu_threads=args.cpu_threads,
            radiation_modes=tuple(
                value.strip()
                for value in args.radiation_modes.split(",")
                if value.strip()
            ),
            detector_kinds=tuple(
                value.strip()
                for value in args.detectors.split(",")
                if value.strip()
            ),
        )
        payload = run_campaign(config, args.output)
        print(json.dumps(payload, indent=2, sort_keys=True))
        return 0
    receipt = verify_output(args.output)
    print(json.dumps(receipt, indent=2, sort_keys=True))
    return 0
