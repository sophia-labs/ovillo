from __future__ import annotations

from dataclasses import dataclass

import torch
from torch import Tensor


@dataclass
class ProgramPosterior:
    """Diagonal-Gaussian posterior over one shared and several local programs."""

    anatomy_mean: Tensor  # [B, A]
    anatomy_log_std: Tensor  # [B, A]
    local_mean: Tensor  # [B, K, F, L]
    local_log_std: Tensor  # [B, K, F, L]
    nuisance_mean: Tensor  # [B, K, N]
    nuisance_log_std: Tensor  # [B, K, N]
    per_utterance_anatomy_mean: Tensor  # [B, K, A]

    def sample(self, *, stochastic: bool = True) -> tuple[Tensor, Tensor, Tensor]:
        if stochastic:
            anatomy = self.anatomy_mean + torch.randn_like(self.anatomy_mean) * self.anatomy_log_std.exp()
            local = self.local_mean + torch.randn_like(self.local_mean) * self.local_log_std.exp()
            nuisance = self.nuisance_mean + torch.randn_like(self.nuisance_mean) * self.nuisance_log_std.exp()
        else:
            anatomy = self.anatomy_mean
            local = self.local_mean
            nuisance = self.nuisance_mean
        return anatomy, local, nuisance


@dataclass
class ForwardResult:
    posterior: ProgramPosterior
    anatomy: Tensor
    local: Tensor
    nuisance: Tensor
    reconstruction: Tensor
