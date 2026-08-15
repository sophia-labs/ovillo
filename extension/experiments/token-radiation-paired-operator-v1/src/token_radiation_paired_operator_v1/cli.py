"""Command-line interface for the CPU research tranche."""

from __future__ import annotations

import argparse
from dataclasses import fields
import json
from pathlib import Path
from typing import Any

from .campaign import CampaignConfig, run_campaign, verify_output


def _config_from_json(path: Path) -> CampaignConfig:
    raw = json.loads(path.read_text())
    valid = {field.name for field in fields(CampaignConfig)}
    unknown = sorted(set(raw) - valid)
    if unknown:
        raise ValueError(f"unknown campaign config keys: {unknown}")
    for key in ("radiation_fractions", "paired_seeds", "operator_families"):
        if key in raw:
            raw[key] = tuple(raw[key])
    return CampaignConfig(**raw)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="token-radiation-paired-operator-v1")
    subparsers = parser.add_subparsers(dest="command", required=True)
    run = subparsers.add_parser("run", help="run a deterministic CPU campaign")
    run.add_argument("--config", type=Path, required=True)
    run.add_argument("--output", type=Path, required=True)
    verify = subparsers.add_parser("verify", help="verify a campaign result")
    verify.add_argument("output", type=Path)
    return parser


def main(argv: list[str] | None = None) -> None:
    args = build_parser().parse_args(argv)
    result: dict[str, Any]
    if args.command == "run":
        summary = run_campaign(_config_from_json(args.config), args.output)
        result = {
            "scientific_sha256": summary["sha256"],
            "paired_rows": len(summary["paired_loading"]["rows"]),
            "detector_rows": len(summary["operator_vocabulary"]["detector_rows"]),
            "output": str(args.output),
        }
    else:
        result = verify_output(args.output)
    print(json.dumps(result, indent=2, sort_keys=True))
