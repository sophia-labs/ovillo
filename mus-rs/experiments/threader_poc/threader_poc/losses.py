from __future__ import annotations

from dataclasses import dataclass

import torch
from torch import Tensor
import torch.nn.functional as F

from .config import RendererConfig
from .data import EpisodeBatch
from .features import acoustic_features, split_analysis
from .renderer import ToyGargantaRenderer
from .types import ForwardResult


@dataclass
class LossBreakdown:
    total: Tensor
    latent: Tensor
    waveform: Tensor
    spectral: Tensor
    analysis: Tensor
    voice_consistency: Tensor
    contrast: Tensor
    program: Tensor

    def detached(self) -> dict[str, float]:
        return {name: float(value.detach().cpu()) for name, value in vars(self).items()}


def diagonal_gaussian_nll(mean: Tensor, log_std: Tensor, target: Tensor) -> Tensor:
    variance = torch.exp(2.0 * log_std)
    return 0.5 * (((target - mean).square() / variance) + 2.0 * log_std).mean(dim=tuple(range(1, target.ndim)))


def multi_resolution_spectral_loss(prediction: Tensor, target: Tensor) -> Tensor:
    """Log-magnitude loss at frame, two-frame, and utterance scales.

    The first draft used ``rfft(..., n=64)`` on the whole utterance, which
    silently truncated it.  This version explicitly reshapes all samples into
    non-overlapping analysis windows, so every sample contributes while the toy
    remains fast enough for CPU CI.
    """

    samples = prediction.shape[-1]
    if samples % 64 != 0 or samples % 128 != 0:
        raise ValueError("toy spectral loss expects a sample count divisible by 128")
    pairs = (
        (prediction.reshape(*prediction.shape[:-1], samples // 64, 64), target.reshape(*target.shape[:-1], samples // 64, 64)),
        (prediction.reshape(*prediction.shape[:-1], samples // 128, 128), target.reshape(*target.shape[:-1], samples // 128, 128)),
        (prediction.unsqueeze(-2), target.unsqueeze(-2)),
    )
    losses: list[Tensor] = []
    for pred_windows, target_windows in pairs:
        window_size = pred_windows.shape[-1]
        window = torch.hann_window(window_size, device=prediction.device, dtype=prediction.dtype)
        pred = torch.fft.rfft(pred_windows * window, dim=-1)
        truth = torch.fft.rfft(target_windows * window, dim=-1)
        losses.append(F.l1_loss(torch.log1p(pred.abs()), torch.log1p(truth.abs())))
    return torch.stack(losses).mean()


def compute_loss(
    result: ForwardResult,
    batch: EpisodeBatch,
    renderer: ToyGargantaRenderer,
    config: RendererConfig,
) -> LossBreakdown:
    posterior = result.posterior
    supervision = batch.latent_supervision

    anatomy_nll = diagonal_gaussian_nll(
        posterior.anatomy_mean,
        posterior.anatomy_log_std,
        batch.anatomy_target,
    )
    local_nll = diagonal_gaussian_nll(
        posterior.local_mean,
        posterior.local_log_std,
        batch.local_target,
    )
    nuisance_nll = diagonal_gaussian_nll(
        posterior.nuisance_mean,
        posterior.nuisance_log_std,
        batch.nuisance_target,
    )
    latent_per_voice = anatomy_nll + local_nll + nuisance_nll
    latent = (latent_per_voice * supervision).sum() / supervision.sum().clamp_min(1.0)

    waveform = F.smooth_l1_loss(result.reconstruction, batch.audio)
    spectral = multi_resolution_spectral_loss(result.reconstruction, batch.audio)

    target_acoustic, target_f0 = split_analysis(batch.analysis, config)
    confidence_acoustic, confidence_f0 = split_analysis(batch.analysis_confidence, config)
    reconstructed_acoustic = acoustic_features(result.reconstruction, config)
    acoustic_error = F.smooth_l1_loss(reconstructed_acoustic, target_acoustic, reduction="none")
    acoustic_loss = (acoustic_error * confidence_acoustic).sum() / confidence_acoustic.sum().clamp_min(1.0)

    predicted_source = result.local[..., config.gesture_dim :]
    predicted_f0 = torch.log(renderer.physical_source_controls(predicted_source).f0_hz / 150.0).clamp(-1.5, 1.5)
    f0_error = F.smooth_l1_loss(predicted_f0, target_f0, reduction="none")
    f0_loss = (f0_error * confidence_f0).sum() / confidence_f0.sum().clamp_min(1.0)
    analysis = acoustic_loss + 0.7 * f0_loss

    voice_consistency = F.smooth_l1_loss(
        posterior.per_utterance_anatomy_mean,
        posterior.anatomy_mean[:, None, :].expand_as(posterior.per_utterance_anatomy_mean),
    )

    if batch.audio.shape[1] >= 2:
        first_gesture = posterior.local_mean[:, 0, :, : config.gesture_dim]
        second_gesture = posterior.local_mean[:, 1, :, : config.gesture_dim]
        pair_error = (first_gesture - second_gesture).square().mean(dim=(1, 2))
        contrast = (pair_error * batch.contrast_pair_mask).sum() / batch.contrast_pair_mask.sum().clamp_min(1.0)
    else:
        contrast = waveform.new_zeros(())

    gestures = result.local[..., : config.gesture_dim]
    if config.num_frames >= 3:
        curvature = gestures[..., 2:, :] - 2.0 * gestures[..., 1:-1, :] + gestures[..., :-2, :]
        smoothness = curvature.square().mean()
    else:
        smoothness = waveform.new_zeros(())
    program = 0.55 * smoothness + 0.03 * gestures.abs().mean() + 0.02 * result.nuisance.square().mean()

    total = (
        0.75 * latent
        + 1.4 * waveform
        + 0.85 * spectral
        + 0.9 * analysis
        + 0.15 * voice_consistency
        + 0.12 * contrast
        + 0.12 * program
    )
    return LossBreakdown(total, latent, waveform, spectral, analysis, voice_consistency, contrast, program)
