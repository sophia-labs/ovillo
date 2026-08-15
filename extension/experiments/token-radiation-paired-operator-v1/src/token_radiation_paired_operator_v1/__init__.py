"""Paired loading and operator-vocabulary experiments for Token Radiation."""

from .campaign import CampaignConfig, run_campaign, verify_output
from .model import BodyConfig, BytePredictionModel, RadiatingByteBody
from .paired import PairedConfig, train_paired_condition
from .radiation import metric_radiate, orthonormal_rows

__all__ = [
    "BodyConfig",
    "BytePredictionModel",
    "CampaignConfig",
    "PairedConfig",
    "RadiatingByteBody",
    "metric_radiate",
    "orthonormal_rows",
    "run_campaign",
    "train_paired_condition",
    "verify_output",
]
