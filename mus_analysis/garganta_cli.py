"""Command-line entry point for speaker-specific GARGANTA evolution."""

from __future__ import annotations

import argparse
from hashlib import sha256
import json
from pathlib import Path
import sys

import numpy as np

try:
    import soundfile as sf
except ImportError as exc:  # pragma: no cover
    raise ImportError(
        "garganta-evolve requires soundfile; install the project 'audio' extra"
    ) from exc

from .garganta_anatomy import EvolutionConfig, fit_speaker_throat, render_throat_atlas


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="garganta-evolve",
        description=(
            "Evolve a posterior family of speaker-conditioned acoustic throats "
            "from one consented reference recording."
        ),
    )
    parser.add_argument("reference", type=Path, help="reference WAV/FLAC/AIFF")
    parser.add_argument(
        "--out",
        type=Path,
        default=Path("garganta-speaker-throat"),
        help="output directory",
    )
    parser.add_argument(
        "--consent-attestation",
        default=None,
        help=(
            "free-text consent/provenance statement recorded in the receipt; "
            "omitting it marks the artifact non-distributable"
        ),
    )
    parser.add_argument("--seed", type=int, default=EvolutionConfig.seed)
    parser.add_argument("--generations", type=int, default=EvolutionConfig.generations)
    parser.add_argument("--population", type=int, default=EvolutionConfig.population)
    parser.add_argument(
        "--control-bank", type=int, default=EvolutionConfig.control_bank_size
    )
    parser.add_argument(
        "--states", type=int, default=EvolutionConfig.representative_states
    )
    parser.add_argument(
        "--sections", type=int, default=EvolutionConfig.section_count
    )
    parser.add_argument(
        "--no-polish", action="store_true", help="skip local Powell/control polish"
    )
    parser.add_argument(
        "--no-preview", action="store_true", help="do not write diagnostic atlas WAV"
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    if not args.reference.exists():
        print(f"reference does not exist: {args.reference}", file=sys.stderr)
        return 2

    raw_bytes = args.reference.read_bytes()
    reference_digest = sha256(raw_bytes).hexdigest()
    audio, sample_rate = sf.read(args.reference, always_2d=False)
    audio = np.asarray(audio, dtype=np.float64)
    config = EvolutionConfig(
        seed=args.seed,
        generations=args.generations,
        population=args.population,
        control_bank_size=args.control_bank,
        representative_states=args.states,
        section_count=args.sections,
        local_polish=not args.no_polish,
    )

    args.out.mkdir(parents=True, exist_ok=True)
    progress_path = args.out / "evolution-progress.jsonl"
    progress_file = progress_path.open("w", encoding="utf-8")

    def progress(generation, score) -> None:
        record = {
            "generation": generation,
            "objective_loss": score.objective_loss,
            "training_loss": score.training_loss,
            "holdout_loss": score.holdout_loss,
            "tract_length_cm": score.anatomy.tract_length_cm,
        }
        progress_file.write(json.dumps(record, sort_keys=True) + "\n")
        progress_file.flush()
        print(
            f"generation {generation:03d}  objective={score.objective_loss:.5f}  "
            f"train={score.training_loss:.5f}  holdout={score.holdout_loss:.5f}  "
            f"L={score.anatomy.tract_length_cm:.2f}cm",
            flush=True,
        )

    try:
        fit = fit_speaker_throat(
            audio,
            int(sample_rate),
            reference_name=args.reference.name,
            reference_sha256=reference_digest,
            consent_attestation=args.consent_attestation,
            config=config,
            progress=progress,
        )
    finally:
        progress_file.close()

    receipt_path = fit.write_json(args.out / "speaker-throat.json")
    if not args.no_preview:
        preview = render_throat_atlas(fit)
        sf.write(
            args.out / "speaker-throat-atlas.wav",
            preview,
            48_000,
            subtype="PCM_24",
        )

    summary = {
        "receipt": str(receipt_path),
        "reference_sha256": reference_digest,
        "claim_class": fit.to_dict()["claim"]["class"],
        "training_loss": fit.best_training_loss,
        "holdout_loss": fit.best_holdout_loss,
        "posterior_members": len(fit.posterior),
        "identifiability_rank": fit.identifiability_rank,
        "weak_anatomy_directions": list(fit.weak_anatomy_directions),
        "diagnostics": list(fit.diagnostics),
    }
    (args.out / "summary.json").write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
