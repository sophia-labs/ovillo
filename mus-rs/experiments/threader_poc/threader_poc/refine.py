from __future__ import annotations

from dataclasses import dataclass

import torch
from torch import Tensor
import torch.nn.functional as F

from .config import RendererConfig
from .features import acoustic_features, split_analysis
from .renderer import ToyGargantaRenderer


@dataclass
class RefinementResult:
    anatomy: Tensor
    local: Tensor
    nuisance: Tensor
    reconstruction: Tensor
    initial_objective: float
    final_objective: float
    history: list[float]


def _objective(
    renderer: ToyGargantaRenderer,
    anatomy: Tensor,
    local: Tensor,
    nuisance: Tensor,
    target_audio: Tensor,
    target_analysis: Tensor,
    confidence: Tensor,
    config: RendererConfig,
    anatomy_anchor: Tensor,
    local_anchor: Tensor,
    nuisance_anchor: Tensor,
    anatomy_scale: Tensor,
    local_scale: Tensor,
    nuisance_scale: Tensor,
) -> tuple[Tensor, Tensor]:
    utterances = target_audio.shape[1]
    reconstruction = renderer(anatomy[:, None, :].expand(-1, utterances, -1), local, nuisance)
    waveform = F.smooth_l1_loss(reconstruction, target_audio)
    pred_spectrum = torch.log1p(torch.fft.rfft(reconstruction, n=256).abs())
    target_spectrum = torch.log1p(torch.fft.rfft(target_audio, n=256).abs())
    spectral = F.l1_loss(pred_spectrum, target_spectrum)

    target_acoustic, target_f0 = split_analysis(target_analysis, config)
    confidence_acoustic, confidence_f0 = split_analysis(confidence, config)
    predicted_acoustic = acoustic_features(reconstruction, config)
    acoustic = ((predicted_acoustic - target_acoustic).abs() * confidence_acoustic).sum() / confidence_acoustic.sum().clamp_min(1.0)
    predicted_source = local[..., config.gesture_dim :]
    predicted_f0 = torch.log(renderer.physical_source_controls(predicted_source).f0_hz / 150.0).clamp(-1.5, 1.5)
    f0 = ((predicted_f0 - target_f0).abs() * confidence_f0).sum() / confidence_f0.sum().clamp_min(1.0)

    gestures = local[..., : config.gesture_dim]
    curvature = gestures[..., 2:, :] - 2.0 * gestures[..., 1:-1, :] + gestures[..., :-2, :]
    physical_regularizer = (
        0.012 * anatomy.square().mean()
        + 0.018 * curvature.square().mean()
        + 0.004 * nuisance.square().mean()
    )
    # Refinement is MAP inference, not unconstrained sound matching.  The
    # amortized posterior is the prior that prevents an acoustically equivalent
    # but anatomically implausible solution from wandering arbitrarily.
    posterior_trust = (
        0.055 * ((anatomy - anatomy_anchor) / anatomy_scale).square().mean()
        + 0.012 * ((local - local_anchor) / local_scale).square().mean()
        + 0.012 * ((nuisance - nuisance_anchor) / nuisance_scale).square().mean()
    )
    objective = (
        1.6 * waveform
        + 0.9 * spectral
        + 0.45 * acoustic
        + 0.35 * f0
        + physical_regularizer
        + posterior_trust
    )
    return objective, reconstruction


def refine_episode(
    renderer: ToyGargantaRenderer,
    anatomy_initial: Tensor,
    local_initial: Tensor,
    nuisance_initial: Tensor,
    target_audio: Tensor,
    target_analysis: Tensor,
    confidence: Tensor,
    config: RendererConfig,
    *,
    anatomy_log_std: Tensor | None = None,
    local_log_std: Tensor | None = None,
    nuisance_log_std: Tensor | None = None,
    steps: int = 60,
    learning_rate: float = 3.0e-2,
) -> RefinementResult:
    anatomy = torch.nn.Parameter(anatomy_initial.detach().clone())
    local = torch.nn.Parameter(local_initial.detach().clone())
    nuisance = torch.nn.Parameter(nuisance_initial.detach().clone())
    optimizer = torch.optim.Adam([anatomy, local, nuisance], lr=learning_rate)
    anatomy_anchor = anatomy_initial.detach()
    local_anchor = local_initial.detach()
    nuisance_anchor = nuisance_initial.detach()
    anatomy_scale = (anatomy_log_std.exp() if anatomy_log_std is not None else torch.ones_like(anatomy_anchor)).detach().clamp(0.18, 1.25)
    local_scale = (local_log_std.exp() if local_log_std is not None else torch.ones_like(local_anchor)).detach().clamp(0.22, 1.50)
    nuisance_scale = (nuisance_log_std.exp() if nuisance_log_std is not None else torch.ones_like(nuisance_anchor)).detach().clamp(0.18, 1.25)

    history: list[float] = []
    for _ in range(steps):
        optimizer.zero_grad(set_to_none=True)
        objective, _ = _objective(
            renderer,
            anatomy,
            local,
            nuisance,
            target_audio,
            target_analysis,
            confidence,
            config,
            anatomy_anchor,
            local_anchor,
            nuisance_anchor,
            anatomy_scale,
            local_scale,
            nuisance_scale,
        )
        objective.backward()
        torch.nn.utils.clip_grad_norm_([anatomy, local, nuisance], 5.0)
        optimizer.step()
        history.append(float(objective.detach()))

    with torch.no_grad():
        final_objective, reconstruction = _objective(
            renderer,
            anatomy,
            local,
            nuisance,
            target_audio,
            target_analysis,
            confidence,
            config,
            anatomy_anchor,
            local_anchor,
            nuisance_anchor,
            anatomy_scale,
            local_scale,
            nuisance_scale,
        )
    return RefinementResult(
        anatomy=anatomy.detach(),
        local=local.detach(),
        nuisance=nuisance.detach(),
        reconstruction=reconstruction.detach(),
        initial_objective=history[0],
        final_objective=float(final_objective),
        history=history,
    )
