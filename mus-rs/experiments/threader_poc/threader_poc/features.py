from __future__ import annotations

import torch
from torch import Tensor

from .config import RendererConfig


def acoustic_features(waveform: Tensor, config: RendererConfig) -> Tensor:
    """Cheap differentiable stand-in for the enrichment sidecars.

    Returns flattened frame-level log RMS, zero-crossing rate, normalized
    spectral centroid, and log band energies.  Production replaces these with
    the typed F0, glottal, formant, event, and prosodic tables described in the
    enrichment survey while retaining the same mask/confidence contract.
    """

    frames = waveform.reshape(*waveform.shape[:-1], config.num_frames, config.frame_size)
    log_rms = 0.5 * torch.log(frames.square().mean(dim=-1) + 1.0e-6)
    sign = torch.tanh(20.0 * frames)
    zcr = 0.5 * (sign[..., 1:] - sign[..., :-1]).abs().mean(dim=-1)

    spectrum = torch.fft.rfft(frames)
    power = spectrum.abs().square() + 1.0e-8
    frequencies = torch.linspace(0.0, 1.0, power.shape[-1], device=waveform.device, dtype=waveform.dtype)
    centroid = (power * frequencies).sum(dim=-1) / power.sum(dim=-1)

    edges = torch.linspace(0, power.shape[-1], config.analysis_bands + 1, device=waveform.device)
    bands: list[Tensor] = []
    for index in range(config.analysis_bands):
        lo = int(edges[index].item())
        hi = max(lo + 1, int(edges[index + 1].item()))
        bands.append(torch.log(power[..., lo:hi].mean(dim=-1) + 1.0e-6))
    band_features = torch.stack(bands, dim=-1)

    per_frame = torch.cat(
        [log_rms.unsqueeze(-1), zcr.unsqueeze(-1), centroid.unsqueeze(-1), band_features],
        dim=-1,
    )
    return per_frame.flatten(start_dim=-2)


def make_analysis_bundle(
    waveform: Tensor,
    f0_hz: Tensor,
    config: RendererConfig,
    *,
    missing_probability: float = 0.08,
    generator: torch.Generator | None = None,
) -> tuple[Tensor, Tensor]:
    """Build observations and confidence weights.

    ``f0_hz`` represents generation-side capture or a trusted F0 oracle.  The
    random mask demonstrates that missing labels remain missing rather than
    becoming numerical zeroes.
    """

    acoustic = acoustic_features(waveform, config)
    normalized_f0 = torch.log(f0_hz / 150.0).clamp(-1.5, 1.5)
    analysis = torch.cat([acoustic, normalized_f0], dim=-1)
    confidence = torch.ones_like(analysis)
    if missing_probability > 0.0:
        random = torch.rand(analysis.shape, device=analysis.device, generator=generator)
        confidence = torch.where(random < missing_probability, torch.zeros_like(confidence), confidence)
    return analysis, confidence


def split_analysis(analysis: Tensor, config: RendererConfig) -> tuple[Tensor, Tensor]:
    return analysis[..., : config.acoustic_analysis_dim], analysis[..., config.acoustic_analysis_dim :]
