from __future__ import annotations

import torch
from torch import Tensor, nn
import torch.nn.functional as F

from .config import ModelConfig, RendererConfig
from .renderer import ToyGargantaRenderer
from .types import ForwardResult, ProgramPosterior


class ThreaderPOC(nn.Module):
    """Amortized posterior over shared anatomy and local performance programs."""

    def __init__(
        self,
        renderer: ToyGargantaRenderer,
        model_config: ModelConfig | None = None,
    ) -> None:
        super().__init__()
        self.renderer = renderer
        self.renderer_config: RendererConfig = renderer.config
        self.model_config = model_config or ModelConfig()
        cfg = self.renderer_config
        mc = self.model_config

        self.audio_encoder = nn.Sequential(
            nn.Conv1d(1, 16, kernel_size=9, stride=2, padding=4),
            nn.GELU(),
            nn.Conv1d(16, 32, kernel_size=7, stride=2, padding=3),
            nn.GELU(),
            nn.Conv1d(32, mc.audio_channels, kernel_size=5, stride=2, padding=2),
            nn.GELU(),
        )
        self.analysis_encoder = nn.Sequential(
            nn.Linear(cfg.analysis_dim, mc.analysis_hidden),
            nn.LayerNorm(mc.analysis_hidden),
            nn.GELU(),
            nn.Linear(mc.analysis_hidden, mc.analysis_hidden),
            nn.GELU(),
        )
        utterance_input = mc.audio_channels + mc.analysis_hidden
        self.utterance_encoder = nn.Sequential(
            nn.Linear(utterance_input, mc.utterance_hidden),
            nn.LayerNorm(mc.utterance_hidden),
            nn.GELU(),
        )

        self.per_utterance_anatomy = nn.Linear(mc.utterance_hidden, cfg.anatomy_dim)
        self.anatomy_head = nn.Sequential(
            nn.Linear(2 * mc.utterance_hidden, mc.posterior_hidden),
            nn.GELU(),
            nn.Linear(mc.posterior_hidden, 2 * cfg.anatomy_dim),
        )

        frame_input = mc.audio_channels + mc.analysis_hidden + mc.utterance_hidden + cfg.anatomy_dim
        self.local_head = nn.Sequential(
            nn.Linear(frame_input, mc.posterior_hidden),
            nn.GELU(),
            nn.Linear(mc.posterior_hidden, 2 * cfg.local_dim),
        )
        nuisance_input = mc.utterance_hidden + cfg.anatomy_dim
        self.nuisance_head = nn.Sequential(
            nn.Linear(nuisance_input, mc.posterior_hidden // 2),
            nn.GELU(),
            nn.Linear(mc.posterior_hidden // 2, 2 * cfg.nuisance_dim),
        )

    def _bounded_log_std(self, raw: Tensor) -> Tensor:
        lo = self.model_config.log_std_min
        hi = self.model_config.log_std_max
        return lo + (hi - lo) * torch.sigmoid(raw)

    def infer(
        self,
        audio: Tensor,
        analysis: Tensor,
        *,
        analysis_dropout: float = 0.0,
    ) -> ProgramPosterior:
        cfg = self.renderer_config
        if audio.ndim != 3:
            raise ValueError("audio must have shape [batch, utterance, sample]")
        batch, utterances, samples = audio.shape
        if samples != cfg.num_samples:
            raise ValueError(f"expected {cfg.num_samples} samples")
        if analysis.shape != (batch, utterances, cfg.analysis_dim):
            raise ValueError("analysis shape does not match audio")

        flat_audio = audio.reshape(batch * utterances, 1, samples)
        encoded = self.audio_encoder(flat_audio)
        frame_audio = F.adaptive_avg_pool1d(encoded, cfg.num_frames).transpose(1, 2)
        audio_summary = frame_audio.mean(dim=1)

        flat_analysis = analysis.reshape(batch * utterances, cfg.analysis_dim)
        if self.training and analysis_dropout > 0.0:
            keep = torch.rand(batch * utterances, 1, device=analysis.device) >= analysis_dropout
            flat_analysis = flat_analysis * keep
        analysis_summary = self.analysis_encoder(flat_analysis)

        utterance_summary = self.utterance_encoder(torch.cat([audio_summary, analysis_summary], dim=-1))
        utterance_summary = utterance_summary.reshape(batch, utterances, -1)
        per_utterance_anatomy_mean = self.per_utterance_anatomy(utterance_summary)

        mean = utterance_summary.mean(dim=1)
        std = utterance_summary.std(dim=1, unbiased=False)
        anatomy_stats = self.anatomy_head(torch.cat([mean, std], dim=-1))
        anatomy_mean, anatomy_raw_std = anatomy_stats.chunk(2, dim=-1)
        anatomy_log_std = self._bounded_log_std(anatomy_raw_std)

        frame_audio = frame_audio.reshape(batch, utterances, cfg.num_frames, -1)
        analysis_summary = analysis_summary.reshape(batch, utterances, -1)
        analysis_frames = analysis_summary[:, :, None, :].expand(-1, -1, cfg.num_frames, -1)
        utterance_frames = utterance_summary[:, :, None, :].expand(-1, -1, cfg.num_frames, -1)
        anatomy_frames = anatomy_mean[:, None, None, :].expand(-1, utterances, cfg.num_frames, -1)
        local_stats = self.local_head(
            torch.cat([frame_audio, analysis_frames, utterance_frames, anatomy_frames], dim=-1)
        )
        local_mean, local_raw_std = local_stats.chunk(2, dim=-1)
        local_log_std = self._bounded_log_std(local_raw_std)

        anatomy_utterance = anatomy_mean[:, None, :].expand(-1, utterances, -1)
        nuisance_stats = self.nuisance_head(torch.cat([utterance_summary, anatomy_utterance], dim=-1))
        nuisance_mean, nuisance_raw_std = nuisance_stats.chunk(2, dim=-1)
        nuisance_log_std = self._bounded_log_std(nuisance_raw_std)

        return ProgramPosterior(
            anatomy_mean=anatomy_mean,
            anatomy_log_std=anatomy_log_std,
            local_mean=local_mean,
            local_log_std=local_log_std,
            nuisance_mean=nuisance_mean,
            nuisance_log_std=nuisance_log_std,
            per_utterance_anatomy_mean=per_utterance_anatomy_mean,
        )

    def forward(
        self,
        audio: Tensor,
        analysis: Tensor,
        *,
        stochastic: bool = True,
        analysis_dropout: float = 0.0,
    ) -> ForwardResult:
        posterior = self.infer(audio, analysis, analysis_dropout=analysis_dropout)
        anatomy, local, nuisance = posterior.sample(stochastic=stochastic)
        utterances = audio.shape[1]
        reconstruction = self.renderer(anatomy[:, None, :].expand(-1, utterances, -1), local, nuisance)
        return ForwardResult(posterior, anatomy, local, nuisance, reconstruction)
