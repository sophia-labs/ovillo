from __future__ import annotations

import argparse
import json
from pathlib import Path

from .experiment import ExperimentConfig, run_campaign, verify_result


def main() -> None:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run")
    run.add_argument("config", type=Path)
    run.add_argument("output", type=Path)
    verify = sub.add_parser("verify")
    verify.add_argument("output", type=Path)
    args = parser.parse_args()
    if args.command == "run":
        config = ExperimentConfig(**json.loads(args.config.read_text()))
        result = run_campaign(config, args.output)
        print(json.dumps(result["decision"], indent=2, sort_keys=True))
    else:
        verify_result(args.output)
        print("verified")

if __name__ == "__main__":
    main()
