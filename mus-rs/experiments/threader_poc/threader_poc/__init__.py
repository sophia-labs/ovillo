"""Runnable miniature of the Threader inverse-instrument training loop.

The package deliberately models the architectural seams that matter:

* voice-level anatomy shared across several utterances;
* utterance-local articulatory, glottal, and nuisance programs;
* a differentiable physical-ish source/filter renderer as decoder;
* exact-simulation supervision mixed with bundle-only reconstruction;
* confidence-weighted analysis losses;
* per-voice analysis-by-synthesis refinement and receipts.
"""

from .config import ModelConfig, RendererConfig, TrainConfig
from .data import EpisodeBatch, ToyCorpus
from .model import ThreaderPOC
from .renderer import ToyGargantaRenderer

__all__ = [
    "EpisodeBatch",
    "ModelConfig",
    "RendererConfig",
    "ThreaderPOC",
    "ToyCorpus",
    "ToyGargantaRenderer",
    "TrainConfig",
]
