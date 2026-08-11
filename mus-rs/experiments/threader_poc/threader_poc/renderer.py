from __future__ import annotations

import math
from dataclasses import dataclass

import torch
from torch import Tensor, nn
import torch.nn.functional as F

from .config import RendererConfig


@dataclass(frozen=True)
class PhysicalControls:
    f0_hz: Tensor
    pressure: Tensor
    harmonic_tilt: Tensor
    aspiration: Tensor


class ToyGargantaRenderer(nn.Module):
    """Differentiable miniature of a source/filter pseudobiological throat.

    It is not claimed to be a production vocal-tract solver.  It does preserve
    the important model contract:

    * anatomy is static across an episode;
    * gesture controls deform a positive section-area profile over time;
    * area ratios produce stable reflection coefficients;
    * those coefficients define a differentiable all-pole tract response;
    * glottal controls generate a voiced/aspirated source;
    * a separate nuisance transform models observation-channel coloration.

    Each control frame is filtered in the frequency domain and overlap-free
    concatenated.  That approximation is the explicit seam to replace with the
    real GARGANTA render face.
    """

    def __init__(self, config: RendererConfig) -> None:
        super().__init__()
        self.config = config

        mean_area = torch.tensor([0.65, 1.15, 1.90, 1.55, 1.05, 0.72], dtype=torch.float32)
        anatomy_basis = torch.tensor(
            [
                [-0.55, -0.35, -0.10, 0.20, 0.45, 0.70],
                [0.15, -0.70, -0.78, 0.25, 0.55, 0.18],
                [0.05, 0.08, 0.18, 0.28, -0.35, -0.82],
            ],
            dtype=torch.float32,
        )
        gesture_basis = torch.tensor(
            [
                [0.50, 0.20, -0.42, -0.62, 0.18, 0.42],
                [-0.28, 0.48, 0.70, 0.20, -0.42, -0.22],
            ],
            dtype=torch.float32,
        )
        if mean_area.numel() != config.num_sections:
            raise ValueError("the built-in toy profile expects six sections")
        if anatomy_basis.shape != (config.anatomy_dim, config.num_sections):
            raise ValueError("anatomy basis does not match renderer dimensions")
        if gesture_basis.shape != (config.gesture_dim, config.num_sections):
            raise ValueError("gesture basis does not match renderer dimensions")

        self.register_buffer("mean_log_area", mean_area.log())
        self.register_buffer("anatomy_basis", anatomy_basis)
        self.register_buffer("gesture_basis", gesture_basis)

        sample_index = torch.arange(config.num_samples, dtype=torch.float32)
        t = sample_index / float(config.sample_rate)
        turbulence = (
            torch.sin(2.0 * math.pi * 877.0 * t + 0.13)
            + 0.65 * torch.sin(2.0 * math.pi * 1_233.0 * t + 1.17)
            + 0.35 * torch.sin(2.0 * math.pi * 1_619.0 * t + 2.03)
        )
        turbulence = turbulence / turbulence.square().mean().sqrt()
        self.register_buffer("turbulence_carrier", turbulence)

    def physical_source_controls(self, source_raw: Tensor) -> PhysicalControls:
        if source_raw.shape[-1] != self.config.source_dim:
            raise ValueError(f"expected source dim {self.config.source_dim}, got {source_raw.shape[-1]}")
        f0_hz = 85.0 + 235.0 * torch.sigmoid(source_raw[..., 0])
        pressure = 0.12 + 0.88 * torch.sigmoid(source_raw[..., 1])
        harmonic_tilt = 0.75 + 2.15 * torch.sigmoid(source_raw[..., 2])
        aspiration = 0.02 + 0.33 * torch.sigmoid(source_raw[..., 3])
        return PhysicalControls(f0_hz, pressure, harmonic_tilt, aspiration)

    def area_profiles(self, anatomy: Tensor, gestures: Tensor) -> Tensor:
        """Return positive section areas with shape ``[..., frames, sections]``."""
        anatomy_shape = anatomy.shape
        if anatomy_shape[-1] != self.config.anatomy_dim:
            raise ValueError("bad anatomy dimension")
        if gestures.shape[-2:] != (self.config.num_frames, self.config.gesture_dim):
            raise ValueError("bad gesture shape")

        static = self.mean_log_area + torch.einsum("...a,as->...s", anatomy, self.anatomy_basis)
        dynamic = torch.einsum("...fg,gs->...fs", gestures, self.gesture_basis)
        log_area = static.unsqueeze(-2) + dynamic
        # The clamp is a declared toy physical envelope: 0.12–6.0 relative area.
        return torch.exp(log_area.clamp(math.log(0.12), math.log(6.0)))

    @staticmethod
    def reflection_to_lpc(reflection: Tensor) -> Tensor:
        """Step-up recursion from stable PARCOR coefficients to LPC terms."""
        order = reflection.shape[-1]
        coefficients = torch.zeros(*reflection.shape[:-1], order + 1, dtype=reflection.dtype, device=reflection.device)
        coefficients[..., 0] = 1.0
        for m in range(order):
            previous = coefficients.clone()
            k = reflection[..., m]
            coefficients[..., m + 1] = k
            if m:
                left = previous[..., 1 : m + 1]
                right = previous[..., 1 : m + 1].flip(-1)
                coefficients[..., 1 : m + 1] = left + k.unsqueeze(-1) * right
        return coefficients

    def tract_response(self, area: Tensor, n_fft: int) -> Tensor:
        reflection = (area[..., :-1] - area[..., 1:]) / (area[..., :-1] + area[..., 1:] + 1.0e-6)
        reflection = 0.94 * reflection.clamp(-0.98, 0.98)
        lpc = self.reflection_to_lpc(reflection)

        bins = n_fft // 2 + 1
        omega = 2.0 * math.pi * torch.arange(bins, device=area.device, dtype=area.dtype) / float(n_fft)
        delays = torch.arange(lpc.shape[-1], device=area.device, dtype=area.dtype)
        phase = torch.exp(-1j * omega[:, None] * delays[None, :])
        denominator = torch.einsum("...p,fp->...f", lpc.to(torch.complex64), phase.to(torch.complex64))
        response = 1.0 / (denominator + 1.0e-4)
        # Normalize energy, retaining spectral shape but preventing a near-pole
        # from dominating the toy optimization numerically.
        response = response / response.abs().square().mean(dim=-1, keepdim=True).sqrt().clamp_min(1.0e-4)
        return response

    def glottal_source(self, source_raw: Tensor) -> Tensor:
        controls = self.physical_source_controls(source_raw)
        cfg = self.config
        f0_samples = controls.f0_hz.repeat_interleave(cfg.frame_size, dim=-1)
        pressure_samples = controls.pressure.repeat_interleave(cfg.frame_size, dim=-1)
        tilt_samples = controls.harmonic_tilt.repeat_interleave(cfg.frame_size, dim=-1)
        aspiration_samples = controls.aspiration.repeat_interleave(cfg.frame_size, dim=-1)

        phase = 2.0 * math.pi * torch.cumsum(f0_samples / float(cfg.sample_rate), dim=-1)
        harmonic_index = torch.arange(1, cfg.harmonic_count + 1, device=phase.device, dtype=phase.dtype)
        harmonic_phase = phase.unsqueeze(-2) * harmonic_index.view(*([1] * (phase.ndim - 1)), -1, 1)
        harmonic_gain = harmonic_index.view(*([1] * (phase.ndim - 1)), -1, 1).pow(-tilt_samples.unsqueeze(-2))
        voiced = (harmonic_gain * torch.sin(harmonic_phase)).sum(dim=-2)
        voiced = voiced / harmonic_gain.sum(dim=-2).clamp_min(1.0e-4)

        turbulence = self.turbulence_carrier.view(*([1] * (phase.ndim - 1)), -1)
        source = pressure_samples * (voiced + aspiration_samples * turbulence)
        return source

    def apply_channel(self, waveform: Tensor, nuisance_raw: Tensor) -> Tensor:
        cfg = self.config
        if nuisance_raw.shape[-1] != cfg.nuisance_dim:
            raise ValueError("bad nuisance dimension")
        n_fft = 2 * cfg.num_samples
        spectrum = torch.fft.rfft(waveform, n=n_fft)
        bins = spectrum.shape[-1]
        normalized_frequency = torch.linspace(0.04, 1.0, bins, device=waveform.device, dtype=waveform.dtype)
        tilt = 0.85 * torch.tanh(nuisance_raw[..., 0])
        gain = normalized_frequency.view(*([1] * tilt.ndim), bins).pow(tilt.unsqueeze(-1))
        gain = gain / gain.square().mean(dim=-1, keepdim=True).sqrt().clamp_min(1.0e-4)
        return torch.fft.irfft(spectrum * gain, n=n_fft)[..., : cfg.num_samples]

    def forward(self, anatomy: Tensor, local_raw: Tensor, nuisance_raw: Tensor) -> Tensor:
        cfg = self.config
        if local_raw.shape[-2:] != (cfg.num_frames, cfg.local_dim):
            raise ValueError(f"expected local shape (..., {cfg.num_frames}, {cfg.local_dim})")
        gestures = local_raw[..., : cfg.gesture_dim]
        source_raw = local_raw[..., cfg.gesture_dim :]

        area = self.area_profiles(anatomy, gestures)
        source = self.glottal_source(source_raw)
        frames = source.reshape(*source.shape[:-1], cfg.num_frames, cfg.frame_size)
        n_fft = 2 * cfg.frame_size
        response = self.tract_response(area, n_fft)
        filtered = torch.fft.irfft(torch.fft.rfft(frames, n=n_fft) * response, n=n_fft)[..., : cfg.frame_size]
        waveform = filtered.reshape(*filtered.shape[:-2], cfg.num_samples)
        waveform = self.apply_channel(waveform, nuisance_raw)
        # Bounded observation output, not an internal physical-state clamp.
        return 0.72 * torch.tanh(waveform)
