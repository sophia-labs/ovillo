from __future__ import annotations

from dataclasses import dataclass

import torch
from torch import Tensor

from .config import RendererConfig
from .features import make_analysis_bundle
from .renderer import ToyGargantaRenderer


@dataclass
class EpisodeBatch:
    """One minibatch of voice episodes.

    A row is one voice.  ``K`` utterances share exactly one anatomy target but
    have independent local and nuisance programs.  ``latent_supervision`` is
    the seam between exact simulator examples and audio-bundle-only examples.
    """

    audio: Tensor  # [B, K, T]
    analysis: Tensor  # [B, K, D]
    analysis_confidence: Tensor  # [B, K, D]
    anatomy_target: Tensor  # [B, A], retained for evaluation even when hidden
    local_target: Tensor  # [B, K, F, L]
    nuisance_target: Tensor  # [B, K, N]
    latent_supervision: Tensor  # [B], 1 for exact simulator examples
    contrast_pair_mask: Tensor  # [B], first two utterances share gestures

    def to(self, device: torch.device | str) -> "EpisodeBatch":
        return EpisodeBatch(**{name: value.to(device) for name, value in vars(self).items()})


class ToyCorpus:
    def __init__(self, renderer: ToyGargantaRenderer, *, seed: int = 7) -> None:
        self.renderer = renderer
        self.config: RendererConfig = renderer.config
        self.generator = torch.Generator(device="cpu")
        self.generator.manual_seed(seed)

    def _randn(self, *shape: int) -> Tensor:
        return torch.randn(shape, generator=self.generator)

    def _smooth_trajectory(self, batch: int, utterances: int, channels: int) -> Tensor:
        cfg = self.config
        start = 0.65 * self._randn(batch, utterances, 1, channels)
        end = 0.65 * self._randn(batch, utterances, 1, channels)
        alpha = torch.linspace(0.0, 1.0, cfg.num_frames).view(1, 1, cfg.num_frames, 1)
        trajectory = (1.0 - alpha) * start + alpha * end
        phase = self._randn(batch, utterances, 1, channels)
        oscillation = 0.18 * torch.sin(2.0 * torch.pi * alpha + phase)
        return trajectory + oscillation

    def sample(
        self,
        batch_size: int,
        utterances_per_voice: int,
        *,
        supervised_fraction: float = 0.75,
        missing_probability: float = 0.08,
    ) -> EpisodeBatch:
        cfg = self.config
        anatomy = 0.8 * self._randn(batch_size, cfg.anatomy_dim).clamp(-1.5, 1.5)
        gestures = self._smooth_trajectory(batch_size, utterances_per_voice, cfg.gesture_dim)
        source = self._smooth_trajectory(batch_size, utterances_per_voice, cfg.source_dim)

        # The first pair is a controlled intervention: same articulation, a
        # changed source program.  It gives the inverse model an explicit
        # static/dynamic disentanglement test.
        contrast_pair_mask = torch.zeros(batch_size)
        if utterances_per_voice >= 2:
            gestures[:, 1] = gestures[:, 0]
            source[:, 1] = source[:, 0]
            source[:, 1, :, 0] += 0.85  # higher F0
            source[:, 1, :, 3] += 1.10  # more aspiration
            contrast_pair_mask.fill_(1.0)

        local = torch.cat([gestures, source], dim=-1)
        nuisance = 0.55 * self._randn(batch_size, utterances_per_voice, cfg.nuisance_dim)
        expanded_anatomy = anatomy[:, None, :].expand(-1, utterances_per_voice, -1)
        audio = self.renderer(expanded_anatomy, local, nuisance)
        f0 = self.renderer.physical_source_controls(source).f0_hz
        analysis, confidence = make_analysis_bundle(
            audio,
            f0,
            cfg,
            missing_probability=missing_probability,
            generator=self.generator,
        )

        latent_supervision = (
            torch.rand(batch_size, generator=self.generator) < supervised_fraction
        ).to(torch.float32)
        return EpisodeBatch(
            audio=audio.detach(),
            analysis=analysis.detach(),
            analysis_confidence=confidence.detach(),
            anatomy_target=anatomy.detach(),
            local_target=local.detach(),
            nuisance_target=nuisance.detach(),
            latent_supervision=latent_supervision,
            contrast_pair_mask=contrast_pair_mask,
        )
