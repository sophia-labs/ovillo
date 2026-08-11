from __future__ import annotations

from dataclasses import asdict, dataclass
from typing import Any


@dataclass(frozen=True)
class RendererConfig:
    """Dimensions and numerical envelope for the toy throat.

    The sample rate and utterance length are intentionally tiny so the full
    analysis-by-synthesis loop runs on a laptop CPU.  The latent factorization
    mirrors the intended production system even though the renderer is only a
    differentiable all-pole tube surrogate.
    """

    sample_rate: int = 4_000
    num_frames: int = 8
    frame_size: int = 64
    num_sections: int = 6
    anatomy_dim: int = 3
    gesture_dim: int = 2
    source_dim: int = 4  # f0, pressure, harmonic tilt, aspiration
    nuisance_dim: int = 1  # observation-channel spectral tilt
    harmonic_count: int = 10
    analysis_bands: int = 6

    @property
    def num_samples(self) -> int:
        return self.num_frames * self.frame_size

    @property
    def local_dim(self) -> int:
        return self.gesture_dim + self.source_dim

    @property
    def acoustic_analysis_dim_per_frame(self) -> int:
        # log RMS, zero-crossing rate, centroid, and log band energies.
        return 3 + self.analysis_bands

    @property
    def acoustic_analysis_dim(self) -> int:
        return self.num_frames * self.acoustic_analysis_dim_per_frame

    @property
    def analysis_dim(self) -> int:
        # Acoustic measurements plus a generation/oracle-side F0 tier.
        return self.acoustic_analysis_dim + self.num_frames

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class ModelConfig:
    audio_channels: int = 48
    analysis_hidden: int = 48
    utterance_hidden: int = 96
    posterior_hidden: int = 128
    log_std_min: float = -5.0
    log_std_max: float = 1.5

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class TrainConfig:
    seed: int = 7
    steps: int = 300
    batch_size: int = 16
    utterances_per_voice: int = 4
    learning_rate: float = 2.0e-3
    weight_decay: float = 1.0e-5
    supervised_fraction: float = 0.75
    analysis_dropout: float = 0.15
    refinement_steps: int = 60
    refinement_learning_rate: float = 3.0e-2
    eval_batch_size: int = 32
    device: str = "cpu"
    cpu_threads: int = 1

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)
