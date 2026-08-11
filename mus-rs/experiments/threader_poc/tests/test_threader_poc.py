from __future__ import annotations

import math

import torch

torch.set_num_threads(1)

from threader_poc.config import ModelConfig, RendererConfig
from threader_poc.data import ToyCorpus
from threader_poc.losses import compute_loss
from threader_poc.model import ThreaderPOC
from threader_poc.receipt import canonical_digest
from threader_poc.refine import refine_episode
from threader_poc.renderer import ToyGargantaRenderer


def _fixture() -> tuple[RendererConfig, ToyGargantaRenderer, ToyCorpus, ThreaderPOC]:
    torch.manual_seed(3)
    config = RendererConfig()
    renderer = ToyGargantaRenderer(config)
    corpus = ToyCorpus(renderer, seed=4)
    model = ThreaderPOC(
        renderer,
        ModelConfig(
            audio_channels=24,
            analysis_hidden=24,
            utterance_hidden=48,
            posterior_hidden=64,
        ),
    )
    return config, renderer, corpus, model


def test_renderer_is_finite_and_differentiable() -> None:
    config, renderer, corpus, _ = _fixture()
    batch = corpus.sample(2, 3, supervised_fraction=1.0, missing_probability=0.0)
    anatomy = batch.anatomy_target.clone().requires_grad_(True)
    audio = renderer(
        anatomy[:, None, :].expand(-1, 3, -1),
        batch.local_target,
        batch.nuisance_target,
    )
    assert audio.shape == (2, 3, config.num_samples)
    assert torch.isfinite(audio).all()
    audio.square().mean().backward()
    assert anatomy.grad is not None
    assert torch.isfinite(anatomy.grad).all()
    assert anatomy.grad.abs().max() > 0


def test_model_returns_one_anatomy_per_voice() -> None:
    config, _, corpus, model = _fixture()
    batch = corpus.sample(3, 4, supervised_fraction=1.0, missing_probability=0.0)
    posterior = model.infer(batch.audio, batch.analysis)
    assert posterior.anatomy_mean.shape == (3, config.anatomy_dim)
    assert posterior.per_utterance_anatomy_mean.shape == (3, 4, config.anatomy_dim)
    assert posterior.local_mean.shape == (3, 4, config.num_frames, config.local_dim)


def test_short_training_reduces_fixed_batch_loss() -> None:
    _, renderer, corpus, model = _fixture()
    batch = corpus.sample(8, 4, supervised_fraction=1.0, missing_probability=0.0)
    optimizer = torch.optim.Adam(model.parameters(), lr=3.0e-3)
    values: list[float] = []
    for _ in range(30):
        optimizer.zero_grad(set_to_none=True)
        result = model(batch.audio, batch.analysis, stochastic=False)
        loss = compute_loss(result, batch, renderer, renderer.config).total
        loss.backward()
        optimizer.step()
        values.append(float(loss.detach()))
    assert math.isfinite(values[-1])
    assert values[-1] < 0.75 * values[0]


def test_refinement_improves_objective() -> None:
    config, renderer, corpus, model = _fixture()
    batch = corpus.sample(1, 3, supervised_fraction=0.0, missing_probability=0.0)
    with torch.no_grad():
        posterior = model.infer(batch.audio, batch.analysis)
    refined = refine_episode(
        renderer,
        posterior.anatomy_mean,
        posterior.local_mean,
        posterior.nuisance_mean,
        batch.audio,
        batch.analysis,
        batch.analysis_confidence,
        config,
        steps=25,
        learning_rate=4.0e-2,
    )
    assert refined.final_objective < refined.initial_objective


def test_receipt_digest_is_canonical() -> None:
    left = canonical_digest({"a": 1, "b": [2, 3]})
    right = canonical_digest({"b": [2, 3], "a": 1})
    assert left == right
