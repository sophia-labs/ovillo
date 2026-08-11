from __future__ import annotations

import argparse
import json
import random
import time
from pathlib import Path

import numpy as np
import torch
from torch import Tensor

from .config import ModelConfig, RendererConfig, TrainConfig
from .data import EpisodeBatch, ToyCorpus
from .losses import compute_loss
from .model import ThreaderPOC
from .receipt import save_wav, tensor_digest, write_receipt
from .refine import refine_episode
from .renderer import ToyGargantaRenderer


def set_seed(seed: int, *, cpu_threads: int = 1) -> None:
    random.seed(seed)
    np.random.seed(seed)
    torch.manual_seed(seed)
    # These tensors are tiny; BLAS/OpenMP fan-out is dramatically slower than
    # one thread on the toy workload.  Production should re-benchmark this per
    # worker shape rather than inherit a workstation-wide default.
    torch.set_num_threads(max(1, cpu_threads))
    try:
        torch.set_num_interop_threads(max(1, cpu_threads))
    except RuntimeError:
        # Safe when an embedding application has already initialized the pool.
        pass
    torch.use_deterministic_algorithms(True)


def _snr_db(prediction: Tensor, target: Tensor) -> Tensor:
    signal = target.square().mean()
    noise = (prediction - target).square().mean().clamp_min(1.0e-9)
    return 10.0 * torch.log10(signal.clamp_min(1.0e-9) / noise)


def evaluate(model: ThreaderPOC, batch: EpisodeBatch) -> dict[str, float]:
    model.eval()
    with torch.no_grad():
        result = model(batch.audio, batch.analysis, stochastic=False)
        losses = compute_loss(result, batch, model.renderer, model.renderer_config)
        anatomy_rmse = (result.posterior.anatomy_mean - batch.anatomy_target).square().mean().sqrt()
        local_rmse = (result.posterior.local_mean - batch.local_target).square().mean().sqrt()
        nuisance_rmse = (result.posterior.nuisance_mean - batch.nuisance_target).square().mean().sqrt()
        anatomy_std = result.posterior.anatomy_log_std.exp()
        anatomy_90_coverage = (
            (batch.anatomy_target >= result.posterior.anatomy_mean - 1.645 * anatomy_std)
            & (batch.anatomy_target <= result.posterior.anatomy_mean + 1.645 * anatomy_std)
        ).to(torch.float32).mean()
        snr = _snr_db(result.reconstruction, batch.audio)
    metrics = losses.detached()
    metrics.update(
        anatomy_rmse=float(anatomy_rmse),
        local_rmse=float(local_rmse),
        nuisance_rmse=float(nuisance_rmse),
        anatomy_posterior_std_mean=float(anatomy_std.mean()),
        anatomy_90_coverage=float(anatomy_90_coverage),
        reconstruction_snr_db=float(snr),
    )
    return metrics


def run_toy(
    output_dir: Path,
    renderer_config: RendererConfig,
    model_config: ModelConfig,
    train_config: TrainConfig,
) -> dict[str, object]:
    set_seed(train_config.seed, cpu_threads=train_config.cpu_threads)
    device = torch.device(train_config.device)
    renderer = ToyGargantaRenderer(renderer_config).to(device)
    corpus = ToyCorpus(renderer, seed=train_config.seed + 1)
    model = ThreaderPOC(renderer, model_config).to(device)
    optimizer = torch.optim.AdamW(
        model.parameters(),
        lr=train_config.learning_rate,
        weight_decay=train_config.weight_decay,
    )

    history: list[dict[str, float]] = []
    started = time.perf_counter()
    for step in range(1, train_config.steps + 1):
        model.train()
        batch = corpus.sample(
            train_config.batch_size,
            train_config.utterances_per_voice,
            supervised_fraction=train_config.supervised_fraction,
        ).to(device)
        optimizer.zero_grad(set_to_none=True)
        result = model(
            batch.audio,
            batch.analysis,
            stochastic=True,
            analysis_dropout=train_config.analysis_dropout,
        )
        losses = compute_loss(result, batch, renderer, renderer_config)
        losses.total.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), 5.0)
        optimizer.step()

        if step == 1 or step % max(1, train_config.steps // 6) == 0 or step == train_config.steps:
            record = {"step": float(step), **losses.detached()}
            history.append(record)
            print(
                f"step={step:04d} total={record['total']:.4f} "
                f"latent={record['latent']:.4f} wav={record['waveform']:.4f} "
                f"analysis={record['analysis']:.4f}"
            )

    train_seconds = time.perf_counter() - started
    eval_batch = corpus.sample(
        train_config.eval_batch_size,
        train_config.utterances_per_voice,
        supervised_fraction=1.0,
        missing_probability=0.0,
    ).to(device)
    eval_metrics = evaluate(model, eval_batch)

    # One held-out voice episode becomes a receipt-carrying program solution.
    target = corpus.sample(
        1,
        train_config.utterances_per_voice,
        supervised_fraction=0.0,
        missing_probability=0.0,
    ).to(device)
    model.eval()
    with torch.no_grad():
        posterior = model.infer(target.audio, target.analysis)
        initial_anatomy = posterior.anatomy_mean
        initial_local = posterior.local_mean
        initial_nuisance = posterior.nuisance_mean
        initial_reconstruction = renderer(
            initial_anatomy[:, None, :].expand(-1, target.audio.shape[1], -1),
            initial_local,
            initial_nuisance,
        )

    refinement = refine_episode(
        renderer,
        initial_anatomy,
        initial_local,
        initial_nuisance,
        target.audio,
        target.analysis,
        target.analysis_confidence,
        renderer_config,
        anatomy_log_std=posterior.anatomy_log_std,
        local_log_std=posterior.local_log_std,
        nuisance_log_std=posterior.nuisance_log_std,
        steps=train_config.refinement_steps,
        learning_rate=train_config.refinement_learning_rate,
    )

    output_dir.mkdir(parents=True, exist_ok=True)
    torch.save(
        {
            "model_state": model.state_dict(),
            "renderer_config": renderer_config.to_dict(),
            "model_config": model_config.to_dict(),
            "train_config": train_config.to_dict(),
        },
        output_dir / "threader-poc.pt",
    )
    save_wav(output_dir / "target.wav", target.audio[0, 0], renderer_config.sample_rate)
    save_wav(output_dir / "amortized.wav", initial_reconstruction[0, 0], renderer_config.sample_rate)
    save_wav(output_dir / "refined.wav", refinement.reconstruction[0, 0], renderer_config.sample_rate)

    silence = torch.zeros(renderer_config.sample_rate // 20, device=device)

    def sequence(utterances: Tensor) -> Tensor:
        pieces: list[Tensor] = []
        for index in range(utterances.shape[0]):
            if index:
                pieces.append(silence)
            pieces.append(utterances[index])
        return torch.cat(pieces)

    save_wav(output_dir / "target-sequence.wav", sequence(target.audio[0]), renderer_config.sample_rate)
    save_wav(
        output_dir / "amortized-sequence.wav",
        sequence(initial_reconstruction[0]),
        renderer_config.sample_rate,
    )
    save_wav(
        output_dir / "refined-sequence.wav",
        sequence(refinement.reconstruction[0]),
        renderer_config.sample_rate,
    )

    with torch.no_grad():
        heldout_metrics = {
            "amortized_waveform_l1": float((initial_reconstruction - target.audio).abs().mean()),
            "refined_waveform_l1": float((refinement.reconstruction - target.audio).abs().mean()),
            "amortized_snr_db": float(_snr_db(initial_reconstruction, target.audio)),
            "refined_snr_db": float(_snr_db(refinement.reconstruction, target.audio)),
            "amortized_anatomy_rmse": float(
                (initial_anatomy - target.anatomy_target).square().mean().sqrt()
            ),
            "refined_anatomy_rmse": float(
                (refinement.anatomy - target.anatomy_target).square().mean().sqrt()
            ),
            "amortized_local_rmse": float(
                (initial_local - target.local_target).square().mean().sqrt()
            ),
            "refined_local_rmse": float(
                (refinement.local - target.local_target).square().mean().sqrt()
            ),
        }

    program_solution = {
        "schema": "sophia.threader.program-solution.v0",
        "anatomy": refinement.anatomy[0].cpu().tolist(),
        "utterances": [
            {
                "local_program": refinement.local[0, index].cpu().tolist(),
                "nuisance_program": refinement.nuisance[0, index].cpu().tolist(),
            }
            for index in range(target.audio.shape[1])
        ],
        "fit": {
            "initial_objective": refinement.initial_objective,
            "final_objective": refinement.final_objective,
            "improvement_ratio": refinement.final_objective / refinement.initial_objective,
            **heldout_metrics,
        },
        "toy_ground_truth": {
            "anatomy": target.anatomy_target[0].cpu().tolist(),
            "note": "Present only because this held-out episode was generated by the exact toy renderer.",
        },
    }
    (output_dir / "program-solution.json").write_text(
        json.dumps(program_solution, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    receipt = write_receipt(
        output_dir / "receipt.json",
        {
            "schema": "sophia.threader.toy-run-receipt.v0",
            "renderer_config": renderer_config.to_dict(),
            "model_config": model_config.to_dict(),
            "train_config": train_config.to_dict(),
            "runtime": {
                "torch": torch.__version__,
                "numpy": np.__version__,
                "device": str(device),
                "train_seconds": train_seconds,
            },
            "history": history,
            "evaluation": eval_metrics,
            "refinement": program_solution["fit"],
            "artifacts": [
                vars(tensor_digest("target_audio", target.audio)),
                vars(tensor_digest("amortized_audio", initial_reconstruction)),
                vars(tensor_digest("refined_audio", refinement.reconstruction)),
                vars(tensor_digest("refined_anatomy", refinement.anatomy)),
                vars(tensor_digest("refined_local", refinement.local)),
            ],
        },
    )
    print(json.dumps({"evaluation": eval_metrics, "refinement": program_solution["fit"]}, indent=2))
    return receipt


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=Path("artifacts/toy-run"))
    parser.add_argument("--steps", type=int, default=TrainConfig.steps)
    parser.add_argument("--batch-size", type=int, default=TrainConfig.batch_size)
    parser.add_argument("--utterances", type=int, default=TrainConfig.utterances_per_voice)
    parser.add_argument("--seed", type=int, default=TrainConfig.seed)
    parser.add_argument("--refinement-steps", type=int, default=TrainConfig.refinement_steps)
    parser.add_argument("--device", default=TrainConfig.device)
    parser.add_argument("--cpu-threads", type=int, default=TrainConfig.cpu_threads)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    train_config = TrainConfig(
        seed=args.seed,
        steps=args.steps,
        batch_size=args.batch_size,
        utterances_per_voice=args.utterances,
        refinement_steps=args.refinement_steps,
        device=args.device,
        cpu_threads=args.cpu_threads,
    )
    run_toy(args.output, RendererConfig(), ModelConfig(), train_config)


if __name__ == "__main__":
    main()
